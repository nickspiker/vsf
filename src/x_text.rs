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
//! | `x-full` | every Unicode scalar value | 3.25 MB | 3.38 MB |
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

#[cfg(any(feature = "x-compact", feature = "x-latin", feature = "x-lang16"))]
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
#[cfg(any(feature = "x-compact", feature = "x-latin", feature = "x-lang16"))]
#[inline(always)]
fn key_of(bits: u32, len: u32) -> u32 {
    bits << (24 - len)
}

/// A subset of the codebook (build.rs `x_subset`): `run_cp` and `run_slot` are the first codepoint (u32) and first slot (u16) of each run of consecutive codepoints, closed by u32::MAX and the slot count; `codes` is len << 24 | bits per slot in codepoint order; `by_key` is the slots in code order.
#[cfg(any(feature = "x-compact", feature = "x-latin", feature = "x-lang16"))]
struct Subset {
    run_cp: &'static [u8],
    run_slot: &'static [u8],
    codes: &'static [u8],
    by_key: &'static [u8],
}

#[cfg(any(feature = "x-compact", feature = "x-latin", feature = "x-lang16"))]
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

#[cfg(any(feature = "x-compact", feature = "x-latin", feature = "x-lang16"))]
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
/// How the codes were assigned (tools history: build.rs before 0.4.0, commit 8b5b07e^): a textbook Huffman tree built with std's `BinaryHeap` over `frequencies.bin` in file order, the first node popped on the 0 side.
/// Replaying that over the pinned `frequencies.bin` reproduces all 1,112,064 codes.
/// But 1,111,352 of those frequencies are the same floor value, written in the order a std `HashMap` (random SipHash keys) iterated them, so which rare codepoint got which long code is a random permutation: no rule shorter than the permutation itself exists.
/// Its entropy is log2(1,111,352!) bits, about 2.59 MB; this variant stores it in 2.92 MB.
///
/// The tables:
/// - `SHAPE`: the code tree as 335 runs of equal length in code order (first key, first rank, length; a closing run). A complete prefix code tiles the 24-bit keys, so rank and key convert by arithmetic within a run.
/// - `PERM`: the codepoint index (the codepoint, less G#800 above the surrogates) of each rank, 21 bits each.
/// - The inverse, for the encoder, without a second permutation: along each cycle of `PERM` longer than the stride (16), every 16th element is marked (`MARKS`, one bit an index; `BLOCKS`, the marks before each 512 bits) and holds the mark before it on its cycle (`BACK`, 21 bits each). The encoder walks the cycle from the codepoint index to the next mark, jumps back to the mark before, and walks to the element `PERM` sends to the index: at most 32 steps.
/// - `LOW`: the first 256 codepoints' codes directly.
#[cfg(feature = "x-full")]
pub mod full {
    use super::*;
    use unicode_normalization::UnicodeNormalization;

    static SHAPE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/x_full_shape.bin"));
    static PERM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/x_full_perm.bin"));
    static MARKS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/x_full_marks.bin"));
    static BLOCKS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/x_full_blocks.bin"));
    static BACK: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/x_full_back.bin"));
    static LOW: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/x_full_low.bin"));

    /// Distance along a cycle between marks (build.rs `X_FULL_STRIDE`).
    const STRIDE: usize = 1 << 4;

    /// Element `i` of a 21-bit packed table; build.rs pads each with four bytes so the load never runs off the end.
    #[inline(always)]
    fn packed21(t: &[u8], i: usize) -> usize {
        let bit = i * 21;
        let b = bit >> 3;
        let w = u32::from_le_bytes([t[b], t[b + 1], t[b + 2], t[b + 3]]);
        ((w >> (bit & 7)) & 0x1F_FFFF) as usize
    }

    /// Shape runs, the closing one excluded.
    fn runs() -> usize {
        SHAPE.len() / 12 - 1
    }

    /// Run `r`: first key, first rank, length.
    #[inline(always)]
    fn run(r: usize) -> (u32, u32, u32) {
        (u32_at(SHAPE, r * 3), u32_at(SHAPE, r * 3 + 1), u32_at(SHAPE, r * 3 + 2))
    }

    fn marked(x: usize) -> bool {
        MARKS[x >> 3] >> (x & 7) & 1 == 1
    }

    /// Marks before index `x`.
    fn marks_before(x: usize) -> usize {
        let word = x >> 6;
        let mut n = u32_at(BLOCKS, word >> 3) as usize;
        for w in (word & !7)..word {
            n += u64::from_le_bytes(MARKS[w * 8..w * 8 + 8].try_into().unwrap()).count_ones() as usize;
        }
        let last = u64::from_le_bytes(MARKS[word * 8..word * 8 + 8].try_into().unwrap());
        n + (last & ((1u64 << (x & 63)) - 1)).count_ones() as usize
    }

    /// The rank whose codepoint index is `j`: the walk described above.
    fn rank_of(j: usize) -> usize {
        let mut x = j;
        let mut jumped = false;
        let mut steps = 0usize;
        loop {
            let y = packed21(PERM, x);
            if y == j {
                return x;
            }
            if !jumped && marked(x) {
                x = packed21(BACK, marks_before(x));
                jumped = true;
            } else {
                x = y;
            }
            steps += 1;
            debug_assert!(steps <= 2 * STRIDE + 1, "the x-full inverse tables are inconsistent");
        }
    }

    fn code(c: char) -> (u32, u32) {
        let cp = c as u32;
        if cp < 256 {
            let p = u32_at(LOW, cp as usize);
            return (p & 0x00FF_FFFF, p >> 24);
        }
        let j = if cp < 0xD800 { cp } else { cp - 0x800 } as usize;
        let r = rank_of(j) as u32;
        let k = prefix(runs(), |i| run(i).1 <= r);
        let (key, first, len) = run(k - 1);
        ((key + ((r - first) << (24 - len))) >> (24 - len), len)
    }

    struct Full;

    impl Book for Full {
        fn code(&self, c: char) -> Option<(u32, u32)> {
            Some(code(c))
        }

        fn at(&self, w: u32) -> Option<Found> {
            // The runs tile every 24-bit key from 0, so the window lies in exactly one.
            let k = prefix(runs(), |i| run(i).0 <= w);
            let (key, first, len) = run(k - 1);
            let j = packed21(PERM, (first + ((w - key) >> (24 - len))) as usize) as u32;
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
        SHAPE.len() + PERM.len() + MARKS.len() + BLOCKS.len() + BACK.len() + LOW.len()
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
