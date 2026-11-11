//! Kernel heap allocator.
//!
//! A fixed-size block allocator: every allocation is rounded up to one of a
//! few block sizes, and each size has its own free list. Allocating and
//! freeing are then a single list push/pop, which is much faster than
//! searching a general free list and does not fragment for small objects.
//! Requests larger than the biggest block size (or with unusual alignment)
//! fall back to a linked-list allocator that manages the heap as a whole.
//!
//! Design reference: <https://os.phil-opp.com/allocator-designs/>

use core::alloc::{GlobalAlloc, Layout};
use core::mem;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;

/// Block sizes in bytes. Each must be a power of two because the size is also
/// used as the block alignment.
const BLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];

#[global_allocator]
static ALLOCATOR: Locked<FixedSizeBlockAllocator> = Locked::new(FixedSizeBlockAllocator::new());

static HEAP_SIZE: AtomicUsize = AtomicUsize::new(0);
static USED_BYTES: AtomicUsize = AtomicUsize::new(0);
static PEAK_USED_BYTES: AtomicUsize = AtomicUsize::new(0);
static ALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);
static LIVE_ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static FAILED_ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

/// Heap usage counters, shown by the `meminfo` shell command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeapStats {
    /// Total heap size in bytes (0 until the heap is initialised).
    pub size: usize,
    /// Bytes currently handed out (block-size granularity).
    pub used: usize,
    /// Highest value `used` has reached.
    pub peak_used: usize,
    /// Allocations made since boot.
    pub allocations: usize,
    /// Allocations that have not been freed yet.
    pub live_allocations: usize,
    /// Allocation requests that could not be satisfied.
    pub failed_allocations: usize,
}

impl HeapStats {
    pub fn free(&self) -> usize {
        self.size.saturating_sub(self.used)
    }
}

/// Hands the heap memory to the allocator.
///
/// # Safety
/// `heap_start..heap_start + heap_size` must be mapped, writable, unused
/// memory, and this must be called only once.
pub unsafe fn init(heap_start: usize, heap_size: usize) {
    unsafe { ALLOCATOR.lock().init(heap_start, heap_size) };
    HEAP_SIZE.store(heap_size, Ordering::Relaxed);
}

/// Current heap usage.
pub fn stats() -> HeapStats {
    HeapStats {
        size: HEAP_SIZE.load(Ordering::Relaxed),
        used: USED_BYTES.load(Ordering::Relaxed),
        peak_used: PEAK_USED_BYTES.load(Ordering::Relaxed),
        allocations: ALLOCATION_COUNT.load(Ordering::Relaxed),
        live_allocations: LIVE_ALLOCATIONS.load(Ordering::Relaxed),
        failed_allocations: FAILED_ALLOCATIONS.load(Ordering::Relaxed),
    }
}

/// Releases the allocator lock unconditionally.
///
/// # Safety
/// Only for the panic path, where the interrupted code that held the lock
/// will never run again.
pub unsafe fn force_unlock() {
    unsafe { ALLOCATOR.inner.force_unlock() };
}

/// A spinlock wrapper, needed because `GlobalAlloc` cannot be implemented
/// directly for the foreign type `spin::Mutex`.
pub struct Locked<A> {
    inner: Mutex<A>,
}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Self {
            inner: Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> spin::MutexGuard<'_, A> {
        self.inner.lock()
    }
}

/// A free block stores the pointer to the next free block inside itself.
struct ListNode {
    next: Option<&'static mut ListNode>,
}

pub struct FixedSizeBlockAllocator {
    list_heads: [Option<&'static mut ListNode>; BLOCK_SIZES.len()],
    fallback_allocator: linked_list_allocator::Heap,
}

impl FixedSizeBlockAllocator {
    pub const fn new() -> Self {
        const EMPTY: Option<&'static mut ListNode> = None;
        Self {
            list_heads: [EMPTY; BLOCK_SIZES.len()],
            fallback_allocator: linked_list_allocator::Heap::empty(),
        }
    }

    /// # Safety
    /// See [`init`].
    unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        unsafe { self.fallback_allocator.init(heap_start as *mut u8, heap_size) };
    }

    fn fallback_alloc(&mut self, layout: Layout) -> *mut u8 {
        match self.fallback_allocator.allocate_first_fit(layout) {
            Ok(ptr) => ptr.as_ptr(),
            Err(_) => ptr::null_mut(),
        }
    }
}

impl Default for FixedSizeBlockAllocator {
    fn default() -> Self {
        Self::new()
    }
}

