//! Access to the active page tables.
//!
//! The bootloader maps the *complete* physical memory at a fixed virtual
//! offset (see `BOOTLOADER_CONFIG` in `lib.rs`). Any physical address can
//! therefore be reached at `physical_memory_offset + address`, which is how
//! the kernel reads and edits its own page tables.

use x86_64::VirtAddr;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{OffsetPageTable, PageTable};

/// Creates a mapper for the page tables that are currently active.
///
/// # Safety
/// The caller must guarantee that all physical memory is mapped at
/// `physical_memory_offset` and must call this only once, because the
/// returned mapper holds a `&mut` to the level 4 table.
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    unsafe {
        let level_4_table = active_level_4_table(physical_memory_offset);
        OffsetPageTable::new(level_4_table, physical_memory_offset)
    }
}

/// Returns the level 4 page table the CPU is using right now (from CR3).
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    let (level_4_frame, _flags) = Cr3::read();
    let virt = physical_memory_offset + level_4_frame.start_address().as_u64();
    let table: *mut PageTable = virt.as_mut_ptr();
    unsafe { &mut *table }
}
