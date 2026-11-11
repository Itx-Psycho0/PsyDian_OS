//! Serial console input: turns the byte stream of a terminal into key events.
//!
//! A terminal sends printable keys as plain bytes and special keys as ANSI
//! escape sequences (`ESC [ A` for the up arrow, and so on).

use super::Key;
use spin::Mutex;
use x86_64::instructions::interrupts::without_interrupts;

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

/// Decodes a received byte and queues the resulting key. Called from the
/// serial interrupt handler.
pub fn handle_byte(byte: u8) {
    if let Some(key) = decode_byte(byte) {
        super::push_key(key);
    }
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
