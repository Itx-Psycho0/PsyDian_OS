//! PS/2 keyboard: scancode decoding.

use super::Key;
use pc_keyboard::{DecodedKey, HandleControl, KeyCode, Keyboard, ScancodeSet1, layouts};
use spin::Mutex;
use x86_64::instructions::interrupts::without_interrupts;
use x86_64::instructions::port::Port;

/// I/O port of the PS/2 controller's data register.
const DATA_PORT: u16 = 0x60;

/// Decoder state (pressed modifiers, partially received multi-byte scancodes).
/// `MapLettersToUnicode` turns Ctrl+C / Ctrl+L into the control characters
/// U+0003 / U+000C, which `Key::from_char` understands.
static KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> = Mutex::new(Keyboard::new(
    ScancodeSet1::new(),
    layouts::Us104Key,
    HandleControl::MapLettersToUnicode,
));

/// Reads the scancode that caused the keyboard interrupt.
///
/// The controller does not send another interrupt until this is read.
pub fn read_scancode() -> u8 {
    let mut port: Port<u8> = Port::new(DATA_PORT);
    unsafe { port.read() }
}

/// Feeds one scancode to the decoder and returns the key it completes, if any.
///
/// Key releases, modifier keys and the first bytes of multi-byte scancodes
/// produce `None`.
pub fn decode_key(scancode: u8) -> Option<Key> {
    let decoded = without_interrupts(|| {
        let mut keyboard = KEYBOARD.lock();
        let event = keyboard.add_byte(scancode).ok().flatten()?;
        keyboard.process_keyevent(event)
    })?;

    match decoded {
        DecodedKey::Unicode(c) => Key::from_char(c),
        DecodedKey::RawKey(code) => match code {
            KeyCode::ArrowUp => Some(Key::Up),
            KeyCode::ArrowDown => Some(Key::Down),
            KeyCode::ArrowLeft => Some(Key::Left),
            KeyCode::ArrowRight => Some(Key::Right),
            KeyCode::Home => Some(Key::Home),
            KeyCode::End => Some(Key::End),
            KeyCode::Delete => Some(Key::Delete),
            _ => None,
        },
    }
}

/// Decodes a scancode and queues the resulting key. Called from the keyboard
/// interrupt handler.
pub fn handle_scancode(scancode: u8) {
    if let Some(key) = decode_key(scancode) {
        super::push_key(key);
    }
}

#[test_case]
fn letter_press_and_release() {
    // Scancode set 1: 0x1e = 'A' pressed, 0x9e = 'A' released.
    assert_eq!(decode_key(0x1e), Some(Key::Char('a')));
    assert_eq!(decode_key(0x9e), None);
}

#[test_case]
fn shift_produces_upper_case() {
    assert_eq!(decode_key(0x2a), None); // left shift pressed
    assert_eq!(decode_key(0x1e), Some(Key::Char('A')));
    assert_eq!(decode_key(0x9e), None);
    assert_eq!(decode_key(0xaa), None); // left shift released
    assert_eq!(decode_key(0x1e), Some(Key::Char('a')));
    assert_eq!(decode_key(0x9e), None);
}

#[test_case]
fn editing_keys() {
    assert_eq!(decode_key(0x1c), Some(Key::Enter));
    assert_eq!(decode_key(0x9c), None);
    assert_eq!(decode_key(0x0e), Some(Key::Backspace));
    assert_eq!(decode_key(0x8e), None);
    assert_eq!(decode_key(0x0f), Some(Key::Tab));
    assert_eq!(decode_key(0x8f), None);
}

#[test_case]
fn extended_arrow_keys() {
    // Arrow keys are two-byte scancodes starting with 0xe0.
    assert_eq!(decode_key(0xe0), None);
    assert_eq!(decode_key(0x48), Some(Key::Up));
    assert_eq!(decode_key(0xe0), None);
    assert_eq!(decode_key(0xc8), None);
    assert_eq!(decode_key(0xe0), None);
    assert_eq!(decode_key(0x50), Some(Key::Down));
    assert_eq!(decode_key(0xe0), None);
    assert_eq!(decode_key(0xd0), None);
}

#[test_case]
fn ctrl_c_is_interrupt() {
    assert_eq!(decode_key(0x1d), None); // left ctrl pressed
    assert_eq!(decode_key(0x2e), Some(Key::Interrupt)); // 'C'
    assert_eq!(decode_key(0xae), None);
    assert_eq!(decode_key(0x9d), None); // left ctrl released
}
