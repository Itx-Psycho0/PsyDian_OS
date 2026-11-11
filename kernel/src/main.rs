#![no_std]
#![no_main]

use bootloader_api::{BootInfo, entry_point};
use core::panic::PanicInfo;
use kernel::{println, serial_println};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    // 1. Serial first: it needs no memory mapping, so it works even if everything else breaks.
    kernel::serial::init();
    serial_println!("[INFO] Psydian kernel booted.");

    // 2. Screen output: take over the framebuffer the bootloader gave us.
    //    (The old VGA text buffer at 0xb8000 is not mapped by bootloader 0.11 -> page fault.)
    match boot_info.framebuffer.as_mut() {
        Some(framebuffer) => kernel::framebuffer::init(framebuffer),
        None => {
            serial_println!("[WARN] bootloader provided no framebuffer; serial output only");
        }
    }

    println!("Hello World from Psydian OS!");
    println!("Framebuffer console is working!");

    // 3. GDT -> IDT -> PIC -> enable interrupts.
    kernel::init();

    println!("It did not crash!");

    println!("Waiting for interrupts...");
    kernel::hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    kernel::hlt_loop();
}
