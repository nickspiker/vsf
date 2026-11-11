//! Build script for VSF.
//!
//! Verifies that the vendored Huffman corpus and codebook (`frequencies.bin`, `huffman_codes.bin`) are byte-identical to their pinned BLAKE3 hashes. The codebook IS the wire format for `VsfType::x`, so drift here silently bifurcates the identity namespace thru every downstream consumer (ihi handle hashing, photon avatar keys, contact tables, anywhere else that round-trips text thru VSF).
//!
//! Both files are vendored in the repo and tracked in version control. Regeneration is an explicit offline operation performed via `tools/frequency-analyzer/` — the build script never regenerates, never downloads, never silently mutates either file. A hash mismatch is a hard build failure.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

// Pinned BLAKE3 hashes. To change the codebook:
//   1. Regenerate frequencies.bin and huffman_codes.bin in tools/frequency-analyzer/
//   2. Update both constants below
//   3. Bump vsf MAJOR version (any codebook change is a wire-format break)
//   4. Update ihi to depend on the new vsf version and re-attest existing identities
const FREQUENCIES_BLAKE3: &str = "0652a53f8ac8b1989a3c0a6f7290af1d41d4b58cd960b77f879b38223ccd7b0d";
const HUFFMAN_CODES_BLAKE3: &str =
    "523769a934a407aa87d9ddef37ad186c546e205cb1280b0e9300d6e98c6dad0e";

/// Reads `path` and returns its bytes if they hash to `expected_hex`.
fn verify_blake3(path: &str, expected_hex: &str) -> Result<Vec<u8>, String> {
    let bytes = fs::read(path).map_err(|e| format!("cannot read {}: {}", path, e))?;
    let actual = blake3::hash(&bytes).to_hex();
    if actual.as_str() != expected_hex {
        return Err(format!(
            "BLAKE3 mismatch for {}\n  expected: {}\n  actual:   {}\n\n\
             The Huffman codebook has been tampered with or regenerated without updating \
             the pinned hash. Any change here changes every VsfType::x byte and every \
             downstream identity hash. If the change is intentional, update the constant \
             in build.rs and bump vsf MAJOR version per the determinism contract.",
            path, expected_hex, actual
        ));
    }
    Ok(bytes)
}

fn main() {
    println!("cargo::rustc-check-cfg=cfg(huffman_available)");
    println!("cargo:rerun-if-changed=frequencies.bin");
    println!("cargo:rerun-if-changed=huffman_codes.bin");
    println!("cargo:rerun-if-changed=build.rs");

    // Docs.rs and similar restricted environments still need to compile even without the vendored codebook. If huffman_codes.bin is genuinely absent, leave the cfg flag unset — text_encoding.rs sees the missing cfg and returns an empty HashMap. Encoding then panics with a helpful message rather than silently producing wrong bytes.
    if !Path::new("huffman_codes.bin").exists() {
        // The no_std x variants are built from the codebook and cannot be built without it.
        if env::vars_os().any(|(k, _)| k.to_str().is_some_and(|k| k.starts_with("CARGO_FEATURE_X_"))) {
            panic!("huffman_codes.bin missing: the x-compact, x-latin, x-lang16 and x-full features build their tables from it");
        }
        eprintln!(
            "cargo:warning=huffman_codes.bin missing — text encoding will be unavailable in this build"
        );
        return;
    }

    // frequencies.bin is the input corpus; it's vendored in the git repo but excluded from the published .crate to stay under the crates.io 10MiB limit. Verify it when present (developer builds from a clone), skip when absent (downstream users from crates.io).
    // huffman_codes.bin — the runtime-load artifact — is ALWAYS verified.
    if Path::new("frequencies.bin").exists() {
        if let Err(msg) = verify_blake3("frequencies.bin", FREQUENCIES_BLAKE3) {
            panic!("{}", msg);
        }
    }
    let codebook = verify_blake3("huffman_codes.bin", HUFFMAN_CODES_BLAKE3).unwrap_or_else(|msg| panic!("{}", msg));

    // The no_std x variants (src/x_text.rs): their tables come from the codebook just verified, so they carry its pinned hash by construction.
    x_tables(&codebook);

    println!("cargo:rustc-cfg=huffman_available");
}

// ==================== x text tables for the no_std codec (src/x_text.rs) ====================

