//! Build script for VSF.
//!
//! Verifies that the vendored Huffman corpus and codebook (`frequencies.bin`, `huffman_codes.bin`) are byte-identical to their pinned BLAKE3 hashes. The codebook IS the wire format for `VsfType::x`, so drift here silently bifurcates the identity namespace thru every downstream consumer (ihi handle hashing, photon avatar keys, contact tables, anywhere else that round-trips text thru VSF).
//!
//! Both files are vendored in the repo and tracked in version control. The build script never regenerates, never downloads, never silently mutates either file. A hash mismatch is a hard build failure.
//!
//! `huffman_codes.bin` is written by `tools/codebook/` (since vsf 0.11): the tree of the original Huffman build over `frequencies.bin`, kept codeword for codeword, with every code of 20 bits or fewer and every ASCII character pinned to its original codeword and every other codepoint, ascending, on the remaining codewords in code order. Running it twice gives the same bytes. `frequencies.bin` is the corpus that tree came from, kept as its record; the codebook no longer reads it.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

// Pinned BLAKE3 hashes. To change the codebook:
//   1. Change tools/codebook/ and regenerate huffman_codes.bin with it (`cargo run --release -- ../../huffman_codes.bin` there; its test checks the vendored file is its output)
//   2. Update the constant below
//   3. Bump vsf MAJOR version (any codebook change is a wire-format break)
//   4. Update ihi to depend on the new vsf version and re-attest existing identities
const FREQUENCIES_BLAKE3: &str = "0652a53f8ac8b1989a3c0a6f7290af1d41d4b58cd960b77f879b38223ccd7b0d";
const HUFFMAN_CODES_BLAKE3: &str =
    "51b6e8eb748c56c4991ce8fe556422ec95f2bcfff78927a7778b1a892d5806f9";

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
        // No pinned codepoints known: VsfType::since then takes every x value for the newest codebook (the safe side).
        x_write(&PathBuf::from(env::var("OUT_DIR").expect("cargo sets OUT_DIR")), "x_pinned_cp.bin", &[]);
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

// ==================== the codebook's order, and the x text tables (src/x_text.rs, src/types/since.rs) ====================

/// The codepoint ranges the codebook pins to their original codewords whatever their length, besides every code of 20 bits or fewer (tools/codebook/src/shape.rs, `PINNED_BLOCKS`; tools/codebook/src/lib.rs says why).
const X_PINNED_BLOCKS: &[(u32, u32)] = &[(0x0000, 0x007F)];

/// The Unicode blocks a variant adds to the codes of 20 bits or fewer (the compact set), as inclusive codepoint ranges; each variant wider is a superset of the one before.
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

/// Whether the codebook pins `e` to its original codeword.
fn x_pinned(e: &XEntry) -> bool {
    e.len <= 20 || X_PINNED_BLOCKS.iter().any(|&(a, b)| e.cp >= a && e.cp <= b)
}

fn x_write(out: &Path, name: &str, bytes: &[u8]) {
    fs::write(out.join(name), bytes).unwrap_or_else(|e| panic!("cannot write {name}: {e}"));
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

/// The free codewords (every codeword not pinned) in code order, as runs of equal length: first codeword left-aligned to 24 bits, the free index of that codeword, the length (little-endian u32 each), closed by (1 << 24, the free count, 0).
fn x_free_runs(entries: &[XEntry]) -> Vec<u8> {
    let mut free: Vec<&XEntry> = entries.iter().filter(|e| !x_pinned(e)).collect();
    free.sort_unstable_by_key(|e| x_key(e));
    let mut runs = Vec::new();
    for (i, e) in free.iter().enumerate() {
        let prev = if i == 0 { None } else { Some(free[i - 1]) };
        let continues = prev.is_some_and(|p| p.len == e.len && x_key(p) + (1 << (24 - p.len)) == x_key(e));
        if !continues {
            runs.extend_from_slice(&x_key(e).to_le_bytes());
            runs.extend_from_slice(&(i as u32).to_le_bytes());
            runs.extend_from_slice(&e.len.to_le_bytes());
        }
    }
    runs.extend_from_slice(&(1u32 << 24).to_le_bytes());
    runs.extend_from_slice(&(free.len() as u32).to_le_bytes());
    runs.extend_from_slice(&0u32.to_le_bytes());
    runs
}

/// Checks the codebook against its documented order (tools/codebook/src/lib.rs): a complete prefix code, every scalar value in codepoint order, and every codepoint not pinned holding the free codewords in code order. The full variant's tables and `since` rely on it, so a codebook in any other order fails the build.
fn x_check_order(entries: &[XEntry]) {
    for (i, e) in entries.iter().enumerate() {
        let cp = if i < 0xD800 { i as u32 } else { i as u32 + 0x800 };
        assert_eq!(e.cp, cp, "the codebook must hold every scalar value in codepoint order");
    }
    let mut keys: Vec<(u32, u32)> = entries.iter().map(|e| (x_key(e), e.len)).collect();
    keys.sort_unstable();
    let mut next = 0u32;
    for &(k, len) in &keys {
        assert_eq!(k, next, "the codebook must be a complete prefix code");
        next += 1 << (24 - len);
    }
    assert_eq!(next, 1 << 24, "the codebook must be a complete prefix code");
    let mut last: Option<u32> = None;
    for e in entries.iter().filter(|e| !x_pinned(e)) {
        assert!(last.is_none_or(|k| k < x_key(e)), "U+{:04X}: codepoints not pinned must hold the free codewords in code order", e.cp);
        last = Some(x_key(e));
    }
}

/// Checks the codebook's order, writes the pinned codepoints (always: `VsfType::since` reads them), and generates the tables of every enabled x variant from the codebook, which `main` has already checked against its pinned hash: a table is only ever built from the pinned wire format.
fn x_tables(codebook: &[u8]) {
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
    x_check_order(&entries);
    let pinned: Vec<u8> = entries.iter().filter(|e| x_pinned(e)).flat_map(|e| e.cp.to_le_bytes()).collect();
    x_write(&out, "x_pinned_cp.bin", &pinned);

    let on = |f: &str| env::var_os(format!("CARGO_FEATURE_{f}")).is_some();
    let short = |e: &XEntry| e.len <= 20;
    let in_blocks = |e: &XEntry, blocks: &[(u32, u32)]| blocks.iter().any(|&(a, b)| e.cp >= a && e.cp <= b);
    if on("X_COMPACT") {
        x_subset(&entries, &out, "compact", short);
    }
    if on("X_LATIN") {
        x_subset(&entries, &out, "latin", |e| short(e) || in_blocks(e, X_LATIN_BLOCKS));
    }
    if on("X_LANG16") {
        x_subset(&entries, &out, "lang16", |e| short(e) || in_blocks(e, X_LATIN_BLOCKS) || in_blocks(e, X_LANG16_BLOCKS));
    }
    if on("X_FULL") {
        x_subset(&entries, &out, "full", x_pinned);
        x_write(&out, "x_full_free.bin", &x_free_runs(&entries));
    }
}
