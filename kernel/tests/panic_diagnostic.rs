//! A kernel panic must leave a structured PANIC-001 diagnostic record that
//! names the panic message and its source location.

#![no_std]
#![no_main]

use bootloader_api::{BootInfo, entry_point};
use core::panic::PanicInfo;
use kernel::logger::{Level, Subsystem};
use kernel::{QemuExitCode, diagnostics, exit_qemu, serial_println};

entry_point!(main, config = &kernel::BOOTLOADER_CONFIG);

fn main(boot_info: &'static mut BootInfo) -> ! {
    kernel::init(boot_info);
    serial_println!("panic_diagnostic::panic_produces_structured_record...");
    panic!("controlled test panic {}", 42);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let frames_before = kernel::protocol::frames_sent();
    let panics_before = kernel::logger::count(Level::Panic);

    let id = kernel::panic::capture(info);

    let passed = match diagnostics::find(id.0) {
        Some(record) => {
            record.code == "PANIC-001"
                && record.severity == Level::Panic
                && record.subsystem == Subsystem::Kernel
                && record.message == "controlled test panic 42"
                && record.context_value("file") == Some("kernel/tests/panic_diagnostic.rs")
                && record.context_value("line").is_some()
                && kernel::protocol::frames_sent() == frames_before + 1
                && kernel::logger::count(Level::Panic) == panics_before + 1
        }
        None => false,
    };

    if passed {
        serial_println!("[ok]");
        exit_qemu(QemuExitCode::Success);
    } else {
        serial_println!("[failed]");
        exit_qemu(QemuExitCode::Failed);
    }
    kernel::hlt_loop();
}
