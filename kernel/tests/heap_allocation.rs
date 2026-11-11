//! Exercises the kernel heap through the `alloc` collection types.

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use bootloader_api::{BootInfo, entry_point};
use core::panic::PanicInfo;
use kernel::memory::heap::HEAP_SIZE;

entry_point!(main, config = &kernel::BOOTLOADER_CONFIG);

fn main(boot_info: &'static mut BootInfo) -> ! {
    kernel::init(boot_info);
    test_main();
    kernel::hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel::test_panic_handler(info)
}

#[test_case]
fn simple_allocation() {
    let heap_value_1 = Box::new(41);
    let heap_value_2 = Box::new(13);
    assert_eq!(*heap_value_1, 41);
    assert_eq!(*heap_value_2, 13);
}

#[test_case]
fn large_vec() {
    let n = 1000;
    let mut vec = Vec::new();
    for i in 0..n {
        vec.push(i);
    }
    assert_eq!(vec.iter().sum::<u64>(), (n - 1) * n / 2);
}

#[test_case]
fn string_growth() {
    let mut text = String::new();
    for _ in 0..100 {
        text.push_str("psydian ");
    }
    assert_eq!(text.len(), 800);
    assert!(text.starts_with("psydian psydian"));
}

/// Allocates more bytes in total than the heap holds. This only works if
/// freed memory is really reused.
#[test_case]
fn many_boxes() {
    for i in 0..HEAP_SIZE {
        let x = Box::new(i);
        assert_eq!(*x, i);
    }
}

/// Same, while a long-lived allocation sits in the middle of the heap.
#[test_case]
fn many_boxes_long_lived() {
    let long_lived = Box::new(1);
    for i in 0..HEAP_SIZE {
        let x = Box::new(i);
        assert_eq!(*x, i);
    }
    assert_eq!(*long_lived, 1);
}

/// Allocations bigger than the largest block size use the fallback allocator.
#[test_case]
fn large_allocation_uses_fallback() {
    let big: Vec<u8> = alloc::vec![7; 64 * 1024];
    assert_eq!(big.len(), 64 * 1024);
    assert!(big.iter().all(|&byte| byte == 7));
}

#[test_case]
fn heap_is_fully_released_after_use() {
    let before = kernel::allocator::stats();
    {
        let _a = Box::new([0u8; 100]);
        let _b: Vec<u32> = Vec::with_capacity(500);
    }
    let after = kernel::allocator::stats();
    assert_eq!(after.used, before.used);
    assert_eq!(after.live_allocations, before.live_allocations);
}
