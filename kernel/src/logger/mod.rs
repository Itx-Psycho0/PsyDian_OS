//! Centralized kernel logger.
//!
//! Every kernel message that is not direct shell output goes through here so
//! it has one consistent, machine-readable shape (Docs/14-observability.md):
//!
//! ```text
//! [LEVEL][SUBSYSTEM][EVENT-ID] message
//! [INFO][BOOT][BOOT-001] Psydian kernel booted.
//! ```
//!
//! A record is written to the console (serial + screen) and kept in a bounded
//! in-memory history that the shell's `logs` command can show. Nothing here
//! allocates, so the logger works before the heap exists and inside the panic
//! path.

use crate::fixed::FixedString;
use crate::ring::RingBuffer;
use core::fmt::{self, Write};
use core::sync::atomic::{AtomicU32, Ordering};
use spin::Mutex;
use x86_64::instructions::interrupts::without_interrupts;

/// Longest message kept in the history; longer messages are truncated there
/// (the console always receives the full text).
pub const MESSAGE_CAPACITY: usize = 120;

/// Number of records kept in the in-memory history.
pub const HISTORY_CAPACITY: usize = 64;

/// Severity of a log record, ordered from least to most severe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// Normal system events.
    Info,
    /// Unexpected but non-fatal conditions.
    Warn,
    /// A failure that affects a component but not the whole system.
    Error,
    /// An unrecoverable kernel condition.
    Panic,
}

impl Level {
    pub const ALL: [Level; 4] = [Level::Info, Level::Warn, Level::Error, Level::Panic];

    /// Upper-case label used in log lines.
    pub const fn as_str(self) -> &'static str {
        match self {
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
            Level::Panic => "PANIC",
        }
    }

    /// Lower-case name used in protocol messages.
    pub const fn as_lower_str(self) -> &'static str {
        match self {
            Level::Info => "info",
            Level::Warn => "warn",
            Level::Error => "error",
            Level::Panic => "panic",
        }
    }

    /// Parses a level name, ignoring case.
    pub fn parse(name: &str) -> Option<Level> {
        Level::ALL
            .into_iter()
            .find(|level| level.as_str().eq_ignore_ascii_case(name))
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The kernel component a record belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subsystem {
    Boot,
    Kernel,
    Memory,
    Interrupt,
    Input,
    Serial,
    Shell,
    Diagnostic,
    Ai,
}

impl Subsystem {
    /// Upper-case label used in log lines.
    pub const fn as_str(self) -> &'static str {
        match self {
            Subsystem::Boot => "BOOT",
            Subsystem::Kernel => "KERNEL",
            Subsystem::Memory => "MEMORY",
            Subsystem::Interrupt => "INTERRUPT",
            Subsystem::Input => "INPUT",
            Subsystem::Serial => "SERIAL",
            Subsystem::Shell => "SHELL",
            Subsystem::Diagnostic => "DIAGNOSTIC",
            Subsystem::Ai => "AI",
        }
    }

    /// Lower-case name used in protocol messages.
    pub const fn as_lower_str(self) -> &'static str {
        match self {
            Subsystem::Boot => "boot",
            Subsystem::Kernel => "kernel",
            Subsystem::Memory => "memory",
            Subsystem::Interrupt => "interrupt",
            Subsystem::Input => "input",
            Subsystem::Serial => "serial",
            Subsystem::Shell => "shell",
            Subsystem::Diagnostic => "diagnostic",
            Subsystem::Ai => "ai",
        }
    }
}

impl fmt::Display for Subsystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One entry of the log history.
#[derive(Debug, Clone, Copy)]
pub struct LogRecord {
    /// Position of this record in the stream of all records since boot (1-based).
    pub sequence: u32,
    /// Kernel uptime when the record was created.
    pub uptime_ms: u64,
    pub level: Level,
    pub subsystem: Subsystem,
    /// Stable event identifier such as `BOOT-001` or `MEM-003`.
    pub code: &'static str,
    pub message: FixedString<MESSAGE_CAPACITY>,
}

impl fmt::Display for LogRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}][{}][{}] {}",
            self.level, self.subsystem, self.code, self.message
        )
    }
}

static HISTORY: Mutex<RingBuffer<LogRecord, HISTORY_CAPACITY>> = Mutex::new(RingBuffer::new());
static SEQUENCE: AtomicU32 = AtomicU32::new(0);
static COUNTS: [AtomicU32; 4] = [
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
];

