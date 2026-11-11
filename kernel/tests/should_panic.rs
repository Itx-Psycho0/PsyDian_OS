//! Verifies that a failing assertion reaches the panic handler.

#![no_std]
#![no_main]

use bootloader_api::{BootInfo, entry_point};
use core::panic::PanicInfo;
use kernel::{QemuExitCode, exit_qemu, serial_print, serial_println};

entry_point!(main, config = &kernel::BOOTLOADER_CONFIG);

fn main(_boot_info: &'static mut BootInfo) -> ! {
    kernel::serial::init();
    should_fail();
    serial_println!("[test did not panic]");
    exit_qemu(QemuExitCode::Failed);
    kernel::hlt_loop();
}

fn should_fail() {
    serial_print!("should_panic::should_fail...\t");
    assert_eq!(0, 1);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
    kernel::hlt_loop();
}