/// The four x text variants, each a cargo feature; each one wider is a superset of the one before.
/// The Unicode blocks a variant adds to the codes of 20 bits or fewer (the compact set), as inclusive codepoint ranges.
const X_LATIN_BLOCKS: &[(u32, u32)] = &[
    (0x0000, 0x007F), // Basic Latin, the controls and `^` included (the compact set lacks both)
    (0x0080, 0x024F), // Latin-1 Supplement, Latin Extended-A and -B (Polish, Turkish, Māori macrons, Vietnamese ơ ư)
    (0x0250, 0x02FF), // IPA Extensions and Spacing Modifier Letters (ʼ, the Ukrainian apostrophe)
    (0x0300, 0x036F), // Combining Diacritical Marks (decomposed input that NFC cannot fold)
    (0x1E00, 0x1EFF), // Latin Extended Additional (Vietnamese ạ ả ấ …, German ẞ)
    (0x2000, 0x206F), // General Punctuation (quotes, dashes, ellipsis, ZWJ and ZWNJ)
    (0x20A0, 0x20CF), // Currency Symbols (€ ₹ ₽ ₴ ₫)
];
const X_LANG16_BLOCKS: &[(u32, u32)] = &[
    (0x0400, 0x052F), // Cyrillic and Cyrillic Supplement (Russian, Ukrainian)
    (0x0900, 0x097F), // Devanagari (Hindi)
];

/// Distance along a permutation cycle between two marked elements of the full variant's inverse (a power of two, the encoder's walk is at most twice it).
const X_FULL_STRIDE: usize = 1 << 4;

/// One codebook entry: its codepoint, its code right-aligned, its length.
struct XEntry {
    cp: u32,
    bits: u32,
    len: u32,
}

/// An entry's code left-aligned to 24 bits.
fn x_key(e: &XEntry) -> u32 {
    e.bits << (24 - e.len)
}

fn x_write(out: &Path, name: &str, bytes: &[u8]) {
    fs::write(out.join(name), bytes).unwrap_or_else(|e| panic!("cannot write {name}: {e}"));
}

/// Packs `values` (each below 2^21) into 21 bits each, least significant first, with four bytes of zero after the last so a reader may always load four bytes.
fn x_pack21(values: &[u32]) -> Vec<u8> {
    let mut out = vec![0u8; (values.len() * 21 + 7) / 8 + 4];
    for (i, &v) in values.iter().enumerate() {
        assert!(v < 1 << 21);
        let bit = i * 21;
        let w = (v as u64) << (bit & 7);
        for k in 0..4 {
            out[(bit >> 3) + k] |= (w >> (8 * k)) as u8;
        }
    }
    out
}

/// A subset variant: the entries `member` admits, as runs of consecutive codepoints (`run_cp`, each run's first codepoint as a little-endian u32, closed by u32::MAX; `run_slot`, its first slot as a u16, closed by the slot count), their codes in codepoint order (`codes`: len << 24 | bits, u32 each), and their slots in code order (`by_key`: u16 each).
fn x_subset(entries: &[XEntry], out: &Path, name: &str, member: impl Fn(&XEntry) -> bool) {
    let slots: Vec<&XEntry> = entries.iter().filter(|e| member(e)).collect();
    assert!(slots.len() < 1 << 16, "{name}: a slot and the slot count must fit in u16");
    let mut run_cp = Vec::new();
    let mut run_slot = Vec::new();
    let mut codes = Vec::new();
    for (s, e) in slots.iter().enumerate() {
        if s == 0 || e.cp != slots[s - 1].cp + 1 {
            run_cp.extend_from_slice(&e.cp.to_le_bytes());
            run_slot.extend_from_slice(&(s as u16).to_le_bytes());
        }
        codes.extend_from_slice(&(e.len << 24 | e.bits).to_le_bytes());
    }
    run_cp.extend_from_slice(&u32::MAX.to_le_bytes());
    run_slot.extend_from_slice(&(slots.len() as u16).to_le_bytes());
    let mut order: Vec<u16> = (0..slots.len() as u32).map(|s| s as u16).collect();
    order.sort_unstable_by_key(|&s| x_key(slots[s as usize]));
    let by_key: Vec<u8> = order.iter().flat_map(|s| s.to_le_bytes()).collect();
    x_write(out, &format!("x_{name}_run_cp.bin"), &run_cp);
    x_write(out, &format!("x_{name}_run_slot.bin"), &run_slot);
    x_write(out, &format!("x_{name}_codes.bin"), &codes);
    x_write(out, &format!("x_{name}_by_key.bin"), &by_key);
}

