//! Answers one `ai_request`: rate limit, context, prompt, provider call,
//! validation and safety, then the frame for the kernel.
//!
//! ```text
//! ai_request ─► rate limit ─► context (+ diagnostics) ─► prompt ─► provider
//!                                                                    │
//!        ai_response / error frame ◄─ safety ◄─ validation ◄─────────┘
//! ```
//!
//! Every failure becomes an `error` frame, so the kernel always gets a
//! reply and its shell keeps working.

use crate::context;
use crate::diagnostics::DiagnosticStore;
use crate::prompt;
use crate::protocol::{self, AiRequest};
use crate::provider::{Provider, ProviderError};
use crate::validation;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// At most `limit` requests in any 60 s window (SEC-012).
pub struct RateLimiter {
    limit: usize,
    window: Duration,
    recent: VecDeque<Instant>,
}

impl RateLimiter {
    pub fn per_minute(limit: u32) -> Self {
        Self {
            limit: limit as usize,
            window: Duration::from_secs(60),
            recent: VecDeque::new(),
        }
    }

    pub fn allow(&mut self, now: Instant) -> bool {
        while self
            .recent
            .front()
            .is_some_and(|&t| now.duration_since(t) >= self.window)
        {
            self.recent.pop_front();
        }
        if self.recent.len() >= self.limit {
            return false;
        }
        self.recent.push_back(now);
        true
    }
}

/// What happened to a request, for the terminal and the log.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    /// The frame to send to the kernel.
    pub frame: String,
    /// `answered`, or the error code.
    pub result: String,
    /// Extra detail for the log (dropped commands, provider error).
    pub detail: Option<String>,
    pub latency: Duration,
}

pub struct Answerer {
    pub provider: Box<dyn Provider>,
    /// Set when the configuration cannot work (e.g. no API key).
    pub config_problem: Option<String>,
    pub limiter: RateLimiter,
}

impl Answerer {
    pub fn answer(&mut self, request: &AiRequest, store: &DiagnosticStore) -> Outcome {
        let started = Instant::now();
        let fail = |error: ProviderError, started: Instant| Outcome {
            frame: protocol::error_frame(Some(&request.id), error.code(), &error.user_message()),
            result: error.code().to_string(),
            detail: Some(error.to_string()),
            latency: started.elapsed(),
        };

        if let Some(problem) = &self.config_problem {
            return fail(ProviderError::NotConfigured(problem.clone()), started);
        }
        if !self.limiter.allow(started) {
            return fail(ProviderError::RateLimited, started);
        }

        let context = context::build(request, store);
        let raw = match self.provider.complete(
            &prompt::system_prompt(),
            &prompt::user_message(&context),
            &context,
        ) {
            Ok(raw) => raw,
            Err(error) => return fail(error, started),
        };
        match validation::validate(&raw) {
            Ok(valid) => Outcome {
                frame: protocol::ai_response_frame(&request.id, &valid.answer),
                result: "answered".into(),
                detail: (!valid.dropped_commands.is_empty()).then(|| {
                    let dropped: Vec<String> = valid
                        .dropped_commands
                        .iter()
                        .map(|(c, why)| format!("'{c}' ({why})"))
                        .collect();
                    format!("dropped suggested commands: {}", dropped.join(", "))
                }),
                latency: started.elapsed(),
            },
            Err(invalid) => fail(ProviderError::InvalidResponse(invalid.0.into()), started),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{Diagnostic, frame_body};
    use crate::provider::mock::Mock;
    use serde_json::Value;

    fn answerer() -> Answerer {
        Answerer {
            provider: Box::new(Mock),
            config_problem: None,
            limiter: RateLimiter::per_minute(100),
        }
    }

    fn request(query: &str, diagnostic_id: Option<&str>) -> AiRequest {
        AiRequest {
            id: "req-004".into(),
            query: query.into(),
            diagnostic_id: diagnostic_id.map(Into::into),
        }
    }

    fn payload(outcome: &Outcome) -> Value {
        let v: Value = serde_json::from_str(frame_body(&outcome.frame).unwrap()).unwrap();
        assert_eq!(v["version"], 1);
        assert_eq!(v["id"], "req-004");
        v
    }

    #[test]
    fn answers_with_the_referenced_diagnostic() {
        let mut store = DiagnosticStore::new();
        store.add(Diagnostic {
            id: "diag-001".into(),
            severity: "error".into(),
            subsystem: "memory".into(),
            code: "MEM-003".into(),
            message: "Heap allocation failed".into(),
            uptime_ms: 5,
            context: vec![
                ("size".into(), "67108864".into()),
                ("heap_size".into(), "1048576".into()),
            ],
        });
        let outcome = answerer().answer(&request("why did it fail?", Some("diag-001")), &store);
        assert_eq!(outcome.result, "answered");
        let v = payload(&outcome);
        assert_eq!(v["type"], "ai_response");
        assert!(v["payload"]["summary"].as_str().unwrap().contains("64 MiB"));
        assert_eq!(v["payload"]["commands"][0], "meminfo");
    }

    #[test]
    fn dangerous_suggestions_are_filtered() {
        let outcome =
            answerer().answer(&request("can you crash it?", None), &DiagnosticStore::new());
        let v = payload(&outcome);
        assert_eq!(v["payload"]["commands"], serde_json::json!(["shutdown"]));
        assert_eq!(v["payload"]["requiresConfirmation"], true);
        assert!(outcome.detail.unwrap().contains("panic-test"));
    }

    #[test]
    fn failures_become_error_frames() {
        let mut a = answerer();
        for (query, code) in [
            ("trigger a timeout", "AI_TIMEOUT"),
            ("simulate an error", "AI_SERVICE_UNAVAILABLE"),
            ("return garbage", "AI_INVALID_RESPONSE"),
        ] {
            let outcome = a.answer(&request(query, None), &DiagnosticStore::new());
            let v = payload(&outcome);
            assert_eq!(v["type"], "error");
            assert_eq!(v["payload"]["code"], code);
            assert_eq!(v["payload"]["request_id"], "req-004");
        }
    }

    #[test]
    fn missing_configuration_is_reported_not_fatal() {
        let mut a = answerer();
        a.config_problem = Some("no API key".into());
        let outcome = a.answer(&request("hi", None), &DiagnosticStore::new());
        assert_eq!(payload(&outcome)["payload"]["code"], "AI_NOT_CONFIGURED");
    }

    #[test]
    fn repeated_requests_are_rate_limited() {
        let mut a = answerer();
        a.limiter = RateLimiter::per_minute(2);
        let store = DiagnosticStore::new();
        assert_eq!(a.answer(&request("hi", None), &store).result, "answered");
        assert_eq!(a.answer(&request("hi", None), &store).result, "answered");
        assert_eq!(
            a.answer(&request("hi", None), &store).result,
            "AI_RATE_LIMITED"
        );
    }

    #[test]
    fn rate_limit_window_slides() {
        let mut limiter = RateLimiter::per_minute(1);
        let start = Instant::now();
        assert!(limiter.allow(start));
        assert!(!limiter.allow(start + Duration::from_secs(30)));
        assert!(limiter.allow(start + Duration::from_secs(61)));
    }
}
