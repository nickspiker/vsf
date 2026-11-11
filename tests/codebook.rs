//! The reissued codebook (v11) against the one it replaced (BLAKE3 523769a934a407aa87d9ddef37ad186c546e205cb1280b0e9300d6e98c6dad0e): the pinned characters keep their codes byte for byte, the tree keeps every codeword, and the rest follow the documented order.
//!
//! The two digests below were taken from the v11 codebook (git 842a5eb, huffman_codes.bin) and are the proof that nothing pinned moved; they never change unless a codebook breaks that promise.

const CODEBOOK: &[u8] = include_bytes!("../huffman_codes.bin");

/// BLAKE3 over every pinned entry of the v11 codebook (a code of 20 bits or fewer, or an ASCII character), in codepoint order: the codepoint, then len << 24 | bits, little-endian u32 each.
const V11_PINNED: &str = "6de57c457f7ccb53f35cc9cd588af72473d21811275d618833297d79a3d014db";
/// BLAKE3 over every codeword of the v11 codebook, ascending: the code left-aligned to 24 bits, then its length, little-endian u32 each.
const V11_CODEWORDS: &str = "4d8f3863c6bd3644891dbb9f4014f12da74f428cefd91dddf6de807eed380ac3";

/// (codepoint, bits, length) per entry, in codepoint order.
fn entries() -> Vec<(u32, u32, u32)> {
    let n = u32::from_le_bytes(CODEBOOK[8..12].try_into().unwrap()) as usize;
    (0..n)
        .map(|i| {
            let o = 12 + i * 8;
            let cp = u32::from_le_bytes(CODEBOOK[o..o + 4].try_into().unwrap());
            let p = u32::from_le_bytes(CODEBOOK[o + 4..o + 8].try_into().unwrap());
            (cp, p & 0x00FF_FFFF, p >> 24)
        })
        .collect()
}

fn pinned(cp: u32, len: u32) -> bool {
    len <= 20 || cp <= 0x7F
}

#[test]
fn pinned_codes_unchanged_since_v11() {
    let e = entries();
    let mut h = blake3::Hasher::new();
    let mut short = 0;
    for &(cp, bits, len) in &e {
        if pinned(cp, len) {
            h.update(&cp.to_le_bytes());
            h.update(&(len << 24 | bits).to_le_bytes());
        }
        if len <= 20 {
            short += 1;
        }
    }
    assert_eq!(short, 712, "the codes of 20 bits or fewer");
    assert_eq!(h.finalize().to_hex().as_str(), V11_PINNED, "a pinned character's code moved");
}

#[test]
fn codewords_unchanged_since_v11() {
    let mut words: Vec<(u32, u32)> = entries().iter().map(|&(_, bits, len)| (bits << (24 - len), len)).collect();
    words.sort_unstable();
    let mut h = blake3::Hasher::new();
    for (key, len) in words {
        h.update(&key.to_le_bytes());
        h.update(&len.to_le_bytes());
    }
    assert_eq!(h.finalize().to_hex().as_str(), V11_CODEWORDS, "the tree's codewords changed");
}

/// Every codepoint not pinned, ascending, holds the free codewords in code order, 23-bit ones first.
#[test]
fn free_codepoints_in_code_order() {
    let free: Vec<(u32, u32, u32)> = entries().into_iter().filter(|&(cp, _, len)| !pinned(cp, len)).collect();
    assert_eq!(free.len(), 1_112_064 - 745);
    for w in free.windows(2) {
        let (a, b) = (w[0].1 << (24 - w[0].2), w[1].1 << (24 - w[1].2));
        assert!(w[0].0 < w[1].0 && a < b, "U+{:04X} then U+{:04X}", w[0].0, w[1].0);
        assert!(w[0].2 <= w[1].2, "23-bit codes first");
    }
}

/// An `x` of kept characters still says v9 (every reader since reads it alike); one holding a moved character says v11.
#[test]
fn x_since_follows_its_characters() {
    use vsf::VsfType;
    for s in ["", "Hello, world!", "tab\t^caret\r\n", "café Привет 中 नमस्ते 😀"] {
        assert_eq!(VsfType::x(s.to_string()).since().version, 9, "{s:?}");
    }
    for s in ["Ωmega", "it’s", "😎", "\u{10FFFF}"] {
        assert_eq!(VsfType::x(s.to_string()).since().version, 11, "{s:?}");
    }
}
