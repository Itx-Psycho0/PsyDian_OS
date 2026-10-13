//! `print!` / `println!` for the kernel.
//!
//! Every message goes to the serial port (visible with `-serial stdio`, and the
//! only output that survives a crash very early in boot) and, once
//! [`crate::framebuffer::init`] has run, also to the screen.

use core::fmt::{self, Write};

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::console::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    // Interrupts are disabled while we hold the locks. Otherwise the timer
    // interrupt could fire while `println!` holds a lock, its handler would call
    // `print!` too, and the CPU would spin forever waiting for a lock that can
    // never be released (a deadlock).
    x86_64::instructions::interrupts::without_interrupts(|| {
        let _ = crate::serial::SERIAL1.lock().write_fmt(args);
        if let Some(writer) = crate::framebuffer::WRITER.lock().as_mut() {
            let _ = writer.write_fmt(args);
        }
    });
}
