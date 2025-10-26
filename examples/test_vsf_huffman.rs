use vsf::types::VsfType;

fn main() {
    println!("Testing VSF Huffman text encoding...\n");

    // Test various strings
    let test_strings = vec![
        "hello",
        "The quick brown fox jumps over the lazy dog",
        "café",
        "你好世界",
        "مرحبا بالعالم",
        "🌍🌎🌏",
    ];

    for text in test_strings {
        println!("Testing: {:?}", text);

        // Create VSF x type
        let vsf = VsfType::x(text.to_string());

        // Flatten to bytes
        let flattened = vsf.flatten();
        println!("  Flattened: {} bytes (UTF-8 would be {} bytes)",
                 flattened.len(), text.len());

        // Parse back
        let mut pointer = 0;
        let decoded = vsf::decoding::parse::parse(&flattened, &mut pointer)
            .expect("Decode failed");

        // Verify round-trip
        match decoded {
            VsfType::x(decoded_text) => {
                assert_eq!(decoded_text, text, "Round-trip failed!");
                println!("  ✓ Round-trip successful\n");
            }
            _ => panic!("Expected VsfType::x, got something else"),
        }
    }

    println!("✓ All VSF Huffman tests passed!");
}
