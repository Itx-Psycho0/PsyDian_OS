#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

use bootloader_api::{BootInfo, entry_point};
use core::panic::PanicInfo;
use kernel::println;

entry_point!(kernel_main);

fn kernel_main(_boot_info: &'static mut BootInfo) -> ! {
    kernel::serial::init();
    kernel::serial::write_str("[INFO] Psydian kernel booted.\n");

    println!("Hello World from Psydian OS!");
    println!("VGA text mode is working!");

    kernel::init();

    // invoke a breakpoint exception
    x86_64::instructions::interrupts::int3();

    println!("It did not crash!");

    #[cfg(test)]
    test_main();

    loop {}
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel::test_panic_handler(info)
}