/// The full variant: every entry, as the code tree's shape plus one permutation (see src/x_text.rs, `full`, for how each table is read).
fn x_full(entries: &[XEntry], out: &Path) {
    let n = entries.len();
    // Ranks: the entries in code order. `perm[rank]` is the codepoint index (the entry's own index in codepoint order).
    let mut perm: Vec<u32> = (0..n as u32).collect();
    perm.sort_unstable_by_key(|&i| x_key(&entries[i as usize]));
    // The shape: maximal runs of equal length in code order; a complete prefix code tiles the 24-bit keys, so a run's keys are its first key plus multiples of 2^(24 - len).
    let mut shape = Vec::new();
    let mut expect_key = 0u32;
    for (r, &i) in perm.iter().enumerate() {
        let e = &entries[i as usize];
        assert_eq!(x_key(e), expect_key, "the codebook must be a complete prefix code");
        expect_key += 1 << (24 - e.len);
        if r == 0 || e.len != entries[perm[r - 1] as usize].len {
            shape.extend_from_slice(&x_key(e).to_le_bytes());
            shape.extend_from_slice(&(r as u32).to_le_bytes());
            shape.extend_from_slice(&e.len.to_le_bytes());
        }
    }
    assert_eq!(expect_key, 1 << 24, "the codebook must be a complete prefix code");
    shape.extend_from_slice(&(1u32 << 24).to_le_bytes());
    shape.extend_from_slice(&(n as u32).to_le_bytes());
    shape.extend_from_slice(&0u32.to_le_bytes());
    // The inverse, for the encoder: along every cycle of `perm` longer than the stride, every stride-th element is marked and points back to the mark before it.
    let words = (n + 63) / 64;
    let mut marks = vec![0u64; words];
    let mut back_of = vec![u32::MAX; n];
    let mut seen = vec![false; n];
    for start in 0..n {
        if seen[start] {
            continue;
        }
        let mut cycle = Vec::new();
        let mut x = start;
        while !seen[x] {
            seen[x] = true;
            cycle.push(x as u32);
            x = perm[x] as usize;
        }
        if cycle.len() <= X_FULL_STRIDE {
            continue;
        }
        let picked: Vec<u32> = cycle.iter().copied().step_by(X_FULL_STRIDE).collect();
        for (k, &m) in picked.iter().enumerate() {
            marks[m as usize >> 6] |= 1 << (m & 63);
            back_of[m as usize] = picked[(k + picked.len() - 1) % picked.len()];
        }
    }
    let back: Vec<u32> = (0..n).filter(|&x| marks[x >> 6] >> (x & 63) & 1 == 1).map(|x| back_of[x]).collect();
    // Marks before each block of eight words, and one closing count.
    let mut blocks = Vec::new();
    let mut before = 0u32;
    for (w, word) in marks.iter().enumerate() {
        if w % 8 == 0 {
            blocks.extend_from_slice(&before.to_le_bytes());
        }
        before += word.count_ones();
    }
    blocks.extend_from_slice(&before.to_le_bytes());
    let marks_bytes: Vec<u8> = marks.iter().flat_map(|w| w.to_le_bytes()).collect();
    // The first 256 codepoints' codes directly (len << 24 | bits), so Latin-1 text skips the walk.
    let low: Vec<u8> = entries[..256].iter().flat_map(|e| (e.len << 24 | e.bits).to_le_bytes()).collect();
    x_write(out, "x_full_shape.bin", &shape);
    x_write(out, "x_full_perm.bin", &x_pack21(&perm));
    x_write(out, "x_full_marks.bin", &marks_bytes);
    x_write(out, "x_full_blocks.bin", &blocks);
    x_write(out, "x_full_back.bin", &x_pack21(&back));
    x_write(out, "x_full_low.bin", &low);
}

/// Generates the tables of every enabled x variant from the codebook, which `main` has already checked against its pinned hash: a table is only ever built from the pinned wire format.
fn x_tables(codebook: &[u8]) {
    let on = |f: &str| env::var_os(format!("CARGO_FEATURE_{f}")).is_some();
    let (compact, latin, lang16, full) = (on("X_COMPACT"), on("X_LATIN"), on("X_LANG16"), on("X_FULL"));
    if !(compact || latin || lang16 || full) {
        return;
    }
    let out = PathBuf::from(env::var("OUT_DIR").expect("cargo sets OUT_DIR"));
    let n = u32::from_le_bytes(codebook[8..12].try_into().unwrap()) as usize;
    assert_eq!(codebook.len(), 12 + n * 8, "codebook length");
    let entries: Vec<XEntry> = (0..n)
        .map(|i| {
            let o = 12 + i * 8;
            let cp = u32::from_le_bytes(codebook[o..o + 4].try_into().unwrap());
            let packed = u32::from_le_bytes(codebook[o + 4..o + 8].try_into().unwrap());
            XEntry { cp, bits: packed & 0x00FF_FFFF, len: (packed >> 24) & 0x1F }
        })
        .collect();
    // Every scalar value in codepoint order is what lets an entry's index stand for its codepoint (index = cp below U+D800, cp - G#800 above).
    for (i, e) in entries.iter().enumerate() {
        let cp = if i < 0xD800 { i as u32 } else { i as u32 + 0x800 };
        assert_eq!(e.cp, cp, "the codebook must hold every scalar value in codepoint order");
    }
    let short = |e: &XEntry| e.len <= 20;
    let in_blocks = |e: &XEntry, blocks: &[(u32, u32)]| blocks.iter().any(|&(a, b)| e.cp >= a && e.cp <= b);
    if compact {
        x_subset(&entries, &out, "compact", short);
    }
    if latin {
        x_subset(&entries, &out, "latin", |e| short(e) || in_blocks(e, X_LATIN_BLOCKS));
    }
    if lang16 {
        x_subset(&entries, &out, "lang16", |e| short(e) || in_blocks(e, X_LATIN_BLOCKS) || in_blocks(e, X_LANG16_BLOCKS));
    }
    if full {
        x_full(&entries, &out);
    }
}
