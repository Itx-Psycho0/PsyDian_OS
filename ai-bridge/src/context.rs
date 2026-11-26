//! Builds the context sent to the AI service: the user's question, the
//! diagnostic it refers to and a short list of other recent diagnostics.
//! Only what is needed is included (Docs/10-genai-architecture.md).

use crate::diagnostics::DiagnosticStore;
use crate::protocol::{AiRequest, Diagnostic};

/// Other recent diagnostics included besides the referenced one.
pub const RECENT_DIAGNOSTICS: usize = 3;
/// Longest single context value passed on (addresses, sizes, file names).
pub const MAX_VALUE_LENGTH: usize = 120;

#[derive(Debug, Clone, PartialEq)]
pub struct Context {
    pub query: String,
    /// The diagnostic the kernel attached to the question, if still known.
    pub diagnostic: Option<Diagnostic>,
    /// Set when the kernel named a diagnostic the bridge never saw.
    pub missing_diagnostic: Option<String>,
    pub recent: Vec<Diagnostic>,
}

pub fn build(request: &AiRequest, store: &DiagnosticStore) -> Context {
    let diagnostic = request
        .diagnostic_id
        .as_deref()
        .and_then(|id| store.get(id))
        .cloned()
        .map(clean);
    let missing_diagnostic = match (&request.diagnostic_id, &diagnostic) {
        (Some(id), None) => Some(id.clone()),
        _ => None,
    };
    let referenced = diagnostic.as_ref().map(|d| d.id.clone());
    let recent = store
        .recent(RECENT_DIAGNOSTICS + 1)
        .filter(|d| Some(&d.id) != referenced.as_ref())
        .take(RECENT_DIAGNOSTICS)
        .cloned()
        .map(clean)
        .collect();
    Context {
        query: request.query.trim().to_string(),
        diagnostic,
        missing_diagnostic,
        recent,
    }
}

/// Removes control characters and caps value lengths: diagnostic text comes
/// from the kernel and is treated as data, never as instructions.
fn clean(mut d: Diagnostic) -> Diagnostic {
    let tidy = |s: &str| -> String {
        let s: String = s
            .chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect();
        crate::protocol::truncate(&s, MAX_VALUE_LENGTH)
    };
    d.message = tidy(&d.message);
    d.context = d.context.iter().map(|(k, v)| (tidy(k), tidy(v))).collect();
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diag(id: &str, message: &str) -> Diagnostic {
        Diagnostic {
            id: id.into(),
            severity: "error".into(),
            subsystem: "memory".into(),
            code: "MEM-003".into(),
            message: message.into(),
            uptime_ms: 1,
            context: vec![("size".into(), "67108864".into())],
        }
    }

    fn request(diagnostic_id: Option<&str>) -> AiRequest {
        AiRequest {
            id: "req-001".into(),
            query: "  why?  ".into(),
            diagnostic_id: diagnostic_id.map(Into::into),
        }
    }

    #[test]
    fn includes_the_referenced_and_recent_diagnostics() {
        let mut store = DiagnosticStore::new();
        for n in 1..=6 {
            store.add(diag(&format!("diag-00{n}"), "x"));
        }
        let c = build(&request(Some("diag-002")), &store);
        assert_eq!(c.query, "why?");
        assert_eq!(c.diagnostic.unwrap().id, "diag-002");
        let ids: Vec<_> = c.recent.iter().map(|d| d.id.as_str()).collect();
        assert_eq!(ids, ["diag-006", "diag-005", "diag-004"]);
    }

    #[test]
    fn notes_a_diagnostic_the_bridge_never_saw() {
        let c = build(&request(Some("diag-009")), &DiagnosticStore::new());
        assert_eq!(c.missing_diagnostic.as_deref(), Some("diag-009"));
        assert!(c.diagnostic.is_none());
    }

    #[test]
    fn cleans_kernel_text() {
        let mut store = DiagnosticStore::new();
        store.add(diag(
            "diag-001",
            &format!("bad\u{1b}[2Jtext{}", "y".repeat(500)),
        ));
        let c = build(&request(Some("diag-001")), &store);
        let message = c.diagnostic.unwrap().message;
        assert!(!message.contains('\u{1b}'));
        assert!(message.len() <= MAX_VALUE_LENGTH);
    }
}
