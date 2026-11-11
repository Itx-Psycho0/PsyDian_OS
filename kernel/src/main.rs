#![no_std]
#![no_main]

use bootloader_api::{BootInfo, entry_point};
use core::panic::PanicInfo;
use kernel::logger::Subsystem;
use kernel::shell::Shell;
use kernel::info;

entry_point!(kernel_main, config = &kernel::BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    kernel::init(boot_info);

    info!(Subsystem::Shell, "SHELL-001", "Shell started.");
    // Tells a listening AI Bridge that the kernel is up and accepting input.
    kernel::protocol::send_status("ready");
    Shell::new().run();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel::panic::handle_panic(info)
}
