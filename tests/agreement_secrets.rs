//! `krx` (X25519 private key, the first of the `kr` family) and `ksb` (BLAKE3-derived shared secret): each survives a flatten and parse with its bytes and its marker, its constraint takes 32 bytes of its own type only, and the inspector never prints either's bytes.

use vsf::schema::TypeConstraint;
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
fn x25519_private_key_round_trips() {
    // An X25519 private key is 32 bytes (RFC 7748's scalar); the bytes are only a pattern here.
    let key: Vec<u8> = (0..32).map(|i| (i * 11 + 9) as u8).collect();
    match round_trip(VsfType::krx(key.clone()), b"krx") {
        VsfType::krx(b) => assert_eq!(b, key),
        _ => panic!("krx came back as another type"),
    }
}

#[test]
fn derived_secret_round_trips() {
    let secret: Vec<u8> = (0..32).map(|i| (i * 7 + 3) as u8).collect();
    match round_trip(VsfType::ksb(secret.clone()), b"ksb") {
        VsfType::ksb(b) => assert_eq!(b, secret),
        _ => panic!("ksb came back as another type"),
    }
}

#[test]
fn constraints_take_32_bytes_of_their_own_type() {
    assert!(TypeConstraint::X25519PrivateKey.matches(&VsfType::krx(vec![1; 32])));
    assert!(!TypeConstraint::X25519PrivateKey.matches(&VsfType::krx(vec![1; 31])));
    assert!(!TypeConstraint::X25519PrivateKey.matches(&VsfType::kx(vec![1; 32])));
    assert!(!TypeConstraint::X25519PrivateKey.matches(&VsfType::ksx(vec![1; 32])));
    assert!(TypeConstraint::DerivedSecret.matches(&VsfType::ksb(vec![1; 32])));
    assert!(!TypeConstraint::DerivedSecret.matches(&VsfType::ksb(vec![1; 33])));
    assert!(!TypeConstraint::DerivedSecret.matches(&VsfType::ksx(vec![1; 32])));
}

#[test]
fn an_unknown_agreement_key_is_refused() {
    let mut flat = VsfType::krx(vec![1; 32]).flatten();
    flat[2] = b'q';
    let mut at = 0;
    assert!(vsf::decoding::parse::parse(&flat, &mut at).is_err());
}

#[cfg(feature = "inspect")]
#[test]
fn secrets_never_shown() {
    for v in [VsfType::krx(vec![0xA5; 32]), VsfType::ksb(vec![0xA5; 32])] {
        for shown in [vsf::inspect::format_value_literal(&v), vsf::inspect::format_value(&v)] {
            assert!(shown.contains("krx") || shown.contains("ksb"), "names the type: {shown}");
            assert!(!shown.to_ascii_lowercase().contains("a5a5"), "prints no secret bytes: {shown}");
        }
    }
}
