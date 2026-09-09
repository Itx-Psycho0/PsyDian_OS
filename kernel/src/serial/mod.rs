use spin::Mutex;
use uart_16550::SerialPort;

pub static SERIAL1: Mutex<SerialPort> = Mutex::new(unsafe { SerialPort::new(0x3F8) });

pub fn init() {
    SERIAL1.lock().init();
}

pub fn write_byte(byte: u8) {
    SERIAL1.lock().send(byte);
}

pub fn write_str(message: &str) {
    for byte in message.bytes() {
        write_byte(byte);
    }
}
