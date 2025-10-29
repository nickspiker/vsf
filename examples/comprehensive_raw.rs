//! Comprehensive RAW image example with:
//! - Full camera/lens metadata
//! - Ed25519 digital signature
//! - Encrypted GPS location
//! - Encrypted serial numbers
//! - JPEG thumbnail
//! - Multiple sections demonstrating VSF capabilities

use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng as AeadRng},
    ChaCha20Poly1305, Nonce,
};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use vsf::builders::*;
use vsf::crypto_algorithms::*;
use vsf::types::*;
use vsf::verification::{add_encryption_metadata, sign_section};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Comprehensive VSF RAW Image Example ===\n");

    // ========== STEP 1: Generate Ed25519 Keypair (Real) ==========
    println!("Step 1: Generating Ed25519 keypair...");

    // Generate real Ed25519 keypair
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    println!("  Private key: 32 bytes (secure random)");
    println!("  Public key: {} bytes", verifying_key.as_bytes().len());
    println!("  Public key: {:02X?}\n", &verifying_key.as_bytes()[..8]);

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
    println!(
        "  Packed {} pixels into {} bytes\n",
        samples.len(),
        image.data.len()
    );

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

    println!(
        "  Thumbnail: {}×{}×{} = {} bytes\n",
        thumb_width,
        thumb_height,
        thumb_channels,
        thumbnail_data.len()
    );

    // ========== STEP 4: Build Encrypted Blob Section (before encryption) ==========
    println!("Step 4: Building encrypted blob section with structured data...");

    // Build the blob section as a normal VSF section with structure
    use vsf::file_format::{VsfItem, VsfSection};
    let blob_section = VsfSection {
        name: "blob".to_string(),
        items: vec![
            VsfItem {
                name: "gps_location".to_string(),
                value: VsfType::x("37.7749,-122.4194".to_string()),
            },
            VsfItem {
                name: "camera_serial".to_string(),
                value: VsfType::x("CAM-A7R5-87654321".to_string()),
            },
            VsfItem {
                name: "lens_serial".to_string(),
                value: VsfType::x("LENS-GM-12345678".to_string()),
            },
        ],
    };

    // Encode the entire section: [d"blob" (d"gps_location":"...") (d"camera_serial":"...") (d"lens_serial":"...")]
    let blob_plaintext = blob_section.encode();

    println!("  Plaintext VSF section: {} bytes", blob_plaintext.len());
    println!("  Contains: GPS location, camera serial, lens serial (all structured)\n");

    // ========== STEP 5: Encrypt Entire Section ==========
    println!("Step 5: Encrypting entire VSF section with ChaCha20-Poly1305...");

    // Generate encryption key
    let cipher = ChaCha20Poly1305::generate_key(&mut AeadRng);
    let chacha = ChaCha20Poly1305::new(&cipher);
    let nonce = Nonce::from_slice(b"unique nonce"); // 12 bytes for ChaCha20Poly1305

    println!("  Encryption key: 32 bytes (ChaCha20-Poly1305)");
    println!("  Nonce: 12 bytes");

    // Encrypt the ENTIRE section bytes
    let encrypted_blob = chacha
        .encrypt(nonce, blob_plaintext.as_ref())
        .map_err(|e| format!("Encryption failed: {}", e))?;

    println!(
        "  Encrypted blob: {} bytes (plaintext {} + 16-byte Poly1305 tag)\n",
        encrypted_blob.len(),
        blob_plaintext.len()
    );

    // ========== STEP 6: Prepare RAW image metadata ==========
    println!("Step 6: Preparing RAW image metadata...\n");

    // ========== STEP 7: Build Complete VSF with Multiple Sections ==========
    println!("Step 7: Building complete VSF file with multiple sections...");

    use vsf::vsf_builder::VsfBuilder;

    let mut vsf_bytes = VsfBuilder::new()
        // Section 1: RAW image data as bit-packed tensor
        .add_section(
            "raw".to_string(),
            vec![("image".to_string(), VsfType::p(image))],
        )
        // Section 2: Thumbnail - RGB tensor [width, height, channels]
        .add_section(
            "thumbnail".to_string(),
            vec![(
                "rgb".to_string(),
                VsfType::t_u3(Tensor {
                    shape: vec![thumb_width, thumb_height, thumb_channels],
                    data: thumbnail_data,
                }),
            )],
        )
        // Section 3: Camera metadata - comprehensive shooting info
        .add_section(
            "metadata".to_string(),
            vec![
                ("camera_make".to_string(), VsfType::x("Sony".to_string())),
                (
                    "camera_model".to_string(),
                    VsfType::x("ILCE-7RM5".to_string()),
                ),
                ("lens_make".to_string(), VsfType::x("Sony".to_string())),
                (
                    "lens_model".to_string(),
                    VsfType::x("FE 24-70mm F2.8 GM II".to_string()),
                ),
                ("focal_length".to_string(), VsfType::u3(50)), // 50mm
                ("aperture".to_string(), VsfType::f5(2.8)),
                ("shutter_speed".to_string(), VsfType::x("1/250".to_string())),
                ("iso".to_string(), VsfType::u4(400)),
                ("white_balance".to_string(), VsfType::u4(5600)), // 5600K
                (
                    "color_space".to_string(),
                    VsfType::x("Adobe RGB".to_string()),
                ),
                ("bit_depth".to_string(), VsfType::u3(14)), // 14-bit ADC
            ],
        )
        // Section 4: Encrypted blob - ENCRYPTED VSF SECTION as n[0] unboxed blob
        // The encrypted_blob contains: [d"blob" (gps) (camera) (lens)]
        // Stored as raw bytes with n[0] (no structure until decrypted)
        .add_unboxed("blob", encrypted_blob)
        // Section 5: Provenance info (no signature here - signature goes in header!)
        .add_section(
            "provenance".to_string(),
            vec![
                (
                    "photographer".to_string(),
                    VsfType::x("Jane Doe".to_string()),
                ),
                (
                    "copyright".to_string(),
                    VsfType::x("© 2025 Jane Doe Photography".to_string()),
                ),
                ("timestamp".to_string(), VsfType::e(EtType::u(1735689600))), // 2025-01-01
                (
                    "software".to_string(),
                    VsfType::x("VSF Creator v0.1.3".to_string()),
                ),
            ],
        )
        .build()?;
    println!("  Total VSF size: {} bytes\n", vsf_bytes.len());

    // ========== STEP 8: Sign RAW Image Section (Real Ed25519) ==========
    println!("Step 8: Signing raw image section with Ed25519...");

    vsf_bytes = sign_section(vsf_bytes, "raw", signing_key.as_bytes())?;

    println!("  RAW image section signed with Ed25519");
    println!("  Signature: 64 bytes embedded in header");
    println!("  Ensures image data integrity and authenticity\n");

    // ========== STEP 9: Sign Provenance Section (Real Ed25519) ==========
    println!("Step 9: Signing provenance section with Ed25519...");

    vsf_bytes = sign_section(vsf_bytes, "provenance", signing_key.as_bytes())?;

    println!("  Provenance section signed with Ed25519");
    println!("  Signature: 64 bytes embedded in header");
    println!(
        "  Public key for verification: {}\n",
        hex::encode(verifying_key.as_bytes())
    );

    // ========== STEP 10: Add Encryption Metadata to Blob Section ==========
    println!("Step 10: Adding encryption metadata to blob section header...");

    vsf_bytes = add_encryption_metadata(vsf_bytes, "blob", WRAP_CHACHA20POLY1305, cipher.as_ref())?;

    println!("  Encryption algorithm: ChaCha20-Poly1305");
    println!("  Encryption key: 32 bytes embedded in header\n");

    // ========== STEP 11: Write to File ==========
    println!("Step 11: Writing to comprehensive_example.vsf...");

    std::fs::write("comprehensive_example.vsf", &vsf_bytes)?;

    println!("  ✓ Saved {} bytes\n", vsf_bytes.len());

    // ========== Summary ==========
    println!("=== Complete! ===");
    println!("Created comprehensive VSF file with:");
    println!("  • 24MP RAW image (6000×4000, 12-bit) - SIGNED");
    println!("  • Full camera/lens metadata (11 fields)");
    println!("  • RGB thumbnail (1500×1000×3)");
    println!("  • Encrypted blob (GPS, serial numbers) - ChaCha20-Poly1305");
    println!("  • Photographer provenance - SIGNED with Ed25519");
    println!("  • Mandatory BLAKE3 file integrity hash");
    println!(
        "\n5 sections: raw (signed), thumbnail, metadata, blob (encrypted), provenance (signed)"
    );
    println!("\nCrypto architecture:");
    println!("  • RAW image signed: (d\"raw\" sig[Ed25519 64 bytes] ...)");
    println!("  • Provenance signed: (d\"provenance\" sig[Ed25519 64 bytes] ...)");
    println!("  • Blob encrypted: (d\"blob\" key[ChaCha20-Poly1305] wrap[...] ...)");
    println!("  • Encrypted section is opaque blob (no n field - implied n[0])");
    println!("\nTest with: ./target/debug/vsfinfo comprehensive_example.vsf");

    Ok(())
}
