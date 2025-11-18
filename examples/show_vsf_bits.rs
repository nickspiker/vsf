use std::fs::File;
use std::io::Read;

fn main() {
    // Read Huffman codes file
    let mut file = File::open("huffman_codes.bin").expect("huffman_codes.bin not found");
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).unwrap();

    // Parse header
    if &bytes[0..4] != b"HUFF" {
        panic!("Invalid Huffman file");
    }

    let count = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;

    // Parse entries and find v, s, f
    let mut chars_found = std::collections::HashMap::new();

    for i in 0..count {
        let offset = 12 + i * 8;
        let codepoint = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        let packed = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap());

        let bits = packed & 0x00FFFFFF;
        let length = (packed >> 24) as u8;

        if let Some(ch) = char::from_u32(codepoint) {
            if ch == 'v' || ch == 's' || ch == 'f' {
                chars_found.insert(ch, (bits, length));
            }
        }
    }

    println!("VSF Huffman Encoding:");
    println!();

    for ch in ['v', 's', 'f'] {
        if let Some((bits, length)) = chars_found.get(&ch) {
            let binary = format!("{:0width$b}", bits, width = *length as usize);
            println!("'{}': {} bits - binary: {}", ch, length, binary);
        }
    }

    println!();
    println!(
        "Total: 'vsf' would use {} bits ({} bytes with overhead)",
        chars_found.get(&'v').unwrap().1
            + chars_found.get(&'s').unwrap().1
            + chars_found.get(&'f').unwrap().1,
        3
    );
}
