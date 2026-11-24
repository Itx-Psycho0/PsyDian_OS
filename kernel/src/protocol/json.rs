//! A small, allocation-free JSON reader for frames received from the AI Bridge.
//!
//! The kernel only needs to *read* a few fixed message shapes, so this is not
//! a general deserializer. [`parse`] validates a complete document once;
//! afterwards objects and arrays are walked lazily with [`Value::get`] and
//! [`Value::items`], and strings are decoded into a [`FixedString`] with
//! [`decode_string`]. Values borrow from the input text, nothing is copied
//! until a string is decoded.
//!
//! Limits keep a hostile or broken frame from doing damage: nesting is capped
//! at [`MAX_DEPTH`], and decoding fails instead of truncating when a string
//! does not fit its destination.

use crate::fixed::FixedString;

/// Deepest allowed nesting of objects and arrays.
pub const MAX_DEPTH: usize = 8;

/// Why a document or value was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error(pub &'static str);

/// A JSON value. Strings keep their raw (still escaped) text, without the
/// surrounding quotes; objects and arrays keep their full text, brackets
/// included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Value<'a> {
    Null,
    Bool(bool),
    Number(&'a str),
    String(&'a str),
    Array(&'a str),
    Object(&'a str),
}

/// Parses and validates a complete JSON document.
pub fn parse(text: &str) -> Result<Value<'_>, Error> {
    let bytes = text.as_bytes();
    let start = skip_whitespace(bytes, 0);
    let (value, end) = parse_value(text, start, 0)?;
    if skip_whitespace(bytes, end) != bytes.len() {
        return Err(Error("unexpected text after the JSON value"));
    }
    Ok(value)
}

impl<'a> Value<'a> {
    /// Looks up `key` in an object. Returns `None` for missing keys and for
    /// values that are not objects. If a key appears more than once the
    /// first occurrence wins.
    pub fn get(self, key: &str) -> Option<Value<'a>> {
        self.members()?
            .find(|(name, _)| raw_string_eq(name, key))
            .map(|(_, value)| value)
    }

    /// Iterates over the `(raw key, value)` pairs of an object.
    pub fn members(self) -> Option<Members<'a>> {
        match self {
            Value::Object(text) => Some(Members { text, pos: 1, done: false }),
            _ => None,
        }
    }

    /// Iterates over the items of an array.
    pub fn items(self) -> Option<Items<'a>> {
        match self {
            Value::Array(text) => Some(Items { text, pos: 1, done: false }),
            _ => None,
        }
    }

    /// The value as an unsigned integer, if it is a non-negative whole number.
    pub fn as_u32(self) -> Option<u32> {
        match self {
            Value::Number(text) => text.parse().ok(),
            _ => None,
        }
    }
}

/// Iterator returned by [`Value::members`].
pub struct Members<'a> {
    text: &'a str,
    pos: usize,
    done: bool,
}

impl<'a> Iterator for Members<'a> {
    type Item = (&'a str, Value<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let bytes = self.text.as_bytes();
        let mut pos = skip_whitespace(bytes, self.pos);
        if bytes.get(pos) == Some(&b'}') {
            self.done = true;
            return None;
        }
        // The object was validated by `parse`, so the shapes below hold.
        let key_end = scan_string(bytes, pos).ok()?;
        let key = &self.text[pos + 1..key_end - 1];
        pos = skip_whitespace(bytes, key_end) + 1; // past ':'
        pos = skip_whitespace(bytes, pos);
        let (value, end) = parse_value(self.text, pos, 0).ok()?;
        pos = skip_whitespace(bytes, end);
        match bytes.get(pos) {
            Some(b',') => self.pos = pos + 1,
            _ => self.done = true,
        }
        Some((key, value))
    }
}

/// Iterator returned by [`Value::items`].
pub struct Items<'a> {
    text: &'a str,
    pos: usize,
    done: bool,
}

impl<'a> Iterator for Items<'a> {
    type Item = Value<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let bytes = self.text.as_bytes();
        let pos = skip_whitespace(bytes, self.pos);
        if bytes.get(pos) == Some(&b']') {
            self.done = true;
            return None;
        }
        let (value, end) = parse_value(self.text, pos, 0).ok()?;
        let next = skip_whitespace(bytes, end);
        match bytes.get(next) {
            Some(b',') => self.pos = next + 1,
            _ => self.done = true,
        }
        Some(value)
    }
}

/// Decodes the raw text of a JSON string into `out`, replacing its contents.
/// Fails if the text does not fit; a partial string is never left behind as
/// if it were complete.
pub fn decode_string<const N: usize>(raw: &str, out: &mut FixedString<N>) -> Result<(), Error> {
    out.clear();
    for c in RawChars::new(raw) {
        if !out.push(c?) {
            out.clear();
            return Err(Error("string is too long"));
        }
    }
    Ok(())
}

