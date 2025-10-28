use vsf::builders::*;
use vsf::types::{BitPackedTensor, EtType};
use vsf::WorldCoord;

fn main() -> Result<(), String> {
    println!("=== VSF RAW Image Example ===\n");

    // Example: 17x163 11-bit single-plane RGGB bayer sensor
    let width = 17;
    let height = 163;
    let planes = 1;
    let bits = 11;
    let cfa = vec![b'R', b'G', b'C', b'Y']; // Red Green Cyan Yellow Bayer pattern
    let blackpoint = 499;
    let whitepoint = 8047;

    println!("Sensor specs:");
    println!("  Resolution: {}×{} (single plane)", width, height);
    println!("  Bit depth: {} bits per sample", bits);
    println!("  CFA pattern: RGCY (Red, Green, Cyan, Yellow)");
    println!("  Black level: {}", blackpoint);
    println!("  White level: {}\n", whitepoint);

    // Generate simulated RAW sensor data
    let total_samples = width * height * planes;
    let mut samples = Vec::with_capacity(total_samples);
    let mut rng = 0usize;
    for sample in 0..total_samples {
        rng ^= rng.rotate_left(13).wrapping_add(sample);
        let value = rng as u8 as u16 + blackpoint; // Simulate RAW image data
        samples.push(value);
    }

    // Pack into BitPackedTensor (efficient storage for 11-bit samples)
    let image = BitPackedTensor::pack(bits, vec![width, height], &samples);
    println!(
        "Packed {} samples into {} bytes\n",
        total_samples,
        image.data.len()
    );

    // ========== BUILDER PATTERN API ==========
    println!("Building VSF RAW image with metadata...");

    let mut raw = RawImageBuilder::new(image);

    // Set RAW metadata (sensor characteristics)
    raw.raw.cfa_pattern = Some(cfa);
    raw.raw.black_level = Some(blackpoint as f32);
    raw.raw.white_level = Some(whitepoint as f32);
    raw.raw.magic_9 = Some(vec![
        1.5, -0.3, -0.2, // Sensor RGB → LMS colour matrix (3×3)
        -0.4, 1.6, -0.2, -0.1, -0.5, 1.6,
    ]);

    // Set camera settings
    raw.camera.iso_speed = Some(800.);
    raw.camera.shutter_time_s = Some(1. / 60.); // 1/60 second
    raw.camera.aperture_f_number = Some(2.8);
    raw.camera.focal_length_m = Some(0.024); // 24mm = 0.024m
    raw.camera.exposure_compensation = Some(0.);
    raw.camera.focus_distance_m = Some(5.);
    raw.camera.flash_fired = Some(false);
    raw.camera.metering_mode = Some("matrix".to_string());

    // Set lens info
    raw.lens.make = Some("Sony".to_string());
    raw.lens.model = Some("FE 24mm F2.8 G".to_string());
    raw.lens.serial_number = Some("12345678".to_string());
    raw.lens.min_focal_length_m = Some(0.024); // Prime lens
    raw.lens.max_focal_length_m = Some(0.024);
    raw.lens.min_aperture_f = Some(2.8);
    raw.lens.max_aperture_f = Some(22.);

    // Build the VSF file
    let raw_bytes = raw.build()?;

    println!("✓ Built VSF RAW image: {} bytes\n", raw_bytes.len());

    // Write to file
    std::fs::write("example_raw.vsf", &raw_bytes).expect("Failed to write file");
    println!("✓ Saved to example_raw.vsf");

    // ========== OPTIONAL: ADD VERIFICATION ==========
    println!("\n=== Optional: Add file hash for integrity verification ===");

    // To add a file hash, we'd need to rebuild with VsfBuilder directly:
    // let builder = VsfBuilder::new().with_file_hash();
    // ... build sections ...
    // let bytes = builder.build()?;
    // let verified = verification::add_file_hash(bytes)?;

    println!("To add verification:");
    println!("  1. Use VsfBuilder::new().with_file_hash()");
    println!("  2. Call verification::add_file_hash(bytes)");
    println!("  3. Call verification::verify_file_hash(&bytes) to validate");

    println!("\n=== Complete! ===");
    println!("The RAW image includes:");
    println!("  • Bitpacked sensor data ({}-bit samples)", bits);
    println!("  • CFA pattern (RGCY Bayer)");
    println!("  • Black/white levels");
    println!("  • Colour transformation matrix (Magic 9)");
    println!("  • Camera settings (ISO, shutter, aperture, etc.)");
    println!("  • Lens information");

    Ok(())
}
