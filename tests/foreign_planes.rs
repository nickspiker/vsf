//! A plane can keep a source's samples exactly as written (format v12): `primaries` and a source `encoding` say what the codes mean, `prior` says what was done before the file, and the graph's Technical `decode` op takes the plane into VSF RGB. `idt/fields.md`, "Untouched sources and prior".

use vsf::spectral_image::{IdtClass, PlaneLayout, SpectralChannel};
use vsf::types::BitPackedTensor;
use vsf::visual::{Encoding, Plane, Primaries, Visual, OP_DECODE};

/// A 2 × 1 sRGB JPEG's worth of samples, planar `[3, h, w]`: a white pixel and a pure red one.
fn srgb_plane() -> Plane {
    let codes: Vec<u8> = vec![255, 255, /* G */ 255, 0, /* B */ 255, 0];
    Plane::encoded("b0", "s0", 2, 1, Primaries::Srgb, Encoding::Srgb, BitPackedTensor::pack(8, vec![3, 1, 2], &codes))
}

#[test]
fn every_encodings_white_is_one_and_vsf_quantisation_is_unchanged() {
    for e in [Encoding::Srgb, Encoding::Bt1886, Encoding::AdobeRgb, Encoding::Prophoto] {
        assert!((e.light(255, 8) - 1.).abs() < 1e-6, "{} white", e.as_str());
        assert_eq!(e.light(0, 8), 0., "{} black", e.as_str());
        assert!(!e.is_vsf());
    }
    // VSF's own: ×2ⁿ, bucket bottom, exactly as before.
    assert_eq!(Encoding::Gamma2.light(128, 8), 0.25);
    assert_eq!(Encoding::Linear.light(128, 8), 0.5);
    // HDR signals run above white: PQ's top is 10 000 cd/m² over BT.2408's 203.
    assert!((Encoding::Pq.light(1023, 10) - 10_000. / 203.).abs() < 0.1);
    assert!(Encoding::Hlg.light(1023, 10) > 3.7);
    assert!(Encoding::Gamma2Tanh.light_f32(1.).is_finite());
}

#[test]
fn every_primaries_white_lands_on_vsf_white() {
    for p in [Primaries::Srgb, Primaries::DisplayP3, Primaries::AdobeRgb, Primaries::Prophoto, Primaries::Rec2020] {
        let m = p.to_vsf_rgb().unwrap();
        let w = vsf::colour::convert::apply_matrix_3x3_f32(&m, &[1., 1., 1.]);
        assert!(w.iter().all(|v| (v - 1.).abs() < 1e-5), "{} white → {w:?}", p.as_str());
    }
    assert!(Primaries::Vsf.to_vsf_rgb().is_none());
}

#[test]
fn an_encoded_source_round_trips_untouched_with_a_decode_op() {
    let plane = srgb_plane();
    assert_eq!(plane.prior, Some(IdtClass::Creative), "a display-encoded source starts as rendered");
    let bytes = Visual::still(plane.clone(), None, None, &[]).write().unwrap();
    let back = Visual::read(&bytes).unwrap();
    let p = back.primary_plane().unwrap();
    assert_eq!((p.primaries, p.encoding, p.prior), (Primaries::Srgb, Encoding::Srgb, Some(IdtClass::Creative)));
    assert_eq!(p.samples, plane.samples, "the source's codes are kept exactly");
    assert_eq!((p.black.clone(), p.white.clone()), (vec![0.; 3], vec![255.; 3]));

    let g = back.default_graph().unwrap();
    let names: Vec<&str> = g.ops.iter().map(|o| o.name.as_str()).collect();
    assert_eq!(names, ["plane", "black_white", "decode"]);
    assert_eq!(g.ops[2].class, IdtClass::Technical);
    assert!(g.to_view_transform().ops.iter().all(|o| o.name != OP_DECODE), "decode is prologue, not a view op");
}

#[test]
fn the_bridge_decodes_into_vsf_rgb() {
    let img = Visual::still(srgb_plane(), None, None, &[]).to_spectral_image().unwrap();
    assert!(img.profile.is_none(), "an encoding is not a characterization");
    let c = img.samples.unpack_u16();
    let white = img.white[0];
    // Pixel 0, sRGB white, is VSF white in every channel.
    for ch in 0..3 {
        assert!((c[ch * 2] as f32 / white - 1.).abs() < 1e-3, "white channel {ch}: {}", c[ch * 2] as f32 / white);
    }
    // Pixel 1, sRGB red, is the first column of the sRGB → VSF RGB matrix.
    let m = Primaries::Srgb.to_vsf_rgb().unwrap();
    for ch in 0..3 {
        let got = c[ch * 2 + 1] as f32 / white;
        assert!((got - m[ch]).abs() < 1e-3, "red channel {ch}: {got} vs {}", m[ch]);
    }
}

#[test]
fn a_plane_that_needs_neither_field_writes_neither() {
    let p = Plane::sensor("b0", "s0", 2, 1, vec![SpectralChannel { name: "Y".into(), curve: None }], PlaneLayout::Planar, vec![0.], vec![255.], BitPackedTensor::pack(8, vec![1, 1, 2], &[1u8, 2]));
    let bytes = Visual::still(p, None, None, &[]).write().unwrap();
    let back = Visual::read(&bytes).unwrap();
    let p = back.primary_plane().unwrap();
    assert_eq!((p.primaries, p.prior), (Primaries::Vsf, None));
    let doc = vsf::container::Document::open(bytes.as_slice()).unwrap();
    let entry = doc.entry("plane.b0.s0.sensor").unwrap();
    let s = doc.section(entry).unwrap();
    assert!(s.get_field("primaries").is_none() && s.get_field("prior").is_none());
    assert!(!back.default_graph().unwrap().ops.iter().any(|o| o.name == OP_DECODE));
}

#[test]
fn foreign_primaries_need_planar_rgb() {
    let mut p = Plane::sensor("b0", "s0", 2, 1, vec![SpectralChannel { name: "Y".into(), curve: None }], PlaneLayout::Planar, vec![0.], vec![255.], BitPackedTensor::pack(8, vec![1, 1, 2], &[1u8, 2]));
    p.primaries = Primaries::Srgb;
    assert!(Visual::still(p, None, None, &[]).write().is_err());
}

#[test]
fn unknown_values_fail_loud() {
    assert!(Primaries::parse("srgb_linear").is_err());
    assert!(Encoding::parse("gamma22").is_err());
}

#[test]
fn the_flat_image_section_stays_vsf_rgb() {
    assert!(vsf::builders::compressed_image_encoded(vec![0; 4], Encoding::Srgb).is_err());
    assert!(vsf::builders::compressed_image_encoded(vec![0; 4], Encoding::Gamma2Tanh).is_ok());
}

#[test]
fn a_pq_source_keeps_its_highlights_above_white() {
    // 10-bit PQ: code 594 ≈ 203 cd/m² (diffuse white, signal 0.58069 × 1023), 1023 = 10 000 cd/m².
    let codes: Vec<u16> = vec![594, 1023, 594, 1023, 594, 1023];
    let p = Plane::encoded("b0", "s0", 2, 1, Primaries::Rec2020, Encoding::Pq, BitPackedTensor::pack(10, vec![3, 1, 2], &codes));
    let (t, _, white) = p.linear_samples().unwrap();
    let c = t.unpack_u16();
    assert_eq!(c[1], 65535, "the brightest light takes the top count");
    assert!((c[0] as f32 / white[0] - 1.).abs() < 0.01, "diffuse white stays at white: {}", c[0] as f32 / white[0]);
}
