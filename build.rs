//! Build script for VSF.
//!
//! Verifies that the vendored Huffman corpus and codebook (`frequencies.bin`,
//! `huffman_codes.bin`) are byte-identical to their pinned BLAKE3 hashes. The codebook IS the
//! wire format for `VsfType::x`, so drift here silently bifurcates the identity namespace
//! through every downstream consumer (ihi handle hashing, photon avatar keys, contact tables,
//! anywhere else that round-trips text through VSF).
//!
//! Both files are vendored in the repo and tracked in version control. Regeneration is an
//! explicit offline operation performed via `tools/frequency-analyzer/` — the build script
//! never regenerates, never downloads, never silently mutates either file. A hash mismatch is
//! a hard build failure.

use std::fs;
use std::path::Path;

// Pinned BLAKE3 hashes. To change the codebook:
//   1. Regenerate frequencies.bin and huffman_codes.bin in tools/frequency-analyzer/
//   2. Update both constants below
//   3. Bump vsf MAJOR version (any codebook change is a wire-format break)
//   4. Update ihi to depend on the new vsf version and re-attest existing identities
const FREQUENCIES_BLAKE3: &str =
    "0652a53f8ac8b1989a3c0a6f7290af1d41d4b58cd960b77f879b38223ccd7b0d";
const HUFFMAN_CODES_BLAKE3: &str =
    "523769a934a407aa87d9ddef37ad186c546e205cb1280b0e9300d6e98c6dad0e";

fn verify_blake3(path: &str, expected_hex: &str) -> Result<(), String> {
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
    Ok(())
}

fn main() {
    println!("cargo::rustc-check-cfg=cfg(huffman_available)");
    println!("cargo:rerun-if-changed=frequencies.bin");
    println!("cargo:rerun-if-changed=huffman_codes.bin");
    println!("cargo:rerun-if-changed=build.rs");

    // Docs.rs and similar restricted environments still need to compile even without the
    // vendored codebook. If huffman_codes.bin is genuinely absent, leave the cfg flag unset —
    // text_encoding.rs sees the missing cfg and returns an empty HashMap. Encoding then panics
    // with a helpful message rather than silently producing wrong bytes.
    if !Path::new("huffman_codes.bin").exists() {
        eprintln!(
            "cargo:warning=huffman_codes.bin missing — text encoding will be unavailable in this build"
        );
        return;
    }

    // frequencies.bin is the input corpus; it's vendored in the git repo but excluded from
    // the published .crate to stay under the crates.io 10MiB limit. Verify it when present
    // (developer builds from a clone), skip when absent (downstream users from crates.io).
    // huffman_codes.bin — the runtime-load artifact — is ALWAYS verified.
    if Path::new("frequencies.bin").exists() {
        if let Err(msg) = verify_blake3("frequencies.bin", FREQUENCIES_BLAKE3) {
            panic!("{}", msg);
        }
    }
    if let Err(msg) = verify_blake3("huffman_codes.bin", HUFFMAN_CODES_BLAKE3) {
        panic!("{}", msg);
    }

    println!("cargo:rustc-cfg=huffman_available");
}
