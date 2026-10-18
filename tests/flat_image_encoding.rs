//! The flat `image` section's `encoding` tag: untagged is VSF RGB gamma 2 and writes nothing; `g2t` writes the tag, parses back, and decodes for the display instead of being shown as gamma 2.

use vsf::builders::{compressed_image, compressed_image_encoded, parse_compressed_image};
use vsf::types::{Tensor, VsfType};
use vsf::visual::Encoding;
use vsf::VsfBuilder;

#[test]
fn gamma2_writes_no_tag_and_g2t_round_trips() {
    let plain = compressed_image(vec![1, 2, 3]).unwrap();
    assert_eq!(plain, compressed_image_encoded(vec![1, 2, 3], Encoding::Gamma2).unwrap());
    assert_eq!(parse_compressed_image(&plain).unwrap().colour_encoding, Encoding::Gamma2);
    let tagged = compressed_image_encoded(vec![1, 2, 3], Encoding::Gamma2Tanh).unwrap();
    let parsed = parse_compressed_image(&tagged).unwrap();
    assert_eq!(parsed.colour_encoding, Encoding::Gamma2Tanh);
    assert_eq!(parsed.data, vec![1, 2, 3]);
    assert!(tagged.windows(3).any(|w| w == b"g2t"));
}

#[cfg(feature = "image-decode")]
fn tensor_image(codes: &[u8], tag: Option<&str>) -> Vec<u8> {
    let mut data = Vec::new();
    for &c in codes {
        data.extend_from_slice(&[c, c, c]);
    }
    let mut fields = vec![("data".to_string(), VsfType::t_u3(Tensor::new(vec![1, codes.len(), 3], data)))];
    if let Some(t) = tag {
        fields.push(("encoding".to_string(), VsfType::a(t.into())));
    }
    VsfBuilder::new().provenance_only().add_section("image", fields).build().unwrap()
}

#[cfg(feature = "image-decode")]
fn visible(px: u32) -> u32 {
    255 - (px & 0xFF)
}

#[cfg(feature = "image-decode")]
#[test]
fn untagged_decodes_unchanged_and_g2t_is_mapped_for_the_display() {
    let codes = [0u8, 40, 120, 194, 255];
    let plain = vsf::image::decode(&tensor_image(&codes, None)).unwrap();
    assert_eq!(plain.encoding, Encoding::Gamma2);
    assert_eq!(plain.pixels.iter().map(|&p| visible(p)).collect::<Vec<_>>(), codes.iter().map(|&c| c as u32).collect::<Vec<_>>());

    let hdr = vsf::image::decode(&tensor_image(&codes, Some("g2t"))).unwrap();
    assert_eq!(hdr.encoding, Encoding::Gamma2Tanh);
    let shown: Vec<u32> = hdr.pixels.iter().map(|&p| visible(p)).collect();
    assert_eq!(shown[0], 0);
    assert_eq!(shown[4], 255, "the image's brightest code reaches the display's peak");
    assert!(shown[3] > 194 && shown[3] < 250, "diffuse white sits below peak, brighter than its raw code: {}", shown[3]);
    assert!(shown.windows(2).all(|w| w[0] <= w[1]));
    assert!((shown[1] as i32 - 40).abs() <= 1, "shadows are gamma 2: {}", shown[1]);

    // On a display with room for the whole image, nothing is compressed: white sits at 1/headroom of peak.
    let roomy = vsf::image::decode_for_display(&tensor_image(&codes, Some("gamma2tanh")), 16.).unwrap();
    let white = visible(roomy.pixels[3]) as f32 / 256.;
    assert!((white * white - 1. / 16.).abs() < 0.01, "white at {}", white * white);
}
