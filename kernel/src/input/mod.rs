//! Input subsystem.
//!
//! Two interrupt-driven sources feed one bounded queue of [`Key`] events:
//!
//! ```text
//! PS/2 keyboard (IRQ 1) ─ scancode ─► keyboard::handle_scancode ─┐
//!                                                                ├─► QUEUE ─► read_key()
//! Serial port COM1 (IRQ 4) ─ byte ──► serial::handle_byte ───────┘
//! ```
//!
//! The interrupt handlers only decode and enqueue. All real work (echoing,
//! line editing, running commands) happens later in normal kernel context,
//! which keeps the handlers short and free of lock-ordering problems.
//!
//! Serial input makes the shell usable from the host terminal
//! (`-serial stdio`), in headless runs, and later by the AI Bridge.

pub mod keyboard;
pub mod serial;

use crate::ring::RingBuffer;
use core::sync::atomic::{AtomicU32, Ordering};
use spin::Mutex;
use x86_64::instructions::interrupts;

/// Maximum number of key events waiting to be read.
pub const QUEUE_CAPACITY: usize = 128;

/// A decoded input event, independent of where it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// A printable character.
    Char(char),
    Enter,
    Backspace,
    Tab,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    Delete,
    /// Ctrl+C: abandon the current input line.
    Interrupt,
    /// Ctrl+L: clear the screen.
    ClearScreen,
}

impl Key {
    /// Maps a character produced by a keyboard layout or received over serial
    /// to a key event. Returns `None` for control characters with no meaning
    /// to the shell.
    pub fn from_char(c: char) -> Option<Key> {
        match c {
            '\n' | '\r' => Some(Key::Enter),
            '\u{8}' | '\u{7f}' => Some(Key::Backspace),
            '\t' => Some(Key::Tab),
            '\u{3}' => Some(Key::Interrupt),
            '\u{c}' => Some(Key::ClearScreen),
            c if !c.is_control() => Some(Key::Char(c)),
            _ => None,
        }
    }
}

static QUEUE: Mutex<RingBuffer<Key, QUEUE_CAPACITY>> = Mutex::new(RingBuffer::new());
static DROPPED: AtomicU32 = AtomicU32::new(0);

/// Adds a key event to the queue. If the queue is full the event is dropped
/// and counted: losing a keystroke is better than blocking an interrupt handler.
pub fn push_key(key: Key) {
    interrupts::without_interrupts(|| {
        if QUEUE.lock().push(key).is_err() {
            DROPPED.fetch_add(1, Ordering::Relaxed);
        }
    });
}

/// Returns the next key event if one is waiting.
pub fn try_read_key() -> Option<Key> {
    interrupts::without_interrupts(|| QUEUE.lock().pop())
}

/// Waits for the next key event, sleeping the CPU until an interrupt arrives.
pub fn read_key() -> Key {
    loop {
        // Check with interrupts off, then enable + halt in one atomic step.
        // Otherwise a key arriving between the check and `hlt` would be
        // missed until the next unrelated interrupt.
        interrupts::disable();
        if let Some(key) = QUEUE.lock().pop() {
            interrupts::enable();
            return key;
        }
        interrupts::enable_and_hlt();
    }
}

/// Number of key events currently waiting.
pub fn pending() -> usize {
    interrupts::without_interrupts(|| QUEUE.lock().len())
}

/// Key events discarded because the queue was full.
pub fn dropped() -> u32 {
    DROPPED.load(Ordering::Relaxed)
}

/// Discards all waiting key events.
pub fn clear() {
    interrupts::without_interrupts(|| QUEUE.lock().clear());
}

#[test_case]
fn control_characters_map_to_keys() {
    assert_eq!(Key::from_char('a'), Some(Key::Char('a')));
    assert_eq!(Key::from_char(' '), Some(Key::Char(' ')));
    assert_eq!(Key::from_char('\n'), Some(Key::Enter));
    assert_eq!(Key::from_char('\r'), Some(Key::Enter));
    assert_eq!(Key::from_char('\u{8}'), Some(Key::Backspace));
    assert_eq!(Key::from_char('\u{7f}'), Some(Key::Backspace));
    assert_eq!(Key::from_char('\t'), Some(Key::Tab));
    assert_eq!(Key::from_char('\u{3}'), Some(Key::Interrupt));
    assert_eq!(Key::from_char('\u{c}'), Some(Key::ClearScreen));
    assert_eq!(Key::from_char('\u{1}'), None);
}

#[test_case]
fn queue_preserves_order() {
    clear();
    push_key(Key::Char('h'));
    push_key(Key::Char('i'));
    push_key(Key::Enter);
    assert_eq!(pending(), 3);
    assert_eq!(read_key(), Key::Char('h'));
    assert_eq!(try_read_key(), Some(Key::Char('i')));
    assert_eq!(try_read_key(), Some(Key::Enter));
    assert_eq!(try_read_key(), None);
}

#[test_case]
fn full_queue_drops_new_keys() {
    clear();
    let before = dropped();
    for _ in 0..QUEUE_CAPACITY + 3 {
        push_key(Key::Char('x'));
    }
    assert_eq!(pending(), QUEUE_CAPACITY);
    assert_eq!(dropped(), before + 3);
    clear();
    assert_eq!(pending(), 0);
}
