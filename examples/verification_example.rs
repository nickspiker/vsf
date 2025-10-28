//! Example demonstrating VSF verification strategies
//!
//! Shows both Strategy 1 (full file hash) and Strategy 2 (per-section hash/sign)

use vsf::builders::RawImageBuilder;
use vsf::types::BitPackedTensor;
use vsf::verification::{add_file_hash, verify_file_hash};
use vsf::vsf_builder::VsfBuilder;

fn main() -> Result<(), String> {
    println!("=== VSF Verification Strategies ===\n");

    // ========== STRATEGY 1: Full File Hash ==========
    println!("Strategy 1: Full File Hash");
    println!("- Single hash in header covering entire file");
    println!("- Simple integrity check for archives\n");

    // Create a RAW image
    let samples: Vec<u64> = (0..64).map(|i| i * 4).collect();
    let image = BitPackedTensor::pack(8, vec![8, 8], &samples);

    let mut raw = RawImageBuilder::new(image);
    raw.camera.iso_speed = Some(800.0);
    raw.camera.shutter_time_s = Some(1.0 / 60.0);

    // Build with file hash placeholder
    let mut builder = vsf::VsfBuilder::new();

    // Add raw section manually (since RawImageBuilder uses VsfBuilder internally)
    // For now, let's use the simple approach:
    let bytes_without_hash = raw.build()?;
    println!("  Built VSF file: {} bytes", bytes_without_hash.len());

    // Add file hash using VsfBuilder directly
    let samples2: Vec<u64> = (0..64).map(|i| i * 4).collect();
    let image2 = BitPackedTensor::pack(8, vec![8, 8], &samples2);
    let mut raw2 = RawImageBuilder::new(image2);
    raw2.camera.iso_speed = Some(800.0);

    // For demonstration, rebuild using VsfBuilder with hash
    println!("  TODO: Demonstrate with_file_hash() on VsfBuilder");
    println!("  (RawImageBuilder wraps VsfBuilder internally)\n");

    // ========== STRATEGY 2: Per-Section Hash ==========
    println!("Strategy 2: Per-Section Hash/Signature");
    println!("- Hash/sig stored in label definition");
    println!("- Signs only specific sections");
    println!("- Allows selective editing (lock image, edit metadata)\n");

    println!("  TODO: Implement add_section_hash()");
    println!("  TODO: Implement sign_section()\n");

    // ========== VERIFICATION WORKFLOW ==========
    println!("=== Verification Workflow ===\n");

    println!("1. Build VSF file:");
    println!("   let bytes = raw.build()?;\n");

    println!("2. Add verification (choose strategy):");
    println!("   Strategy 1: let bytes = add_file_hash(bytes)?;");
    println!("   Strategy 2: let bytes = add_section_hash(bytes, \"raw\")?;");
    println!("   Strategy 2: let bytes = sign_section(bytes, \"raw\", &key)?;\n");

    println!("3. Verify later:");
    println!("   Strategy 1: verify_file_hash(&bytes)?;");
    println!("   Strategy 2: verify_section_hash(&bytes, \"raw\")?;");
    println!("   Strategy 2: verify_section_signature(&bytes, \"raw\", &pubkey)?;\n");

    // ========== COMBINED STRATEGIES ==========
    println!("=== Combined Strategies ===\n");
    println!("You can use both:");
    println!("  let bytes = raw.build()?;");
    println!("  let bytes = add_file_hash(bytes)?;        // Full file integrity");
    println!("  let bytes = sign_section(bytes, \"raw\", &key)?;  // Lock image data\n");

    println!("Benefits:");
    println!("  - File hash: Simple integrity check");
    println!("  - Section sig: Cryptographic provenance");
    println!("  - Both: Maximum verification + selective editing");

    Ok(())
}
