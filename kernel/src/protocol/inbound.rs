//! Frames the kernel receives from the AI Bridge.
//!
//! ```text
//! COM1 byte ─► input::serial (IRQ 4) ─ "@PSY …\n" line ─► push_raw_frame ─► RAW inbox
//!                                                                              │
//!            ai command ─► wait_for_reply ─► poll ─► parse_frame + validate ◄──┘
//! ```
//!
//! The serial interrupt handler only cuts frames out of the byte stream and
//! stores the raw line. Parsing and validation happen later, in normal kernel
//! context, when the `ai` command waits for its answer.
//!
//! Everything that arrives here is untrusted: a frame is accepted only if it
//! is valid JSON, uses a known message type and protocol version, refers to a
//! request id, and every field fits its fixed-size slot. Anything else is
//! rejected and counted; it can never execute anything.

use super::json::{self, Value};
use super::{MessageType, RequestId, MAX_FRAME_LENGTH, PROTOCOL_VERSION};
use crate::fixed::FixedString;
use crate::input::{self, Key};
use crate::logger::{self, Level, Subsystem};
use crate::ring::RingBuffer;
use core::fmt;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use spin::Mutex;
use x86_64::instructions::interrupts;

/// Longest accepted answer summary, in characters.
pub const MAX_SUMMARY_LENGTH: usize = 480;
/// Most suggestions shown for one answer.
pub const MAX_SUGGESTIONS: usize = 4;
pub const MAX_SUGGESTION_LENGTH: usize = 160;
/// Most suggested commands accepted for one answer.
pub const MAX_COMMANDS: usize = 3;
pub const MAX_COMMAND_LENGTH: usize = 80;
pub const MAX_ERROR_CODE_LENGTH: usize = 32;
pub const MAX_ERROR_MESSAGE_LENGTH: usize = 200;

/// How long `ai` waits once a bridge has been heard from.
pub const DEFAULT_BRIDGE_TIMEOUT_MS: u64 = 30_000;
/// How long `ai` waits when no bridge has ever sent a frame. Kept short so
/// the shell does not hang when the kernel runs without a bridge.
pub const DEFAULT_NO_BRIDGE_TIMEOUT_MS: u64 = 3_000;

/// Raw frames waiting to be parsed.
const RAW_CAPACITY: usize = 2;
/// Parsed replies waiting to be picked up by the `ai` command.
const MAILBOX_CAPACITY: usize = 4;

pub type RawFrame = FixedString<MAX_FRAME_LENGTH>;

/// A validated answer from the AI Bridge.
#[derive(Clone, Copy)]
pub struct AiResponse {
    pub request: RequestId,
    pub summary: FixedString<MAX_SUMMARY_LENGTH>,
    suggestions: [FixedString<MAX_SUGGESTION_LENGTH>; MAX_SUGGESTIONS],
    suggestion_count: usize,
    commands: [FixedString<MAX_COMMAND_LENGTH>; MAX_COMMANDS],
    command_count: usize,
    pub requires_confirmation: bool,
}

impl AiResponse {
    pub fn suggestions(&self) -> impl Iterator<Item = &str> {
        self.suggestions[..self.suggestion_count].iter().map(|s| s.as_str())
    }

    pub fn commands(&self) -> impl Iterator<Item = &str> {
        self.commands[..self.command_count].iter().map(|s| s.as_str())
    }
}

/// An error the AI Bridge reports instead of an answer, e.g. `AI_TIMEOUT`.
#[derive(Clone, Copy)]
pub struct BridgeError {
    pub request: Option<RequestId>,
    pub code: FixedString<MAX_ERROR_CODE_LENGTH>,
    pub message: FixedString<MAX_ERROR_MESSAGE_LENGTH>,
}

/// A frame that passed validation.
#[derive(Clone, Copy)]
pub enum Inbound {
    Response(AiResponse),
    Error(BridgeError),
    /// The bridge announcing itself (for example `bridge_ready`).
    Status,
}

/// Why a frame was rejected. `request` is set when the frame named a request
/// id, so the waiting command can fail at once instead of timing out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rejected {
    pub request: Option<RequestId>,
    pub reason: &'static str,
}

