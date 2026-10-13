#![no_std]
#![no_main]

use bootloader_api::{BootInfo, entry_point};
use core::panic::PanicInfo;

mod serial;
mod vga_buffer;

entry_point!(kernel_main);

fn kernel_main(_boot_info: &'static mut BootInfo) -> ! {
    serial::init();
    serial::write_str("[INFO] Psydian kernel booted.\n");

    println!("Hello World from Psydian OS!");
    println!("VGA text mode is working!");

    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}
