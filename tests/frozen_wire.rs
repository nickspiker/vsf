//! v8 wire-freeze regression tests.
//!
//! These pin the exact on-wire bytes of the text types. If any assertion here fails, the wire format has broken: bump VSF_VERSION, bump the crate minor to match (0.z.patch alignment), update VSF_BACKWARD_COMPAT if the change is not additive, and face the changelog. The z7 era shipped two wire breaks (l→a marker remap ~0.3.5, x codebook remap 0.4.0) without version bumps — this file exists so that can never happen silently again.

#![cfg(feature = "text-encode")]

use vsf::types::VsfType;

#[test]
fn frozen_x_huffman_v8() {
    // Pinned v8 bytes for x("octopus") — NFC + full-codespace Huffman codebook.
    let bytes = VsfType::x("octopus".to_string()).flatten();
    assert_eq!(
        bytes,
        X_OCTOPUS_V8,
        "x wire encoding shifted — this is a format break, see module docs"
    );
}

#[test]
fn frozen_x_nfc_equivalence_v8() {
    // Precomposed é (U+00E9) and decomposed e+◌́ (U+0065 U+0301) must produce byte-identical streams forever.
    let precomposed = VsfType::x("café".to_string()).flatten();
    let decomposed = VsfType::x("cafe\u{0301}".to_string()).flatten();
    assert_eq!(precomposed, decomposed, "NFC canonicalization broke");
    assert_eq!(precomposed, X_CAFE_V8, "x wire encoding for café shifted");
}

#[test]
fn frozen_a_ascii_v8() {
    // a is raw ASCII bytes + length, always — never Huffman, never anything else.
    let bytes = VsfType::a("octopus".to_string()).flatten();
    assert_eq!(
        bytes,
        A_OCTOPUS_V8,
        "a wire encoding shifted — this is a format break, see module docs"
    );
}

#[test]
fn frozen_d_dict_key_v8() {
    // d is raw bytes + length, always.
    let bytes = VsfType::d("octopus".to_string()).flatten();
    assert_eq!(
        bytes,
        D_OCTOPUS_V8,
        "d wire encoding shifted — this is a format break, see module docs"
    );
}

// Pinned bytes. To regenerate after an INTENTIONAL format break: cargo test -p vsf --test frozen_wire -- --nocapture with the consts emptied, then paste from the failure output.
const X_OCTOPUS_V8: &[u8] = &[120, 51, 7, 180, 84, 182, 169, 160];
const X_CAFE_V8: &[u8] = &[120, 51, 4, 139, 168, 103, 240];
const A_OCTOPUS_V8: &[u8] = &[97, 51, 7, 111, 99, 116, 111, 112, 117, 115];
const D_OCTOPUS_V8: &[u8] = &[100, 51, 7, 111, 99, 116, 111, 112, 117, 115];

#[test]
#[ignore = "generator — run with --ignored --nocapture to print current wire bytes"]
fn print_current_wire_bytes() {
    for (label, bytes) in [
        ("X_OCTOPUS_V8", VsfType::x("octopus".to_string()).flatten()),
        ("X_CAFE_V8", VsfType::x("café".to_string()).flatten()),
        ("A_OCTOPUS_V8", VsfType::a("octopus".to_string()).flatten()),
        ("D_OCTOPUS_V8", VsfType::d("octopus".to_string()).flatten()),
    ] {
        println!("const {}: &[u8] = &{:?};", label, bytes);
    }
}