/// What `wait_for_reply` hands to the `ai` command.
#[derive(Clone, Copy)]
pub enum Reply {
    Answer(AiResponse),
    BridgeError(BridgeError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitError {
    /// No reply arrived in time. `bridge_seen` tells whether any bridge has
    /// ever talked to this kernel.
    Timeout { bridge_seen: bool },
    /// The user pressed Ctrl+C.
    Cancelled,
    /// A reply for this request arrived but failed validation.
    Invalid(&'static str),
}

#[derive(Clone, Copy)]
enum MailboxItem {
    Valid(Inbound),
    Invalid(Rejected),
}

static RAW: Mutex<RingBuffer<RawFrame, RAW_CAPACITY>> = Mutex::new(RingBuffer::new());
static MAILBOX: Mutex<RingBuffer<MailboxItem, MAILBOX_CAPACITY>> = Mutex::new(RingBuffer::new());

static FRAMES_RECEIVED: AtomicU32 = AtomicU32::new(0);
static FRAMES_REJECTED: AtomicU32 = AtomicU32::new(0);
static BRIDGE_SEEN: AtomicBool = AtomicBool::new(false);
static BRIDGE_TIMEOUT_MS: AtomicU64 = AtomicU64::new(DEFAULT_BRIDGE_TIMEOUT_MS);
static NO_BRIDGE_TIMEOUT_MS: AtomicU64 = AtomicU64::new(DEFAULT_NO_BRIDGE_TIMEOUT_MS);

/// Stores one complete frame line (without the `@PSY ` marker). Called from
/// the serial interrupt handler, so it only copies; if both slots are taken
/// the frame is dropped and counted as rejected.
pub fn push_raw_frame(frame: &RawFrame) {
    interrupts::without_interrupts(|| {
        if RAW.lock().push(*frame).is_err() {
            FRAMES_REJECTED.fetch_add(1, Ordering::Relaxed);
        }
    });
}

/// Records a frame that was cut off while it was being received (too long or
/// containing bytes that are not printable ASCII).
pub fn note_broken_frame() {
    FRAMES_REJECTED.fetch_add(1, Ordering::Relaxed);
}

/// Parses and validates one frame body.
pub fn parse_frame(text: &str) -> Result<Inbound, Rejected> {
    let reject = |reason: &'static str| Rejected { request: None, reason };
    let envelope = json::parse(text).map_err(|error| reject(error.0))?;
    if envelope.members().is_none() {
        return Err(reject("frame is not a JSON object"));
    }
    if envelope.get("version").and_then(Value::as_u32) != Some(PROTOCOL_VERSION) {
        return Err(reject("unsupported protocol version"));
    }
    let id = match envelope.get("id") {
        Some(Value::String(raw)) => raw,
        _ => return Err(reject("missing message id")),
    };
    let payload = match envelope.get("payload") {
        Some(payload @ Value::Object(_)) => payload,
        _ => return Err(reject("missing payload object")),
    };
    let message_type = match envelope.get("type") {
        Some(Value::String(raw)) => raw,
        _ => return Err(reject("missing message type")),
    };

    if message_type == MessageType::AiResponse.as_str() {
        let request = RequestId::parse(id).ok_or(reject("ai_response id is not a request id"))?;
        parse_response(request, payload)
            .map(Inbound::Response)
            .map_err(|reason| Rejected { request: Some(request), reason })
    } else if message_type == MessageType::Error.as_str() {
        // The error names the request either in the envelope id or in the
        // payload (`request_id`); an error without one is still accepted.
        let request = RequestId::parse(id).or_else(|| match payload.get("request_id") {
            Some(Value::String(raw)) => RequestId::parse(raw),
            _ => None,
        });
        parse_error(request, payload)
            .map(Inbound::Error)
            .map_err(|reason| Rejected { request, reason })
    } else if message_type == MessageType::Status.as_str() {
        Ok(Inbound::Status)
    } else {
        Err(reject("message type not accepted by the kernel"))
    }
}

fn parse_response(request: RequestId, payload: Value) -> Result<AiResponse, &'static str> {
    let mut response = AiResponse {
        request,
        summary: FixedString::new(),
        suggestions: [FixedString::new(); MAX_SUGGESTIONS],
        suggestion_count: 0,
        commands: [FixedString::new(); MAX_COMMANDS],
        command_count: 0,
        requires_confirmation: false,
    };

    match payload.get("summary") {
        Some(Value::String(raw)) => {
            json::decode_string(raw, &mut response.summary).map_err(|_| "summary is too long")?
        }
        _ => return Err("summary is missing"),
    }
    if response.summary.as_str().trim().is_empty() {
        return Err("summary is empty");
    }

    response.suggestion_count = string_list(
        payload.get("suggestions"),
        &mut response.suggestions,
        "too many suggestions",
        "suggestion is too long or not a string",
    )?;
    response.command_count = string_list(
        payload.get("commands"),
        &mut response.commands,
        "too many suggested commands",
        "suggested command is too long or not a string",
    )?;

    response.requires_confirmation = match payload.get("requiresConfirmation") {
        None | Some(Value::Null) => false,
        Some(Value::Bool(flag)) => flag,
        Some(_) => return Err("requiresConfirmation is not a boolean"),
    };
    Ok(response)
}

/// Decodes an optional array of strings into `slots`, returning the count.
fn string_list<const N: usize, const M: usize>(
    value: Option<Value>,
    slots: &mut [FixedString<N>; M],
    too_many: &'static str,
    bad_item: &'static str,
) -> Result<usize, &'static str> {
    let items = match value {
        None | Some(Value::Null) => return Ok(0),
        Some(value) => value.items().ok_or("list field is not an array")?,
    };
    let mut count = 0;
    for item in items {
        let slot = slots.get_mut(count).ok_or(too_many)?;
        match item {
            Value::String(raw) => json::decode_string(raw, slot).map_err(|_| bad_item)?,
            _ => return Err(bad_item),
        }
        count += 1;
    }
    Ok(count)
}

fn parse_error(request: Option<RequestId>, payload: Value) -> Result<BridgeError, &'static str> {
    let mut error = BridgeError {
        request,
        code: FixedString::new(),
        message: FixedString::new(),
    };
    match payload.get("code") {
        Some(Value::String(raw)) => {
            json::decode_string(raw, &mut error.code).map_err(|_| "error code is too long")?
        }
        _ => return Err("error code is missing"),
    }
    match payload.get("message") {
        None | Some(Value::Null) => {}
        Some(Value::String(raw)) => {
            json::decode_string(raw, &mut error.message).map_err(|_| "error message is too long")?
        }
        Some(_) => return Err("error message is not a string"),
    }
    Ok(error)
}

