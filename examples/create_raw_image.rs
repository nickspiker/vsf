use vsf::builders::*;
use vsf::types::{BitPackedTensor, EtType};
use vsf::WorldCoord;

fn main() -> Result<(), String> {
    println!("Creating example VSF RAW image file...\n");
    println!("═══════════════════════════════════════════════════════════");
    println!("IMPORTANT: BitPackedTensor is SELF-DESCRIBING!");
    println!("  - It contains bit_depth (how many bits per pixel)");
    println!("  - It contains shape ([width, height])");
    println!("  - It contains the bitpacked pixel data");
    println!("  NO redundant width/height/bits_per_pixel fields needed!");
    println!("═══════════════════════════════════════════════════════════\n");

    // Example: 8x8 8-bit grayscale RAW image
    // Create simple gradient pattern: 0, 18, 36, 54, ... 252 (max 8-bit is 255)
    let mut samples = Vec::new();
    for y in 0..8 {
        for x in 0..8 {
            let value = ((x + y) * 18) as u64; // Max: 14*18 = 252 (fits in 8-bit)
            samples.push(value);
        }
    }

    println!("Creating complete RAW image with full metadata...");

    // Create BitPackedTensor - this self-describes the image!
    let image = BitPackedTensor::pack(8, vec![8, 8], &samples);

    // Create complete RAW image with full metadata
    let raw_bytes = complete_raw_image(
        image,
        Some(RawMetadata {
            cfa_pattern: Some(vec![b'R', b'G', b'G', b'B']), // RGGB Bayer pattern
            black_level: Some(64.),
            white_level: Some(255.),
            dark_frame_hash: Some(vec![0xAB; 32]), // Example hash
            flat_field_hash: None,
            bias_frame_hash: None,
            vignette_correction_hash: None,
            distortion_correction_hash: None,
            magic_9: Some(vec![1.5, -0.3, -0.2, -0.4, 1.6, -0.2, -0.1, -0.5, 1.6]),
        }),
        Some(CameraSettings {
            iso_speed: Some(800.),
            shutter_time_s: Some(1. / 60.),
            aperture_f_number: Some(2.8),
            focal_length_m: Some(0.024), // 24mm = 0.024m
            exposure_compensation: Some(0.),
            focus_distance_m: Some(5.),
            flash_fired: Some(false),
            metering_mode: Some("matrix".to_string()),
        }),
        Some(LensInfo {
            make: Some("Sony".to_string()),
            model: Some("FE 24mm F2.8 G".to_string()),
            serial_number: Some("12345678".to_string()),
            min_focal_length_m: Some(0.024), // 24mm = 0.024m
            max_focal_length_m: Some(0.024), // Prime lens
            min_aperture_f: Some(2.8),
            max_aperture_f: Some(22.0),
        }),
        Some(TokenAuth {
            creator_pubkey: vec![0xAB; 32],
            device_serial: 123456789, // Numeric serial
            timestamp_et: EtType::f6(1234567890.123456),
            location: Some(WorldCoord::from_lat_lon(47.6062, -122.3321)), // Seattle
            signature: vec![0xCD; 64],
        }),
    )?;

    println!("✓ Created VSF RAW file: {} bytes", raw_bytes.len());
    println!("  - Magic number: RÅ<");
    println!("  - Contains: token auth + imaging raw + pixels");
    println!("  - Sensor: 8×8 @ 8 bits/pixel (self-described in BitPackedTensor)");
    println!("  - Camera: ISO 800, 1/60s, f/2.8, 24mm");
    println!("  - Location: Seattle, WA");
    println!();

    // Example: Lumis-style capture (12-bit, 4096x3072)
    println!("Creating Lumis 12-bit RAW capture...");
    println!("  Note: lumis_raw_capture takes SAMPLES (u64 values 0-4095)");
    println!("        It will bitpack them into minimal representation\n");

    let lumis_pixel_count = 4096 * 3072;
    let lumis_samples: Vec<u64> = vec![2048; lumis_pixel_count]; // Mid-gray (12-bit)

    let lumis_bytes = lumis_raw_capture(
        lumis_samples,
        800.0,
        1. / 60., // 1/60 second shutter
        987654321, // Numeric device serial
        vec![0xAB; 32],
        vec![0xCD; 64],
        EtType::f6(1234567890.123456),
        Some(WorldCoord::from_lat_lon(47.6062, -122.3321)),
    )?;

    println!("✓ Created Lumis RAW file: {} bytes", lumis_bytes.len());
    println!("  - Resolution: 4096×3072 (12.6 MP)");
    println!("  - Bit depth: 12-bit");
    println!("  - Bayer pattern: RGGB");
    println!("  - TOKEN authenticated");
    println!();

    // You can write these to disk:
    // std::fs::write("example_raw.vsf", &raw_bytes)?;
    // std::fs::write("lumis_capture.vsf", &lumis_bytes)?;

    println!("✓ All examples completed successfully!");
    println!();
    println!("Files are ready to write with:");
    println!("  std::fs::write(\"example_raw.vsf\", &raw_bytes)");
    println!("  std::fs::write(\"lumis_capture.vsf\", &lumis_bytes)");

    Ok(())
}
