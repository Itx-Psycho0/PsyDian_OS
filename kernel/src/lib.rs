#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader_api::BootInfo;
use bootloader_api::config::{BootloaderConfig, Mapping};
use core::panic::PanicInfo;
use logger::Subsystem;

pub mod allocator;
pub mod console;
pub mod fixed;
pub mod framebuffer;
pub mod gdt;
pub mod input;
pub mod interrupts;
pub mod logger;
pub mod memory;
pub mod ring;
pub mod serial;
pub mod shell;
pub mod time;

/// What the kernel asks of the bootloader. Every entry point (the kernel
/// binary and each integration test) must pass this to `entry_point!`.
pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    // Map all physical memory somewhere in the kernel's address space so the
    // kernel can reach its own page tables (see `memory::paging`).
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config.kernel_stack_size = 256 * 1024;
    config
};

/// Brings the kernel from "just entered by the bootloader" to "ready".
///
/// Order matters: serial comes first because it needs nothing else and is the
/// only output that survives an early failure; interrupts are enabled last,
/// once every handler they can reach is installed.
pub fn init(boot_info: &'static mut BootInfo) {
    serial::init();
    info!(Subsystem::Boot, "BOOT-001", "Psydian kernel booted.");
    info!(Subsystem::Serial, "SER-001", "Serial port COM1 initialized.");

    // The old VGA text buffer at 0xb8000 is not mapped by bootloader 0.11;
    // screen output goes through the framebuffer it hands us.
    match boot_info.framebuffer.as_mut() {
        Some(framebuffer) => {
            let info = framebuffer.info();
            framebuffer::init(framebuffer);
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

    gdt::init();
    interrupts::init_idt();
    info!(Subsystem::Interrupt, "INT-001", "GDT, TSS and IDT loaded.");

    let physical_memory_offset = boot_info.physical_memory_offset.into_option();
    match memory::init(physical_memory_offset, &boot_info.memory_regions) {
        Ok(()) => {
            if let Some(stats) = memory::stats() {
                info!(
                    Subsystem::Memory,
                    "MEM-010",
                    "Memory map: {} MiB usable in {} of {} regions.",
                    stats.usable_bytes / (1024 * 1024),
                    stats.usable_regions,
                    stats.total_regions
                );
                info!(
                    Subsystem::Memory,
                    "MEM-011",
                    "Paging initialized (physical memory mapped at {:#x}).",
                    stats.physical_memory_offset
                );
                info!(
                    Subsystem::Memory,
                    "MEM-012",
                    "Kernel heap initialized: {} KiB at {:#x}.",
                    stats.heap.size / 1024,
                    memory::heap::HEAP_START
                );
            }
        }
        // The kernel can still run without a heap: the logger, the shell and
        // the diagnostics use fixed-size buffers only.
        Err(err) => error!(
            Subsystem::Memory,
            "MEM-001",
            "Memory initialization failed: {}. Continuing without a heap.",
            err
        ),
    }

    time::init();
    unsafe {
        let mut pics = interrupts::PICS.lock();
        pics.initialize();
        // Unmask only IRQ0 (timer), IRQ1 (keyboard) and IRQ4 (serial COM1).
        // Every other IRQ line stays masked because it has no handler.
        pics.write_masks(0b1110_1100, 0b1111_1111);
    }
    x86_64::instructions::interrupts::enable();
    info!(
        Subsystem::Interrupt,
        "INT-002",
        "Hardware interrupts enabled (timer at {} Hz, keyboard, serial).",
        time::TICK_HZ
    );
    info!(
        Subsystem::Input,
        "INPUT-001",
        "Input ready: PS/2 keyboard and serial console."
    );

    info!(Subsystem::Boot, "BOOT-004", "Boot complete.");
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    hlt_loop();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

#[cfg(test)]
bootloader_api::entry_point!(test_kernel_main, config = &BOOTLOADER_CONFIG);

#[cfg(test)]
fn test_kernel_main(boot_info: &'static mut BootInfo) -> ! {
    init(boot_info);
    test_main();
    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
