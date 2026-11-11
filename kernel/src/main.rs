#![no_std]
#![no_main]

use bootloader_api::{BootInfo, entry_point};
use core::panic::PanicInfo;
use kernel::input::Key;
use kernel::{print, println};

entry_point!(kernel_main, config = &kernel::BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    kernel::init(boot_info);

    // Temporary echo loop until the shell lands: shows that keyboard and
    // serial input arrive as decoded key events.
    println!("Type something (keyboard or serial terminal):");
    loop {
        match kernel::input::read_key() {
            Key::Char(c) => print!("{}", c),
            Key::Enter => println!(),
            other => print!("<{:?}>", other),
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    kernel::hlt_loop();
}