/// Parses every raw frame that has arrived and files the results in the
/// mailbox. Runs in normal kernel context.
pub fn poll() {
    loop {
        let Some(raw) = interrupts::without_interrupts(|| RAW.lock().pop()) else {
            return;
        };
        FRAMES_RECEIVED.fetch_add(1, Ordering::Relaxed);
        let item = match parse_frame(raw.as_str()) {
            Ok(inbound) => {
                BRIDGE_SEEN.store(true, Ordering::Relaxed);
                match inbound {
                    Inbound::Status => continue,
                    other => MailboxItem::Valid(other),
                }
            }
            Err(rejected) => {
                FRAMES_REJECTED.fetch_add(1, Ordering::Relaxed);
                logger::record(
                    Level::Warn,
                    Subsystem::Ai,
                    "COMM-002",
                    format_args!("Rejected frame from the AI Bridge: {}", rejected.reason),
                );
                if rejected.request.is_none() {
                    continue;
                }
                MailboxItem::Invalid(rejected)
            }
        };
        interrupts::without_interrupts(|| MAILBOX.lock().push_overwrite(item));
    }
}

/// Removes the mailbox entry for `request`, discarding entries for other
/// (older, abandoned) requests on the way.
fn take_reply(request: RequestId) -> Option<MailboxItem> {
    interrupts::without_interrupts(|| {
        let mut mailbox = MAILBOX.lock();
        while let Some(item) = mailbox.pop() {
            let item_request = match item {
                MailboxItem::Valid(Inbound::Response(response)) => Some(response.request),
                MailboxItem::Valid(Inbound::Error(error)) => error.request,
                MailboxItem::Valid(Inbound::Status) => None,
                MailboxItem::Invalid(rejected) => rejected.request,
            };
            // An error without a request id is about the current exchange.
            let unaddressed_error = matches!(item, MailboxItem::Valid(Inbound::Error(_)))
                && item_request.is_none();
            if item_request == Some(request) || unaddressed_error {
                return Some(item);
            }
        }
        None
    })
}

