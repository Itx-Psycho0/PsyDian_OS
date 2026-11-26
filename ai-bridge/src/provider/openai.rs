//! Any service with an OpenAI-style `/chat/completions` endpoint, such as a
//! local Ollama model (`AI_PROVIDER=ollama`, no API key needed). Useful when
//! no paid API key is available.

use super::{Provider, ProviderError, agent, http_error};
use crate::config::{Config, Secret};
use crate::context::Context;
use serde_json::{Value, json};
use std::time::Duration;

pub struct OpenAiCompatible {
    endpoint: String,
    model: String,
    api_key: Secret,
    timeout: Duration,
}

impl OpenAiCompatible {
    pub fn new(config: &Config) -> Self {
        Self {
            endpoint: config.endpoint.clone(),
            model: config.model.clone(),
            api_key: config.api_key.clone(),
            timeout: config.timeout,
        }
    }

    pub fn request_body(&self, system: &str, user: &str) -> Value {
        json!({
            "model": self.model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user }
            ],
            "response_format": { "type": "json_object" },
            "stream": false
        })
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
        format!("openai-compatible {} at {}", self.model, self.endpoint)
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
