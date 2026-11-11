#![no_std]
#![no_main]

use bootloader_api::{BootInfo, entry_point};
use core::panic::PanicInfo;
use kernel::logger::Subsystem;
use kernel::{info, println, warn};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    // 1. Serial first: it needs no memory mapping, so it works even if everything else breaks.
    kernel::serial::init();
    info!(Subsystem::Boot, "BOOT-001", "Psydian kernel booted.");
    info!(Subsystem::Serial, "SER-001", "Serial port COM1 initialized.");

    // 2. Screen output: take over the framebuffer the bootloader gave us.
    //    (The old VGA text buffer at 0xb8000 is not mapped by bootloader 0.11 -> page fault.)
    match boot_info.framebuffer.as_mut() {
        Some(framebuffer) => {
            let info = framebuffer.info();
            kernel::framebuffer::init(framebuffer);
            info!(
                Subsystem::Boot,
                "BOOT-002",
                "Framebuffer console initialized ({}x{}).",
                info.width,
                info.height
            );
        }
        None => warn!(
            Subsystem::Boot,
            "BOOT-003",
            "Bootloader provided no framebuffer; serial output only."
        ),
    }

    // 3. GDT -> IDT -> PIC -> enable interrupts.
    kernel::init();

    info!(Subsystem::Interrupt, "INT-001", "Interrupt subsystem initialized.");
    info!(Subsystem::Boot, "BOOT-004", "Boot complete.");

    kernel::hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    kernel::hlt_loop();
}
