//! Hostile input is refused with an error, never a panic or an abort (security audit 2026-11-01, VSF-01..04): every reader below once crashed the process from a few bytes.

use vsf::file_format::VsfSection;
use vsf::{parse, VsfType};

/// A wire length near `usize::MAX` once wrapped `pointer + length` past the bounds check in a release build, and the slice after it panicked (VSF-01); a count of 2^62 characters once aborted in the allocator (VSF-02).
#[test]
fn huge_lengths_and_counts_are_errors() {
    let inputs: [&[u8]; 6] = [
        b"d6\xff\xff\xff\xff\xff\xff\xff\xfe",
        b"a6\xff\xff\xff\xff\xff\xff\xff\xff",
        b"x6\x40\x00\x00\x00\x00\x00\x00\x00",
        b"hb6\xff\xff\xff\xff\xff\xff\xff\xff",
        b"tn6\xff\xff\xff\xff\xff\xff\xff\xfcu3",
        b"t6\x10\x00\x00\x00\x00\x00\x00\x00",
    ];
    for input in inputs {
        let r = std::panic::catch_unwind(|| parse(input, &mut 0));
        assert!(r.is_ok(), "{input:?} panicked");
        assert!(r.unwrap().is_err(), "{input:?} parsed");
    }
}

/// A section named by a table of contents is checked, never constructed with the panicking `new` (VSF-03).
#[test]
fn a_bad_section_name_is_an_error() {
    assert!(VsfSection::try_new("Abc").is_err());
    assert!(VsfSection::try_new("").is_err());
    assert!(VsfSection::try_new("a b").is_err());
    assert!(VsfSection::try_new("ping").is_ok());
}

/// A header carrying two signer keys is refused: the verifier reads the first and the decoder once reported the last, so the signature and the named signer could belong to different keys (VSF-04).
#[test]
fn a_header_with_two_keys_is_refused() {
    let doc = vsf::VsfBuilder::new().creation_time_oscillations(1).provenance_only().add_section("ping", vec![("x".to_string(), VsfType::u(1, false))]).build().expect("build");
    assert!(vsf::file_format::VsfHeader::decode(&doc).is_ok(), "the plain document decodes");
    // Splice two keys in right after the provenance hash's bytes: find "hp" and its 32 bytes (one length byte for a 32-byte hash).
    let hp_at = doc.windows(2).position(|w| w == b"hp").expect("hp in header");
    let after_hp = hp_at + 2 + 2 + 32;
    let key = VsfType::ke(vec![7u8; 32]).flatten();
    let mut one = doc[..after_hp].to_vec();
    one.extend_from_slice(&key);
    one.extend_from_slice(&doc[after_hp..]);
    assert!(vsf::file_format::VsfHeader::decode(&one).is_ok(), "the splice point is right: one key decodes");
    let mut two = doc[..after_hp].to_vec();
    two.extend_from_slice(&key);
    two.extend_from_slice(&key);
    two.extend_from_slice(&doc[after_hp..]);
    let r = vsf::file_format::VsfHeader::decode(&two);
    assert!(r.is_err(), "a second ke must be refused");
}