/// Logs a record: stores it in the history and prints it on the console.
///
/// Prefer the [`info!`](crate::info), [`warn!`](crate::warn) and
/// [`error!`](crate::error) macros.
pub fn log(level: Level, subsystem: Subsystem, code: &'static str, args: fmt::Arguments) {
    record(level, subsystem, code, args);
    crate::println!("[{}][{}][{}] {}", level, subsystem, code, args);
}

/// Stores a record in the history without printing it.
///
/// Used for events the user already sees in another form (for example the
/// shell prints its own error for an unknown command) but that should still
/// be visible in `logs`.
pub fn record(level: Level, subsystem: Subsystem, code: &'static str, args: fmt::Arguments) {
    let mut message = FixedString::new();
    let _ = message.write_fmt(args);

    let entry = LogRecord {
        sequence: SEQUENCE.fetch_add(1, Ordering::Relaxed) + 1,
        uptime_ms: crate::time::uptime_ms(),
        level,
        subsystem,
        code,
        message,
    };
    COUNTS[level as usize].fetch_add(1, Ordering::Relaxed);

    // Interrupt handlers log too; taking the lock with interrupts enabled
    // could deadlock against them.
    without_interrupts(|| HISTORY.lock().push_overwrite(entry));
}

/// Number of records logged at `level` since boot.
pub fn count(level: Level) -> u32 {
    COUNTS[level as usize].load(Ordering::Relaxed)
}

/// Number of records currently held in the history.
pub fn history_len() -> usize {
    without_interrupts(|| HISTORY.lock().len())
}

/// Returns the `index`-th record of the history (0 = oldest).
///
/// Records are copied out one at a time so the lock is never held while the
/// caller prints, which itself may log.
pub fn history_get(index: usize) -> Option<LogRecord> {
    without_interrupts(|| HISTORY.lock().get(index).copied())
}

/// Releases the history lock unconditionally.
///
/// # Safety
/// Only for the panic path, where the interrupted code that held the lock
/// will never run again.
pub unsafe fn force_unlock() {
    unsafe { HISTORY.force_unlock() };
}

/// Logs a normal system event.
///
/// `info!(Subsystem::Boot, "BOOT-001", "Psydian kernel booted.")`
#[macro_export]
macro_rules! info {
    ($subsystem:expr, $code:expr, $($arg:tt)*) => {
        $crate::logger::log($crate::logger::Level::Info, $subsystem, $code, format_args!($($arg)*))
    };
}

/// Logs an unexpected but non-fatal condition.
#[macro_export]
macro_rules! warn {
    ($subsystem:expr, $code:expr, $($arg:tt)*) => {
        $crate::logger::log($crate::logger::Level::Warn, $subsystem, $code, format_args!($($arg)*))
    };
}

/// Logs a failure that affects a component but not the whole system.
#[macro_export]
macro_rules! error {
    ($subsystem:expr, $code:expr, $($arg:tt)*) => {
        $crate::logger::log($crate::logger::Level::Error, $subsystem, $code, format_args!($($arg)*))
    };
}

#[test_case]
fn levels_are_ordered_by_severity() {
    assert!(Level::Info < Level::Warn);
    assert!(Level::Warn < Level::Error);
    assert!(Level::Error < Level::Panic);
}

#[test_case]
fn level_parse_ignores_case() {
    assert_eq!(Level::parse("warn"), Some(Level::Warn));
    assert_eq!(Level::parse("ERROR"), Some(Level::Error));
    assert_eq!(Level::parse("verbose"), None);
}

#[test_case]
fn record_is_stored_with_metadata() {
    let before = count(Level::Warn);
    record(
        Level::Warn,
        Subsystem::Memory,
        "TEST-001",
        format_args!("value={}", 42),
    );
    assert_eq!(count(Level::Warn), before + 1);

    let last = history_get(history_len() - 1).expect("history must not be empty");
    assert_eq!(last.level, Level::Warn);
    assert_eq!(last.subsystem, Subsystem::Memory);
    assert_eq!(last.code, "TEST-001");
    assert_eq!(last.message, "value=42");
}

#[test_case]
fn record_formats_like_the_documented_log_line() {
    record(
        Level::Error,
        Subsystem::Memory,
        "MEM-003",
        format_args!("Heap allocation failed."),
    );
    let last = history_get(history_len() - 1).expect("history must not be empty");
    let mut line: FixedString<80> = FixedString::new();
    let _ = write!(line, "{}", last);
    assert_eq!(line, "[ERROR][MEMORY][MEM-003] Heap allocation failed.");
}

#[test_case]
fn history_is_bounded() {
    for i in 0..(HISTORY_CAPACITY + 10) {
        record(Level::Info, Subsystem::Kernel, "TEST-002", format_args!("{}", i));
    }
    assert_eq!(history_len(), HISTORY_CAPACITY);
}