/// Index of the smallest block size that fits `layout`, if any.
fn list_index(layout: &Layout) -> Option<usize> {
    let required = layout.size().max(layout.align());
    BLOCK_SIZES.iter().position(|&size| size >= required)
}

/// Bytes the allocator really reserves for `layout`.
fn reserved_size(layout: &Layout) -> usize {
    match list_index(layout) {
        Some(index) => BLOCK_SIZES[index],
        None => layout.size(),
    }
}

unsafe impl GlobalAlloc for Locked<FixedSizeBlockAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();
        let ptr = match list_index(&layout) {
            Some(index) => match allocator.list_heads[index].take() {
                Some(node) => {
                    allocator.list_heads[index] = node.next.take();
                    node as *mut ListNode as *mut u8
                }
                None => {
                    // No free block of this size yet: carve a new one out of
                    // the fallback heap.
                    let block_size = BLOCK_SIZES[index];
                    match Layout::from_size_align(block_size, block_size) {
                        Ok(block_layout) => allocator.fallback_alloc(block_layout),
                        Err(_) => ptr::null_mut(),
                    }
                }
            },
            None => allocator.fallback_alloc(layout),
        };
        drop(allocator);

        if ptr.is_null() {
            FAILED_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        } else {
            let used = USED_BYTES.fetch_add(reserved_size(&layout), Ordering::Relaxed)
                + reserved_size(&layout);
            PEAK_USED_BYTES.fetch_max(used, Ordering::Relaxed);
            ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
            LIVE_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut allocator = self.lock();
        match list_index(&layout) {
            Some(index) => {
                // The block is big and aligned enough to hold a ListNode
                // because the smallest block size is 8 bytes.
                debug_assert!(mem::size_of::<ListNode>() <= BLOCK_SIZES[index]);
                debug_assert!(mem::align_of::<ListNode>() <= BLOCK_SIZES[index]);
                let new_node = ListNode {
                    next: allocator.list_heads[index].take(),
                };
                let new_node_ptr = ptr as *mut ListNode;
                unsafe {
                    new_node_ptr.write(new_node);
                    allocator.list_heads[index] = Some(&mut *new_node_ptr);
                }
            }
            None => {
                if let Some(ptr) = NonNull::new(ptr) {
                    unsafe { allocator.fallback_allocator.deallocate(ptr, layout) };
                }
            }
        }
        drop(allocator);

        USED_BYTES.fetch_sub(reserved_size(&layout), Ordering::Relaxed);
        LIVE_ALLOCATIONS.fetch_sub(1, Ordering::Relaxed);
    }
}

#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!(
        "heap allocation failed: size={} align={}",
        layout.size(),
        layout.align()
    );
}

#[test_case]
fn block_size_selection() {
    let layout = |size, align| Layout::from_size_align(size, align).unwrap();
    assert_eq!(list_index(&layout(1, 1)), Some(0));
    assert_eq!(list_index(&layout(8, 8)), Some(0));
    assert_eq!(list_index(&layout(9, 1)), Some(1));
    // Alignment larger than the size decides the block.
    assert_eq!(list_index(&layout(4, 64)), Some(3));
    assert_eq!(list_index(&layout(2048, 8)), Some(BLOCK_SIZES.len() - 1));
    // Too big for any block: handled by the fallback allocator.
    assert_eq!(list_index(&layout(2049, 8)), None);
    assert_eq!(reserved_size(&layout(100, 4)), 128);
    assert_eq!(reserved_size(&layout(5000, 8)), 5000);
}

#[test_case]
fn stats_track_allocations() {
    use alloc::boxed::Box;

    let before = stats();
    let value = Box::new(0xdead_beef_u64);
    let during = stats();
    assert_eq!(during.used, before.used + 8);
    assert_eq!(during.live_allocations, before.live_allocations + 1);
    assert_eq!(during.allocations, before.allocations + 1);
    assert_eq!(*value, 0xdead_beef_u64);
    drop(value);

    let after = stats();
    assert_eq!(after.used, before.used);
    assert_eq!(after.live_allocations, before.live_allocations);
    assert!(after.peak_used >= during.used);
}

#[test_case]
fn failed_allocation_returns_null_and_is_counted() {
    let before = stats().failed_allocations;
    // Far larger than the whole heap.
    let layout = Layout::from_size_align(1 << 30, 8).unwrap();
    let ptr = unsafe { alloc::alloc::alloc(layout) };
    assert!(ptr.is_null());
    assert_eq!(stats().failed_allocations, before + 1);
}
