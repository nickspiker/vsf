//! `codebook <out>`: writes the x text codebook (vsf's huffman_codes.bin), the same bytes every run.
//! `codebook --extract <huffman_codes.bin> <shape.rs>`: writes src/shape.rs from a codebook (how shape.rs was made, once, from the vsf 0.11 codebook).

use std::fs;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.as_slice() {
        [_, out] => {
            fs::write(out, vsf_codebook::file()).expect("write the codebook");
        }
        [_, flag, from, out] if flag == "--extract" => {
            fs::write(out, extract(&fs::read(from).expect("read the codebook"))).expect("write shape.rs");
        }
        _ => {
            eprintln!("usage: codebook <huffman_codes.bin>\n       codebook --extract <huffman_codes.bin> <shape.rs>");
            std::process::exit(2);
        }
    }
}

/// The tree's shape (runs of equal length in code order) and the pinned entries of a codebook, as Rust source.
fn extract(book: &[u8]) -> String {
    let n = u32::from_le_bytes(book[8..12].try_into().unwrap()) as usize;
    let mut e: Vec<vsf_codebook::Entry> = (0..n)
        .map(|i| {
            let o = 12 + i * 8;
            let cp = u32::from_le_bytes(book[o..o + 4].try_into().unwrap());
            let p = u32::from_le_bytes(book[o + 4..o + 8].try_into().unwrap());
            vsf_codebook::Entry {
                cp,
                bits: p & 0x00FF_FFFF,
                len: p >> 24,
            }
        })
        .collect();
    let blocks = vsf_codebook::shape::PINNED_BLOCKS;
    let mut s = String::new();
    s += "//! The x text codebook's pinned shape, written by `codebook --extract` from vsf 0.11's huffman_codes.bin (BLAKE3 523769a934a407aa87d9ddef37ad186c546e205cb1280b0e9300d6e98c6dad0e).\n";
    s += "//! Do not edit by hand: a change here changes the wire format of every VsfType::x holding the characters it touches.\n\n";
    s += "/// The codepoint ranges pinned whatever their code's length (lib.rs says why).\n";
    s += "pub const PINNED_BLOCKS: &[(u32, u32)] = &[(0x0000, 0x007F)];\n\n";
    s += "/// Every codepoint pinned to the codeword the original Huffman build gave it, ascending: (codepoint, code right-aligned, length).\n";
    s += "#[rustfmt::skip]\npub const PINNED: &[(u32, u32, u32)] = &[\n";
    for x in &e {
        if x.len <= 20 || blocks.iter().any(|&(a, b)| x.cp >= a && x.cp <= b) {
            s += &format!("    (0x{:04X}, 0x{:06X}, {}),\n", x.cp, x.bits, x.len);
        }
    }
    s += "];\n\n";
    e.sort_by_key(|x| x.key());
    s += "/// The tree: runs of codewords of equal length in code order, (first codeword left-aligned to 24 bits, length, count). They tile every 24-bit key.\n";
    s += "#[rustfmt::skip]\npub const SHAPE: &[(u32, u32, u32)] = &[\n";
    let mut k = 0;
    while k < e.len() {
        let mut j = k;
        while j < e.len() && e[j].len == e[k].len {
            j += 1;
        }
        s += &format!("    (0x{:06X}, {}, {}),\n", e[k].key(), e[k].len, j - k);
        k = j;
    }
    s += "];\n";
    s
}
