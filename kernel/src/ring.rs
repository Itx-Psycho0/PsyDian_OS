//! Fixed-capacity FIFO ring buffer.
//!
//! Used for the keyboard/serial input queue (reject new items when full) and
//! for the log / diagnostic history (overwrite the oldest item when full).

#[derive(Clone, Copy)]
pub struct RingBuffer<T: Copy, const N: usize> {
    items: [Option<T>; N],
    /// Index of the oldest item.
    head: usize,
    len: usize,
}

impl<T: Copy, const N: usize> RingBuffer<T, N> {
    pub const fn new() -> Self {
        Self {
            items: [None; N],
            head: 0,
            len: 0,
        }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub const fn is_full(&self) -> bool {
        self.len == N
    }

    pub const fn capacity(&self) -> usize {
        N
    }

    pub fn clear(&mut self) {
        self.items = [None; N];
        self.head = 0;
        self.len = 0;
    }

    /// Appends `item`, or hands it back if the buffer is full.
    pub fn push(&mut self, item: T) -> Result<(), T> {
        if self.is_full() {
            return Err(item);
        }
        let tail = (self.head + self.len) % N;
        self.items[tail] = Some(item);
        self.len += 1;
        Ok(())
    }

    /// Appends `item`, dropping the oldest entry if the buffer is full.
    pub fn push_overwrite(&mut self, item: T) {
        if N == 0 {
            return;
        }
        if self.is_full() {
            self.pop();
        }
        let _ = self.push(item);
    }

    /// Removes and returns the oldest item.
    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        let item = self.items[self.head].take();
        self.head = (self.head + 1) % N;
        self.len -= 1;
        item
    }

    /// Returns the item at `index`, where 0 is the oldest one.
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len {
            return None;
        }
        self.items[(self.head + index) % N].as_ref()
    }

    /// Iterates from the oldest item to the newest.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        (0..self.len).filter_map(move |i| self.get(i))
    }
}

impl<T: Copy, const N: usize> Default for RingBuffer<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

#[test_case]
fn fifo_order() {
    let mut ring: RingBuffer<u8, 4> = RingBuffer::new();
    assert!(ring.is_empty());
    for value in 1..=3 {
        assert!(ring.push(value).is_ok());
    }
    assert_eq!(ring.len(), 3);
    assert_eq!(ring.pop(), Some(1));
    assert_eq!(ring.pop(), Some(2));
    assert_eq!(ring.pop(), Some(3));
    assert_eq!(ring.pop(), None);
}

#[test_case]
fn push_rejects_when_full() {
    let mut ring: RingBuffer<u8, 2> = RingBuffer::new();
    assert!(ring.push(1).is_ok());
    assert!(ring.push(2).is_ok());
    assert!(ring.is_full());
    assert_eq!(ring.push(3), Err(3));
    assert_eq!(ring.pop(), Some(1));
}

#[test_case]
fn push_overwrite_drops_oldest() {
    let mut ring: RingBuffer<u8, 3> = RingBuffer::new();
    for value in 1..=5 {
        ring.push_overwrite(value);
    }
    assert_eq!(ring.len(), 3);
    assert_eq!(ring.get(0), Some(&3));
    assert_eq!(ring.get(2), Some(&5));
    assert_eq!(ring.get(3), None);
}

#[test_case]
fn wraps_around() {
    let mut ring: RingBuffer<u32, 3> = RingBuffer::new();
    for round in 0..10 {
        assert!(ring.push(round).is_ok());
        assert_eq!(ring.pop(), Some(round));
    }
    assert!(ring.is_empty());
    ring.push_overwrite(7);
    ring.push_overwrite(8);
    let mut seen = [0u32; 2];
    for (slot, value) in seen.iter_mut().zip(ring.iter()) {
        *slot = *value;
    }
    assert_eq!(seen, [7, 8]);
}
