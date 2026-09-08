//! `#[derive(Vsf)]` contract tests: byte-identity against a hand-built section (the migration guarantee — converting a codec must not move the wire), round-trip fidelity, the additive/total reader rules, and the required-field Err.
#![cfg(feature = "derive")]

use vsf::types::EtType;
use vsf::{Vsf, VsfSection, VsfType};

#[derive(Vsf, Debug, PartialEq, Default, Clone)]
#[vsf(section = "depart_req")]
struct DepartReq {
    #[vsf(eagle, name = "t")]
    t: i64,
    #[vsf(kind = "ge", name = "cs")]
    cs: Vec<u8>,
    #[vsf(name = "it")]
    it: Option<u64>,
    #[vsf(kind = "hp", name = "wc")]
    wc: Option<[u8; 32]>,
}

#[test]
fn derive_bytes_match_the_hand_built_section() {
    let req = DepartReq { t: 123_456_789, cs: vec![7u8; 64], it: Some(1), wc: Some([9u8; 32]) };
    let derived = req.to_section().encode();
    // The hand-built twin, exactly as network/fgtw/protocol.rs builds it today.
    let mut hand = VsfSection::new("depart_req");
    hand.add_field("t", VsfType::e(EtType::e6(123_456_789)));
    hand.add_field("cs", VsfType::ge(vec![7u8; 64]));
    hand.add_field("it", VsfType::u(1, false));
    hand.add_field("wc", VsfType::hp(vec![9u8; 32]));
    assert_eq!(derived, hand.encode(), "derive output must be byte-identical to the hand codec — the migration guarantee");
}

#[test]
fn derive_round_trips_and_optionals_are_total() {
    let full = DepartReq { t: -5, cs: vec![1, 2, 3], it: Some(255), wc: Some([4u8; 32]) };
    let bytes = full.to_section().encode();
    let mut ptr = 0;
    let sec = VsfSection::parse(&bytes, &mut ptr).unwrap();
    assert_eq!(DepartReq::from_section(&sec).unwrap(), full);

    // Optionals absent → None; a legacy section (pre-intent build) parses clean.
    let minimal = DepartReq { t: 9, cs: vec![5], it: None, wc: None };
    let bytes = minimal.to_section().encode();
    let mut ptr = 0;
    let sec = VsfSection::parse(&bytes, &mut ptr).unwrap();
    let got = DepartReq::from_section(&sec).unwrap();
    assert_eq!(got, minimal);
    // Unknown extra fields skip (a NEWER sender): additive both directions.
    let mut future = minimal.to_section();
    future.add_field("brand_new", VsfType::u(3, false));
    let bytes = future.encode();
    let mut ptr = 0;
    let sec = VsfSection::parse(&bytes, &mut ptr).unwrap();
    assert_eq!(DepartReq::from_section(&sec).unwrap(), minimal);
}

#[test]
fn missing_required_field_is_one_err_not_a_panic() {
    let mut sec = VsfSection::new("depart_req");
    sec.add_field("cs", VsfType::ge(vec![1]));
    let err = DepartReq::from_section(&sec).unwrap_err();
    assert!(err.contains('t'), "the error names the missing field: {err}");
}

#[derive(Vsf, Debug, PartialEq, Default)]
#[vsf(section = "m")]
struct Repeats {
    #[vsf(eagle, name = "wt")]
    woven: Vec<i64>,
    // ascii: the derive-feature test build carries no Huffman codebook; the x path is photon's (text feature) and the attr is the point under test either way.
    #[vsf(ascii, name = "md")]
    dests: Vec<String>,
    armed: bool,
}

#[test]
fn repeated_fields_and_flags() {
    let m = Repeats { woven: vec![10, -20], dests: vec!["a".into(), "b".into()], armed: true };
    let bytes = m.to_section().encode();
    let mut ptr = 0;
    let sec = VsfSection::parse(&bytes, &mut ptr).unwrap();
    assert_eq!(Repeats::from_section(&sec).unwrap(), m);
    // Empty vecs + false flag encode NO FIELDS (additive). A fieldless section body is ZERO bytes by design (the header TOC declares it — the ping idiom), so the bare-body round trip goes thru encode_encrypted, the sealed form that carries the name+counts itself (exactly how pongsec travels).
    let empty = Repeats::default();
    assert!(empty.to_section().encode().is_empty(), "fieldless body encodes empty — header-declared");
    let bytes = empty.to_section().encode_encrypted();
    let mut ptr = 0;
    let sec = VsfSection::parse(&bytes, &mut ptr).unwrap();
    assert_eq!(Repeats::from_section(&sec).unwrap(), empty);
}
