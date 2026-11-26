//! Configuration from the host environment (Docs/10-genai-architecture.md,
//! section 20). Nothing here is compiled into the kernel, and the API key is
//! never written to a log or a frame.

use std::env;
use std::fmt;
use std::time::Duration;

/// Which AI service answers the questions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    /// Claude through the Anthropic Messages API (paid key).
    Anthropic,
    /// Google Gemini through its OpenAI-compatible endpoint. Free API keys
    /// from Google AI Studio.
    Gemini,
    /// OmniRoute, a self-hosted AI gateway (OpenAI-style API on port 20128)
    /// that routes to free and paid providers.
    OmniRoute,
    /// OpenRouter (OpenAI-style API); models ending in `:free` cost nothing.
    OpenRouter,
    /// Any other service with an OpenAI-style `/chat/completions` endpoint
    /// (by default a local Ollama model).
    OpenAiCompatible,
    /// Canned answers, no network. For testing and demos.
    Mock,
}

impl ProviderKind {
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "anthropic" | "claude" => Some(Self::Anthropic),
            "gemini" | "google" => Some(Self::Gemini),
            "omniroute" => Some(Self::OmniRoute),
            "openrouter" => Some(Self::OpenRouter),
            "openai" | "openai-compatible" | "ollama" => Some(Self::OpenAiCompatible),
            "mock" => Some(Self::Mock),
            _ => None,
        }
    }

    /// Short name for the log and the start-up line.
    pub fn name(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
            Self::OmniRoute => "omniroute",
            Self::OpenRouter => "openrouter",
            Self::OpenAiCompatible => "openai-compatible",
            Self::Mock => "mock",
        }
    }

    /// Whether this provider speaks the OpenAI-style chat completions API.
    pub fn is_openai_style(self) -> bool {
        matches!(
            self,
            Self::Gemini | Self::OmniRoute | Self::OpenRouter | Self::OpenAiCompatible
        )
    }

    /// Environment variables that may hold this provider's API key, in order.
    fn key_names(self) -> &'static [&'static str] {
        match self {
            Self::Anthropic => &["AI_API_KEY", "ANTHROPIC_API_KEY"],
            Self::Gemini => &["AI_API_KEY", "GEMINI_API_KEY", "GOOGLE_API_KEY"],
            Self::OmniRoute => &["AI_API_KEY", "OMNIROUTE_API_KEY"],
            Self::OpenRouter => &["AI_API_KEY", "OPENROUTER_API_KEY"],
            Self::OpenAiCompatible => &["AI_API_KEY", "OPENAI_API_KEY"],
            Self::Mock => &[],
        }
    }

    /// (default model, default endpoint). An empty model means the user
    /// must choose one with `AI_MODEL`.
    fn defaults(self) -> (&'static str, &'static str) {
        match self {
            Self::Anthropic => (DEFAULT_ANTHROPIC_MODEL, DEFAULT_ANTHROPIC_ENDPOINT),
            Self::Gemini => (DEFAULT_GEMINI_MODEL, DEFAULT_GEMINI_ENDPOINT),
            Self::OmniRoute => ("", DEFAULT_OMNIROUTE_ENDPOINT),
            Self::OpenRouter => ("", DEFAULT_OPENROUTER_ENDPOINT),
            Self::OpenAiCompatible => (DEFAULT_OPENAI_MODEL, DEFAULT_OPENAI_ENDPOINT),
            Self::Mock => ("mock", ""),
        }
    }

    /// Whether calls fail without an API key. Local services (Ollama,
    /// OmniRoute) can run without one.
    fn needs_key(self) -> bool {
        matches!(self, Self::Anthropic | Self::Gemini | Self::OpenRouter)
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
pub const DEFAULT_GEMINI_MODEL: &str = "gemini-flash-latest";
pub const DEFAULT_GEMINI_ENDPOINT: &str =
    "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions";
pub const DEFAULT_OMNIROUTE_ENDPOINT: &str = "http://localhost:20128/v1/chat/completions";
pub const DEFAULT_OPENROUTER_ENDPOINT: &str = "https://openrouter.ai/api/v1/chat/completions";
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
                    "AI_PROVIDER '{name}' is not supported (use gemini, omniroute, \
                     openrouter, ollama, openai, anthropic or mock)"
                ))
            })?,
            None => Self::detect_provider(&get),
        };
        let (default_model, default_endpoint) = provider.defaults();

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

        let api_key = provider
            .key_names()
            .iter()
            .find_map(|name| get(name))
            .map(Secret::new)
            .unwrap_or_default();

        Ok(Self {
            provider,
            model: get("AI_MODEL").unwrap_or_else(|| default_model.to_string()),
            endpoint: match get("AI_ENDPOINT") {
                Some(url) if provider.is_openai_style() => chat_completions_url(&url),
                Some(url) => url.trim().to_string(),
                None => default_endpoint.to_string(),
            },
            api_key,
            timeout: Duration::from_secs(timeout_secs),
            effort,
            max_requests_per_minute,
        })
    }

    /// With no `AI_PROVIDER`, the first provider whose own key variable is
    /// set; Gemini (free keys) when none is.
    fn detect_provider(get: &impl Fn(&str) -> Option<String>) -> ProviderKind {
        [
            ("GEMINI_API_KEY", ProviderKind::Gemini),
            ("GOOGLE_API_KEY", ProviderKind::Gemini),
            ("OMNIROUTE_API_KEY", ProviderKind::OmniRoute),
            ("OPENROUTER_API_KEY", ProviderKind::OpenRouter),
            ("ANTHROPIC_API_KEY", ProviderKind::Anthropic),
        ]
        .into_iter()
        .find(|(name, _)| get(name).is_some())
        .map_or(ProviderKind::Gemini, |(_, kind)| kind)
    }

    /// Why AI calls cannot work with this configuration, if they cannot.
    /// The bridge still starts: every request then gets an `AI_NOT_CONFIGURED`
    /// error frame and the kernel keeps working (SEC-009).
    pub fn problem(&self) -> Option<String> {
        if self.provider == ProviderKind::Mock {
            return None;
        }
        if self.provider.needs_key() && self.api_key.is_empty() {
            return Some(match self.provider {
                ProviderKind::Gemini => "no API key: set GEMINI_API_KEY (free from \
                    https://aistudio.google.com/apikey), or use AI_PROVIDER=ollama, \
                    omniroute or mock"
                    .to_string(),
                ProviderKind::OpenRouter => "no API key: set OPENROUTER_API_KEY".to_string(),
                _ => "no API key: set ANTHROPIC_API_KEY (or AI_API_KEY), or use a free \
                    provider such as AI_PROVIDER=gemini"
                    .to_string(),
            });
        }
        if self.model.trim().is_empty() {
            return Some(format!(
                "no model: set AI_MODEL to a model name from your {} account \
                 (for OmniRoute a model or combo from its dashboard)",
                self.provider.name()
            ));
        }
        if !(self.endpoint.starts_with("https://")
            || self.endpoint.starts_with("http://localhost")
            || self.endpoint.starts_with("http://127.0.0.1"))
        {
            return Some(format!(
                "AI_ENDPOINT must use https:// (plain http is only allowed for localhost): {}",
                self.endpoint
            ));
        }
        None
    }
}

