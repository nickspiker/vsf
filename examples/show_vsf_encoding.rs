use vsf::VsfType;

fn main() {
    // Encode individual letters
    let v_encoded = VsfType::x("v".to_string()).flatten();
    let s_encoded = VsfType::x("s".to_string()).flatten();
    let f_encoded = VsfType::x("f".to_string()).flatten();

    println!("VSF Letter Encoding (Huffman compressed):");
    println!();
    println!("Letter 'v': {} bytes total", v_encoded.len());
    println!("  Hex: {}", hex::encode(&v_encoded));
    println!();
    println!("Letter 's': {} bytes total", s_encoded.len());
    println!("  Hex: {}", hex::encode(&s_encoded));
    println!();
    println!("Letter 'f': {} bytes total", f_encoded.len());
    println!("  Hex: {}", hex::encode(&f_encoded));
    println!();

    // Encode the word "vsf"
    let vsf_encoded = VsfType::x("vsf".to_string()).flatten();
    println!("Word 'vsf': {} bytes total", vsf_encoded.len());
    println!("  Hex: {}", hex::encode(&vsf_encoded));
}
