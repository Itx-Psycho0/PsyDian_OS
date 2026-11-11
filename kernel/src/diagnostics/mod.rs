//! Structured diagnostic records.
//!
//! A log line says *that* something happened. A diagnostic record describes a
//! failure precisely enough for software to analyse it: a stable identifier,
//! severity, subsystem, error code, message and key/value context.
//!
//! ```text
//! Failure ─► Diagnostic::new(..).with(..) ─► report()
//!                                              ├─► stored in a bounded history   (`diag` command)
//!                                              ├─► logged through the logger     (console + `logs`)
//!                                              └─► sent as a protocol frame      (serial -> AI Bridge)
//! ```
//!
//! Records never allocate, so they can be created while the heap is broken
//! and from the panic and exception paths.

use crate::fixed::FixedString;
use crate::logger::{self, Level, Subsystem};
use crate::ring::RingBuffer;
use core::fmt::{self, Write};
use core::sync::atomic::{AtomicU32, Ordering};
use spin::Mutex;
use x86_64::instructions::interrupts::without_interrupts;

/// Longest diagnostic message, in bytes.
pub const MESSAGE_CAPACITY: usize = 160;
/// Most context entries one record can carry.
pub const MAX_CONTEXT_ENTRIES: usize = 6;
/// Longest context value, in bytes.
pub const CONTEXT_VALUE_CAPACITY: usize = 64;
/// Number of records kept in memory.
pub const HISTORY_CAPACITY: usize = 16;

/// Identifier of a diagnostic record, formatted as `diag-001`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticId(pub u32);

impl fmt::Display for DiagnosticId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "diag-{:03}", self.0)
    }
}

#[derive(Debug, Clone, Copy)]
struct ContextEntry {
    key: &'static str,
    value: FixedString<CONTEXT_VALUE_CAPACITY>,
}

/// One diagnostic record.
#[derive(Debug, Clone, Copy)]
pub struct Diagnostic {
    /// Assigned by [`report`]; 0 until then.
    pub id: u32,
    /// Kernel uptime when the record was created.
    pub uptime_ms: u64,
    pub severity: Level,
    pub subsystem: Subsystem,
    /// Stable error code such as `MEM-003` or `PANIC-001`.
    pub code: &'static str,
    pub message: FixedString<MESSAGE_CAPACITY>,
    context: [Option<ContextEntry>; MAX_CONTEXT_ENTRIES],
}

impl Diagnostic {
    /// Starts a record. Add context with [`with`](Self::with), then pass it
    /// to [`report`].
    pub fn new(
        severity: Level,
        subsystem: Subsystem,
        code: &'static str,
        message: fmt::Arguments,
    ) -> Self {
        let mut text = FixedString::new();
        let _ = text.write_fmt(message);
        Self {
            id: 0,
            uptime_ms: crate::time::uptime_ms(),
            severity,
            subsystem,
            code,
            message: text,
            context: [None; MAX_CONTEXT_ENTRIES],
        }
    }

    /// Adds a key/value pair of context. Entries beyond
    /// [`MAX_CONTEXT_ENTRIES`] are ignored.
    pub fn with(mut self, key: &'static str, value: impl fmt::Display) -> Self {
        if let Some(slot) = self.context.iter_mut().find(|slot| slot.is_none()) {
            let mut text = FixedString::new();
            let _ = write!(text, "{}", value);
            *slot = Some(ContextEntry { key, value: text });
        }
        self
    }

    /// The context entries in insertion order.
    pub fn context(&self) -> impl Iterator<Item = (&'static str, &str)> {
        self.context
            .iter()
            .flatten()
            .map(|entry| (entry.key, entry.value.as_str()))
    }

    /// Looks up one context value.
    pub fn context_value(&self, key: &str) -> Option<&str> {
        self.context()
            .find(|(entry_key, _)| *entry_key == key)
            .map(|(_, value)| value)
    }
}

/// Multi-line, human-readable form used by the `diag` command and the panic
/// screen (layout from Docs/04-ux-requirements.md).
impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "[{}] {}", self.severity, DiagnosticId(self.id))?;
        writeln!(f, "  Subsystem: {}", self.subsystem)?;
        writeln!(f, "  Code:      {}", self.code)?;
        writeln!(
            f,
            "  Time:      {}.{:03} s after boot",
            self.uptime_ms / 1000,
            self.uptime_ms % 1000
        )?;
        write!(f, "  Message:   {}", self.message)?;
        for (key, value) in self.context() {
            write!(f, "\n  {:<10} {}", key, value)?;
        }
        Ok(())
    }
}

static HISTORY: Mutex<RingBuffer<Diagnostic, HISTORY_CAPACITY>> = Mutex::new(RingBuffer::new());
static NEXT_ID: AtomicU32 = AtomicU32::new(1);

