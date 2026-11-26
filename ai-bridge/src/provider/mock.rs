//! Canned answers without any network access (`AI_PROVIDER=mock`), for tests
//! and for demos without an API key. The answers go through the same
//! validation and safety checks as a real provider's.

use super::{Provider, ProviderError};
use crate::context::Context;
use serde_json::json;

pub struct Mock;

impl Provider for Mock {
    fn describe(&self) -> String {
        "mock (canned answers, no network)".into()
    }

    fn complete(
        &self,
        _system: &str,
        _user: &str,
        context: &Context,
    ) -> Result<String, ProviderError> {
        let q = context.query.to_ascii_lowercase();
        if q.contains("timeout") {
            return Err(ProviderError::Timeout);
        }
        if q.contains("error") {
            return Err(ProviderError::Unavailable("mock outage".into()));
        }
        if q.contains("garbage") {
            return Ok("this is not JSON".into());
        }
        if q.contains("crash") || q.contains("danger") {
            return Ok(json!({
                "summary": "Some risky suggestions, to show the safety checks.",
                "suggestions": ["The bridge drops destructive and unknown commands"],
                "commands": ["panic-test", "rm -rf /", "shutdown"],
                "requiresConfirmation": false
            })
            .to_string());
        }
        let answer = match &context.diagnostic {
            Some(d) if d.code == "MEM-003" => {
                let get = |key: &str| {
                    d.context
                        .iter()
                        .find(|(k, _)| k == key)
                        .and_then(|(_, v)| v.parse::<u64>().ok())
                };
                let size = get("size")
                    .map(|b| format!("{} MiB", b / (1024 * 1024)))
                    .unwrap_or("a large block".into());
                let heap = get("heap_size")
                    .map(|b| format!("{} KiB", b / 1024))
                    .unwrap_or("smaller".into());
                json!({
                    "summary": format!("Observed: a request for {size} failed because the kernel heap is only {heap}. \
                                        The allocator returned an error, which was recorded as MEM-003, and the kernel kept running."),
                    "suggestions": ["Run meminfo to see heap usage", "Allocate smaller buffers or in parts"],
                    "commands": ["meminfo", "diag last"],
                    "requiresConfirmation": false
                })
            }
            Some(d) => json!({
                "summary": format!("The diagnostic is {}: {} (subsystem {}).", d.code, d.message, d.subsystem),
                "suggestions": ["Run diag last for the full record"],
                "commands": ["diag last"],
                "requiresConfirmation": false
            }),
            None => json!({
                "summary": "Hello from the AI Bridge (mock provider). No diagnostic is attached yet; \
                            try fault-test heap and ask why it failed.",
                "suggestions": ["Run fault-test heap, then ask: ai why did the heap fail?"],
                "commands": ["meminfo", "help"],
                "requiresConfirmation": false
            }),
        };
        Ok(answer.to_string())
    }
}
