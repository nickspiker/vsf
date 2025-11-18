use vsf::verification::sign_section;
use vsf::file_format::{VsfHeader, HeaderField};
use vsf::{VsfType, VSF_VERSION, VSF_BACKWARD_COMPAT};
use ed25519_dalek::{SigningKey, SECRET_KEY_LENGTH};

#[test]
fn test_sign_section_writes_real_signature() {
    // Generate a test signing key
    let secret_bytes = [42u8; SECRET_KEY_LENGTH];
    let signing_key = SigningKey::from_bytes(&secret_bytes);

    // Build a simple VSF file with a section
    let mut header = VsfHeader::new(VSF_VERSION, VSF_BACKWARD_COMPAT);

    let field = HeaderField {
        name: "test".to_string(),
        hash: None,
        signature: None,
        key: Some(VsfType::ke(signing_key.verifying_key().to_bytes().to_vec())),
        wrap: None,
        offset_bytes: 0,
        size_bytes: 0,
        child_count: 0,
    };
    header.add_field(field);

    let mut vsf_bytes = header.encode().unwrap();
    vsf::file_format::VsfHeader::update_header_length(&mut vsf_bytes).unwrap();

    // Add section data
    let section_bytes = b"[d3.testx4.data]";
    vsf_bytes.extend_from_slice(section_bytes);

    // Sign the section
    eprintln!("\n=== Testing sign_section ===");
    let signed_vsf = sign_section(vsf_bytes, "test", signing_key.as_bytes()).unwrap();

    eprintln!("\n=== Checking result ===");
    // Find the 'ge' marker in the signed VSF
    let mut found_signature = false;
    for i in 0..signed_vsf.len()-1 {
        if signed_vsf[i] == b'g' && signed_vsf[i+1] == b'e' {
            eprintln!("Found 'ge' marker at offset 0x{:X}", i);
            if i+4 < signed_vsf.len() {
                eprintln!("  Encoding bytes: 0x{:02X} 0x{:02X}", signed_vsf[i+2], signed_vsf[i+3]);
                let sig_start = i + 4;
                if sig_start + 64 <= signed_vsf.len() {
                    let sig_bytes = &signed_vsf[sig_start..sig_start+64];
                    let all_zeros = sig_bytes.iter().all(|&b| b == 0);
                    eprintln!("  Signature: all zeros = {}", all_zeros);
                    eprintln!("  First 8 bytes: {:02X?}", &sig_bytes[0..8]);
                    eprintln!("  Last 8 bytes: {:02X?}", &sig_bytes[56..64]);

                    assert!(!all_zeros, "Signature should not be all zeros!");
                    found_signature = true;
                }
            }
        }
    }

    assert!(found_signature, "Should have found a signature in the signed VSF");
}
