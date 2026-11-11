#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use kernel::println;
use bootloader_api::{entry_point, BootInfo};

entry_point!(main, config = &kernel::BOOTLOADER_CONFIG);

fn main(_boot_info: &'static mut BootInfo) -> ! {
    kernel::serial::init();
    test_main();
    kernel::hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel::test_panic_handler(info)
}

#[test_case]
fn test_println() {
    println!("test_println output");
}
