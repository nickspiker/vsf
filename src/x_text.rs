//! The `x` text codec without std: the same wire bytes as [`crate::text_encoding`], from tables a few kilobytes to a few megabytes in size, chosen by cargo feature.
//!
//! The codebook (`huffman_codes.bin`, pinned by BLAKE3 in build.rs) is the wire format for `x`.
//! build.rs generates every table here from it after checking that hash, so no table can come from any other codebook.
//!
//! | feature | covers | tables | linked into an aarch64-unknown-none payload |
//! |---|---|---|---|
//! | `x-compact` | the 712 codes of 20 bits or fewer: printable ASCII but `^`, newline, 49 of Latin-1 Supplement's 96, some of Latin Extended-A, the Russian and Ukrainian alphabets, and the commonest Arabic, Devanagari, Thai, kana, CJK, Hangul and eight emoji and symbols | 6.5 KB | 10 KB |
//! | `x-latin` | compact, plus all of Basic Latin, Latin-1 Supplement, Latin Extended-A and -B, IPA, Spacing Modifiers, Combining Diacritical Marks, Latin Extended Additional, General Punctuation and Currency Symbols (1,835 characters) | 13.0 KB | 138 KB, 120 KB of it NFC |
//! | `x-lang16` | latin, plus Cyrillic, Cyrillic Supplement and Devanagari (2,164 characters): every script of ferros's sixteen languages | 14.8 KB | 140 KB, 120 KB of it NFC |
//! | `x-full` | every Unicode scalar value (the codebook's fixed order since v11 makes it small) | 7.3 KB | 133 KB, 120 KB of it NFC |
//!
//! Several may be enabled at once; each has its own module, and [`encode`] and [`decode_text_with_size`] use the widest enabled.
//! With the std `text` or `text-encode` feature on, `VsfType::x` still goes thru [`crate::text_encoding`]; these modules are then only reachable by name (the tests compare the two).
//!
//! ## What every variant guarantees
//!
//! For a string the variant supports, [`encode`] returns exactly the bytes and the NFC character count `text_encoding::encode_text` returns.
//! For any other string it returns [`Unsupported`] naming a character the variant lacks, and writes nothing.
//! [`decode_text_with_size`] returns exactly what `text_encoding::decode_text_with_size` returns for every stream whose characters the variant holds, and an error for any stream holding one it lacks.
//! A decoder holding only some codes cannot step over one it lacks (the code's length is unknown), so an unknown code refuses the whole string.
//!
//! ## NFC
//!
//! `x-latin`, `x-lang16` and `x-full` normalize with `unicode-normalization` (no_std, the crate the std path uses), then refuse a character outside the variant.
//! `x-compact` needs no tables for it: every one of its 712 characters is NFC_Quick_Check=Yes, so no two of them compose, and NFC of a string made of them only reorders runs of combining marks by canonical combining class.
//! Three of them are marks (U+094D virama, class 9; U+0E48 and U+0E49 Thai tone marks, class 107), and every combining mark in a compact character's own decomposition has a class above 107, so a moved mark never blocks a recomposition.
//! So the compact encoder sorts each run of those three marks by class, stably, and refuses anything else outside the set (input that NFC would fold into the set, such as `e` + U+0301, included).
//! tests/x_variants.rs checks those facts against unicode-normalization and every pair of compact characters against the std encoder.

use crate::prelude::*;

/// The character an encoder refused: the variant built in has no code for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unsupported(pub char);

impl core::fmt::Display for Unsupported {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "U+{:04X} is outside this build's x text table", self.0 as u32)
    }
}

/// The code a decoder found at a position: the codepoint and the code's length in bits.
type Found = (u32, u32);

/// A codebook as one variant holds it.
trait Book {
    /// The code for `c` (bits right-aligned, length), or None if this variant lacks it.
    fn code(&self, c: char) -> Option<(u32, u32)>;
    /// The character whose code begins the 24-bit window `w`, or None if this variant lacks it.
    fn at(&self, w: u32) -> Option<Found>;
}

