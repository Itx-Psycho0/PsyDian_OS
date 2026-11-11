//! Kernel <-> AI Bridge message protocol (kernel side).
//!
//! The kernel has no network stack. Everything it wants the host-side AI
//! Bridge to see is written to the serial port as a *frame*: one line of text
//! that starts with a marker and carries a JSON envelope.
//!
//! ```text
//! @PSY {"type":"diagnostic","version":1,"id":"diag-001","payload":{...}}
//! ```
//!
//! * The marker [`FRAME_PREFIX`] separates frames from ordinary console text
//!   that shares the serial line (boot logs, shell output). A frame runs from
//!   the marker to the end of the line.
//! * The envelope (`type`, `version`, `id`, `payload`) is the one specified
//!   in Docs/08-api-specification.md section 8.
//! * JSON is produced by a small hand-written encoder: the kernel only ever
//!   *writes* a handful of fixed shapes, so a general serializer is not
//!   needed and nothing here allocates.

use crate::diagnostics::{Diagnostic, DiagnosticId};
use crate::fixed::FixedString;
use core::fmt::{self, Write};
use core::sync::atomic::{AtomicU32, Ordering};
use x86_64::instructions::interrupts::without_interrupts;

/// Version of the envelope format. Sent in every message.
pub const PROTOCOL_VERSION: u32 = 1;

/// Marks the start of a frame on the serial line.
pub const FRAME_PREFIX: &str = "@PSY ";

/// Upper bound for one encoded message, in bytes.
pub const MAX_FRAME_LENGTH: usize = 2048;

/// A complete JSON message, ready to be framed.
pub type Message = FixedString<MAX_FRAME_LENGTH>;

/// Message types of the protocol. The kernel currently sends `Diagnostic`,
/// `AiRequest` and `Status`; `AiResponse` and `Error` travel from the bridge
/// to the kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    AiRequest,
    Diagnostic,
    AiResponse,
    Error,
    Status,
}

impl MessageType {
    pub const fn as_str(self) -> &'static str {
        match self {
            MessageType::AiRequest => "ai_request",
            MessageType::Diagnostic => "diagnostic",
            MessageType::AiResponse => "ai_response",
            MessageType::Error => "error",
            MessageType::Status => "status",
        }
    }
}

/// Identifier of an AI request, formatted as `req-001`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestId(pub u32);

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "req-{:03}", self.0)
    }
}

static FRAMES_SENT: AtomicU32 = AtomicU32::new(0);
static FRAMES_DROPPED: AtomicU32 = AtomicU32::new(0);
static NEXT_REQUEST_ID: AtomicU32 = AtomicU32::new(1);
static NEXT_STATUS_ID: AtomicU32 = AtomicU32::new(1);

/// Writes `text` as a JSON string literal, including the quotes.
pub fn write_json_string(out: &mut impl Write, text: &str) -> fmt::Result {
    out.write_char('"')?;
    for c in text.chars() {
        match c {
            '"' => out.write_str("\\\"")?,
            '\\' => out.write_str("\\\\")?,
            '\n' => out.write_str("\\n")?,
            '\r' => out.write_str("\\r")?,
            '\t' => out.write_str("\\t")?,
            c if (c as u32) < 0x20 => write!(out, "\\u{:04x}", c as u32)?,
            c => out.write_char(c)?,
        }
    }
    out.write_char('"')
}

/// Writes the opening of an envelope up to and including `"payload":`.
fn write_envelope_start(
    out: &mut impl Write,
    message_type: MessageType,
    id: impl fmt::Display,
) -> fmt::Result {
    write!(
        out,
        "{{\"type\":\"{}\",\"version\":{},\"id\":\"{}\",\"payload\":",
        message_type.as_str(),
        PROTOCOL_VERSION,
        id
    )
}