/// Records a diagnostic: assigns its id, stores it, logs it and sends it to
/// the AI Bridge channel. Returns the assigned id.
pub fn report(mut diagnostic: Diagnostic) -> DiagnosticId {
    let id = DiagnosticId(NEXT_ID.fetch_add(1, Ordering::Relaxed));
    diagnostic.id = id.0;

    without_interrupts(|| HISTORY.lock().push_overwrite(diagnostic));

    logger::log(
        diagnostic.severity,
        diagnostic.subsystem,
        diagnostic.code,
        format_args!("{} ({})", diagnostic.message, id),
    );

    if !crate::protocol::send_diagnostic(&diagnostic) {
        logger::record(
            Level::Error,
            Subsystem::Diagnostic,
            "COMM-001",
            format_args!("{} did not fit into a protocol frame and was not sent", id),
        );
    }
    id
}

/// Number of records reported since boot.
pub fn total_reported() -> u32 {
    NEXT_ID.load(Ordering::Relaxed) - 1
}

/// Number of records currently held in the history.
pub fn history_len() -> usize {
    without_interrupts(|| HISTORY.lock().len())
}

/// Returns the `index`-th stored record (0 = oldest).
pub fn history_get(index: usize) -> Option<Diagnostic> {
    without_interrupts(|| HISTORY.lock().get(index).copied())
}

/// The most recently reported record.
pub fn last() -> Option<Diagnostic> {
    without_interrupts(|| {
        let history = HISTORY.lock();
        history.len().checked_sub(1).and_then(|index| history.get(index).copied())
    })
}

/// Finds a stored record by id. Returns `None` if it was never reported or
/// has already been pushed out of the history.
pub fn find(id: u32) -> Option<Diagnostic> {
    without_interrupts(|| HISTORY.lock().iter().find(|record| record.id == id).copied())
}

/// Releases the history lock unconditionally.
///
/// # Safety
/// Only for the panic path, where the interrupted code that held the lock
/// will never run again.
pub unsafe fn force_unlock() {
    unsafe { HISTORY.force_unlock() };
}

#[test_case]
fn report_assigns_increasing_ids_and_stores_the_record() {
    let first = report(
        Diagnostic::new(Level::Warn, Subsystem::Kernel, "TEST-010", format_args!("first"))
            .with("answer", 42),
    );
    let second = report(Diagnostic::new(
        Level::Error,
        Subsystem::Memory,
        "TEST-011",
        format_args!("second {}", "record"),
    ));
    assert_eq!(second.0, first.0 + 1);

    let stored = find(first.0).expect("record must be stored");
    assert_eq!(stored.code, "TEST-010");
    assert_eq!(stored.severity, Level::Warn);
    assert_eq!(stored.context_value("answer"), Some("42"));
    assert_eq!(stored.context_value("missing"), None);

    let newest = last().expect("history must not be empty");
    assert_eq!(newest.id, second.0);
    assert_eq!(newest.message, "second record");
}

#[test_case]
fn report_also_writes_a_log_record() {
    let errors = logger::count(Level::Error);
    let id = report(Diagnostic::new(
        Level::Error,
        Subsystem::Memory,
        "TEST-012",
        format_args!("logged"),
    ));
    assert_eq!(logger::count(Level::Error), errors + 1);

    let record = logger::history_get(logger::history_len() - 1).unwrap();
    assert_eq!(record.code, "TEST-012");
    let mut expected: FixedString<32> = FixedString::new();
    let _ = write!(expected, "logged ({})", id);
    assert_eq!(record.message, expected);
}

#[test_case]
fn report_sends_a_protocol_frame() {
    let sent = crate::protocol::frames_sent();
    report(Diagnostic::new(
        Level::Info,
        Subsystem::Diagnostic,
        "TEST-013",
        format_args!("framed"),
    ));
    assert_eq!(crate::protocol::frames_sent(), sent + 1);
}

#[test_case]
fn extra_context_entries_are_ignored() {
    let mut diagnostic =
        Diagnostic::new(Level::Info, Subsystem::Kernel, "TEST-014", format_args!("x"));
    for _ in 0..MAX_CONTEXT_ENTRIES + 3 {
        diagnostic = diagnostic.with("key", "value");
    }
    assert_eq!(diagnostic.context().count(), MAX_CONTEXT_ENTRIES);
}

#[test_case]
fn history_is_bounded_and_drops_the_oldest() {
    let first = report(Diagnostic::new(
        Level::Info,
        Subsystem::Kernel,
        "TEST-015",
        format_args!("oldest"),
    ));
    for _ in 0..HISTORY_CAPACITY {
        report(Diagnostic::new(
            Level::Info,
            Subsystem::Kernel,
            "TEST-015",
            format_args!("filler"),
        ));
    }
    assert_eq!(history_len(), HISTORY_CAPACITY);
    assert!(find(first.0).is_none());
}

#[test_case]
fn identifiers_are_zero_padded() {
    let mut text: FixedString<32> = FixedString::new();
    let _ = write!(text, "{} {}", DiagnosticId(1), DiagnosticId(1234));
    assert_eq!(text, "diag-001 diag-1234");
}
