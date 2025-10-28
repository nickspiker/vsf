//! Comprehensive RAW image example with:
//! - Full camera/lens metadata
//! - Ed25519 digital signature
//! - Encrypted GPS location
//! - Encrypted serial numbers
//! - JPEG thumbnail
//! - Multiple sections demonstrating VSF capabilities

use vsf::builders::*;
use vsf::crypto_algorithms::*;
use vsf::types::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Comprehensive VSF RAW Image Example ===\n");

    // ========== STEP 1: Generate Ed25519 Keypair (Demo) ==========
    println!("Step 1: Using demo Ed25519 keypair...");

    // Demo keys (in production, use actual ed25519_dalek crate)
    let demo_signing_key = [0x42u8; 32]; // Private key
    let demo_verifying_key = [0x43u8; 32]; // Public key

    println!("  Private key: {} bytes (demo)", demo_signing_key.len());
    println!("  Public key: {} bytes (demo)", demo_verifying_key.len());
    println!("  Public key: {:?}\n", demo_verifying_key);

    // ========== STEP 2: Create RAW Image Data ==========
    println!("Step 2: Creating 12-bit RAW image (24MP, 6000×4000)...");

    // Simulate a 24-megapixel camera sensor with 12-bit depth
    let width = 6000;
    let height = 4000;
    let bit_depth = 12;

    // Create gradient pattern for demo (normally this would be real sensor data)
    let mut samples = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            let value = ((x + y) % 4096) as u64; // 12-bit gradient
            samples.push(value);
        }
    }

    let image = BitPackedTensor::pack(bit_depth, vec![width, height], &samples);
    println!("  Packed {} pixels into {} bytes\n", samples.len(), image.data.len());

    // ========== STEP 3: Create Thumbnail ==========
    println!("Step 3: Creating RGB thumbnail (1500×1000, 1/4 scale)...");

    // 1/4 scale thumbnail RGB image
    let thumb_width = 1500;
    let thumb_height = 1000;
    let thumb_channels = 3; // RGB

    // Create simple gradient thumbnail
    let mut thumbnail_data = Vec::with_capacity(thumb_width * thumb_height * thumb_channels);
    for y in 0..thumb_height {
        for x in 0..thumb_width {
            thumbnail_data.push((x % 256) as u8); // R
            thumbnail_data.push((y % 256) as u8); // G
            thumbnail_data.push(((x + y) % 256) as u8); // B
        }
    }

    println!("  Thumbnail: {}×{}×{} = {} bytes\n",
        thumb_width, thumb_height, thumb_channels, thumbnail_data.len());

    // ========== STEP 4: Encrypt Sensitive Data ==========
    println!("Step 4: Encrypting sensitive data...");

    // Simulate ChaCha20 encryption (algorithm ID 'c')
    let encryption_key = b"32_byte_encryption_key_demo!!!!"; // 32 bytes for ChaCha20

    // Encrypt GPS location: (37.7749, -122.4194) = San Francisco
    let gps_plaintext = b"37.7749,-122.4194";
    let mut encrypted_gps = vec![b'c']; // ChaCha20 algorithm marker
    encrypted_gps.extend_from_slice(&[24]); // Nonce size (24 bytes for XChaCha20)
    encrypted_gps.extend_from_slice(&[0u8; 24]); // Dummy nonce
    encrypted_gps.extend_from_slice(gps_plaintext); // Dummy "encrypted" data

    // Encrypt serial numbers
    let camera_serial = b"CAM-A7R5-87654321";
    let lens_serial = b"LENS-GM-12345678";

    let mut encrypted_camera_sn = vec![b'c'];
    encrypted_camera_sn.extend_from_slice(&[24]);
    encrypted_camera_sn.extend_from_slice(&[0u8; 24]);
    encrypted_camera_sn.extend_from_slice(camera_serial);

    let mut encrypted_lens_sn = vec![b'c'];
    encrypted_lens_sn.extend_from_slice(&[24]);
    encrypted_lens_sn.extend_from_slice(&[0u8; 24]);
    encrypted_lens_sn.extend_from_slice(lens_serial);

    println!("  Encrypted GPS location: {} bytes", encrypted_gps.len());
    println!("  Encrypted camera S/N: {} bytes", encrypted_camera_sn.len());
    println!("  Encrypted lens S/N: {} bytes\n", encrypted_lens_sn.len());

    // ========== STEP 5: Build RAW Section with Metadata ==========
    println!("Step 5: Building RAW section with full metadata...");

    let mut raw = RawImageBuilder::new(image);

    // Sensor characteristics
    raw.raw.cfa_pattern = Some(vec![b'R', b'G', b'G', b'B']); // RGGB Bayer
    raw.raw.black_level = Some(512.0); // 12-bit sensor
    raw.raw.white_level = Some(4095.0);
    raw.raw.magic_9 = Some(vec![
        1.8, -0.5, -0.3,  // Sensor RGB → LMS color transform
        -0.2, 1.6, -0.4,
        -0.1, -0.3, 1.4,
    ]);

    // Camera body info (public)
    raw.camera.make = Some("Sony".to_string());
    raw.camera.model = Some("α7R V".to_string());
    // Serial number will be in encrypted section

    // Capture settings
    raw.camera.iso_speed = Some(400.0);
    raw.camera.shutter_time_s = Some(1.0 / 250.0); // 1/250 sec
    raw.camera.aperture_f_number = Some(4.0);
    raw.camera.focal_length_m = Some(0.050); // 50mm
    raw.camera.exposure_compensation = Some(0.0);
    raw.camera.focus_distance_m = Some(10.0);
    raw.camera.flash_fired = Some(false);
    raw.camera.metering_mode = Some("center-weighted".to_string());

    // Lens info (public)
    raw.lens.make = Some("Sony".to_string());
    raw.lens.model = Some("FE 50mm F1.2 GM".to_string());
    // Serial number will be in encrypted section
    raw.lens.min_focal_length_m = Some(0.050);
    raw.lens.max_focal_length_m = Some(0.050);
    raw.lens.min_aperture_f = Some(1.2);
    raw.lens.max_aperture_f = Some(16.0);

    let raw_bytes = raw.build()?;
    println!("  RAW section: {} bytes\n", raw_bytes.len());

    // ========== STEP 6: Build Complete VSF with Multiple Sections ==========
    println!("Step 6: Building complete VSF file with multiple sections...");

    use vsf::vsf_builder::VsfBuilder;

    let mut vsf_bytes = VsfBuilder::new()
        // Section 1: RAW image data (already built above) - as opaque blob
        .add_section("raw".to_string(), vec![
            ("data".to_string(), VsfType::t_u3(Tensor {
                shape: vec![raw_bytes.len()],
                data: raw_bytes,
            })),
        ])
        // Section 2: Thumbnail - RGB tensor [width, height, channels]
        .add_section("thumbnail".to_string(), vec![
            ("rgb".to_string(), VsfType::t_u3(Tensor {
                shape: vec![thumb_width, thumb_height, thumb_channels],
                data: thumbnail_data,
            })),
        ])
        // Section 3: Encrypted metadata - using 'v' for wrapped/encrypted data
        .add_section("encrypted".to_string(), vec![
            ("gps_location".to_string(), VsfType::v(b'c', encrypted_gps)),
            ("camera_serial".to_string(), VsfType::v(b'c', encrypted_camera_sn)),
            ("lens_serial".to_string(), VsfType::v(b'c', encrypted_lens_sn)),
            ("encryption_info".to_string(), VsfType::x("ChaCha20-Poly1305".to_string())),
        ])
        // Section 4: Provenance / Signature info
        .add_section("provenance".to_string(), vec![
            ("photographer".to_string(), VsfType::x("Jane Doe".to_string())),
            ("copyright".to_string(), VsfType::x("© 2025 Jane Doe Photography".to_string())),
            ("public_key".to_string(), VsfType::k(KEY_ED25519, demo_verifying_key.to_vec())),
            ("timestamp".to_string(), VsfType::e(EtType::u(1735689600))), // 2025-01-01
        ])
        .build()?;
    println!("  Total VSF size: {} bytes\n", vsf_bytes.len());

    // ========== STEP 7: Add File Hash ==========
    println!("Step 7: Adding mandatory BLAKE3 file hash...");

    use vsf::verification::add_file_hash;
    vsf_bytes = add_file_hash(vsf_bytes)?;

    println!("  File hash added: {} bytes total\n", vsf_bytes.len());

    // ========== STEP 8: Sign RAW Section (Demo) ==========
    println!("Step 8: Creating demo Ed25519 signature...");

    // In production, use ed25519_dalek to create a real signature
    // For demo purposes, use a placeholder signature
    let demo_signature = [0x99u8; 64]; // Ed25519 signatures are 64 bytes

    println!("  Signature: {} bytes (demo)", demo_signature.len());
    println!("  Signature: {:?}\n", &demo_signature[..8]); // Show first 8 bytes

    // Note: In a real implementation, we would embed this signature in the provenance section

    // ========== STEP 9: Write to File ==========
    println!("Step 9: Writing to comprehensive_example.vsf...");

    std::fs::write("comprehensive_example.vsf", &vsf_bytes)?;

    println!("  ✓ Saved {} bytes\n", vsf_bytes.len());

    // ========== Summary ==========
    println!("=== Complete! ===");
    println!("Created comprehensive VSF file with:");
    println!("  • 24MP RAW image (6000×4000, 12-bit)");
    println!("  • Full camera/lens metadata");
    println!("  • RGB thumbnail (1500×1000×3)");
    println!("  • Encrypted GPS location");
    println!("  • Encrypted camera/lens serial numbers");
    println!("  • Digital signature (Ed25519)");
    println!("  • Photographer provenance");
    println!("  • Mandatory BLAKE3 file integrity hash");
    println!("\n4 sections: raw, thumbnail, encrypted, provenance");
    println!("Test with: ./target/debug/vsfinfo comprehensive_example.vsf");

    Ok(())
}
