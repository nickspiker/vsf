//! Create a sample VSF file for testing vsfinfo

use vsf::builders::{
    build_raw_image, Aperture, BlackLevel, CameraSettings, CfaPattern, FlashFired, FocalLength,
    FocusDistance, IsoSpeed, MeteringMode, RawMetadata, ShutterTime, WhiteLevel,
};
use vsf::types::BitPackedTensor;

fn main() {
    // Create a simple 8x8 test image
    let samples: Vec<u64> = (0..64).map(|i| i * 4).collect(); // 0, 4, 8, 12, ..., 252
    let image = BitPackedTensor::pack(8, vec![8, 8], &samples);

    // Create metadata
    let metadata = RawMetadata {
        cfa_pattern: Some(CfaPattern::new(vec![b'R', b'G', b'G', b'B']).unwrap()), // RGGB Bayer pattern
        black_level: Some(BlackLevel::new(64).unwrap()),
        white_level: Some(WhiteLevel::new(255).unwrap()),
        dark_frame_hash: None,
        flat_field_hash: None,
        bias_frame_hash: None,
        vignette_correction_hash: None,
        distortion_correction_hash: None,
        magic_9: None,
    };

    // Create camera settings
    let camera = CameraSettings {
        make: None,
        model: None,
        serial_number: None,
        iso_speed: Some(IsoSpeed::new(800, 1).unwrap()),
        exposure_osc: Some(ShutterTime::from_seconds(1, 60).unwrap()), // 1/60 second
        aperture_n2: Some(Aperture::from_marking(28, 10).unwrap()),
        aperture_setting_twelfths: None,
        iso_setting_twelfths: None,
        focal_length_m: Some(FocalLength::from_millimetres(50).unwrap()), // 50mm
        exposure_bias_twelfths: None,
        focus_distance_m: Some(FocusDistance::new(7, 2).unwrap()),
        flash_fired: Some(FlashFired::new(false).unwrap()),
        metering_mode: Some(MeteringMode::new("matrix".to_string()).unwrap()),
    };

    // Build the VSF file
    let bytes = build_raw_image(image, Some(metadata), Some(camera), None)
        .expect("Failed to build VSF file");

    // Write to file
    std::fs::write("test_sample.vsf", &bytes).expect("Failed to write file");

    println!("Created test_sample.vsf ({} bytes)", bytes.len());
}