/// Accepts a base URL such as `http://localhost:20128/v1` (what most
/// gateways show) as well as the full `.../chat/completions` URL.
fn chat_completions_url(url: &str) -> String {
    let url = url.trim().trim_end_matches('/');
    if url.ends_with("/chat/completions") {
        url.to_string()
    } else {
        format!("{url}/chat/completions")
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
    fn defaults_to_free_gemini() {
        let c = config(&[("GEMINI_API_KEY", "g-test")]).unwrap();
        assert_eq!(c.provider, ProviderKind::Gemini);
        assert_eq!(c.model, DEFAULT_GEMINI_MODEL);
        assert_eq!(c.endpoint, DEFAULT_GEMINI_ENDPOINT);
        assert_eq!(c.api_key.expose(), "g-test");
        assert_eq!(c.timeout, Duration::from_secs(25));
        assert!(c.problem().is_none());
    }

    #[test]
    fn provider_follows_the_key_that_is_set() {
        let c = config(&[("ANTHROPIC_API_KEY", "sk-test")]).unwrap();
        assert_eq!(c.provider, ProviderKind::Anthropic);
        assert_eq!(c.model, "claude-opus-5-5");
        let c = config(&[("OMNIROUTE_API_KEY", "o-test")]).unwrap();
        assert_eq!(c.provider, ProviderKind::OmniRoute);
        let c = config(&[("OPENROUTER_API_KEY", "r-test")]).unwrap();
        assert_eq!(c.provider, ProviderKind::OpenRouter);
    }

    #[test]
    fn missing_key_is_a_controlled_problem() {
        let c = config(&[]).unwrap();
        assert_eq!(c.provider, ProviderKind::Gemini);
        assert!(c.problem().unwrap().contains("GEMINI_API_KEY"));
        let c = config(&[("AI_PROVIDER", "anthropic")]).unwrap();
        assert!(c.problem().unwrap().contains("ANTHROPIC_API_KEY"));
    }

    #[test]
    fn omniroute_runs_locally_and_needs_a_model() {
        let c = config(&[("AI_PROVIDER", "omniroute")]).unwrap();
        assert_eq!(c.endpoint, DEFAULT_OMNIROUTE_ENDPOINT);
        assert!(c.problem().unwrap().contains("AI_MODEL"));
        let c = config(&[
            ("AI_PROVIDER", "omniroute"),
            ("AI_MODEL", "free-combo"),
            ("AI_ENDPOINT", "http://localhost:20128/v1/"),
        ])
        .unwrap();
        assert_eq!(c.endpoint, "http://localhost:20128/v1/chat/completions");
        assert!(c.problem().is_none()); // no key needed
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