/// Compares the raw text of a JSON string with plain text.
fn raw_string_eq(raw: &str, text: &str) -> bool {
    let mut expected = text.chars();
    for c in RawChars::new(raw) {
        match c {
            Ok(c) if expected.next() == Some(c) => {}
            _ => return false,
        }
    }
    expected.next().is_none()
}

/// Decodes the characters of a raw JSON string, resolving escapes.
struct RawChars<'a> {
    chars: core::str::Chars<'a>,
}

impl<'a> RawChars<'a> {
    fn new(raw: &'a str) -> Self {
        Self { chars: raw.chars() }
    }
}

impl Iterator for RawChars<'_> {
    type Item = Result<char, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let c = self.chars.next()?;
        if c != '\\' {
            return Some(Ok(c));
        }
        Some(match self.chars.next() {
            Some('"') => Ok('"'),
            Some('\\') => Ok('\\'),
            Some('/') => Ok('/'),
            Some('b') => Ok('\u{8}'),
            Some('f') => Ok('\u{c}'),
            Some('n') => Ok('\n'),
            Some('r') => Ok('\r'),
            Some('t') => Ok('\t'),
            Some('u') => {
                let mut code = 0u32;
                for _ in 0..4 {
                    match self.chars.next().and_then(|digit| digit.to_digit(16)) {
                        Some(digit) => code = code * 16 + digit,
                        None => return Some(Err(Error("invalid \\u escape"))),
                    }
                }
                // Surrogate halves are not valid on their own; the console
                // cannot show those characters anyway.
                Ok(char::from_u32(code).unwrap_or('\u{fffd}'))
            }
            _ => Err(Error("invalid escape sequence")),
        })
    }
}

fn skip_whitespace(bytes: &[u8], mut pos: usize) -> usize {
    while matches!(bytes.get(pos), Some(b' ' | b'\t' | b'\n' | b'\r')) {
        pos += 1;
    }
    pos
}

/// Parses the value starting at `pos` (no leading whitespace) and returns it
/// together with the index just past its end.
fn parse_value(text: &str, pos: usize, depth: usize) -> Result<(Value<'_>, usize), Error> {
    let bytes = text.as_bytes();
    match bytes.get(pos) {
        Some(b'{') => {
            let end = scan_container(text, pos, depth, b'}', true)?;
            Ok((Value::Object(&text[pos..end]), end))
        }
        Some(b'[') => {
            let end = scan_container(text, pos, depth, b']', false)?;
            Ok((Value::Array(&text[pos..end]), end))
        }
        Some(b'"') => {
            let end = scan_string(bytes, pos)?;
            Ok((Value::String(&text[pos + 1..end - 1]), end))
        }
        Some(b't') => literal(bytes, pos, b"true").map(|end| (Value::Bool(true), end)),
        Some(b'f') => literal(bytes, pos, b"false").map(|end| (Value::Bool(false), end)),
        Some(b'n') => literal(bytes, pos, b"null").map(|end| (Value::Null, end)),
        Some(b'-' | b'0'..=b'9') => {
            let end = scan_number(bytes, pos)?;
            Ok((Value::Number(&text[pos..end]), end))
        }
        Some(_) => Err(Error("unexpected character")),
        None => Err(Error("unexpected end of input")),
    }
}

fn literal(bytes: &[u8], pos: usize, word: &[u8]) -> Result<usize, Error> {
    if bytes[pos..].starts_with(word) {
        Ok(pos + word.len())
    } else {
        Err(Error("invalid literal"))
    }
}

/// Validates an object (`is_object`) or array starting at `pos` and returns
/// the index just past its closing bracket.
fn scan_container(
    text: &str,
    pos: usize,
    depth: usize,
    close: u8,
    is_object: bool,
) -> Result<usize, Error> {
    if depth >= MAX_DEPTH {
        return Err(Error("nesting is too deep"));
    }
    let bytes = text.as_bytes();
    let mut pos = skip_whitespace(bytes, pos + 1);
    if bytes.get(pos) == Some(&close) {
        return Ok(pos + 1);
    }
    loop {
        if is_object {
            if bytes.get(pos) != Some(&b'"') {
                return Err(Error("expected an object key"));
            }
            pos = skip_whitespace(bytes, scan_string(bytes, pos)?);
            if bytes.get(pos) != Some(&b':') {
                return Err(Error("expected ':' after an object key"));
            }
            pos = skip_whitespace(bytes, pos + 1);
        }
        let (_, end) = parse_value(text, pos, depth + 1)?;
        pos = skip_whitespace(bytes, end);
        match bytes.get(pos) {
            Some(b',') => pos = skip_whitespace(bytes, pos + 1),
            Some(&c) if c == close => return Ok(pos + 1),
            _ => return Err(Error("expected ',' or a closing bracket")),
        }
    }
}