/// Encodes a diagnostic record.
pub fn encode_diagnostic(out: &mut impl Write, diagnostic: &Diagnostic) -> fmt::Result {
    write_envelope_start(out, MessageType::Diagnostic, DiagnosticId(diagnostic.id))?;
    write!(
        out,
        "{{\"severity\":\"{}\",\"subsystem\":\"{}\",\"code\":\"{}\",\"uptime_ms\":{},\"message\":",
        diagnostic.severity.as_lower_str(),
        diagnostic.subsystem.as_lower_str(),
        diagnostic.code,
        diagnostic.uptime_ms
    )?;
    write_json_string(out, diagnostic.message.as_str())?;
    out.write_str(",\"context\":{")?;
    for (index, (key, value)) in diagnostic.context().enumerate() {
        if index > 0 {
            out.write_char(',')?;
        }
        write_json_string(out, key)?;
        out.write_char(':')?;
        write_json_string(out, value)?;
    }
    out.write_str("}}}")
}

/// Encodes a question for the AI assistant. `diagnostic` optionally names
/// the diagnostic record the question refers to.
pub fn encode_ai_request(
    out: &mut impl Write,
    id: RequestId,
    query: &str,
    diagnostic: Option<DiagnosticId>,
) -> fmt::Result {
    write_envelope_start(out, MessageType::AiRequest, id)?;
    out.write_str("{\"query\":")?;
    write_json_string(out, query)?;
    match diagnostic {
        Some(diagnostic) => write!(out, ",\"diagnostic_id\":\"{}\"", diagnostic)?,
        None => out.write_str(",\"diagnostic_id\":null")?,
    }
    out.write_str("}}")
}

/// Encodes a kernel status announcement such as `ready`.
pub fn encode_status(out: &mut impl Write, sequence: u32, state: &str) -> fmt::Result {
    write_envelope_start(out, MessageType::Status, format_args!("stat-{:03}", sequence))?;
    out.write_str("{\"state\":")?;
    write_json_string(out, state)?;
    write!(
        out,
        ",\"kernel\":\"{}\",\"uptime_ms\":{}}}}}",
        env!("CARGO_PKG_VERSION"),
        crate::time::uptime_ms()
    )
}

/// `true` if `message` is a complete envelope, i.e. it was not cut off by the
/// size limit.
fn is_complete(message: &Message) -> bool {
    message.len() < MAX_FRAME_LENGTH && message.as_str().ends_with("}}")
}

/// Sends one encoded message as a frame on the serial port.
///
/// The whole frame is written under a single lock so console output from an
/// interrupt cannot end up in the middle of it. A message that exceeded
/// [`MAX_FRAME_LENGTH`] would be invalid JSON, so it is dropped (and counted)
/// rather than sent. Returns whether the frame was sent.
pub fn send(message: &Message) -> bool {
    if !is_complete(message) {
        FRAMES_DROPPED.fetch_add(1, Ordering::Relaxed);
        return false;
    }
    without_interrupts(|| {
        let mut serial = crate::serial::SERIAL1.lock();
        let _ = serial.write_str(FRAME_PREFIX);
        let _ = serial.write_str(message.as_str());
        let _ = serial.write_char('\n');
    });
    FRAMES_SENT.fetch_add(1, Ordering::Relaxed);
    true
}

/// Encodes and sends a diagnostic record.
pub fn send_diagnostic(diagnostic: &Diagnostic) -> bool {
    let mut message = Message::new();
    let _ = encode_diagnostic(&mut message, diagnostic);
    send(&message)
}

/// Encodes and sends an AI request. Returns the id assigned to it, or `None`
/// if the request did not fit into a frame.
pub fn send_ai_request(query: &str, diagnostic: Option<DiagnosticId>) -> Option<RequestId> {
    let id = RequestId(NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed));
    let mut message = Message::new();
    let _ = encode_ai_request(&mut message, id, query, diagnostic);
    send(&message).then_some(id)
}

/// Announces a kernel state change (for example `ready`) to the bridge.
pub fn send_status(state: &str) -> bool {
    let sequence = NEXT_STATUS_ID.fetch_add(1, Ordering::Relaxed);
    let mut message = Message::new();
    let _ = encode_status(&mut message, sequence, state);
    send(&message)
}

