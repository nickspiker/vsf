//! Tests for Toka Tree (Loom) type encoding/decoding with vt wrapping

#[cfg(test)]
#[cfg(feature = "spirix")]
mod tests {
    use crate::decoding::toka_tree::parse_vt_toka_node;
    use crate::types::{
        ButtonVariant, PathCommand, TokaBox, TokaButton, TokaCircle, TokaGroup, TokaImage,
        TokaLine, TokaNode, TokaPath, TokaSurface, TokaText, VsfType,
    };
    use spirix::{CircleF4E4, ScalarF4E4};

    /// Helper to create a test CircleF4E4
    fn test_circle(r: i16, i: i16, e: i16) -> CircleF4E4 {
        CircleF4E4 {
            real: r,
            imaginary: i,
            exponent: e,
        }
    }

    /// Helper to create a test ScalarF4E4
    fn test_scalar(f: i16, e: i16) -> ScalarF4E4 {
        ScalarF4E4 {
            fraction: f,
            exponent: e,
        }
    }

    #[test]
    fn test_box_vt_roundtrip() {
        let node = TokaNode::Box(TokaBox {
            pos: test_circle(100, 200, 0),
            size: test_circle(300, 400, 0),
            colour: test_circle(255, 0, 0),
        });

        // Encode to vt wrapped VsfType
        let vsf_type = node.to_vsf_type();

        // Flatten to bytes
        let encoded = vsf_type.flatten();

        // Parse back
        let mut pointer = 0;
        let parsed_vsf = crate::decoding::parse(&encoded, &mut pointer).expect("Failed to parse");
        let decoded_node = parse_vt_toka_node(&parsed_vsf).expect("Failed to decode vt");

        // Re-encode and compare bytes
        let re_encoded = decoded_node.to_vsf_type().flatten();
        assert_eq!(encoded, re_encoded);
    }

    #[test]
    fn test_circle_vt_roundtrip() {
        let node = TokaNode::Circle(TokaCircle {
            pos: test_circle(500, 500, 0),
            span: test_scalar(200, 0),
            colour: test_circle(0, 0, 255),
        });

        let vsf_type = node.to_vsf_type();
        let encoded = vsf_type.flatten();

        let mut pointer = 0;
        let parsed_vsf = crate::decoding::parse(&encoded, &mut pointer).expect("Failed to parse");
        let decoded_node = parse_vt_toka_node(&parsed_vsf).expect("Failed to decode vt");

        let re_encoded = decoded_node.to_vsf_type().flatten();
        assert_eq!(encoded, re_encoded);
    }

    #[test]
    fn test_line_vt_roundtrip() {
        let node = TokaNode::Line(TokaLine {
            start: test_circle(0, 0, 0),
            end: test_circle(1000, 1000, 0),
            width: test_scalar(10, 0),
            colour: test_circle(128, 128, 128),
        });

        let vsf_type = node.to_vsf_type();
        let encoded = vsf_type.flatten();

        let mut pointer = 0;
        let parsed_vsf = crate::decoding::parse(&encoded, &mut pointer).expect("Failed to parse");
        let decoded_node = parse_vt_toka_node(&parsed_vsf).expect("Failed to decode vt");

        let re_encoded = decoded_node.to_vsf_type().flatten();
        assert_eq!(encoded, re_encoded);
    }

    #[test]
    fn test_text_vt_roundtrip() {
        let node = TokaNode::Text(TokaText {
            pos: test_circle(10, 10, 0),
            size: test_circle(200, 50, 0),
            content: "Hello, Loom!".to_string(),
            colour: test_circle(0, 0, 0),
        });

        let vsf_type = node.to_vsf_type();
        let encoded = vsf_type.flatten();

        let mut pointer = 0;
        let parsed_vsf = crate::decoding::parse(&encoded, &mut pointer).expect("Failed to parse");
        let decoded_node = parse_vt_toka_node(&parsed_vsf).expect("Failed to decode vt");

        let re_encoded = decoded_node.to_vsf_type().flatten();
        assert_eq!(encoded, re_encoded);
    }

