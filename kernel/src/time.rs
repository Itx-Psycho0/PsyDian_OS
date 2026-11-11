//! Kernel timekeeping based on the Programmable Interval Timer (PIT).
//!
//! The PIT raises IRQ 0 at a fixed rate. Counting those interrupts gives the
//! kernel an uptime clock, which the logger and diagnostics use as a timestamp.

use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::instructions::port::Port;

/// Timer interrupts per second.
pub const TICK_HZ: u64 = 100;

/// Input clock of the PIT in Hz (fixed by the hardware).
const PIT_BASE_HZ: u64 = 1_193_182;

static TICKS: AtomicU64 = AtomicU64::new(0);

/// Programs PIT channel 0 to fire `TICK_HZ` times per second.
///
/// Without this the BIOS default of ~18.2 Hz stays active.
pub fn init() {
    let divisor = (PIT_BASE_HZ / TICK_HZ) as u16;
    let mut command: Port<u8> = Port::new(0x43);
    let mut channel0: Port<u8> = Port::new(0x40);
    unsafe {
        // 0x36 = channel 0, low byte then high byte, mode 3 (square wave).
        command.write(0x36);
        channel0.write((divisor & 0xff) as u8);
        channel0.write((divisor >> 8) as u8);
    }
}

/// Called by the timer interrupt handler.
pub fn tick() {
    TICKS.fetch_add(1, Ordering::Relaxed);
}

/// Timer interrupts since boot.
pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

/// Milliseconds since the timer was started.
pub fn uptime_ms() -> u64 {
    ticks() * 1000 / TICK_HZ
}

#[test_case]
fn timer_interrupts_advance_the_clock() {
    let start = ticks();
    // Each `hlt` returns on the next interrupt; a few of them must be timer ticks.
    for _ in 0..5 {
        x86_64::instructions::hlt();
    }
    assert!(ticks() > start);
}
