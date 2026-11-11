//! Physical frame allocator backed by the bootloader's memory map.

use bootloader_api::info::{MemoryRegion, MemoryRegionKind};
use x86_64::PhysAddr;
use x86_64::structures::paging::{FrameAllocator, PhysFrame, Size4KiB};

const FRAME_SIZE: u64 = 4096;

/// Frames below 1 MiB are never handed out: that range holds BIOS data and
/// legacy device memory even where the firmware reports it as usable.
const LOWEST_FRAME_ADDRESS: u64 = 0x10_0000;

/// Hands out unused 4 KiB frames, walking the usable regions in order.
///
/// It is a bump allocator: frames are never returned. That is enough for the
/// kernel today, which only maps memory (heap, page tables) and never unmaps.
pub struct BootInfoFrameAllocator {
    regions: &'static [MemoryRegion],
    /// Region the next frame is taken from.
    region_index: usize,
    /// Physical address of the next candidate frame.
    next_address: u64,
    allocated: usize,
}

impl BootInfoFrameAllocator {
    /// # Safety
    /// `regions` must be the memory map passed by the bootloader: every region
    /// marked `Usable` must really be unused RAM.
    pub unsafe fn new(regions: &'static [MemoryRegion]) -> Self {
        Self {
            regions,
            region_index: 0,
            next_address: 0,
            allocated: 0,
        }
    }

    /// Number of frames handed out so far.
    pub fn allocated_frames(&self) -> usize {
        self.allocated
    }

    /// Number of frames the memory map can provide in total.
    pub fn total_frames(&self) -> usize {
        self.regions
            .iter()
            .filter_map(usable_frame_range)
            .map(|(start, end)| ((end - start) / FRAME_SIZE) as usize)
            .sum()
    }
}

/// The frame-aligned part of a usable region, or `None` if it holds no frame.
fn usable_frame_range(region: &MemoryRegion) -> Option<(u64, u64)> {
    if region.kind != MemoryRegionKind::Usable {
        return None;
    }
    let start = region.start.max(LOWEST_FRAME_ADDRESS).next_multiple_of(FRAME_SIZE);
    let end = region.end / FRAME_SIZE * FRAME_SIZE;
    (start < end).then_some((start, end))
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        while let Some(region) = self.regions.get(self.region_index) {
            if let Some((start, end)) = usable_frame_range(region) {
                let candidate = self.next_address.max(start);
                if candidate < end {
                    self.next_address = candidate + FRAME_SIZE;
                    self.allocated += 1;
                    return Some(PhysFrame::containing_address(PhysAddr::new(candidate)));
                }
            }
            // Region exhausted or not usable: continue with the next one.
            self.region_index += 1;
            self.next_address = 0;
        }
        None
    }
}

#[cfg(test)]
fn region(start: u64, end: u64, kind: MemoryRegionKind) -> MemoryRegion {
    MemoryRegion { start, end, kind }
}

#[test_case]
fn skips_unusable_regions_and_low_memory() {
    static REGIONS: spin::Once<[MemoryRegion; 3]> = spin::Once::new();
    let regions = REGIONS.call_once(|| {
        [
            region(0x0, 0x9_f000, MemoryRegionKind::Usable), // below 1 MiB
            region(0x10_0000, 0x20_0000, MemoryRegionKind::Bootloader),
            region(0x20_0000, 0x20_3000, MemoryRegionKind::Usable), // 3 frames
        ]
    });
    let mut allocator = unsafe { BootInfoFrameAllocator::new(regions) };
    assert_eq!(allocator.total_frames(), 3);

    for expected in [0x20_0000u64, 0x20_1000, 0x20_2000] {
        let frame = allocator.allocate_frame().expect("frame must be available");
        assert_eq!(frame.start_address().as_u64(), expected);
    }
    assert!(allocator.allocate_frame().is_none());
    assert_eq!(allocator.allocated_frames(), 3);
}

#[test_case]
fn aligns_partial_regions_to_frame_boundaries() {
    static REGIONS: spin::Once<[MemoryRegion; 1]> = spin::Once::new();
    let regions =
        REGIONS.call_once(|| [region(0x40_0800, 0x40_2800, MemoryRegionKind::Usable)]);
    let mut allocator = unsafe { BootInfoFrameAllocator::new(regions) };
    // Only 0x401000..0x402000 is a complete frame inside the region.
    assert_eq!(allocator.total_frames(), 1);
    let frame = allocator.allocate_frame().expect("frame must be available");
    assert_eq!(frame.start_address().as_u64(), 0x40_1000);
    assert!(allocator.allocate_frame().is_none());
}