    #[test]
    fn test_button_vt_roundtrip() {
        let node = TokaNode::Button(TokaButton {
            pos: test_circle(50, 50, 0),
            size: test_circle(100, 30, 0),
            label: "Click Me".to_string(),
            variant: ButtonVariant::Filled,
            colour: test_circle(0, 128, 255),
        });

        let vsf_type = node.to_vsf_type();
        let encoded = vsf_type.flatten();

        let mut pointer = 0;
        let parsed_vsf = crate::decoding::parse(&encoded, &mut pointer).expect("Failed to parse");
        let decoded_node = parse_vt_toka_node(&parsed_vsf).expect("Failed to decode vt");

        let re_encoded = decoded_node.to_vsf_type().flatten();
        assert_eq!(encoded, re_encoded);
    }

    #[test]
    fn test_path_vt_roundtrip() {
        let node = TokaNode::Path(TokaPath {
            colour: test_circle(255, 0, 0),
            width: test_scalar(5, 0),
            commands: vec![
                PathCommand::MoveTo(test_circle(0, 0, 0)),
                PathCommand::LineTo(test_circle(100, 0, 0)),
                PathCommand::LineTo(test_circle(100, 100, 0)),
                PathCommand::Close,
            ],
        });

        let vsf_type = node.to_vsf_type();
        let encoded = vsf_type.flatten();

        let mut pointer = 0;
        let parsed_vsf = crate::decoding::parse(&encoded, &mut pointer).expect("Failed to parse");
        let decoded_node = parse_vt_toka_node(&parsed_vsf).expect("Failed to decode vt");

        let re_encoded = decoded_node.to_vsf_type().flatten();
        assert_eq!(encoded, re_encoded);
    }

    #[test]
    fn test_image_vt_roundtrip() {
        let node = TokaNode::Image(TokaImage {
            pos: test_circle(0, 0, 0),
            size: test_circle(640, 480, 0),
            handle: 0x123456789ABCDEF0,
            tint: test_circle(255, 255, 255),
        });

        let vsf_type = node.to_vsf_type();
        let encoded = vsf_type.flatten();

        let mut pointer = 0;
        let parsed_vsf = crate::decoding::parse(&encoded, &mut pointer).expect("Failed to parse");
        let decoded_node = parse_vt_toka_node(&parsed_vsf).expect("Failed to decode vt");

        let re_encoded = decoded_node.to_vsf_type().flatten();
        assert_eq!(encoded, re_encoded);
    }

    #[test]
    fn test_surface_vt_roundtrip() {
        let node = TokaNode::Surface(TokaSurface {
            pos: test_circle(0, 0, 0),
            size: test_circle(1920, 1080, 0),
            handle: 0xFEDCBA9876543210,
        });

        let vsf_type = node.to_vsf_type();
        let encoded = vsf_type.flatten();

        let mut pointer = 0;
        let parsed_vsf = crate::decoding::parse(&encoded, &mut pointer).expect("Failed to parse");
        let decoded_node = parse_vt_toka_node(&parsed_vsf).expect("Failed to decode vt");

        let re_encoded = decoded_node.to_vsf_type().flatten();
        assert_eq!(encoded, re_encoded);
    }

    #[test]
    fn test_group_vt_roundtrip() {
        let node = TokaNode::Group(TokaGroup {
            pos: test_circle(0, 0, 0),
            size: test_circle(1000, 1000, 0),
            children: vec![
                TokaNode::Box(TokaBox {
                    pos: test_circle(10, 10, 0),
                    size: test_circle(100, 100, 0),
                    colour: test_circle(255, 0, 0),
                }),
                TokaNode::Circle(TokaCircle {
                    pos: test_circle(500, 500, 0),
                    span: test_scalar(50, 0),
                    colour: test_circle(0, 255, 0),
                }),
            ],
        });

        let vsf_type = node.to_vsf_type();
        let encoded = vsf_type.flatten();

        let mut pointer = 0;
        let parsed_vsf = crate::decoding::parse(&encoded, &mut pointer).expect("Failed to parse");
        let decoded_node = parse_vt_toka_node(&parsed_vsf).expect("Failed to decode vt");

        let re_encoded = decoded_node.to_vsf_type().flatten();
        assert_eq!(encoded, re_encoded);
    }
}
