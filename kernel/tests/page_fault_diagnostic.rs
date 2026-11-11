//! A page fault in kernel code must be caught by the exception handler and
//! reported as an EXC-014 diagnostic that carries the faulting address.

#![no_std]
#![no_main]

use bootloader_api::{BootInfo, entry_point};
use core::panic::PanicInfo;
use kernel::logger::{Level, Subsystem};
use kernel::{QemuExitCode, diagnostics, exit_qemu, serial_println};

/// Nothing is mapped here.
const UNMAPPED_ADDRESS: u64 = 0xdead_beef_000;

entry_point!(main, config = &kernel::BOOTLOADER_CONFIG);

fn main(boot_info: &'static mut BootInfo) -> ! {
    kernel::init(boot_info);
    serial_println!("page_fault_diagnostic::page_fault_produces_structured_record...");

    unsafe { (UNMAPPED_ADDRESS as *mut u8).write_volatile(42) };

    serial_println!("[failed] execution continued after the page fault");
    exit_qemu(QemuExitCode::Failed);
    kernel::hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let id = kernel::panic::capture(info);

    let passed = match diagnostics::find(id.0) {
        Some(record) => {
            record.code == "EXC-014"
                && record.severity == Level::Panic
                && record.subsystem == Subsystem::Interrupt
                && record.context_value("address") == Some("0xdeadbeef000")
                && record.context_value("access") == Some("write")
                && record.context_value("cause") == Some("page not present")
                && record.context_value("rip").is_some()
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