/// Frames written to the serial port since boot.
pub fn frames_sent() -> u32 {
    FRAMES_SENT.load(Ordering::Relaxed)
}

/// Messages dropped because they exceeded the frame size limit.
pub fn frames_dropped() -> u32 {
    FRAMES_DROPPED.load(Ordering::Relaxed)
}

#[cfg(test)]
use crate::logger::{Level, Subsystem};

#[test_case]
fn json_strings_are_escaped() {
    let mut out: FixedString<64> = FixedString::new();
    write_json_string(&mut out, "say \"hi\"\\\n\tend\u{1}").unwrap();
    assert_eq!(out, "\"say \\\"hi\\\"\\\\\\n\\tend\\u0001\"");
}

#[test_case]
fn diagnostic_envelope_matches_the_specification() {
    let mut diagnostic = Diagnostic::new(
        Level::Error,
        Subsystem::Memory,
        "MEM-003",
        format_args!("Heap allocation failed"),
    )
    .with("size", 4096)
    .with("align", 8);
    diagnostic.id = 1;
    diagnostic.uptime_ms = 1500;

    let mut message = Message::new();
    encode_diagnostic(&mut message, &diagnostic).unwrap();
    assert_eq!(
        message,
        "{\"type\":\"diagnostic\",\"version\":1,\"id\":\"diag-001\",\"payload\":{\
         \"severity\":\"error\",\"subsystem\":\"memory\",\"code\":\"MEM-003\",\
         \"uptime_ms\":1500,\"message\":\"Heap allocation failed\",\
         \"context\":{\"size\":\"4096\",\"align\":\"8\"}}}"
    );
    assert!(is_complete(&message));
}

#[test_case]
fn diagnostic_without_context_has_an_empty_object() {
    let mut diagnostic = Diagnostic::new(
        Level::Warn,
        Subsystem::Shell,
        "CMD-001",
        format_args!("x"),
    );
    diagnostic.id = 12;
    diagnostic.uptime_ms = 0;
    let mut message = Message::new();
    encode_diagnostic(&mut message, &diagnostic).unwrap();
    assert!(message.as_str().ends_with("\"message\":\"x\",\"context\":{}}}"));
    assert!(message.as_str().contains("\"id\":\"diag-012\""));
}

#[test_case]
fn ai_request_envelope() {
    let mut message = Message::new();
    encode_ai_request(
        &mut message,
        RequestId(7),
        "why did \"it\" panic?",
        Some(DiagnosticId(3)),
    )
    .unwrap();
    assert_eq!(
        message,
        "{\"type\":\"ai_request\",\"version\":1,\"id\":\"req-007\",\"payload\":{\
         \"query\":\"why did \\\"it\\\" panic?\",\"diagnostic_id\":\"diag-003\"}}"
    );

    let mut message = Message::new();
    encode_ai_request(&mut message, RequestId(8), "hello", None).unwrap();
    assert!(message.as_str().ends_with("\"diagnostic_id\":null}}"));
}

#[test_case]
fn status_envelope() {
    let mut message = Message::new();
    encode_status(&mut message, 1, "ready").unwrap();
    assert!(message.as_str().starts_with(
        "{\"type\":\"status\",\"version\":1,\"id\":\"stat-001\",\"payload\":{\"state\":\"ready\","
    ));
    assert!(is_complete(&message));
}

#[test_case]
fn truncated_message_is_dropped_not_sent() {
    let mut message = Message::new();
    while message.push('x') {}
    let sent = frames_sent();
    let dropped = frames_dropped();
    assert!(!send(&message));
    assert_eq!(frames_sent(), sent);
    assert_eq!(frames_dropped(), dropped + 1);
}

#[test_case]
fn sending_counts_frames() {
    let sent = frames_sent();
    assert!(send_status("test"));
    let id = send_ai_request("unit test request", None).expect("request must fit a frame");
    assert!(id.0 >= 1);
    assert_eq!(frames_sent(), sent + 2);
}
