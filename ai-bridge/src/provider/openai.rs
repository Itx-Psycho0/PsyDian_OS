//! Services with an OpenAI-style `/chat/completions` endpoint. These cover
//! the free options: Google Gemini (`AI_PROVIDER=gemini`, free key from
//! Google AI Studio), an OmniRoute gateway (`omniroute`), OpenRouter `:free`
//! models (`openrouter`) and a local Ollama model (`ollama`, no key).

use super::{Provider, ProviderError, agent, http_error};
use crate::config::{Config, ProviderKind, Secret};
use crate::context::Context;
use crate::prompt;
use serde_json::{Value, json};
use std::time::Duration;

pub struct OpenAiCompatible {
    kind: ProviderKind,
    endpoint: String,
    model: String,
    api_key: Secret,
    timeout: Duration,
    effort: String,
}

impl OpenAiCompatible {
    pub fn new(config: &Config) -> Self {
        Self {
            kind: config.provider,
            endpoint: config.endpoint.clone(),
            model: config.model.clone(),
            api_key: config.api_key.clone(),
            timeout: config.timeout,
            effort: config.effort.clone(),
        }
    }

    pub fn request_body(&self, system: &str, user: &str) -> Value {
        let mut body = json!({
            "model": self.model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user }
            ],
            "response_format": { "type": "json_object" },
            "stream": false
        });
        if self.kind == ProviderKind::Gemini {
            // Gemini supports the full schema (structured output) and a
            // reasoning level; low keeps answers inside the kernel's wait.
            body["response_format"] = json!({
                "type": "json_schema",
                "json_schema": { "name": "psydian_answer", "schema": prompt::answer_schema() }
            });
            body["reasoning_effort"] = json!(gemini_effort(&self.effort));
        }
        body
    }
}

/// Gemini accepts low, medium and high.
fn gemini_effort(effort: &str) -> &'static str {
    match effort {
        "medium" => "medium",
        "high" | "xhigh" | "max" => "high",
        _ => "low",
    }
}

pub fn answer_text(response: &Value) -> Result<String, ProviderError> {
    let choice = response
        .pointer("/choices/0")
        .ok_or_else(|| ProviderError::InvalidResponse("response has no choices".into()))?;
    if choice.get("finish_reason").and_then(Value::as_str) == Some("length") {
        return Err(ProviderError::Truncated);
    }
    choice
        .pointer("/message/content")
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| ProviderError::InvalidResponse("response has no text".into()))
}

impl Provider for OpenAiCompatible {
    fn describe(&self) -> String {
        format!("{} {} at {}", self.kind.name(), self.model, self.endpoint)
    }

    fn complete(
        &self,
        system: &str,
        user: &str,
        _context: &Context,
    ) -> Result<String, ProviderError> {
        let mut request = agent(self.timeout)
            .post(&self.endpoint)
            .set("content-type", "application/json");
        if !self.api_key.is_empty() {
            request = request.set(
                "authorization",
                &format!("Bearer {}", self.api_key.expose()),
            );
        }
        let response = request
            .send_json(self.request_body(system, user))
            .map_err(http_error)?;
        let value = response
            .into_json::<Value>()
            .map_err(|e| ProviderError::InvalidResponse(format!("response is not JSON: {e}")))?;
        answer_text(&value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider(vars: &[(&str, &str)]) -> OpenAiCompatible {
        let config = Config::from_lookup(|name| {
            vars.iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| v.to_string())
        })
        .unwrap();
        OpenAiCompatible::new(&config)
    }

    #[test]
    fn gemini_gets_the_answer_schema() {
        let p = provider(&[("GEMINI_API_KEY", "g-test")]);
        let body = p.request_body("sys", "user");
        assert_eq!(body["model"], "gemini-flash-lite-latest");
        assert_eq!(body["response_format"]["type"], "json_schema");
        assert_eq!(
            body["response_format"]["json_schema"]["schema"],
            prompt::answer_schema()
        );
        assert_eq!(body["reasoning_effort"], "low");
        assert_eq!(body["messages"][0]["content"], "sys");
    }

    #[test]
    fn other_services_get_plain_json_mode() {
        let p = provider(&[("AI_PROVIDER", "omniroute"), ("AI_MODEL", "combo")]);
        let body = p.request_body("sys", "user");
        assert_eq!(body["response_format"]["type"], "json_object");
        assert!(body.get("reasoning_effort").is_none());
        assert!(p.describe().starts_with("omniroute combo"));
    }

    #[test]
    fn reads_chat_completions() {
        let ok = json!({"choices":[{"finish_reason":"stop","message":{"content":"{\"summary\":\"x\"}"}}]});
        assert_eq!(answer_text(&ok).unwrap(), "{\"summary\":\"x\"}");
        assert_eq!(
            answer_text(&json!({"choices":[{"finish_reason":"length","message":{"content":"{"}}]})),
            Err(ProviderError::Truncated)
        );
        assert!(answer_text(&json!({"choices":[]})).is_err());
    }
}
