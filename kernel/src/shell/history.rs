//! Command history with Up/Down navigation.

use super::parser::MAX_LINE_LENGTH;
use crate::fixed::FixedString;
use crate::ring::RingBuffer;

/// Number of command lines remembered.
pub const HISTORY_CAPACITY: usize = 16;

type Line = FixedString<MAX_LINE_LENGTH>;

pub struct History {
    entries: RingBuffer<Line, HISTORY_CAPACITY>,
    /// Entry currently shown while browsing with Up/Down (index into
    /// `entries`, 0 = oldest). `None` while the user is on a fresh line.
    cursor: Option<usize>,
}

impl History {
    pub const fn new() -> Self {
        Self {
            entries: RingBuffer::new(),
            cursor: None,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Remembers an executed line and leaves browsing mode. Blank lines and
    /// an immediate repeat of the previous line are not stored.
    pub fn push(&mut self, line: &str) {
        self.cursor = None;
        let line = line.trim();
        if line.is_empty() {
            return;
        }
        if let Some(last) = self.entries.len().checked_sub(1).and_then(|i| self.entries.get(i)) {
            if last.as_str() == line {
                return;
            }
        }
        self.entries.push_overwrite(Line::from_str_truncated(line));
    }

    /// Up arrow: steps to the next older entry. Stays on the oldest one.
    pub fn previous(&mut self) -> Option<&str> {
        if self.entries.is_empty() {
            return None;
        }
        let index = match self.cursor {
            None => self.entries.len() - 1,
            Some(index) => index.saturating_sub(1),
        };
        self.cursor = Some(index);
        self.entries.get(index).map(|line| line.as_str())
    }

    /// Down arrow: steps to the next newer entry. Returns `None` after the
    /// newest one, meaning "back to an empty line".
    pub fn next(&mut self) -> Option<&str> {
        let index = self.cursor? + 1;
        if index >= self.entries.len() {
            self.cursor = None;
            return None;
        }
        self.cursor = Some(index);
        self.entries.get(index).map(|line| line.as_str())
    }

    /// Leaves browsing mode (the user started editing or submitted a line).
    pub fn reset_cursor(&mut self) {
        self.cursor = None;
    }

    /// Entry at `index`, where 0 is the oldest.
    pub fn get(&self, index: usize) -> Option<&str> {
        self.entries.get(index).map(|line| line.as_str())
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

#[test_case]
fn browse_back_and_forth() {
    let mut history = History::new();
    assert_eq!(history.previous(), None);
    history.push("help");
    history.push("meminfo");
    history.push("info");

    assert_eq!(history.previous(), Some("info"));
    assert_eq!(history.previous(), Some("meminfo"));
    assert_eq!(history.previous(), Some("help"));
    // Stays on the oldest entry.
    assert_eq!(history.previous(), Some("help"));
    assert_eq!(history.next(), Some("meminfo"));
    assert_eq!(history.next(), Some("info"));
    // Past the newest entry: empty line.
    assert_eq!(history.next(), None);
    assert_eq!(history.next(), None);
}

#[test_case]
fn skips_blank_and_repeated_lines() {
    let mut history = History::new();
    history.push("   ");
    history.push("help");
    history.push("help");
    history.push("  help  ");
    assert_eq!(history.len(), 1);
    history.push("info");
    history.push("help");
    assert_eq!(history.len(), 3);
}

#[test_case]
fn keeps_only_the_newest_entries() {
    use core::fmt::Write;

    let mut history = History::new();
    for i in 0..HISTORY_CAPACITY + 4 {
        let mut line: FixedString<16> = FixedString::new();
        let _ = write!(line, "echo {}", i);
        history.push(line.as_str());
    }
    assert_eq!(history.len(), HISTORY_CAPACITY);
    assert_eq!(history.get(0), Some("echo 4"));
    assert_eq!(history.previous(), Some("echo 19"));
}

#[test_case]
fn push_leaves_browsing_mode() {
    let mut history = History::new();
    history.push("help");
    history.push("info");
    assert_eq!(history.previous(), Some("info"));
    assert_eq!(history.previous(), Some("help"));
    history.push("meminfo");
    assert_eq!(history.previous(), Some("meminfo"));
}
