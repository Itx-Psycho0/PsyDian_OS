//! Kernel heap region.
//!
//! The heap is a fixed virtual address range. `init` backs it with physical
//! frames and then hands the range to the global allocator.

use super::MemoryError;
use x86_64::VirtAddr;
use x86_64::structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB};

/// Start of the heap in virtual memory. Any unused address works; this one is
/// easy to recognise in a debugger or a page-fault report.
pub const HEAP_START: usize = 0x_4444_4444_0000;

/// Heap size: 1 MiB.
pub const HEAP_SIZE: usize = 1024 * 1024;

/// Maps the heap pages and initialises the global allocator.
pub fn init(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MemoryError> {
    let heap_start = VirtAddr::new(HEAP_START as u64);
    let heap_end = heap_start + (HEAP_SIZE as u64 - 1);
    let pages = Page::range_inclusive(
        Page::containing_address(heap_start),
        Page::containing_address(heap_end),
    );

    for page in pages {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MemoryError::OutOfFrames)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            mapper
                .map_to(page, frame, flags, frame_allocator)
                .map_err(|_| MemoryError::HeapMappingFailed)?
                .flush();
        }
    }

    unsafe { crate::allocator::init(HEAP_START, HEAP_SIZE) };
    Ok(())
}
