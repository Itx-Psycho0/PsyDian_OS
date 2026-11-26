//! The AI provider abstraction (Docs/10-genai-architecture.md section 19).
//! The kernel never knows which provider answered.

pub mod anthropic;
pub mod mock;
pub mod openai;

use crate::config::{Config, ProviderKind};
use crate::context::Context;
use std::fmt;
use std::time::Duration;

/// Why a provider could not produce an answer. Each maps to an `error`
/// frame code the kernel shows to the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderError {
    NotConfigured(String),
    Auth,
    RateLimited,
    Timeout,
    Unavailable(String),
    BadRequest(String),
    Refused,
    Truncated,
    InvalidResponse(String),
}

impl ProviderError {
    /// The `code` of the error frame (at most 32 characters).
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotConfigured(_) => "AI_NOT_CONFIGURED",
            Self::Auth => "AI_AUTH_FAILED",
            Self::RateLimited => "AI_RATE_LIMITED",
            Self::Timeout => "AI_TIMEOUT",
            Self::Unavailable(_) => "AI_SERVICE_UNAVAILABLE",
            Self::BadRequest(_) => "AI_BAD_REQUEST",
            Self::Refused => "AI_REFUSED",
            Self::Truncated | Self::InvalidResponse(_) => "AI_INVALID_RESPONSE",
        }
    }

    /// A short message for the user (shown by the kernel).
    pub fn user_message(&self) -> String {
        match self {
            Self::NotConfigured(why) => format!("The AI Bridge is not configured: {why}."),
            Self::Auth => "The AI service rejected the API key.".into(),
            Self::RateLimited => "Too many AI requests. Wait a minute and try again.".into(),
            Self::Timeout => "The AI service did not answer in time.".into(),
            Self::Unavailable(_) => "The AI service could not be reached. Try again later.".into(),
            Self::BadRequest(_) => "The AI service rejected the request (check AI_MODEL).".into(),
            Self::Refused => "The AI service declined to answer this question.".into(),
            Self::Truncated => "The AI answer was cut off.".into(),
            Self::InvalidResponse(_) => {
                "The AI service sent an answer the bridge could not use.".into()
            }
        }
    }
}

impl fmt::Display for ProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConfigured(d)
            | Self::Unavailable(d)
            | Self::BadRequest(d)
            | Self::InvalidResponse(d) => {
                write!(f, "{}: {}", self.code(), d)
            }
            _ => f.write_str(self.code()),
        }
    }
}

pub trait Provider: Send {
    /// Provider and model, for the log.
    fn describe(&self) -> String;

    /// Returns the model's raw text answer (expected to be the JSON object
    /// described in the system prompt).
    fn complete(
        &self,
        system: &str,
        user: &str,
        context: &Context,
    ) -> Result<String, ProviderError>;
}

pub fn from_config(config: &Config) -> Box<dyn Provider> {
    match config.provider {
        ProviderKind::Anthropic => Box::new(anthropic::Anthropic::new(config)),
        ProviderKind::OpenAiCompatible => Box::new(openai::OpenAiCompatible::new(config)),
        ProviderKind::Mock => Box::new(mock::Mock),
    }
}

/// HTTP agent with an overall deadline per call.
pub(crate) fn agent(timeout: Duration) -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(timeout)
        .user_agent(concat!("psydian-ai-bridge/", env!("CARGO_PKG_VERSION")))
        .build()
}

/// Maps a ureq failure. `error_message` extracts the service's own error
/// text from a JSON error body.
pub(crate) fn http_error(error: ureq::Error) -> ProviderError {
    match error {
        ureq::Error::Status(code, response) => {
            let body = response.into_string().unwrap_or_default();
            let detail = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|v| {
                    v.pointer("/error/message")
                        .or_else(|| v.get("message"))
                        .and_then(|m| m.as_str())
                        .map(str::to_string)
                })
                .unwrap_or_else(|| format!("HTTP {code}"));
            match code {
                401 | 403 => ProviderError::Auth,
                408 => ProviderError::Timeout,
                429 => ProviderError::RateLimited,
                400 | 404 | 413 | 422 => ProviderError::BadRequest(detail),
                _ => ProviderError::Unavailable(format!("HTTP {code}: {detail}")),
            }
        }
        ureq::Error::Transport(transport) => {
            let text = transport.to_string();
            if text.contains("timed out") || text.contains("Timeout") {
                ProviderError::Timeout
            } else {
                ProviderError::Unavailable(text)
            }
        }
    }
}

/// Whether one quick retry is worth it (overloaded or a dropped connection).
pub(crate) fn retryable(error: &ProviderError) -> bool {
    matches!(error, ProviderError::Unavailable(_))
}