/// Waits for the reply to `request`. Returns early on Ctrl+C. Other keys
/// typed while waiting are discarded.
pub fn wait_for_reply(request: RequestId) -> Result<Reply, WaitError> {
    poll();
    let bridge_seen = bridge_seen();
    let timeout = if bridge_seen {
        BRIDGE_TIMEOUT_MS.load(Ordering::Relaxed)
    } else {
        NO_BRIDGE_TIMEOUT_MS.load(Ordering::Relaxed)
    };
    let deadline = crate::time::uptime_ms().saturating_add(timeout);

    loop {
        poll();
        match take_reply(request) {
            Some(MailboxItem::Valid(Inbound::Response(response))) => return Ok(Reply::Answer(response)),
            Some(MailboxItem::Valid(Inbound::Error(error))) => return Ok(Reply::BridgeError(error)),
            Some(MailboxItem::Invalid(rejected)) => return Err(WaitError::Invalid(rejected.reason)),
            Some(MailboxItem::Valid(Inbound::Status)) | None => {}
        }
        while let Some(key) = input::try_read_key() {
            if key == Key::Interrupt {
                return Err(WaitError::Cancelled);
            }
        }
        if crate::time::uptime_ms() >= deadline {
            return Err(WaitError::Timeout { bridge_seen: bridge_seen || self::bridge_seen() });
        }
        if interrupts::are_enabled() {
            // The timer (100 Hz) or the next serial byte wakes us up.
            x86_64::instructions::hlt();
        } else {
            core::hint::spin_loop();
        }
    }
}

/// `true` once any valid frame has arrived from a bridge since boot.
pub fn bridge_seen() -> bool {
    BRIDGE_SEEN.load(Ordering::Relaxed)
}

/// Frames taken from the serial line since boot (valid or not).
pub fn frames_received() -> u32 {
    FRAMES_RECEIVED.load(Ordering::Relaxed)
}

/// Frames rejected since boot (invalid, too long, or inbox full).
pub fn frames_rejected() -> u32 {
    FRAMES_REJECTED.load(Ordering::Relaxed)
}

/// Overrides the reply timeouts. Used by tests to keep them fast.
pub fn set_timeouts(bridge_ms: u64, no_bridge_ms: u64) {
    BRIDGE_TIMEOUT_MS.store(bridge_ms, Ordering::Relaxed);
    NO_BRIDGE_TIMEOUT_MS.store(no_bridge_ms, Ordering::Relaxed);
}

/// Writes `text` for the ASCII-only console: non-ASCII characters become
/// `?` and control characters become spaces, so a reply cannot move the
/// cursor or inject terminal escape sequences.
pub struct Printable<'a>(pub &'a str);