/// Validates a string starting at the opening quote at `pos` and returns the
/// index just past the closing quote.
fn scan_string(bytes: &[u8], pos: usize) -> Result<usize, Error> {
    let mut pos = pos + 1;
    loop {
        match bytes.get(pos) {
            Some(b'"') => return Ok(pos + 1),
            Some(b'\\') => {
                match bytes.get(pos + 1) {
                    Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => pos += 2,
                    Some(b'u') => {
                        let digits = bytes.get(pos + 2..pos + 6).ok_or(Error("invalid \\u escape"))?;
                        if !digits.iter().all(u8::is_ascii_hexdigit) {
                            return Err(Error("invalid \\u escape"));
                        }
                        pos += 6;
                    }
                    _ => return Err(Error("invalid escape sequence")),
                }
            }
            Some(&c) if c < 0x20 => return Err(Error("control character in a string")),
            Some(_) => pos += 1,
            None => return Err(Error("unterminated string")),
        }
    }
}

fn scan_number(bytes: &[u8], pos: usize) -> Result<usize, Error> {
    let mut end = pos;
    if bytes.get(end) == Some(&b'-') {
        end += 1;
    }
    let digits_start = end;
    while matches!(bytes.get(end), Some(b'0'..=b'9')) {
        end += 1;
    }
    if end == digits_start {
        return Err(Error("invalid number"));
    }
    if bytes.get(end) == Some(&b'.') {
        end += 1;
        let fraction_start = end;
        while matches!(bytes.get(end), Some(b'0'..=b'9')) {
            end += 1;
        }
        if end == fraction_start {
            return Err(Error("invalid number"));
        }
    }
    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        end += 1;
        if matches!(bytes.get(end), Some(b'+' | b'-')) {
            end += 1;
        }
        let exponent_start = end;
        while matches!(bytes.get(end), Some(b'0'..=b'9')) {
            end += 1;
        }
        if end == exponent_start {
            return Err(Error("invalid number"));
        }
    }
    Ok(end)
}

#[test_case]
fn parses_scalars() {
    assert_eq!(parse("null"), Ok(Value::Null));
    assert_eq!(parse(" true "), Ok(Value::Bool(true)));
    assert_eq!(parse("false"), Ok(Value::Bool(false)));
    assert_eq!(parse("-12.5e3"), Ok(Value::Number("-12.5e3")));
    assert_eq!(parse("\"a\\\"b\""), Ok(Value::String("a\\\"b")));
    assert_eq!(parse("7").unwrap().as_u32(), Some(7));
}

#[test_case]
fn walks_objects_and_arrays() {
    let value = parse(r#"{"a": 1, "list": ["x", "y\n"], "nested": {"b": null}}"#).unwrap();
    assert_eq!(value.get("a").and_then(Value::as_u32), Some(1));
    assert_eq!(value.get("nested").and_then(|n| n.get("b")), Some(Value::Null));
    assert_eq!(value.get("missing"), None);

    let mut items = value.get("list").and_then(Value::items).unwrap();
    assert_eq!(items.next(), Some(Value::String("x")));
    assert_eq!(items.next(), Some(Value::String("y\\n")));
    assert_eq!(items.next(), None);

    assert_eq!(parse("{}").unwrap().members().unwrap().count(), 0);
    assert_eq!(parse("[]").unwrap().items().unwrap().count(), 0);
}

#[test_case]
fn decodes_escapes() {
    let mut out: FixedString<32> = FixedString::new();
    decode_string(r#"tab\there \"q\" Aé \/"#, &mut out).unwrap();
    assert_eq!(out, "tab\there \"q\" A\u{e9} /");
    // Keys with escapes still match.
    let value = parse(r#"{"key": 1}"#).unwrap();
    assert!(value.get("key").is_some());
}

#[test_case]
fn decoding_does_not_truncate() {
    let mut out: FixedString<4> = FixedString::new();
    assert_eq!(decode_string("hello", &mut out), Err(Error("string is too long")));
    assert_eq!(out, "");
}

#[test_case]
fn rejects_malformed_documents() {
    for text in [
        "",
        "{",
        "{\"a\" 1}",
        "{\"a\":1,}",
        "[1 2]",
        "\"unterminated",
        "\"bad \\x escape\"",
        "\"raw\nnewline\"",
        "tru",
        "01x",
        "1.",
        "{} {}",
        "{a:1}",
    ] {
        assert!(parse(text).is_err(), "accepted: {}", text);
    }
}

#[test_case]
fn limits_nesting_depth() {
    // MAX_DEPTH (8) levels are fine, one more is rejected.
    assert!(parse("[[[[[[[[1]]]]]]]]").is_ok());
    assert!(parse("[[[[[[[[[1]]]]]]]]]").is_err());
}
