#![no_std]
#![no_main]

use bootloader_api::{BootInfo, entry_point};
use core::panic::PanicInfo;
use kernel::println;

entry_point!(kernel_main, config = &kernel::BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    kernel::init(boot_info);
    kernel::hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    kernel::hlt_loop();
}
