//! Serial console input: turns the byte stream of a terminal into key events.
//!
//! A terminal sends printable keys as plain bytes and special keys as ANSI
//! escape sequences (`ESC [ A` for the up arrow, and so on).
//!
//! The same line also carries protocol frames from the AI Bridge. A line that
//! starts with the `@PSY ` marker is cut out of the stream by [`Framer`] and
//! handed to `protocol::inbound`; it never reaches the shell as keystrokes.

use super::Key;
use crate::protocol::{inbound, FRAME_PREFIX};
use spin::Mutex;
use x86_64::instructions::interrupts::without_interrupts;

const PREFIX: &[u8] = FRAME_PREFIX.as_bytes();

/// Separates incoming protocol frames from typed input.
///
/// A frame is recognised only at the start of a line, so a user typing
/// `@PSY` in the middle of a command is never mistaken for the bridge. While
/// the first bytes of a line could still be the marker they are held back;
/// if the line turns out not to be a frame they are released as normal input.
struct Framer {
    at_line_start: bool,
    /// Marker bytes matched so far on this line.
    matched: usize,
    /// Inside a frame, collecting its body.
    collecting: bool,
    /// The current frame exceeded the buffer or contained a byte that is not
    /// printable ASCII; it is discarded at the end of the line.
    broken: bool,
    /// The last frame ended with `\r`; swallow a following `\n`.
    swallow_line_feed: bool,
    body: inbound::RawFrame,
}

static FRAMER: Mutex<Framer> = Mutex::new(Framer {
    at_line_start: true,
    matched: 0,
    collecting: false,
    broken: false,
    swallow_line_feed: false,
    body: crate::fixed::FixedString::new(),
});

/// What to do with a byte after the framer has looked at it.
enum Feed {
    /// Part of a frame; nothing for the shell.
    Consumed,
    /// Ordinary input. The held-back marker bytes (`PREFIX[..held]`) come
    /// first, then the byte itself unless it was consumed.
    Input { held: usize, byte: Option<u8> },
}

impl Framer {
    fn feed(&mut self, byte: u8) -> Feed {
        let swallow = core::mem::replace(&mut self.swallow_line_feed, false);

        if self.collecting {
            if byte == b'\n' || byte == b'\r' {
                if self.broken {
                    inbound::note_broken_frame();
                } else {
                    inbound::push_raw_frame(&self.body);
                }
                self.collecting = false;
                self.broken = false;
                self.body.clear();
                self.at_line_start = true;
                self.swallow_line_feed = byte == b'\r';
            } else if !(0x20..0x7f).contains(&byte) || !self.body.push(byte as char) {
                // The bridge escapes everything else (`\uXXXX`), so any other
                // byte means a damaged frame.
                self.broken = true;
            }
            return Feed::Consumed;
        }

        if byte == b'\n' && swallow {
            return Feed::Consumed;
        }

        if self.matched > 0 || (self.at_line_start && byte == PREFIX[0]) {
            if byte == PREFIX[self.matched] {
                self.matched += 1;
                if self.matched == PREFIX.len() {
                    self.matched = 0;
                    self.collecting = true;
                    self.body.clear();
                }
                return Feed::Consumed;
            }
            // Not a frame after all: release what was held back.
            let held = core::mem::replace(&mut self.matched, 0);
            self.at_line_start = byte == b'\n' || byte == b'\r';
            return Feed::Input { held, byte: Some(byte) };
        }

        self.at_line_start = byte == b'\n' || byte == b'\r';
        Feed::Input { held: 0, byte: Some(byte) }
    }
}

const ESCAPE: u8 = 0x1b;

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    /// Not inside an escape sequence.
    Normal,
    /// Received `ESC`.
    Escape,
    /// Received `ESC [`; the next byte selects the key.
    Csi,
    /// Inside a sequence such as `ESC [ 3 ~`; holds the digit. Ends at `~`.
    CsiNumber(u8),
}

struct Decoder {
    state: State,
    /// Set after a carriage return so the `\n` of a `\r\n` pair is ignored.
    after_carriage_return: bool,
}

static DECODER: Mutex<Decoder> = Mutex::new(Decoder {
    state: State::Normal,
    after_carriage_return: false,
});

impl Decoder {
    fn decode(&mut self, byte: u8) -> Option<Key> {
        let after_carriage_return = core::mem::replace(&mut self.after_carriage_return, false);

        match self.state {
            State::Normal => match byte {
                ESCAPE => {
                    self.state = State::Escape;
                    None
                }
                b'\r' => {
                    self.after_carriage_return = true;
                    Some(Key::Enter)
                }
                b'\n' if after_carriage_return => None,
                // The console font only covers ASCII; ignore everything else.
                byte if byte.is_ascii() => Key::from_char(byte as char),
                _ => None,
            },
            State::Escape => {
                self.state = if byte == b'[' || byte == b'O' {
                    State::Csi
                } else {
                    State::Normal
                };
                None
            }
            State::Csi => {
                self.state = State::Normal;
                match byte {
                    b'A' => Some(Key::Up),
                    b'B' => Some(Key::Down),
                    b'C' => Some(Key::Right),
                    b'D' => Some(Key::Left),
                    b'H' => Some(Key::Home),
                    b'F' => Some(Key::End),
                    b'0'..=b'9' => {
                        self.state = State::CsiNumber(byte - b'0');
                        None
                    }
                    _ => None,
                }
            }
            State::CsiNumber(number) => match byte {
                b'~' => {
                    self.state = State::Normal;
                    match number {
                        1 | 7 => Some(Key::Home),
                        3 => Some(Key::Delete),
                        4 | 8 => Some(Key::End),
                        _ => None,
                    }
                }
                // Parameters we do not interpret (e.g. `ESC [ 1 ; 5 C`).
                b'0'..=b'9' | b';' => None,
                _ => {
                    self.state = State::Normal;
                    None
                }
            },
        }
    }
}

