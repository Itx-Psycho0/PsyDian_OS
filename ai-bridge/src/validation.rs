//! Turns the AI service's raw text into an [`Answer`] the kernel accepts.
//!
//! The AI output is untrusted: it must be a JSON object with the expected
//! fields, text is converted to plain printable ASCII, every field is cut to
//! the kernel's limits, command suggestions go through [`crate::safety`], and
//! the final frame must fit into one kernel frame.

use crate::protocol::{self, Answer};
use crate::safety;
use serde_json::Value;

pub const MAX_SUMMARY: usize = 480;
pub const MAX_SUGGESTIONS: usize = 4;
pub const MAX_SUGGESTION: usize = 160;
pub const MAX_COMMANDS: usize = 3;
pub const MAX_COMMAND: usize = 80;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invalid(pub &'static str);

#[derive(Debug, PartialEq)]
pub struct Validated {
    pub answer: Answer,
    /// Command suggestions removed by the safety checks.
    pub dropped_commands: Vec<(String, &'static str)>,
}

pub fn validate(raw: &str) -> Result<Validated, Invalid> {
    let value = extract_json(raw).ok_or(Invalid("the AI answer is not a JSON object"))?;
    let object = value
        .as_object()
        .ok_or(Invalid("the AI answer is not a JSON object"))?;

    let summary = object
        .get("summary")
        .and_then(Value::as_str)
        .map(|s| clip(&to_ascii(s), MAX_SUMMARY))
        .filter(|s| !s.is_empty())
        .ok_or(Invalid("the AI answer has no summary"))?;

    let suggestions = string_list(object.get("suggestions"))?
        .iter()
        .map(|s| clip(&to_ascii(s), MAX_SUGGESTION))
        .filter(|s| !s.is_empty())
        .take(MAX_SUGGESTIONS)
        .collect();

    let commands: Vec<String> = string_list(object.get("commands"))?
        .iter()
        .map(|s| to_ascii(s))
        .filter(|s| s.len() <= MAX_COMMAND)
        .collect();
    let filtered = safety::filter_commands(&commands);

    let asked = match object.get("requiresConfirmation") {
        None | Some(Value::Null) => false,
        Some(Value::Bool(flag)) => *flag,
        Some(_) => return Err(Invalid("requiresConfirmation is not a boolean")),
    };

    let mut answer = Answer {
        summary,
        suggestions,
        commands: filtered.commands.into_iter().take(MAX_COMMANDS).collect(),
        requires_confirmation: asked || filtered.requires_confirmation,
    };
    fit_into_frame(&mut answer);
    Ok(Validated {
        answer,
        dropped_commands: filtered.dropped,
    })
}

/// The JSON object in `raw`: the whole text, or the first `{...}` in it
/// (models sometimes wrap JSON in prose or a Markdown fence).
fn extract_json(raw: &str) -> Option<Value> {
    let raw = raw.trim();
    if let Ok(value) = serde_json::from_str::<Value>(raw) {
        return Some(value);
    }
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    serde_json::from_str(raw.get(start..=end)?).ok()
}

fn string_list(value: Option<&Value>) -> Result<Vec<String>, Invalid> {
    match value {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                item.as_str()
                    .map(str::to_string)
                    .ok_or(Invalid("a list item is not a string"))
            })
            .collect(),
        Some(_) => Err(Invalid("a list field is not an array")),
    }
}

/// Plain printable ASCII: common typographic characters are replaced by
/// their ASCII forms, line breaks and other whitespace become spaces, and
/// everything else becomes `?`. Runs of spaces are collapsed.
pub fn to_ascii(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        let replacement: &str = match c {
            '\u{2018}' | '\u{2019}' | '\u{201a}' | '\u{2032}' => "'",
            '\u{201c}' | '\u{201d}' | '\u{201e}' | '\u{2033}' => "\"",
            '\u{2010}'..='\u{2015}' | '\u{2212}' => "-",
            '\u{2026}' => "...",
            '\u{2192}' => "->",
            '\u{2264}' => "<=",
            '\u{2265}' => ">=",
            '\u{00d7}' => "x",
            '\u{2022}' | '\u{00b7}' => "-",
            c if c.is_whitespace() => " ",
            c if c.is_ascii_graphic() => {
                out.push(c);
                continue;
            }
            _ => "?",
        };
        out.push_str(replacement);
    }
    let mut collapsed = String::with_capacity(out.len());
    for word in out.split(' ').filter(|w| !w.is_empty()) {
        if !collapsed.is_empty() {
            collapsed.push(' ');
        }
        collapsed.push_str(word);
    }
    collapsed
}

