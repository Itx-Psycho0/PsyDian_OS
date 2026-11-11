//! Memory management: boot-time memory map, paging and the kernel heap.
//!
//! Initialisation order (called once from `kernel::init`):
//!
//! 1. Read the physical-memory offset and memory map from the bootloader.
//! 2. Build a mapper for the active page tables ([`paging`]).
//! 3. Build a physical frame allocator from the usable regions ([`frame`]).
//! 4. Map the heap and start the global allocator ([`heap`]).

pub mod frame;
pub mod heap;
pub mod paging;

use bootloader_api::info::{MemoryRegion, MemoryRegionKind};
use core::fmt;
use frame::BootInfoFrameAllocator;
use spin::Mutex;
use x86_64::instructions::interrupts::without_interrupts;
use x86_64::structures::paging::{OffsetPageTable, Translate};
use x86_64::{PhysAddr, VirtAddr};

/// Why memory initialisation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryError {
    /// The bootloader did not map physical memory (wrong bootloader config).
    NoPhysicalMemoryMapping,
    /// `init` was called a second time.
    AlreadyInitialized,
    /// The memory map has no free frame left.
    OutOfFrames,
    /// A heap page could not be mapped (for example it was already mapped).
    HeapMappingFailed,
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            MemoryError::NoPhysicalMemoryMapping => "bootloader did not map physical memory",
            MemoryError::AlreadyInitialized => "memory subsystem is already initialized",
            MemoryError::OutOfFrames => "no free physical frame available",
            MemoryError::HeapMappingFailed => "failed to map a heap page",
        })
    }
}

/// Snapshot of the memory state, shown by the `meminfo` shell command.
#[derive(Debug, Clone, Copy)]
pub struct MemoryStats {
    /// Number of entries in the bootloader's memory map.
    pub total_regions: usize,
    /// Entries marked usable.
    pub usable_regions: usize,
    /// Bytes of RAM the firmware reported as usable.
    pub usable_bytes: u64,
    /// 4 KiB frames available to the frame allocator.
    pub total_frames: usize,
    /// Frames handed out so far (heap + page tables).
    pub allocated_frames: usize,
    /// Virtual address at which all physical memory is mapped.
    pub physical_memory_offset: u64,
    pub heap: crate::allocator::HeapStats,
}

struct MemoryManager {
    mapper: OffsetPageTable<'static>,
    frame_allocator: BootInfoFrameAllocator,
    regions: &'static [MemoryRegion],
    physical_memory_offset: VirtAddr,
}

static MANAGER: Mutex<Option<MemoryManager>> = Mutex::new(None);

/// Sets up paging access, the frame allocator and the kernel heap.
pub fn init(
    physical_memory_offset: Option<u64>,
    regions: &'static [MemoryRegion],
) -> Result<(), MemoryError> {
    let offset = physical_memory_offset.ok_or(MemoryError::NoPhysicalMemoryMapping)?;
    let offset = VirtAddr::new(offset);

    let mut manager = MANAGER.lock();
    if manager.is_some() {
        return Err(MemoryError::AlreadyInitialized);
    }

    // Safety: the bootloader mapped all physical memory at `offset` (requested
    // in BOOTLOADER_CONFIG) and `regions` is its memory map. The
    // `AlreadyInitialized` check above guarantees this runs only once.
    let mut mapper = unsafe { paging::init(offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::new(regions) };

    heap::init(&mut mapper, &mut frame_allocator)?;

    *manager = Some(MemoryManager {
        mapper,
        frame_allocator,
        regions,
        physical_memory_offset: offset,
    });
    Ok(())
}

/// `true` once [`init`] has completed successfully.
pub fn is_initialized() -> bool {
    without_interrupts(|| MANAGER.lock().is_some())
}

/// Current memory statistics, or `None` before initialisation.
pub fn stats() -> Option<MemoryStats> {
    without_interrupts(|| {
        let guard = MANAGER.lock();
        let manager = guard.as_ref()?;
        let usable = manager
            .regions
            .iter()
            .filter(|region| region.kind == MemoryRegionKind::Usable);
        Some(MemoryStats {
            total_regions: manager.regions.len(),
            usable_regions: usable.clone().count(),
            usable_bytes: usable.map(|region| region.end - region.start).sum(),
            total_frames: manager.frame_allocator.total_frames(),
            allocated_frames: manager.frame_allocator.allocated_frames(),
            physical_memory_offset: manager.physical_memory_offset.as_u64(),
            heap: crate::allocator::stats(),
        })
    })
}

/// Translates a virtual address to the physical address it is mapped to.
///
/// Returns `None` if the address is not mapped or memory is not initialised.
pub fn translate(address: VirtAddr) -> Option<PhysAddr> {
    without_interrupts(|| MANAGER.lock().as_ref()?.mapper.translate_addr(address))
}

/// Releases the manager lock unconditionally.
///
/// # Safety
/// Only for the panic path, where the interrupted code that held the lock
/// will never run again.
pub unsafe fn force_unlock() {
    unsafe { MANAGER.force_unlock() };
}

#[test_case]
fn heap_start_is_mapped() {
    assert!(is_initialized());
    assert!(translate(VirtAddr::new(heap::HEAP_START as u64)).is_some());
    let last_byte = (heap::HEAP_START + heap::HEAP_SIZE - 1) as u64;
    assert!(translate(VirtAddr::new(last_byte)).is_some());
}

#[test_case]
fn unmapped_address_does_not_translate() {
    // The page right after the heap was never mapped.
    let after_heap = (heap::HEAP_START + heap::HEAP_SIZE) as u64;
    assert!(translate(VirtAddr::new(after_heap)).is_none());
}

#[test_case]
fn kernel_code_translates_through_the_page_tables() {
    let address = VirtAddr::new(init as *const () as u64);
    assert!(translate(address).is_some());
}

#[test_case]
fn stats_describe_the_boot_memory_map() {
    let stats = stats().expect("memory must be initialized");
    assert!(stats.usable_regions > 0);
    assert!(stats.usable_bytes > 0);
    // At least the heap pages were taken from the frame allocator.
    assert!(stats.allocated_frames >= heap::HEAP_SIZE / 4096);
    assert!(stats.allocated_frames <= stats.total_frames);
    assert_eq!(stats.heap.size, heap::HEAP_SIZE);
}
