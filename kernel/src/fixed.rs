//! Fixed-capacity string that lives entirely on the stack / in statics.
//!
//! The logger, the diagnostics store and the shell all have to work before the
//! heap exists and must keep working if the heap is exhausted, so they cannot
//! use `alloc::string::String`. `FixedString<N>` holds at most `N` bytes of
//! UTF-8 and silently truncates (on a character boundary) when it is full.

use core::fmt;

#[derive(Clone, Copy)]
pub struct FixedString<const N: usize> {
    buf: [u8; N],
    len: usize,
}

impl<const N: usize> FixedString<N> {
    pub const fn new() -> Self {
        Self { buf: [0; N], len: 0 }
    }

    /// Copies as much of `s` as fits.
    pub fn from_str_truncated(s: &str) -> Self {
        let mut out = Self::new();
        out.push_str(s);
        out
    }

    pub fn as_str(&self) -> &str {
        // Only whole characters are ever stored, so this cannot fail.
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub const fn capacity(&self) -> usize {
        N
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Appends one character. Returns `false` (and stores nothing) if it does not fit.
    pub fn push(&mut self, c: char) -> bool {
        let mut utf8 = [0u8; 4];
        let bytes = c.encode_utf8(&mut utf8).as_bytes();
        if self.len + bytes.len() > N {
            return false;
        }
        self.buf[self.len..self.len + bytes.len()].copy_from_slice(bytes);
        self.len += bytes.len();
        true
    }

    /// Appends as many whole characters of `s` as fit. Returns `false` if the
    /// text had to be truncated.
    pub fn push_str(&mut self, s: &str) -> bool {
        for c in s.chars() {
            if !self.push(c) {
                return false;
            }
        }
        true
    }

    /// Removes and returns the last character.
    pub fn pop(&mut self) -> Option<char> {
        let c = self.as_str().chars().next_back()?;
        self.len -= c.len_utf8();
        Some(c)
    }
}

impl<const N: usize> Default for FixedString<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> fmt::Write for FixedString<N> {
    /// Never fails: text that does not fit is dropped, so `write!` into a
    /// `FixedString` can be used on error paths without another error to handle.
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s);
        Ok(())
    }
}

impl<const N: usize> fmt::Display for FixedString<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<const N: usize> fmt::Debug for FixedString<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl<const N: usize> PartialEq<&str> for FixedString<N> {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl<const N: usize, const M: usize> PartialEq<FixedString<M>> for FixedString<N> {
    fn eq(&self, other: &FixedString<M>) -> bool {
        self.as_str() == other.as_str()
    }
}

#[test_case]
fn push_and_pop() {
    let mut s: FixedString<8> = FixedString::new();
    assert!(s.is_empty());
    assert!(s.push_str("abc"));
    assert_eq!(s, "abc");
    assert_eq!(s.pop(), Some('c'));
    assert_eq!(s.len(), 2);
    s.clear();
    assert_eq!(s.pop(), None);
}

#[test_case]
fn truncates_when_full() {
    let mut s: FixedString<4> = FixedString::new();
    assert!(!s.push_str("abcdef"));
    assert_eq!(s, "abcd");
    assert!(!s.push('x'));
    assert_eq!(s.len(), 4);
}

#[test_case]
fn truncates_on_char_boundary() {
    // 'é' is two bytes: only one of them would fit, so it must be dropped whole.
    let mut s: FixedString<3> = FixedString::new();
    assert!(!s.push_str("abé"));
    assert_eq!(s, "ab");
}

#[test_case]
fn write_macro_never_errors() {
    use core::fmt::Write;
    let mut s: FixedString<6> = FixedString::new();
    assert!(write!(s, "{}-{}", 123, 456789).is_ok());
    assert_eq!(s, "123-45");
}