/// At most `max` characters, ending in "..." when cut.
fn clip(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let mut cut = protocol::truncate(text, max - 3);
    if let Some(space) = cut.rfind(' ').filter(|&i| i > max / 2) {
        cut.truncate(space);
    }
    cut.push_str("...");
    cut
}

/// Drops suggestions (then shortens the summary) until the frame fits into
/// the kernel's frame buffer. Escapes such as `\"` make the JSON longer
/// than the text.
fn fit_into_frame(answer: &mut Answer) {
    let fits = |a: &Answer| {
        protocol::ai_response_frame("req-99999", a).len() - protocol::FRAME_PREFIX.len()
            < protocol::MAX_FRAME_LENGTH - 8
    };
    while !fits(answer) && !answer.suggestions.is_empty() {
        answer.suggestions.pop();
    }
    while !fits(answer) && answer.summary.len() > 80 {
        let shorter = answer.summary.len() - 40;
        answer.summary = clip(&answer.summary, shorter);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_well_formed_answer() {
        let v = validate(
            r#"{"summary":"The heap is only 1 MiB.","suggestions":["Run meminfo"],"commands":["meminfo"],"requiresConfirmation":false}"#,
        )
        .unwrap();
        assert_eq!(v.answer.summary, "The heap is only 1 MiB.");
        assert_eq!(v.answer.commands, ["meminfo"]);
        assert!(!v.answer.requires_confirmation);
    }

    #[test]
    fn finds_json_inside_prose_or_fences() {
        let raw = "Sure! Here it is:\n```json\n{\"summary\":\"ok\"}\n```";
        assert_eq!(validate(raw).unwrap().answer.summary, "ok");
    }

    #[test]
    fn rejects_unusable_answers() {
        assert_eq!(
            validate("no json here"),
            Err(Invalid("the AI answer is not a JSON object"))
        );
        assert!(validate(r#"{"summary":""}"#).is_err());
        assert!(validate(r#"{"summary":"   "}"#).is_err());
        assert!(validate(r#"{"suggestions":["x"]}"#).is_err());
        assert!(validate(r#"{"summary":"x","commands":"meminfo"}"#).is_err());
        assert!(validate(r#"{"summary":"x","suggestions":[1]}"#).is_err());
        assert!(validate(r#"{"summary":"x","requiresConfirmation":"yes"}"#).is_err());
        assert!(validate(r#"["summary"]"#).is_err());
    }

    #[test]
    fn enforces_the_kernel_limits() {
        let long = "word ".repeat(200);
        let raw = serde_json::json!({
            "summary": long,
            "suggestions": vec![long.clone(); 9],
            "commands": ["meminfo", "uptime", "info", "logs", "help"],
        });
        let a = validate(&raw.to_string()).unwrap().answer;
        assert!(a.summary.len() <= MAX_SUMMARY && a.summary.ends_with("..."));
        assert!(a.suggestions.len() <= MAX_SUGGESTIONS);
        assert!(a.suggestions.iter().all(|s| s.len() <= MAX_SUGGESTION));
        assert_eq!(a.commands.len(), MAX_COMMANDS);
        let frame = protocol::ai_response_frame("req-001", &a);
        assert!(frame.len() - protocol::FRAME_PREFIX.len() < protocol::MAX_FRAME_LENGTH);
    }

    #[test]
    fn quote_heavy_text_still_fits_one_frame() {
        let quotes = "\"\\".repeat(240);
        let raw = serde_json::json!({ "summary": quotes, "suggestions": vec![quotes.clone(); 4] });
        let a = validate(&raw.to_string()).unwrap().answer;
        let frame = protocol::ai_response_frame("req-001", &a);
        assert!(frame.len() - protocol::FRAME_PREFIX.len() < protocol::MAX_FRAME_LENGTH);
    }

    #[test]
    fn unsafe_commands_are_dropped_and_risky_ones_flagged() {
        let v =
            validate(r#"{"summary":"x","commands":["panic-test","rm -rf /","shutdown"]}"#).unwrap();
        assert_eq!(v.answer.commands, ["shutdown"]);
        assert!(v.answer.requires_confirmation);
        assert_eq!(v.dropped_commands.len(), 2);
    }

    #[test]
    fn text_becomes_plain_ascii() {
        assert_eq!(
            to_ascii(
                "It\u{2019}s \u{201c}full\u{201d} \u{2014} 64\u{00a0}MiB\u{2026}\n\tdone \u{1F600}"
            ),
            "It's \"full\" - 64 MiB... done ?"
        );
        assert!(to_ascii("\u{1b}[31mred").is_ascii());
    }
}
