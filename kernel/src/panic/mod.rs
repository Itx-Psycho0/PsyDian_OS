//! Kernel panic handling.
//!
//! Every unrecoverable failure ends here, whether it is a Rust `panic!` or a
//! fatal CPU exception:
//!
//! ```text
//! panic!(..) ─────────────────────────────┐
//! fatal exception ─► raise(diagnostic) ───┴─► capture() ─► halt
//! ```
//!
//! [`capture`] turns the failure into a diagnostic record, stores it, prints
//! it and sends it to the AI Bridge channel. [`handle_panic`] then stops the
//! machine in a controlled state.

use crate::diagnostics::{self, Diagnostic, DiagnosticId};
use crate::logger::{Level, Subsystem};
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;

/// Set once a panic is being handled, to detect a panic inside the handler.
static PANICKING: AtomicBool = AtomicBool::new(false);

/// A diagnostic prepared by [`raise`] for the panic that follows it.
static PENDING: Mutex<Option<Diagnostic>> = Mutex::new(None);

/// Stops the kernel with a prepared diagnostic record.
///
/// Used by exception handlers and the allocation-error handler, which know
/// more about the failure (error code, faulting address, ...) than a panic
/// message can carry. The record is picked up by [`capture`].
pub fn raise(diagnostic: Diagnostic) -> ! {
    x86_64::instructions::interrupts::disable();
    unsafe { PENDING.force_unlock() };
    *PENDING.lock() = Some(diagnostic);
    panic!("{}", diagnostic.message);
}

/// Records the failure described by `info` and returns the id of the
/// diagnostic created for it. Does not halt, so tests can inspect the result.
pub fn capture(info: &PanicInfo) -> DiagnosticId {
    x86_64::instructions::interrupts::disable();

    // The code that was interrupted by the failure may have held any of the
    // kernel's locks and will never run again to release them. Without this
    // the very next `println!` could spin forever.
    unsafe { force_unlock_all() };

    let diagnostic = match PENDING.lock().take() {
        Some(diagnostic) => diagnostic,
        None => {
            let diagnostic = Diagnostic::new(
                Level::Panic,
                Subsystem::Kernel,
                "PANIC-001",
                format_args!("{}", info.message()),
            );
            match info.location() {
                Some(location) => diagnostic
                    .with("file", location.file())
                    .with("line", location.line())
                    .with("column", location.column()),
                None => diagnostic,
            }
        }
    };

    crate::println!();
    crate::println!("[PANIC] Kernel encountered an unrecoverable error.");
    let id = diagnostics::report(diagnostic);
    if let Some(stored) = diagnostics::find(id.0) {
        crate::println!("{}", stored);
    }
    crate::println!("Diagnostic information captured: {}", id);
    id
}

/// The kernel's panic handler: capture the failure, then halt for good.
pub fn handle_panic(info: &PanicInfo) -> ! {
    if PANICKING.swap(true, Ordering::SeqCst) {
        // A panic while handling a panic: do the minimum and stop.
        x86_64::instructions::interrupts::disable();
        unsafe { crate::serial::SERIAL1.force_unlock() };
        crate::serial_println!("[PANIC] Panic while handling a panic. System halted.");
        crate::hlt_loop();
    }

    capture(info);
    crate::println!("System halted. Restart the machine to continue.");
    // Interrupts stay disabled, so `hlt` never returns.
    crate::hlt_loop();
}

/// Releases every global kernel lock.
///
/// # Safety
/// Only sound when the code that held the locks will never resume, i.e. on
/// the way to halting the machine with interrupts disabled.
unsafe fn force_unlock_all() {
    unsafe {
        crate::serial::SERIAL1.force_unlock();
        crate::framebuffer::WRITER.force_unlock();
        crate::logger::force_unlock();
        crate::diagnostics::force_unlock();
        crate::memory::force_unlock();
        crate::allocator::force_unlock();
        PENDING.force_unlock();
    }
}