#[inline(always)]
fn u32_at(b: &[u8], i: usize) -> u32 {
    let o = i * 4;
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

#[cfg(any(feature = "x-compact", feature = "x-latin", feature = "x-lang16", feature = "x-full"))]
#[inline(always)]
fn u16_at(b: &[u8], i: usize) -> u32 {
    let o = i * 2;
    u16::from_le_bytes([b[o], b[o + 1]]) as u32
}

/// The count of leading indices in `0..n` for which `le` holds, `le` true on a prefix: a binary search.
#[inline(always)]
fn prefix(n: usize, le: impl Fn(usize) -> bool) -> usize {
    let (mut lo, mut hi) = (0usize, n);
    while lo < hi {
        let mid = (lo + hi) >> 1;
        if le(mid) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

/// A code left-aligned to 24 bits.
#[cfg(any(feature = "x-compact", feature = "x-latin", feature = "x-lang16", feature = "x-full"))]
#[inline(always)]
fn key_of(bits: u32, len: u32) -> u32 {
    bits << (24 - len)
}

/// A subset of the codebook (build.rs `x_subset`): `run_cp` and `run_slot` are the first codepoint (u32) and first slot (u16) of each run of consecutive codepoints, closed by u32::MAX and the slot count; `codes` is len << 24 | bits per slot in codepoint order; `by_key` is the slots in code order.
#[cfg(any(feature = "x-compact", feature = "x-latin", feature = "x-lang16", feature = "x-full"))]
struct Subset {
    run_cp: &'static [u8],
    run_slot: &'static [u8],
    codes: &'static [u8],
    by_key: &'static [u8],
}

#[cfg(any(feature = "x-compact", feature = "x-latin", feature = "x-lang16", feature = "x-full"))]
impl Subset {
    /// Runs, the closing one excluded.
    fn run_count(&self) -> usize {
        self.run_cp.len() / 4 - 1
    }

    fn run(&self, r: usize) -> (u32, u32) {
        (u32_at(self.run_cp, r), u16_at(self.run_slot, r))
    }

    /// Characters held.
    fn len(&self) -> usize {
        self.codes.len() / 4
    }

    /// Bytes of table.
    fn size(&self) -> usize {
        self.run_cp.len() + self.run_slot.len() + self.codes.len() + self.by_key.len()
    }

    fn slot_code(&self, s: usize) -> (u32, u32) {
        let p = u32_at(self.codes, s);
        (p & 0x00FF_FFFF, p >> 24)
    }
}

#[cfg(any(feature = "x-compact", feature = "x-latin", feature = "x-lang16", feature = "x-full"))]
impl Book for Subset {
    fn code(&self, c: char) -> Option<(u32, u32)> {
        let cp = c as u32;
        // The run that would hold cp: the last starting at or below it.
        let r = prefix(self.run_count(), |i| self.run(i).0 <= cp);
        if r == 0 {
            return None;
        }
        let (first_cp, first_slot) = self.run(r - 1);
        // The closing run makes run r always readable, the last real run included.
        let end_slot = self.run(r).1;
        let s = first_slot + (cp - first_cp);
        // The membership test itself, not a guard: a codepoint past its run's end is one this variant lacks.
        if s >= end_slot {
            return None;
        }
        Some(self.slot_code(s as usize))
    }

    fn at(&self, w: u32) -> Option<Found> {
        // The last code at or below the window. If the code the window starts with is held, it is that one: the codes are prefix-free, so no other held code lies between it and the window. If it is not held, the one found does not match the window's first bits.
        let k = prefix(self.len(), |i| {
            let (bits, len) = self.slot_code(u16_at(self.by_key, i) as usize);
            key_of(bits, len) <= w
        });
        if k == 0 {
            return None;
        }
        let s = u16_at(self.by_key, k - 1);
        let (bits, len) = self.slot_code(s as usize);
        if w >> (24 - len) != bits {
            return None;
        }
        let r = prefix(self.run_count(), |i| self.run(i).1 <= s);
        let (first_cp, first_slot) = self.run(r - 1);
        Some((first_cp + (s - first_slot), len))
    }
}

/// Writes codes most significant bit first, the last byte padded with zeros (as `text_encoding::BitVec`).
struct Bits {
    out: Vec<u8>,
    acc: u64,
    held: u32,
}

impl Bits {
    fn new() -> Self {
        Bits {
            out: Vec::new(),
            acc: 0,
            held: 0,
        }
    }

    /// Appends a code of `len` bits (at most 24). `held` stays below 8 between calls, so the accumulator's low 32 bits hold every bit not yet written; bits shifted out of the top were written already.
    #[inline(always)]
    fn push(&mut self, bits: u32, len: u32) {
        self.acc = self.acc << len | bits as u64;
        self.held += len;
        while self.held >= 8 {
            self.held -= 8;
            self.out.push((self.acc >> self.held) as u8);
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.held > 0 {
            self.out.push((self.acc << (8 - self.held)) as u8);
        }
        self.out
    }
}

/// The 24 bits from `bit` on, zero past the stream's end.
#[inline(always)]
fn window(bytes: &[u8], bit: usize) -> u32 {
    let b = bit >> 3;
    let mut w = 0u32;
    for k in 0..4 {
        // A bounds check, on wire input: the last code of a stream lies within its last three bytes, so its window reaches up to three bytes past the end, and indexing there would panic on every well-formed stream. Reading zero there is the std decoder's zero extension; the caller then refuses a code that needed bits past the end.
        w = w << 8 | bytes.get(b + k).copied().unwrap_or(0) as u32;
    }
    (w << (bit & 7)) >> 8
}

/// Encodes characters already in NFC.
#[cfg(any(feature = "x-latin", feature = "x-lang16", feature = "x-full"))]
fn encode_nfc(book: &impl Book, chars: impl Iterator<Item = char>) -> Result<(Vec<u8>, usize), Unsupported> {
    let mut out = Bits::new();
    let mut count = 0usize;
    for c in chars {
        let (bits, len) = book.code(c).ok_or(Unsupported(c))?;
        out.push(bits, len);
        count += 1;
    }
    Ok((out.finish(), count))
}

/// Decodes `char_count` characters, with the std decoder's checks in its order.
fn decode_with(book: &impl Book, bytes: &[u8], char_count: usize) -> Result<(String, usize), &'static str> {
    if char_count == 0 {
        return Ok((String::new(), 0));
    }
    if bytes.is_empty() {
        return Err("No data");
    }
    let max_bits = bytes.len() << 3;
    // Every character costs at least one bit, so a larger count cannot decode; refused before anything is reserved for it (the std decoder's check, security audit VSF-02).
    if char_count > max_bits {
        return Err("character count exceeds the bits present");
    }
    let mut text = String::with_capacity(char_count);
    let mut bit = 0usize;
    for _ in 0..char_count {
        if bit >= max_bits {
            return Err("Unexpected end of data");
        }
        let (cp, len) = book
            .at(window(bytes, bit))
            .ok_or("Huffman code outside this build's x text table")?;
        // A code running past the stream's end: the stream was cut short. bit < max_bits here, so the subtraction cannot wrap.
        if len as usize > max_bits - bit {
            return Err("Premature end of Huffman bitstream");
        }
        text.push(char::from_u32(cp).ok_or("Invalid Huffman code")?);
        bit += len as usize;
    }
    Ok((text, (bit + 7) >> 3))
}

/// `x-compact`: the 712 codes of 20 bits or fewer.
#[cfg(feature = "x-compact")]
pub mod compact {
    use super::*;

    static BOOK: Subset = Subset {
        run_cp: include_bytes!(concat!(env!("OUT_DIR"), "/x_compact_run_cp.bin")),
        run_slot: include_bytes!(concat!(env!("OUT_DIR"), "/x_compact_run_slot.bin")),
        codes: include_bytes!(concat!(env!("OUT_DIR"), "/x_compact_codes.bin")),
        by_key: include_bytes!(concat!(env!("OUT_DIR"), "/x_compact_by_key.bin")),
    };

    /// The compact set's characters of nonzero canonical combining class, with their class (see the module docs; tests/x_variants.rs checks this is all of them).
    pub const MARKS: [(char, u8); 3] = [('\u{094D}', 9), ('\u{0E48}', 107), ('\u{0E49}', 107)];

    fn class(c: char) -> u8 {
        let mut k = 0;
        while k < MARKS.len() {
            if MARKS[k].0 == c {
                return MARKS[k].1;
            }
            k += 1;
        }
        0
    }

    /// Encodes `text` as `text_encoding::encode_text` does, or refuses a character outside the compact set (see the module docs for why reordering the three marks is all NFC does here).
    pub fn encode(text: &str) -> Result<(Vec<u8>, usize), Unsupported> {
        let mut out = Bits::new();
        let mut count = 0usize;
        // The current run of marks, stably sorted by class as it grows: (class, bits, len).
        let mut run: Vec<(u8, u32, u32)> = Vec::new();
        for c in text.chars() {
            let (bits, len) = BOOK.code(c).ok_or(Unsupported(c))?;
            let cc = class(c);
            if cc == 0 {
                for &(_, b, l) in &run {
                    out.push(b, l);
                }
                run.clear();
                out.push(bits, len);
            } else {
                let at = run.iter().rposition(|m| m.0 <= cc).map_or(0, |p| p + 1);
                run.insert(at, (cc, bits, len));
            }
            count += 1;
        }
        for &(_, b, l) in &run {
            out.push(b, l);
        }
        Ok((out.finish(), count))
    }

    /// Decodes as `text_encoding::decode_text_with_size` does, or refuses a stream holding a code outside the compact set.
    pub fn decode_text_with_size(bytes: &[u8], char_count: usize) -> Result<(String, usize), &'static str> {
        decode_with(&BOOK, bytes, char_count)
    }

    /// Whether the compact set holds `c`.
    pub fn holds(c: char) -> bool {
        BOOK.code(c).is_some()
    }

    /// Characters held.
    pub fn len() -> usize {
        BOOK.len()
    }

    /// Bytes of table.
    pub fn size() -> usize {
        BOOK.size()
    }
}

/// Defines a subset variant that normalizes with unicode-normalization.
macro_rules! nfc_subset {
    ($feature:literal, $name:ident, $doc:literal) => {
        #[doc = $doc]
        #[cfg(feature = $feature)]
        pub mod $name {
            use super::*;
            use unicode_normalization::UnicodeNormalization;

            static BOOK: Subset = Subset {
                run_cp: include_bytes!(concat!(env!("OUT_DIR"), "/x_", stringify!($name), "_run_cp.bin")),
                run_slot: include_bytes!(concat!(env!("OUT_DIR"), "/x_", stringify!($name), "_run_slot.bin")),
                codes: include_bytes!(concat!(env!("OUT_DIR"), "/x_", stringify!($name), "_codes.bin")),
                by_key: include_bytes!(concat!(env!("OUT_DIR"), "/x_", stringify!($name), "_by_key.bin")),
            };

            /// Encodes `text` as `text_encoding::encode_text` does (NFC first), or refuses a character of its NFC form outside this set.
            pub fn encode(text: &str) -> Result<(Vec<u8>, usize), Unsupported> {
                encode_nfc(&BOOK, text.nfc())
            }

            /// Decodes as `text_encoding::decode_text_with_size` does, or refuses a stream holding a code outside this set.
            pub fn decode_text_with_size(bytes: &[u8], char_count: usize) -> Result<(String, usize), &'static str> {
                decode_with(&BOOK, bytes, char_count)
            }

            /// Whether this set holds `c`.
            pub fn holds(c: char) -> bool {
                BOOK.code(c).is_some()
            }

            /// Characters held.
            pub fn len() -> usize {
                BOOK.len()
            }

            /// Bytes of table.
            pub fn size() -> usize {
                BOOK.size()
            }
        }
    };
}

nfc_subset!(
    "x-latin",
    latin,
    "`x-latin`: the compact set, plus the Latin, IPA, combining mark, punctuation and currency blocks (build.rs `X_LATIN_BLOCKS`)."
);
nfc_subset!("x-lang16", lang16, "`x-lang16`: the latin set, plus Cyrillic, Cyrillic Supplement and Devanagari (build.rs `X_LANG16_BLOCKS`): every script of ferros's sixteen languages.");

/// `x-full`: every Unicode scalar value.
///
/// The codebook's order (tools/codebook/src/lib.rs, checked by build.rs) makes this small: every code of 20 bits or fewer and every ASCII character sit at a pinned codeword (745 of them), and every other codepoint, ascending, holds the remaining codewords in code order.
/// So the n-th free codepoint holds the n-th free codeword, and both directions are a count of pinned entries below a point plus arithmetic inside a run of equal-length free codewords.
///
/// The tables:
/// - The pinned entries, as a [`Subset`] (build.rs `x_subset`, name `full`).
/// - `FREE`: the free codewords in code order as runs of equal length, each (first codeword left-aligned to 24 bits, its free index, length), closed by (1 << 24, the free count, 0).
#[cfg(feature = "x-full")]
pub mod full {
    use super::*;
    use unicode_normalization::UnicodeNormalization;

    static PINNED: Subset = Subset {
        run_cp: include_bytes!(concat!(env!("OUT_DIR"), "/x_full_run_cp.bin")),
        run_slot: include_bytes!(concat!(env!("OUT_DIR"), "/x_full_run_slot.bin")),
        codes: include_bytes!(concat!(env!("OUT_DIR"), "/x_full_codes.bin")),
        by_key: include_bytes!(concat!(env!("OUT_DIR"), "/x_full_by_key.bin")),
    };
    static FREE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/x_full_free.bin"));

    /// A codepoint's index among the scalar values (the codepoint, less G#800 above the surrogates).
    #[inline(always)]
    fn index(cp: u32) -> u32 {
        if cp < 0xD800 {
            cp
        } else {
            cp - 0x800
        }
    }

    /// Free runs, the closing one excluded.
    fn runs() -> usize {
        FREE.len() / 12 - 1
    }

    /// Free run `r`: first key, first free index, length.
    #[inline(always)]
    fn run(r: usize) -> (u32, u32, u32) {
        (u32_at(FREE, r * 3), u32_at(FREE, r * 3 + 1), u32_at(FREE, r * 3 + 2))
    }

    fn code(c: char) -> (u32, u32) {
        if let Some(code) = PINNED.code(c) {
            return code;
        }
        // The free index: the scalar index less the pinned codepoints below it. c is not pinned, so the pinned run that starts at or below it ends below it, and every slot before the next run's first is below c.
        let cp = c as u32;
        let r = prefix(PINNED.run_count(), |i| PINNED.run(i).0 <= cp);
        let below = if r == 0 { 0 } else { PINNED.run(r).1 };
        let i = index(cp) - below;
        let k = prefix(runs(), |j| run(j).1 <= i);
        let (key, first, len) = run(k - 1);
        ((key + ((i - first) << (24 - len))) >> (24 - len), len)
    }

    struct Full;

    impl Book for Full {
        fn code(&self, c: char) -> Option<(u32, u32)> {
            Some(code(c))
        }

        fn at(&self, w: u32) -> Option<Found> {
            if let Some(found) = PINNED.at(w) {
                return Some(found);
            }
            // The window starts with a free codeword (the code is complete and no pinned one matched), which lies in the last free run starting at or below it.
            let k = prefix(runs(), |j| run(j).0 <= w);
            let (key, first, len) = run(k - 1);
            let i = first + ((w - key) >> (24 - len));
            // The i-th free codepoint: past the last pinned run with at most i free codepoints before it, so its index is i plus every pinned slot up to that run's end.
            let r = prefix(PINNED.run_count(), |p| {
                let (cp, slot) = PINNED.run(p);
                index(cp) - slot <= i
            });
            let j = if r == 0 { i } else { i + PINNED.run(r).1 };
            Some((if j < 0xD800 { j } else { j + 0x800 }, len))
        }
    }

    /// Encodes `text` exactly as `text_encoding::encode_text` does; it never refuses.
    pub fn encode(text: &str) -> Result<(Vec<u8>, usize), Unsupported> {
        encode_nfc(&Full, text.nfc())
    }

    /// Decodes exactly as `text_encoding::decode_text_with_size` does.
    pub fn decode_text_with_size(bytes: &[u8], char_count: usize) -> Result<(String, usize), &'static str> {
        decode_with(&Full, bytes, char_count)
    }

    /// Every scalar value.
    pub fn holds(_c: char) -> bool {
        true
    }

    /// Bytes of table.
    pub fn size() -> usize {
        PINNED.size() + FREE.len()
    }
}

#[cfg(all(feature = "x-compact", not(any(feature = "x-full", feature = "x-lang16", feature = "x-latin"))))]
use compact as widest;
#[cfg(feature = "x-full")]
use full as widest;
#[cfg(all(feature = "x-lang16", not(feature = "x-full")))]
use lang16 as widest;
#[cfg(all(feature = "x-latin", not(any(feature = "x-full", feature = "x-lang16"))))]
use latin as widest;

/// Encodes `text` with the widest variant enabled: the bytes and NFC character count `text_encoding::encode_text` gives, or the character it lacks.
pub fn encode(text: &str) -> Result<(Vec<u8>, usize), Unsupported> {
    widest::encode(text)
}

/// Decodes with the widest variant enabled: what `text_encoding::decode_text_with_size` gives, or an error for a code it lacks.
pub fn decode_text_with_size(bytes: &[u8], char_count: usize) -> Result<(String, usize), &'static str> {
    widest::decode_text_with_size(bytes, char_count)
}
