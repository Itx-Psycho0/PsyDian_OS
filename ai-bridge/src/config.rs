//! Configuration from the host environment (Docs/10-genai-architecture.md,
//! section 20). Nothing here is compiled into the kernel, and the API key is
//! never written to a log or a frame.

use std::env;
use std::fmt;
use std::time::Duration;

/// Which AI service answers the questions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    /// Claude through the Anthropic Messages API.
    Anthropic,
    /// Any service with an OpenAI-style `/chat/completions` endpoint
    /// (for example a local Ollama model).
    OpenAiCompatible,
    /// Canned answers, no network. For testing and demos.
    Mock,
}

impl ProviderKind {
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "anthropic" | "claude" => Some(Self::Anthropic),
            "openai" | "openai-compatible" | "ollama" => Some(Self::OpenAiCompatible),
            "mock" => Some(Self::Mock),
            _ => None,
        }
    }
}

/// An API key that cannot be printed by accident.
#[derive(Clone, Default)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn expose(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.trim().is_empty()
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(if self.is_empty() {
            "<unset>"
        } else {
            "<redacted>"
        })
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub provider: ProviderKind,
    pub model: String,
    pub endpoint: String,
    pub api_key: Secret,
    /// How long one AI call may take. Kept below the kernel's own 30 s wait
    /// so the kernel always gets an answer or an error frame.
    pub timeout: Duration,
    /// `output_config.effort` for Claude: low, medium, high, xhigh or max.
    pub effort: String,
    /// Most AI requests answered per minute; more get `AI_RATE_LIMITED`.
    pub max_requests_per_minute: u32,
}

pub const DEFAULT_ANTHROPIC_MODEL: &str = "claude-opus-5-5";
pub const DEFAULT_ANTHROPIC_ENDPOINT: &str = "https://api.anthropic.com/v1/messages";
pub const DEFAULT_OPENAI_ENDPOINT: &str = "http://localhost:11434/v1/chat/completions";
pub const DEFAULT_OPENAI_MODEL: &str = "llama3.2";
pub const DEFAULT_TIMEOUT_SECS: u64 = 25;
pub const MAX_TIMEOUT_SECS: u64 = 28;

