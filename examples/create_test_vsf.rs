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
        black_level: Some(BlackLevel::new(64.0).unwrap()),
        white_level: Some(WhiteLevel::new(255.0).unwrap()),
        dark_frame_hash: None,
        flat_field_hash: None,
        bias_frame_hash: None,
        vignette_correction_hash: None,
        distortion_correction_hash: None,
        magic_9: None,
    };

    // Create camera settings
    let camera = CameraSettings {
        iso_speed: Some(IsoSpeed::new(800.0).unwrap()),
        shutter_time_s: Some(ShutterTime::new(1.0 / 60.0).unwrap()), // 1/60 second
        aperture_f_number: Some(Aperture::new(2.8).unwrap()),
        focal_length_m: Some(FocalLength::new(0.050).unwrap()), // 50mm
        exposure_compensation: None,
        focus_distance_m: Some(FocusDistance::new(3.5).unwrap()),
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