impl fmt::Display for Printable<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use fmt::Write;
        for c in self.0.chars() {
            f.write_char(match c {
                c if c.is_ascii_control() => ' ',
                c if c.is_ascii() => c,
                _ => '?',
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
fn response_ok(text: &str) -> AiResponse {
    match parse_frame(text) {
        Ok(Inbound::Response(response)) => response,
        Ok(_) => panic!("not a response"),
        Err(rejected) => panic!("rejected: {}", rejected.reason),
    }
}

#[test_case]
fn valid_ai_response_is_accepted() {
    let response = response_ok(
        r#"{"type":"ai_response","version":1,"id":"req-004","payload":{
            "summary":"The heap could not satisfy a 64 MiB request.",
            "suggestions":["Run meminfo","Request less memory"],
            "commands":["meminfo"],
            "requiresConfirmation":true,
            "limitations":["unknown fields are ignored"]}}"#,
    );
    assert_eq!(response.request, RequestId(4));
    assert_eq!(response.summary, "The heap could not satisfy a 64 MiB request.");
    let mut suggestions = response.suggestions();
    assert_eq!(suggestions.next(), Some("Run meminfo"));
    assert_eq!(suggestions.next(), Some("Request less memory"));
    assert_eq!(suggestions.next(), None);
    assert_eq!(response.commands().next(), Some("meminfo"));
    assert!(response.requires_confirmation);
}

#[test_case]
fn optional_response_fields_default_to_empty() {
    let response = response_ok(
        r#"{"type":"ai_response","version":1,"id":"req-001","payload":{"summary":"ok"}}"#,
    );
    assert_eq!(response.suggestions().count(), 0);
    assert_eq!(response.commands().count(), 0);
    assert!(!response.requires_confirmation);
}

#[test_case]
fn invalid_responses_are_rejected_with_their_request() {
    let cases = [
        (r#"{"type":"ai_response","version":1,"id":"req-002","payload":{}}"#, "summary is missing"),
        (r#"{"type":"ai_response","version":1,"id":"req-002","payload":{"summary":"  "}}"#, "summary is empty"),
        (
            r#"{"type":"ai_response","version":1,"id":"req-002","payload":{"summary":"x","suggestions":["1","2","3","4","5"]}}"#,
            "too many suggestions",
        ),
        (
            r#"{"type":"ai_response","version":1,"id":"req-002","payload":{"summary":"x","commands":[1]}}"#,
            "suggested command is too long or not a string",
        ),
        (
            r#"{"type":"ai_response","version":1,"id":"req-002","payload":{"summary":"x","requiresConfirmation":"yes"}}"#,
            "requiresConfirmation is not a boolean",
        ),
    ];
    for (text, reason) in cases {
        let rejected = parse_frame(text).err().expect("must be rejected");
        assert_eq!(rejected, Rejected { request: Some(RequestId(2)), reason });
    }
}

#[test_case]
fn overlong_summary_is_rejected_not_truncated() {
    let mut frame: RawFrame = FixedString::new();
    frame.push_str(r#"{"type":"ai_response","version":1,"id":"req-003","payload":{"summary":""#);
    for _ in 0..MAX_SUMMARY_LENGTH + 1 {
        frame.push('a');
    }
    frame.push_str("\"}}");
    let rejected = parse_frame(frame.as_str()).err().unwrap();
    assert_eq!(rejected.reason, "summary is too long");
}

#[test_case]
fn bad_envelopes_are_rejected() {
    for text in [
        "not json",
        "[1,2]",
        r#"{"type":"ai_response","version":2,"id":"req-001","payload":{"summary":"x"}}"#,
        r#"{"type":"ai_response","version":1,"id":"resp-001","payload":{"summary":"x"}}"#,
        r#"{"type":"ai_response","version":1,"id":"req-001","payload":"x"}"#,
        r#"{"type":"ai_request","version":1,"id":"req-001","payload":{"query":"x"}}"#,
        r#"{"type":"diagnostic","version":1,"id":"diag-001","payload":{}}"#,
    ] {
        let rejected = parse_frame(text).err().expect("must be rejected");
        assert_eq!(rejected.request, None);
    }
}

#[test_case]
fn bridge_errors_and_status_are_accepted() {
    match parse_frame(
        r#"{"type":"error","version":1,"id":"err-001","payload":{"code":"AI_TIMEOUT","message":"The AI service timed out.","request_id":"req-009"}}"#,
    ) {
        Ok(Inbound::Error(error)) => {
            assert_eq!(error.request, Some(RequestId(9)));
            assert_eq!(error.code, "AI_TIMEOUT");
            assert_eq!(error.message, "The AI service timed out.");
        }
        _ => panic!("error frame not accepted"),
    }
    assert!(matches!(
        parse_frame(r#"{"type":"status","version":1,"id":"stat-001","payload":{"state":"bridge_ready"}}"#),
        Ok(Inbound::Status)
    ));
}

#[test_case]
fn printable_strips_control_and_non_ascii() {
    use core::fmt::Write;
    let mut out: FixedString<32> = FixedString::new();
    write!(out, "{}", Printable("a\u{1b}[2Jb\tc\u{e9}")).unwrap();
    assert_eq!(out, "a [2Jb c?");
}

#[test_case]
fn replies_are_matched_to_their_request() {
    set_timeouts(200, 200);
    let mut frame: RawFrame = FixedString::new();
    // A stale reply for an older request is skipped ...
    frame.push_str(r#"{"type":"ai_response","version":1,"id":"req-900","payload":{"summary":"old"}}"#);
    push_raw_frame(&frame);
    frame.clear();
    frame.push_str(r#"{"type":"ai_response","version":1,"id":"req-901","payload":{"summary":"new"}}"#);
    push_raw_frame(&frame);
    match wait_for_reply(RequestId(901)) {
        Ok(Reply::Answer(response)) => assert_eq!(response.summary, "new"),
        _ => panic!("reply not delivered"),
    }
    assert!(bridge_seen());
    // ... and nothing is left behind for the next request.
    assert_eq!(wait_for_reply(RequestId(902)).err(), Some(WaitError::Timeout { bridge_seen: true }));
    set_timeouts(DEFAULT_BRIDGE_TIMEOUT_MS, DEFAULT_NO_BRIDGE_TIMEOUT_MS);
}
