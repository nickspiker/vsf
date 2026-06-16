//! `VsfType::uint` picks the narrowest unsigned width that holds the value (EWE), so a caller passes one `u64` and stores it minimally.

use vsf::VsfType;

#[test]
fn uint_picks_minimal_width() {
    assert!(matches!(VsfType::uint(0), VsfType::u3(0)));
    assert!(matches!(VsfType::uint(255), VsfType::u3(255)));
    assert!(matches!(VsfType::uint(256), VsfType::u4(256)));
    assert!(matches!(VsfType::uint(1996), VsfType::u4(1996)));
    assert!(matches!(VsfType::uint(65_535), VsfType::u4(65_535)));
    assert!(matches!(VsfType::uint(65_536), VsfType::u5(65_536)));
    assert!(matches!(VsfType::uint(4_294_967_295), VsfType::u5(4_294_967_295)));
    assert!(matches!(VsfType::uint(4_294_967_296), VsfType::u6(4_294_967_296)));
    assert!(matches!(VsfType::uint(u64::MAX), VsfType::u6(u64::MAX)));
}