#[derive(Debug, PartialEq, Eq)]
pub struct ConfigError(pub String);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Config {
    /// Reads `AI_PROVIDER`, `AI_MODEL`, `AI_ENDPOINT`, `AI_API_KEY` (or
    /// `ANTHROPIC_API_KEY` / `OPENAI_API_KEY`), `AI_TIMEOUT`, `AI_EFFORT` and
    /// `AI_MAX_REQUESTS_PER_MINUTE`.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|name| env::var(name).ok().filter(|v| !v.trim().is_empty()))
    }

    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let provider = match get("AI_PROVIDER") {
            Some(name) => ProviderKind::parse(&name).ok_or_else(|| {
                ConfigError(format!(
                    "AI_PROVIDER '{name}' is not supported (use anthropic, openai or mock)"
                ))
            })?,
            None => ProviderKind::Anthropic,
        };

        let (default_model, default_endpoint, key_names): (&str, &str, &[&str]) = match provider {
            ProviderKind::Anthropic => (
                DEFAULT_ANTHROPIC_MODEL,
                DEFAULT_ANTHROPIC_ENDPOINT,
                &["AI_API_KEY", "ANTHROPIC_API_KEY"],
            ),
            ProviderKind::OpenAiCompatible => (
                DEFAULT_OPENAI_MODEL,
                DEFAULT_OPENAI_ENDPOINT,
                &["AI_API_KEY", "OPENAI_API_KEY"],
            ),
            ProviderKind::Mock => ("mock", "", &[]),
        };

        let timeout_secs = match get("AI_TIMEOUT") {
            Some(text) => text
                .trim()
                .parse::<u64>()
                .ok()
                .filter(|&secs| (1..=MAX_TIMEOUT_SECS).contains(&secs))
                .ok_or_else(|| {
                    ConfigError(format!(
                        "AI_TIMEOUT must be a number of seconds from 1 to {MAX_TIMEOUT_SECS} \
                         (the kernel waits 30 s)"
                    ))
                })?,
            None => DEFAULT_TIMEOUT_SECS,
        };

        let effort = get("AI_EFFORT").unwrap_or_else(|| "low".to_string());
        if !["low", "medium", "high", "xhigh", "max"].contains(&effort.as_str()) {
            return Err(ConfigError(
                "AI_EFFORT must be one of low, medium, high, xhigh, max".to_string(),
            ));
        }

        let max_requests_per_minute = match get("AI_MAX_REQUESTS_PER_MINUTE") {
            Some(text) => text
                .trim()
                .parse::<u32>()
                .ok()
                .filter(|&n| n > 0)
                .ok_or_else(|| {
                    ConfigError("AI_MAX_REQUESTS_PER_MINUTE must be a positive number".to_string())
                })?,
            None => 10,
        };

        let api_key = key_names
            .iter()
            .find_map(|name| get(name))
            .map(Secret::new)
            .unwrap_or_default();

        Ok(Self {
            provider,
            model: get("AI_MODEL").unwrap_or_else(|| default_model.to_string()),
            endpoint: get("AI_ENDPOINT").unwrap_or_else(|| default_endpoint.to_string()),
            api_key,
            timeout: Duration::from_secs(timeout_secs),
            effort,
            max_requests_per_minute,
        })
    }

    /// Why AI calls cannot work with this configuration, if they cannot.
    /// The bridge still starts: every request then gets an `AI_NOT_CONFIGURED`
    /// error frame and the kernel keeps working (SEC-009).
    pub fn problem(&self) -> Option<String> {
        match self.provider {
            ProviderKind::Anthropic if self.api_key.is_empty() => Some(
                "no API key: set ANTHROPIC_API_KEY (or AI_API_KEY), or use AI_PROVIDER=mock"
                    .to_string(),
            ),
            ProviderKind::Anthropic | ProviderKind::OpenAiCompatible
                if !(self.endpoint.starts_with("https://")
                    || self.endpoint.starts_with("http://localhost")
                    || self.endpoint.starts_with("http://127.0.0.1")) =>
            {
                Some(format!(
                    "AI_ENDPOINT must use https:// (plain http is only allowed for localhost): {}",
                    self.endpoint
                ))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn config(vars: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Config::from_lookup(|name| map.get(name).cloned())
    }

    #[test]
    fn defaults_to_claude() {
        let c = config(&[("ANTHROPIC_API_KEY", "sk-test")]).unwrap();
        assert_eq!(c.provider, ProviderKind::Anthropic);
        assert_eq!(c.model, "claude-opus-5-5");
        assert_eq!(c.endpoint, DEFAULT_ANTHROPIC_ENDPOINT);
        assert_eq!(c.timeout, Duration::from_secs(25));
        assert!(c.problem().is_none());
    }

    #[test]
    fn missing_key_is_a_controlled_problem() {
        let c = config(&[]).unwrap();
        assert!(c.problem().unwrap().contains("ANTHROPIC_API_KEY"));
    }

    #[test]
    fn the_key_never_appears_in_debug_output() {
        let c = config(&[("AI_API_KEY", "sk-very-secret")]).unwrap();
        let text = format!("{c:?}");
        assert!(!text.contains("sk-very-secret"));
        assert!(text.contains("<redacted>"));
    }

    #[test]
    fn rejects_bad_values() {
        assert!(config(&[("AI_PROVIDER", "skynet")]).is_err());
        assert!(config(&[("AI_TIMEOUT", "0")]).is_err());
        assert!(config(&[("AI_TIMEOUT", "60")]).is_err());
        assert!(config(&[("AI_EFFORT", "extreme")]).is_err());
        assert!(config(&[("AI_MAX_REQUESTS_PER_MINUTE", "0")]).is_err());
    }

    #[test]
    fn plain_http_only_for_localhost() {
        let c = config(&[
            ("AI_PROVIDER", "openai"),
            ("AI_ENDPOINT", "http://evil.example/v1"),
        ])
        .unwrap();
        assert!(c.problem().is_some());
        let c = config(&[("AI_PROVIDER", "ollama")]).unwrap();
        assert!(c.problem().is_none()); // local Ollama needs no key
        let c = config(&[("AI_PROVIDER", "mock")]).unwrap();
        assert!(c.problem().is_none());
    }
}
