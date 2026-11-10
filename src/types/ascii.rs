//! ASCII text that cannot hold anything else: the payload of `a` (ASCII text) and `d` (names) values.
//!
//! Until 2026-11-09 both variants held a bare `String`: anything could be put in, the encoder wrote its UTF-8 bytes as they were, and only the reader refused a byte past 127, refusing the whole section with it. A fgtw.org error detail with an em dash in it was written fine and read by no client, and a refusal of a ferros device's genesis was taken for its acceptance. With `Ascii` a non-ASCII value cannot be made: runtime text goes through [`Ascii::new`], which says no, and a literal through [`Ascii::lit`], which fails the build in a `const`.
//!
//! When `core::ascii::Char` is stable (rust-lang/rust#110998) the inside can become `[ascii::Char]` with the same API.

use crate::prelude::*;
use alloc::borrow::Cow;
use core::fmt;

/// ASCII text: every byte under 128, by construction. Static literals are kept borrowed; runtime text owned.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Ascii(Cow<'static, str>);

/// Text that is not ASCII, with the first character that is not and its byte offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotAscii {
    pub at: usize,
    pub found: char,
}

impl fmt::Display for NotAscii {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not ASCII: U+{:04X} at byte {}", self.found as u32, self.at)
    }
}

impl Ascii {
    /// The text, if every character is ASCII; otherwise the first that is not.
    pub fn new(s: impl Into<String>) -> Result<Ascii, NotAscii> {
        let s = s.into();
        match s.char_indices().find(|(_, c)| !c.is_ascii()) {
            Some((at, found)) => Err(NotAscii { at, found }),
            None => Ok(Ascii(Cow::Owned(s))),
        }
    }

    /// A literal, checked where it is evaluated: in a `const` (`const NAME: Ascii = Ascii::lit("...")`) a non-ASCII byte fails the build; elsewhere it panics at the call, which is a bug in the literal.
    pub const fn lit(s: &'static str) -> Ascii {
        let b = s.as_bytes();
        let mut i = 0;
        while i < b.len() {
            if b[i] >= 128 {
                panic!("vsf::Ascii::lit: the literal is not ASCII");
            }
            i += 1;
        }
        Ascii(Cow::Borrowed(s))
    }

    /// A section's or field's name as its `d`: names are the program's own identifiers, so one that is not ASCII is a bug in the program, and it stops here, named, rather than being written for every reader to refuse.
    pub fn name(s: &str) -> Ascii {
        match Ascii::new(s) {
            Ok(a) => a,
            Err(e) => panic!("vsf: the name {s:?} is {e}"),
        }
    }

    /// A value the program itself names (a graph's ops, a track's id, a profile's target) where the writer has no error to return: one that is not ASCII is a bug in the program, and it stops here with `what` named rather than being written for every reader to refuse. Text from outside (a person's words, a file's EXIF) goes through [`Ascii::new`] and its error instead.
    pub fn checked(what: &str, s: impl Into<String>) -> Ascii {
        let s = s.into();
        match Ascii::new(s.clone()) {
            Ok(a) => a,
            Err(e) => panic!("vsf: {what} {s:?} is {e}"),
        }
    }

    /// Bytes a reader has already checked (`decoding::metadata`): None if any is not ASCII after all.
    pub fn from_bytes(b: &[u8]) -> Option<Ascii> {
        if b.is_ascii() {
            // ASCII is valid UTF-8.
            Some(Ascii(Cow::Owned(String::from_utf8(b.to_vec()).ok()?)))
        } else {
            None
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn into_string(self) -> String {
        self.0.into_owned()
    }
}

impl core::ops::Deref for Ascii {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for Ascii {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<[u8]> for Ascii {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl fmt::Display for Ascii {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for Ascii {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&*self.0, f)
    }
}

impl TryFrom<String> for Ascii {
    type Error = NotAscii;
    fn try_from(s: String) -> Result<Ascii, NotAscii> {
        Ascii::new(s)
    }
}

impl TryFrom<&str> for Ascii {
    type Error = NotAscii;
    fn try_from(s: &str) -> Result<Ascii, NotAscii> {
        Ascii::new(s)
    }
}

impl From<Ascii> for String {
    fn from(a: Ascii) -> String {
        a.into_string()
    }
}

impl PartialEq<str> for Ascii {
    fn eq(&self, o: &str) -> bool {
        *self.0 == *o
    }
}

impl PartialEq<&str> for Ascii {
    fn eq(&self, o: &&str) -> bool {
        *self.0 == **o
    }
}

impl PartialEq<String> for Ascii {
    fn eq(&self, o: &String) -> bool {
        *self.0 == **o
    }
}

impl PartialEq<Ascii> for str {
    fn eq(&self, o: &Ascii) -> bool {
        *self == *o.0
    }
}

impl PartialEq<Ascii> for &str {
    fn eq(&self, o: &Ascii) -> bool {
        **self == *o.0
    }
}

impl PartialEq<Ascii> for String {
    fn eq(&self, o: &Ascii) -> bool {
        **self == *o.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAME: Ascii = Ascii::lit("device_owner");

    #[test]
    fn ascii_only() {
        assert_eq!(NAME.as_str(), "device_owner");
        assert!(Ascii::new("a - b").is_ok());
        let e = Ascii::new("a \u{2014} b").unwrap_err();
        assert_eq!((e.at, e.found), (2, '\u{2014}'));
        assert!(Ascii::from_bytes(b"ok").is_some());
        assert!(Ascii::from_bytes(&[b'a', 0xE2]).is_none());
    }

    #[test]
    #[should_panic]
    fn lit_refuses_at_runtime() {
        let s: &'static str = Box::leak(String::from("\u{2014}").into_boxed_str());
        let _ = Ascii::lit(s);
    }
}
