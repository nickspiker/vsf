//! Example demonstrating the builder pattern API for creating VSF files
//!
//! This shows the ergonomic dot notation for incrementally setting fields,
//! which is especially useful when fields are conditional or optional.

use vsf::builders::RawImageBuilder;
use vsf::types::BitPackedTensor;

fn main() {
    // Create a simple 8x8 test image
    let samples: Vec<u64> = (0..64).map(|i| i * 4).collect(); // 0, 4, 8, 12, ..., 252
    let image = BitPackedTensor::pack(8, vec![8, 8], &samples);

    // Create builder with image
    let mut raw = RawImageBuilder::new(image);

    // Set RAW metadata fields using dot notation
    raw.raw.cfa_pattern = Some(vec![b'R', b'G', b'G', b'B']); // RGGB Bayer pattern
    raw.raw.black_level = Some(64.0);
    raw.raw.white_level = Some(255.0);

    // Set camera settings using dot notation
    raw.camera.iso_speed = Some(800.0);
    raw.camera.shutter_time_s = Some(1.0 / 60.0); // 1/60 second
    raw.camera.aperture_f_number = Some(2.8);
    raw.camera.focal_length_m = Some(0.050); // 50mm
    raw.camera.focus_distance_m = Some(3.5);
    raw.camera.flash_fired = Some(false);
    raw.camera.metering_mode = Some("matrix".to_string());

    // Set lens info using dot notation
    raw.lens.make = Some("Sony".to_string());
    raw.lens.model = Some("FE 50mm F1.2 GM".to_string());
    raw.lens.min_focal_length_m = Some(0.050); // 50mm prime
    raw.lens.max_focal_length_m = Some(0.050);
    raw.lens.max_aperture_f = Some(1.2); // f/1.2 maximum aperture

    // Build the VSF file
    let bytes = raw.build().expect("Failed to build VSF file");

    // Write to file
    std::fs::write("builder_example.vsf", &bytes).expect("Failed to write file");

    println!("Created builder_example.vsf ({} bytes)", bytes.len());
    println!("\nThe builder pattern allows you to:");
    println!("  - Use intuitive dot notation: raw.camera.iso_speed = Some(800.0)");
    println!("  - Incrementally set fields based on conditions");
    println!("  - Skip optional fields easily (they default to None)");
}
