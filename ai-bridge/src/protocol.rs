//! The serial protocol shared with the kernel (Docs/08-api-specification.md,
//! section 8.5): one `@PSY <json>` frame per line, envelope
//! `{type, version, id, payload}`.
//!
//! Frames the bridge sends must be printable ASCII and at most
//! [`MAX_FRAME_LENGTH`] bytes, because that is all the kernel accepts. Any
//! non-ASCII character is escaped as `\uXXXX`.

use serde_json::{Value, json};

pub const FRAME_PREFIX: &str = "@PSY ";
pub const PROTOCOL_VERSION: u64 = 1;
/// Longest frame body the kernel accepts (`MAX_FRAME_LENGTH` in the kernel).
pub const MAX_FRAME_LENGTH: usize = 2048;
/// Longest question the bridge forwards to the AI service.
pub const MAX_QUERY_LENGTH: usize = 512;

/// A message from the kernel the bridge acts on.
#[derive(Debug, Clone, PartialEq)]
pub enum KernelMessage {
    /// The kernel booted and accepts input.
    Ready,
    /// A structured diagnostic record.
    Diagnostic(Diagnostic),
    /// The user ran `ai <question>`.
    AiRequest(AiRequest),
    /// A frame with a type the bridge does not handle.
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub id: String,
    pub severity: String,
    pub subsystem: String,
    pub code: String,
    pub message: String,
    pub uptime_ms: u64,
    /// Key/value details such as `size` or `address`, in kernel order.
    pub context: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AiRequest {
    pub id: String,
    pub query: String,
    pub diagnostic_id: Option<String>,
}

/// Why a kernel frame was not accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameError(pub &'static str);

/// Returns the frame body when `line` is a protocol frame.
pub fn frame_body(line: &str) -> Option<&str> {
    line.trim_end_matches(['\r', '\n'])
        .strip_prefix(FRAME_PREFIX)
}

/// Parses and validates one frame body from the kernel.
pub fn parse_kernel_frame(body: &str) -> Result<KernelMessage, FrameError> {
    let value: Value =
        serde_json::from_str(body).map_err(|_| FrameError("frame is not valid JSON"))?;
    if value.get("version").and_then(Value::as_u64) != Some(PROTOCOL_VERSION) {
        return Err(FrameError("unsupported protocol version"));
    }
    let kind = value
        .get("type")
        .and_then(Value::as_str)
        .ok_or(FrameError("missing message type"))?;
    let id = value
        .get("id")
        .and_then(Value::as_str)
        .ok_or(FrameError("missing message id"))?;
    let payload = value
        .get("payload")
        .filter(|p| p.is_object())
        .ok_or(FrameError("missing payload object"))?;
    let text = |key: &str| payload.get(key).and_then(Value::as_str).map(str::to_string);

    match kind {
        "status" => Ok(match text("state").as_deref() {
            Some("ready") => KernelMessage::Ready,
            _ => KernelMessage::Other,
        }),
        "diagnostic" => {
            let context = payload
                .get("context")
                .and_then(Value::as_object)
                .map(|map| {
                    map.iter()
                        .map(|(k, v)| {
                            (
                                k.clone(),
                                v.as_str()
                                    .map(str::to_string)
                                    .unwrap_or_else(|| v.to_string()),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            Ok(KernelMessage::Diagnostic(Diagnostic {
                id: id.to_string(),
                severity: text("severity").ok_or(FrameError("diagnostic without severity"))?,
                subsystem: text("subsystem").ok_or(FrameError("diagnostic without subsystem"))?,
                code: text("code").ok_or(FrameError("diagnostic without code"))?,
                message: text("message").ok_or(FrameError("diagnostic without message"))?,
                uptime_ms: payload
                    .get("uptime_ms")
                    .and_then(Value::as_u64)
                    .unwrap_or(0),
                context,
            }))
        }
        "ai_request" => {
            if !is_request_id(id) {
                return Err(FrameError("ai_request id is not a request id"));
            }
            let query = text("query").ok_or(FrameError("ai_request without query"))?;
            if query.trim().is_empty() {
                return Err(FrameError("ai_request query is empty"));
            }
            if query.len() > MAX_QUERY_LENGTH {
                return Err(FrameError("ai_request query is too long"));
            }
            Ok(KernelMessage::AiRequest(AiRequest {
                id: id.to_string(),
                query,
                diagnostic_id: text("diagnostic_id"),
            }))
        }
        _ => Ok(KernelMessage::Other),
    }
}

/// `req-` followed by digits, the kernel's request id format.
pub fn is_request_id(id: &str) -> bool {
    id.strip_prefix("req-")
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

/// A validated answer, ready to be framed.
#[derive(Debug, Clone, PartialEq)]
pub struct Answer {
    pub summary: String,
    pub suggestions: Vec<String>,
    pub commands: Vec<String>,
    pub requires_confirmation: bool,
}

pub fn ai_response_frame(request_id: &str, answer: &Answer) -> String {
    frame(&json!({
        "type": "ai_response",
        "version": PROTOCOL_VERSION,
        "id": request_id,
        "payload": {
            "summary": answer.summary,
            "suggestions": answer.suggestions,
            "commands": answer.commands,
            "requiresConfirmation": answer.requires_confirmation,
        }
    }))
}

pub fn error_frame(request_id: Option<&str>, code: &str, message: &str) -> String {
    let mut payload = json!({ "code": code, "message": truncate(message, 200) });
    if let Some(id) = request_id {
        payload["request_id"] = json!(id);
    }
    frame(&json!({
        "type": "error",
        "version": PROTOCOL_VERSION,
        "id": request_id.unwrap_or("err-001"),
        "payload": payload,
    }))
}

pub fn bridge_ready_frame() -> String {
    frame(&json!({
        "type": "status",
        "version": PROTOCOL_VERSION,
        "id": "stat-001",
        "payload": { "state": "bridge_ready" },
    }))
}

/// `@PSY <json>` with every non-ASCII character escaped.
fn frame(value: &Value) -> String {
    let mut line = String::from(FRAME_PREFIX);
    line.push_str(&ascii_json(value));
    line
}

pub fn ascii_json(value: &Value) -> String {
    let text = value.to_string();
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii() {
            out.push(c);
        } else {
            let mut units = [0u16; 2];
            for unit in c.encode_utf16(&mut units) {
                out.push_str(&format!("\\u{unit:04x}"));
            }
        }
    }
    out
}

/// At most `max` bytes, cut at a character boundary.
pub fn truncate(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_frames_the_kernel_sends() {
        let ready = r#"{"type":"status","version":1,"id":"stat-001","payload":{"state":"ready","kernel":"0.1.0","uptime_ms":20}}"#;
        assert_eq!(parse_kernel_frame(ready), Ok(KernelMessage::Ready));

        let diag = r#"{"type":"diagnostic","version":1,"id":"diag-001","payload":{"severity":"error","subsystem":"memory","code":"MEM-003","uptime_ms":2070,"message":"Heap allocation failed","context":{"size":"67108864","align":"8"}}}"#;
        let KernelMessage::Diagnostic(d) = parse_kernel_frame(diag).unwrap() else {
            panic!()
        };
        assert_eq!(d.code, "MEM-003");
        assert_eq!(d.context[0], ("size".to_string(), "67108864".to_string()));

        let req = r#"{"type":"ai_request","version":1,"id":"req-002","payload":{"query":"why?","diagnostic_id":"diag-001"}}"#;
        assert_eq!(
            parse_kernel_frame(req),
            Ok(KernelMessage::AiRequest(AiRequest {
                id: "req-002".into(),
                query: "why?".into(),
                diagnostic_id: Some("diag-001".into())
            }))
        );
        let req = r#"{"type":"ai_request","version":1,"id":"req-003","payload":{"query":"hi","diagnostic_id":null}}"#;
        let KernelMessage::AiRequest(r) = parse_kernel_frame(req).unwrap() else {
            panic!()
        };
        assert_eq!(r.diagnostic_id, None);
    }

    #[test]
    fn rejects_malformed_frames() {
        assert!(parse_kernel_frame("{nope").is_err());
        assert!(
            parse_kernel_frame(r#"{"type":"status","version":2,"id":"x","payload":{}}"#).is_err()
        );
        assert!(parse_kernel_frame(r#"{"type":"status","version":1,"id":"x"}"#).is_err());
        assert!(
            parse_kernel_frame(
                r#"{"type":"ai_request","version":1,"id":"bad","payload":{"query":"q"}}"#
            )
            .is_err()
        );
        assert!(
            parse_kernel_frame(
                r#"{"type":"ai_request","version":1,"id":"req-1","payload":{"query":"  "}}"#
            )
            .is_err()
        );
        let long = format!(
            r#"{{"type":"ai_request","version":1,"id":"req-1","payload":{{"query":"{}"}}}}"#,
            "a".repeat(600)
        );
        assert!(parse_kernel_frame(&long).is_err());
    }

    #[test]
    fn frames_are_ascii_and_escaped() {
        let answer = Answer {
            summary: "Heap \u{2014} full \u{1F600} \"quoted\"".into(),
            suggestions: vec![],
            commands: vec!["meminfo".into()],
            requires_confirmation: false,
        };
        let line = ai_response_frame("req-001", &answer);
        assert!(line.starts_with("@PSY {"));
        assert!(line.bytes().all(|b| (0x20..0x7f).contains(&b)));
        assert!(line.contains("\\u2014"));
        assert!(line.contains("\\ud83d\\ude00")); // surrogate pair
        let back: Value = serde_json::from_str(frame_body(&line).unwrap()).unwrap();
        assert_eq!(back["payload"]["summary"], answer.summary);
    }

    #[test]
    fn error_frames_name_the_request() {
        let line = error_frame(Some("req-007"), "AI_TIMEOUT", "took too long");
        let v: Value = serde_json::from_str(frame_body(&line).unwrap()).unwrap();
        assert_eq!(v["type"], "error");
        assert_eq!(v["id"], "req-007");
        assert_eq!(v["payload"]["code"], "AI_TIMEOUT");
    }

    #[test]
    fn request_ids() {
        assert!(is_request_id("req-001"));
        assert!(!is_request_id("req-"));
        assert!(!is_request_id("req-1a"));
        assert!(!is_request_id("diag-001"));
    }

    #[test]
    fn truncation_respects_char_boundaries() {
        assert_eq!(truncate("héllo", 2), "h");
        assert_eq!(truncate("abc", 10), "abc");
    }
}
