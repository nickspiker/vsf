//! `kvr` (RSA verification key) and `kgr` (RSA signing key, the first of the `kg` family): each survives a flatten and parse with its bytes and its marker, and the inspector never prints a signing key's bytes.
#![allow(deprecated)]

use vsf::VsfType;

fn round_trip(v: VsfType, marker: &[u8]) -> VsfType {
    let flat = v.flatten();
    assert_eq!(&flat[..marker.len()], marker, "wire marker");
    assert_eq!(flat.len(), v.byte_len(), "byte_len agrees with flatten");
    let mut at = 0;
    let back = vsf::decoding::parse::parse(&flat, &mut at).expect("parses");
    assert_eq!(at, flat.len(), "parse consumed the whole value");
    back
}

#[test]
fn rsa_verification_key_round_trips() {
    // An RSA-4096 modulus is 512 bytes; the bytes themselves are only a pattern here.
    let n: Vec<u8> = (0..512).map(|i| (i * 7 + 3) as u8).collect();
    match round_trip(VsfType::kvr(n.clone()), b"kvr") {
        VsfType::kvr(b) => assert_eq!(b, n),
        other => panic!("came back as {other:?}"),
    }
}

#[test]
fn rsa_signing_key_round_trips() {
    // p ‖ q of an RSA-4096 key: two 256-byte primes.
    let pq: Vec<u8> = (0..512).map(|i| (i * 13 + 5) as u8).collect();
    match round_trip(VsfType::kgr(pq.clone()), b"kgr") {
        VsfType::kgr(b) => assert_eq!(b, pq),
        _ => panic!("kgr came back as another type"),
    }
}

#[cfg(feature = "inspect")]
#[test]
fn signing_key_never_shown() {
    let pq: Vec<u8> = vec![0xA5; 512];
    for shown in [vsf::inspect::format_value_literal(&VsfType::kgr(pq.clone())), vsf::inspect::format_value(&VsfType::kgr(pq.clone()))] {
        assert!(shown.contains("kgr"), "names the type: {shown}");
        assert!(!shown.to_ascii_lowercase().contains("a5a5"), "prints no key bytes: {shown}");
    }
}
