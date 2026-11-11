use spin::Mutex;
use uart_16550::SerialPort;
use x86_64::instructions::interrupts::without_interrupts;

pub static SERIAL1: Mutex<SerialPort> = Mutex::new(unsafe { SerialPort::new(0x3F8) });

/// Configures COM1 (38400 baud, 8N1) and enables its receive interrupt.
pub fn init() {
    without_interrupts(|| SERIAL1.lock().init());
}

/// Returns a received byte if one is waiting in the UART.
pub fn try_read_byte() -> Option<u8> {
    without_interrupts(|| SERIAL1.lock().try_receive().ok())
}

pub fn write_byte(byte: u8) {
    without_interrupts(|| SERIAL1.lock().send(byte));
}

pub fn write_str(message: &str) {
    for byte in message.bytes() {
        write_byte(byte);
    }
}

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;
    // Interrupts stay off while the port is locked: the serial interrupt
    // handler takes the same lock to read received bytes.
    without_interrupts(|| {
        let _ = SERIAL1.lock().write_fmt(args);
    });
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt, "\n"), $($arg)*));
}