/// Feeds one received byte to the decoder and returns the key it completes.
pub fn decode_byte(byte: u8) -> Option<Key> {
    without_interrupts(|| DECODER.lock().decode(byte))
}

/// Handles one received byte: frame bytes go to the protocol inbox, all
/// other bytes are decoded and queued as keys. Called from the serial
/// interrupt handler.
pub fn handle_byte(byte: u8) {
    let feed = without_interrupts(|| FRAMER.lock().feed(byte));
    if let Feed::Input { held, byte } = feed {
        for &held_byte in &PREFIX[..held] {
            queue_decoded(held_byte);
        }
        if let Some(byte) = byte {
            queue_decoded(byte);
        }
    }
}

fn queue_decoded(byte: u8) {
    if let Some(key) = decode_byte(byte) {
        super::push_key(key);
    }
}

#[cfg(test)]
fn feed_text(text: &str) {
    for byte in text.bytes() {
        handle_byte(byte);
    }
}

#[test_case]
fn frames_are_cut_out_of_the_input_stream() {
    super::clear();
    let received = inbound::frames_received();
    feed_text("\r@PSY {\"type\":\"status\",\"version\":1,\"id\":\"stat-001\",\"payload\":{}}\n");
    // Only the leading line break reached the shell.
    assert_eq!(super::try_read_key(), Some(Key::Enter));
    assert_eq!(super::try_read_key(), None);
    inbound::poll();
    assert_eq!(inbound::frames_received(), received + 1);
    assert!(inbound::bridge_seen());
}

#[test_case]
fn marker_lookalikes_are_released_as_input() {
    super::clear();
    feed_text("\r@PS!");
    assert_eq!(super::try_read_key(), Some(Key::Enter));
    for c in ['@', 'P', 'S', '!'] {
        assert_eq!(super::try_read_key(), Some(Key::Char(c)));
    }
    // A marker in the middle of a line is ordinary text.
    feed_text("x@PSY y");
    let typed: [Option<Key>; 7] = core::array::from_fn(|_| super::try_read_key());
    assert_eq!(typed[1], Some(Key::Char('@')));
    assert_eq!(typed[6], Some(Key::Char('y')));
    feed_text("\r");
    super::clear();
}

#[test_case]
fn broken_frames_are_discarded() {
    super::clear();
    let rejected = inbound::frames_rejected();
    // A raw control byte inside a frame ...
    feed_text("\r@PSY {\"a\":\u{1}}\r\n");
    assert_eq!(inbound::frames_rejected(), rejected + 1);
    // ... and the `\n` of the `\r\n` pair is not turned into an Enter.
    assert_eq!(super::try_read_key(), Some(Key::Enter)); // the leading `\r`
    assert_eq!(super::try_read_key(), None);
}

#[test_case]
fn printable_and_control_bytes() {
    assert_eq!(decode_byte(b'h'), Some(Key::Char('h')));
    assert_eq!(decode_byte(0x7f), Some(Key::Backspace));
    assert_eq!(decode_byte(b'\t'), Some(Key::Tab));
    assert_eq!(decode_byte(0x03), Some(Key::Interrupt));
    assert_eq!(decode_byte(0xc3), None); // non-ASCII
}

#[test_case]
fn carriage_return_line_feed_is_one_enter() {
    assert_eq!(decode_byte(b'\r'), Some(Key::Enter));
    assert_eq!(decode_byte(b'\n'), None);
    // A line feed on its own still counts.
    assert_eq!(decode_byte(b'\n'), Some(Key::Enter));
    assert_eq!(decode_byte(b'x'), Some(Key::Char('x')));
}

#[test_case]
fn arrow_key_escape_sequences() {
    for (final_byte, key) in [(b'A', Key::Up), (b'B', Key::Down), (b'C', Key::Right), (b'D', Key::Left)] {
        assert_eq!(decode_byte(ESCAPE), None);
        assert_eq!(decode_byte(b'['), None);
        assert_eq!(decode_byte(final_byte), Some(key));
    }
    // The decoder is back in the normal state afterwards.
    assert_eq!(decode_byte(b'A'), Some(Key::Char('A')));
}

#[test_case]
fn delete_key_escape_sequence() {
    assert_eq!(decode_byte(ESCAPE), None);
    assert_eq!(decode_byte(b'['), None);
    assert_eq!(decode_byte(b'3'), None);
    assert_eq!(decode_byte(b'~'), Some(Key::Delete));
}

#[test_case]
fn unknown_escape_sequence_is_swallowed() {
    assert_eq!(decode_byte(ESCAPE), None);
    assert_eq!(decode_byte(b'x'), None);
    assert_eq!(decode_byte(b'y'), Some(Key::Char('y')));
}
