//! Claude through the Anthropic Messages API (`POST /v1/messages`).
//!
//! Rust has no official Anthropic SDK, so this is a plain HTTPS request. The
//! answer is constrained to the answer schema with structured outputs
//! (`output_config.format`), and a `refusal` stop reason is reported as such.

use super::{Provider, ProviderError, agent, http_error, retryable};
use crate::config::Config;
use crate::context::Context;
use crate::prompt;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

pub const API_VERSION: &str = "2026-10-21";
pub const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";
/// Room for adaptive thinking plus the short JSON answer.
pub const MAX_TOKENS: u32 = 8000;

/// Models that accept server-side refusal fallbacks (`fallbacks: "default"`).
const FALLBACK_MODELS: &[&str] = &[
    "claude-fable-5-2",
    "claude-fable-5-1",
    "claude-opus-5-5",
    "claude-opus-5",
    "claude-sonnet-5-5",
];

pub struct Anthropic {
    endpoint: String,
    model: String,
    effort: String,
    api_key: crate::config::Secret,
    timeout: Duration,
}

impl Anthropic {
    pub fn new(config: &Config) -> Self {
        Self {
            endpoint: config.endpoint.clone(),
            model: config.model.clone(),
            effort: config.effort.clone(),
            api_key: config.api_key.clone(),
            timeout: config.timeout,
        }
    }

    fn uses_fallbacks(&self) -> bool {
        FALLBACK_MODELS.contains(&self.model.as_str())
    }

    pub fn request_body(&self, system: &str, user: &str) -> Value {
        let mut body = json!({
            "model": self.model,
            "max_tokens": MAX_TOKENS,
            "system": system,
            "messages": [{ "role": "user", "content": user }],
            "output_config": {
                "effort": self.effort,
                "format": { "type": "json_schema", "schema": prompt::answer_schema() }
            }
        });
        if self.uses_fallbacks() {
            body["fallbacks"] = json!("default");
        }
        body
    }

    fn send(&self, body: &Value, timeout: Duration) -> Result<Value, ProviderError> {
        let mut request = agent(timeout)
            .post(&self.endpoint)
            .set("content-type", "application/json")
            .set("x-api-key", self.api_key.expose())
            .set("anthropic-version", API_VERSION);
        if self.uses_fallbacks() {
            request = request.set("anthropic-beta", FALLBACK_BETA);
        }
        let response = request.send_json(body).map_err(http_error)?;
        response
            .into_json::<Value>()
            .map_err(|e| ProviderError::InvalidResponse(format!("response is not JSON: {e}")))
    }
}

/// The text of a Messages API response, after checking why it stopped.
pub fn answer_text(response: &Value) -> Result<String, ProviderError> {
    match response.get("stop_reason").and_then(Value::as_str) {
        Some("refusal") => return Err(ProviderError::Refused),
        Some("max_tokens") => return Err(ProviderError::Truncated),
        _ => {}
    }
    let text: String = response
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|block| block.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|block| block.get("text").and_then(Value::as_str))
        .collect();
    if text.trim().is_empty() {
        return Err(ProviderError::InvalidResponse(
            "response has no text".into(),
        ));
    }
    Ok(text)
}

impl Provider for Anthropic {
    fn describe(&self) -> String {
        format!("anthropic {} (effort {})", self.model, self.effort)
    }

    fn complete(
        &self,
        system: &str,
        user: &str,
        _context: &Context,
    ) -> Result<String, ProviderError> {
        if self.api_key.is_empty() {
            return Err(ProviderError::NotConfigured("no API key".into()));
        }
        let body = self.request_body(system, user);
        let started = Instant::now();
        let response = match self.send(&body, self.timeout) {
            // One retry when the service is overloaded and time is left.
            Err(error) if retryable(&error) && started.elapsed() < self.timeout / 3 => {
                std::thread::sleep(Duration::from_millis(800));
                self.send(&body, self.timeout.saturating_sub(started.elapsed()))?
            }
            other => other?,
        };
        answer_text(&response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn provider(model: &str) -> Anthropic {
        let config = Config::from_lookup(|name| match name {
            "ANTHROPIC_API_KEY" => Some("sk-test".into()),
            "AI_MODEL" => Some(model.into()),
            _ => None,
        })
        .unwrap();
        Anthropic::new(&config)
    }

    #[test]
    fn request_uses_structured_output_and_fallbacks() {
        let body = provider("claude-opus-5-5").request_body("sys", "question");
        assert_eq!(body["model"], "claude-opus-5-5");
        assert_eq!(body["system"], "sys");
        assert_eq!(body["messages"][0]["content"], "question");
        assert_eq!(body["output_config"]["format"]["type"], "json_schema");
        assert_eq!(body["output_config"]["effort"], "low");
        assert_eq!(body["fallbacks"], "default");
        assert!(body.get("thinking").is_none()); // always on for this model
        assert!(body.get("temperature").is_none());
    }

    #[test]
    fn older_models_get_no_fallbacks() {
        assert!(
            provider("claude-haiku-5-5")
                .request_body("s", "u")
                .get("fallbacks")
                .is_none()
        );
    }

    #[test]
    fn reads_text_and_stop_reasons() {
        let ok = json!({"stop_reason":"end_turn","content":[
            {"type":"thinking","thinking":""},{"type":"text","text":"{\"summary\":\"hi\"}"}]});
        assert_eq!(answer_text(&ok).unwrap(), "{\"summary\":\"hi\"}");
        assert_eq!(
            answer_text(&json!({"stop_reason":"refusal","content":[]})),
            Err(ProviderError::Refused)
        );
        assert_eq!(
            answer_text(&json!({"stop_reason":"max_tokens","content":[]})),
            Err(ProviderError::Truncated)
        );
        assert!(answer_text(&json!({"stop_reason":"end_turn","content":[]})).is_err());
    }
}
