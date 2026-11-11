//! The x text codebook (vsf's `huffman_codes.bin`), generated from its pinned shape.
//!
//! The codebook is a complete prefix code over all 1,112,064 Unicode scalar values, codes of 3 to 24 bits.
//! Its tree came from a Huffman build over `frequencies.bin` (vsf build.rs before 0.4.0, std `BinaryHeap` in file order), and that build's tree is kept exactly: every codeword, every length (src/shape.rs, `SHAPE`).
//! What this generator fixes is which codepoint holds which codeword:
//! - **Pinned** (src/shape.rs, `PINNED`): every code of 20 bits or fewer (712 characters) and every ASCII character (U+0000..U+007F: `^`, tab, carriage return and the other controls have long codes), at the codeword the original build gave it, so English text and every ASCII string, handles included, keep their bytes.
//! - **Every other codepoint**, in ascending order, takes the remaining codewords in code order (left-aligned to 24 bits, ascending): every 23-bit codeword lies below every 24-bit one in that order, so the lowest codepoints get the 23-bit codes.
//!
//! That order is the one that makes a decoder smallest: the n-th free codepoint holds the n-th free codeword, so both directions are a count of pinned entries below a point plus arithmetic inside a run of equal-length free codewords. A full decoder is the pinned table and those runs (vsf src/x_text.rs, `full`), kilobytes where any other order needs the whole permutation.
//!
//! Until the reissue (vsf 0.11) the remaining codepoints held their codewords in the order a std `HashMap` had iterated them while `frequencies.bin` was written, a random permutation (about 2.6 MB of entropy).

pub mod shape;

/// Every Unicode scalar value's index: the codepoint below the surrogates, the codepoint less G#800 above.
pub fn scalar(i: u32) -> u32 {
    if i < 0xD800 {
        i
    } else {
        i + 0x800
    }
}

/// Scalar values in all.
pub const SCALARS: u32 = 0x110000 - 0x800;

/// One codebook entry: codepoint, code right-aligned, length in bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    pub cp: u32,
    pub bits: u32,
    pub len: u32,
}

impl Entry {
    /// The code left-aligned to 24 bits.
    pub fn key(&self) -> u32 {
        self.bits << (24 - self.len)
    }
}

/// The codebook, one entry per scalar value in codepoint order.
pub fn entries() -> Vec<Entry> {
    // Every codeword of the tree, in code order: SHAPE's runs tile the 24-bit keys from 0.
    let mut words: Vec<(u32, u32)> = Vec::with_capacity(SCALARS as usize);
    let mut next = 0u32;
    for &(start, len, count) in shape::SHAPE {
        assert_eq!(start, next, "SHAPE must tile the 24-bit keys in order");
        for k in 0..count {
            words.push((start + (k << (24 - len)), len));
        }
        next = start + (count << (24 - len));
    }
    assert_eq!(next, 1 << 24, "SHAPE must cover every 24-bit key (a complete prefix code)");
    assert_eq!(words.len(), SCALARS as usize, "one codeword per scalar value");

    // The pinned entries, checked against the tree: each must sit on a codeword of its own length, no two on one.
    let mut taken = vec![false; words.len()];
    let mut pinned_at = vec![u32::MAX; 0x110000];
    for (k, &(cp, bits, len)) in shape::PINNED.iter().enumerate() {
        if k > 0 {
            assert!(cp > shape::PINNED[k - 1].0, "PINNED must ascend by codepoint");
        }
        let key = bits << (24 - len);
        let w = words.partition_point(|&(wk, _)| wk < key);
        assert!(
            w < words.len() && words[w] == (key, len),
            "U+{cp:04X}: its pinned code must be a codeword of the tree"
        );
        assert!(!taken[w], "U+{cp:04X}: codeword already pinned");
        taken[w] = true;
        pinned_at[cp as usize] = w as u32;
    }

    // Every other scalar value, ascending, takes the next free codeword in code order.
    let mut free = (0..words.len()).filter(|&w| !taken[w]);
    let mut out = Vec::with_capacity(SCALARS as usize);
    for i in 0..SCALARS {
        let cp = scalar(i);
        let w = match pinned_at[cp as usize] {
            u32::MAX => free.next().expect("as many free codewords as free codepoints"),
            w => w as usize,
        };
        let (key, len) = words[w];
        out.push(Entry {
            cp,
            bits: key >> (24 - len),
            len,
        });
    }
    assert!(free.next().is_none(), "every codeword assigned");
    out
}

/// The codebook file: `HUFF`, version 2 (the fixed order), the entry count, then per entry the codepoint and len << 24 | bits, little-endian u32 each.
pub fn file() -> Vec<u8> {
    let entries = entries();
    let mut out = Vec::with_capacity(12 + entries.len() * 8);
    out.extend_from_slice(b"HUFF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for e in &entries {
        out.extend_from_slice(&e.cp.to_le_bytes());
        out.extend_from_slice(&(e.len << 24 | e.bits).to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generating twice gives the same bytes, and they are the vendored codebook (whose BLAKE3 vsf's build.rs pins).
    #[test]
    fn deterministic_and_vendored() {
        let a = file();
        let b = file();
        assert_eq!(a, b, "two runs differ");
        let vendored = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../huffman_codes.bin")).expect("vsf's huffman_codes.bin");
        assert!(a == vendored, "the generator's output is not the vendored huffman_codes.bin: regenerate it (cargo run --release -- ../../huffman_codes.bin) and update build.rs's pin");
    }

    /// The pinned set is exactly what lib.rs says: every code of 20 bits or fewer, and all of ASCII.
    #[test]
    fn pinned_set() {
        for &(cp, _, len) in shape::PINNED {
            assert!(
                len <= 20 || shape::PINNED_BLOCKS.iter().any(|&(a, b)| cp >= a && cp <= b),
                "U+{cp:04X}"
            );
        }
        for &(a, b) in shape::PINNED_BLOCKS {
            for cp in a..=b {
                assert!(
                    shape::PINNED.binary_search_by_key(&cp, |p| p.0).is_ok(),
                    "U+{cp:04X} in a pinned block"
                );
            }
        }
        let short = shape::PINNED.iter().filter(|p| p.2 <= 20).count();
        assert_eq!(short, 712);
        let mut short_words = 0;
        for &(_, len, count) in shape::SHAPE {
            if len <= 20 {
                short_words += count;
            }
        }
        assert_eq!(short_words, 712, "every short codeword is pinned");
    }

    /// The order: free codepoints ascending hold free codewords ascending.
    #[test]
    fn free_order() {
        let e = entries();
        let pinned = |cp: u32| shape::PINNED.binary_search_by_key(&cp, |p| p.0).is_ok();
        let free: Vec<&Entry> = e.iter().filter(|x| !pinned(x.cp)).collect();
        for w in free.windows(2) {
            assert!(w[0].cp < w[1].cp && w[0].key() < w[1].key());
        }
    }
}
