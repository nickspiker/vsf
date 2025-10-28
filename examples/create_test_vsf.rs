//! Create a sample VSF file for testing vsfinfo

use vsf::builders::{build_raw_image, CameraSettings, RawMetadata, TokenAuth};
use vsf::types::{BitPackedTensor, EtType, WorldCoord};

fn main() {
    // Create a simple 8x8 test image
    let samples: Vec<u64> = (0..64).map(|i| i * 4).collect(); // 0, 4, 8, 12, ..., 252
    let image = BitPackedTensor::pack(8, vec![8, 8], &samples);

    // Create metadata
    let metadata = RawMetadata {
        cfa_pattern: Some(vec![b'R', b'G', b'G', b'B']), // RGGB Bayer pattern
        black_level: Some(64.0),
        white_level: Some(255.0),
        dark_frame_hash: None,
        flat_field_hash: None,
        bias_frame_hash: None,
        vignette_correction_hash: None,
        distortion_correction_hash: None,
        magic_9: None,
    };

    // Create camera settings
    let camera = CameraSettings {
        iso_speed: Some(800.0),
        shutter_time_s: Some(1.0 / 60.0), // 1/60 second
        aperture_f_number: Some(2.8),
        focal_length_m: Some(0.050), // 50mm
        exposure_compensation: None,
        focus_distance_m: Some(3.5),
        flash_fired: Some(false),
        metering_mode: Some("matrix".to_string()),
    };

    // Create TOKEN auth
    let token_auth = TokenAuth {
        creator_pubkey: vec![0xAB; 32], // Dummy 32-byte Ed25519 public key
        device_serial: 12345,
        timestamp_et: EtType::f6(1234567890.123456),
        location: Some(WorldCoord::from_lat_lon(47.6062, -122.3321)), // Seattle
        signature: vec![0xCD; 64], // Dummy 64-byte Ed25519 signature
    };

    // Build the VSF file
    let bytes = build_raw_image(image, Some(metadata), Some(camera), None, Some(token_auth))
        .expect("Failed to build VSF file");

    // Write to file
    std::fs::write("test_sample.vsf", &bytes).expect("Failed to write file");

    println!("Created test_sample.vsf ({} bytes)", bytes.len());
}
