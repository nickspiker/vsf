//! The empty 1D vector round trip — pinned against the count-0 decode bug.
//! The 'n' vector form with count 0 used to fall thru the `count > 0` guards into the multi-dim path, parse a shape dimension the encoder never wrote, and eat the NEXT byte of the stream — a field's ',' separator — as a size marker.
//! Photon's persisted phonebook hit this on any peer with no local_ip (an empty t_u3), going "unreadable, starting fresh" on every boot.

use vsf::{Tensor, VsfType};

#[test]
fn empty_u8_vector_round_trips_alone() {
    let bytes = VsfType::t_u3(Tensor::new(vec![0], Vec::new())).flatten();
    let mut ptr = 0;
    let back = vsf::parse(&bytes, &mut ptr).expect("empty vector must parse");
    assert_eq!(ptr, bytes.len(), "the parser must consume exactly the vector's bytes");
    match back {
        VsfType::t_u3(t) => {
            assert_eq!(t.shape, vec![0]);
            assert!(t.data.is_empty());
        }
        other => panic!("expected t_u3, got {:?}", other),
    }
}

#[test]
fn empty_vector_does_not_eat_the_following_field_value() {
    // The phonebook shape: a multi-value field with an empty vector mid-list — the value AFTER it must survive.
    let mut section = vsf::VsfSection::new("row");
    section.add_field_multi(
        "peer".to_string(),
        vec![
            VsfType::u4(4383),
            VsfType::t_u3(Tensor::new(vec![0], Vec::new())),
            VsfType::ge(vec![7u8; 64]),
        ],
    );
    let bytes = section.encode();
    let mut ptr = 0;
    let back = vsf::VsfSection::parse(&bytes, &mut ptr).expect("section must parse");
    let field = back.fields.first().expect("field present");
    assert_eq!(field.values.len(), 3, "all three values survive the empty vector");
    match (&field.values[1], &field.values[2]) {
        (VsfType::t_u3(t), VsfType::ge(sig)) => {
            assert!(t.data.is_empty());
            assert_eq!(sig.len(), 64);
        }
        other => panic!("wrong shapes after the empty vector: {:?}", other),
    }
}

#[test]
fn empty_u16_vector_round_trips() {
    let bytes = VsfType::t_u4(Tensor::new(vec![0], Vec::new())).flatten();
    let mut ptr = 0;
    let back = vsf::parse(&bytes, &mut ptr).expect("empty u16 vector must parse");
    assert_eq!(ptr, bytes.len());
    // 1D u16 vectors decode to the dedicated Vector type (crate convention); empty must still round-trip.
    match back {
        VsfType::t_u4(t) => assert!(t.data.is_empty()),
        VsfType::v_u4(v) => assert!(v.data.is_empty()),
        other => panic!("expected an empty u16 vector shape, got {:?}", other),
    }
}
