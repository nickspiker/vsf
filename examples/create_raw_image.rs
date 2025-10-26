use vsf::builders::*;
use vsf::WorldCoord;

fn main() -> Result<(), String> {
    println!("Creating example VSF RAW image file...\n");

    // Example: 8x8 8-bit grayscale RAW image
    let width = 8;
    let height = 8;
    let bits_per_pixel = 8;

    // Create simple gradient pattern
    let mut pixels = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let value = ((x + y) * 32) as u8;
            pixels.push(value);
        }
    }

    println!("Creating complete RAW image with full metadata...");

    // Create complete RAW image with full metadata
    let raw_bytes = complete_raw_image(
        width,
        height,
        bits_per_pixel,
        pixels,
        Some(RawMetadata {
            cfa_pattern: Some(vec![0, 1, 1, 2]), // RGGB Bayer pattern
            black_level: Some(64),
            white_level: Some(255),
            dark_frame_hash: Some(vec![0xAB; 32]), // Example hash
            flat_field_hash: None,
            bias_frame_hash: None,
            vignette_correction_hash: None,
            distortion_correction_hash: None,
            color_matrix: Some(vec![
                1.5, -0.3, -0.2,
                -0.4, 1.6, -0.2,
                -0.1, -0.5, 1.6,
            ]),
        }),
        Some(CameraSettings {
            iso_speed: Some(800),
            shutter_time_ns: Some(16_666_667), // 1/60 second
            aperture_f_number: Some(2.8),
            focal_length_mm: Some(24.0),
            exposure_compensation: Some(0.0),
            focus_distance_m: Some(5.0),
            flash_fired: Some(false),
            metering_mode: Some("matrix".to_string()),
            white_balance: Some("auto".to_string()),
        }),
        Some(LensInfo {
            make: Some("Sony".to_string()),
            model: Some("FE 24mm F2.8 G".to_string()),
            serial_number: Some("12345678".to_string()),
            min_focal_length_mm: Some(24.0),
            max_focal_length_mm: Some(24.0),
            min_aperture_f: Some(2.8),
            max_aperture_f: Some(22.0),
        }),
        Some(TokenAuth {
            creator_pubkey: vec![0xAB; 32],
            device_serial: "EXAMPLE-001".to_string(),
            timestamp_et: 1234567890.123456,
            location: Some(WorldCoord::from_lat_lon(47.6062, -122.3321)), // Seattle
            signature: vec![0xCD; 64],
        }),
    )?;

    println!("✓ Created VSF RAW file: {} bytes", raw_bytes.len());
    println!("  - Magic number: RÅ<");
    println!("  - Contains: token auth + imaging raw + pixels");
    println!("  - Sensor: {}x{} @ {} bits/pixel", width, height, bits_per_pixel);
    println!("  - Camera: ISO 800, 1/60s, f/2.8, 24mm");
    println!("  - Location: Seattle, WA");
    println!();

    // Example: Lumis-style capture (12-bit, 4096x3072)
    println!("Creating Lumis 12-bit RAW capture...");

    let lumis_pixel_count = 4096 * 3072;
    let lumis_byte_count = (lumis_pixel_count * 12 + 7) / 8;
    let lumis_pixels = vec![0x88; lumis_byte_count]; // Mid-gray pattern

    let lumis_bytes = lumis_raw_capture(
        lumis_pixels,
        800,
        16_666_667,
        "LUMIS-001".to_string(),
        vec![0xAB; 32],
        vec![0xCD; 64],
        1234567890.123456,
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
