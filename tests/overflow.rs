//! Hostile sizes fail loudly — an error, never a panic, never an allocation sized by a forged shape.

use vsf::types::{BitPackedTensor, Tensor, VsfType};

/// A `p` tensor header with the given dims and depth, followed by `tail` bytes.
fn packed_bytes(shape: &[usize], bit_depth: u8, tail: &[u8]) -> Vec<u8> {
    let mut b = vec![b'p'];
    b.extend_from_slice(&vsf::EncodeNumber::encode_number(&(shape.len() as u64)));
    b.push(bit_depth);
    for &d in shape {
        b.extend_from_slice(&vsf::EncodeNumber::encode_number(&(d as u64)));
    }
    b.extend_from_slice(tail);
    b
}

#[test]
fn a_forged_bitpacked_shape_is_an_error_not_a_panic() {
    // 2^40 × 2^40 elements at 16 bits: the product overflows; the parser must say so without touching memory.
    let b = packed_bytes(&[1 << 40, 1 << 40], 16, &[0u8; 8]);
    let mut ptr = 0;
    assert!(vsf::parse(&b, &mut ptr).is_err());
    // A plausible shape whose bit length overflows the address space.
    let b = packed_bytes(&[usize::MAX / 2], 128, &[0u8; 8]);
    let mut ptr = 0;
    assert!(vsf::parse(&b, &mut ptr).is_err());
    // A big but honest shape with too few bytes: still an error, not a read past the end.
    let b = packed_bytes(&[1 << 20, 1 << 20], 12, &[0u8; 16]);
    let mut ptr = 0;
    assert!(vsf::parse(&b, &mut ptr).is_err());
    // And the skip walker agrees.
    let mut ptr = 0;
    assert!(vsf::container::skip::skip_value(&b, &mut ptr).is_err());
}

#[test]
fn a_forged_wrapped_length_is_an_error() {
    let mut b = vec![b'v', b'b'];
    b.extend_from_slice(&vsf::EncodeNumber::encode_number(&u64::MAX));
    b.extend_from_slice(&[0u8; 4]);
    let mut ptr = 0;
    assert!(vsf::parse(&b, &mut ptr).is_err());
    let mut ptr = 0;
    assert!(vsf::container::skip::skip_value(&b, &mut ptr).is_err());
}

#[test]
fn try_new_refuses_overflowing_and_mismatched_shapes() {
    assert!(Tensor::<u8>::try_new(vec![1 << 40, 1 << 40], vec![]).is_err());
    assert!(Tensor::<u8>::try_new(vec![2, 2], vec![1, 2, 3]).is_err());
    assert_eq!(Tensor::<u8>::try_new(vec![2, 2], vec![1, 2, 3, 4]).unwrap().shape, vec![2, 2]);
}

#[test]
fn wrapped_payload_length_is_encoded_in_bits_through_u64() {
    // A 300-byte payload → 2400 bits on the wire, round-tripping at any target width.
    let v = VsfType::v(b'b', vec![7u8; 300]);
    let bytes = v.flatten();
    let mut ptr = 0;
    assert_eq!(vsf::parse(&bytes, &mut ptr).unwrap(), v);
    let _ = BitPackedTensor::pack_u8(8, vec![4], &[1, 2, 3, 4]);
}
