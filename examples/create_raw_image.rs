use vsf::builders::*;
use vsf::types::{BitPackedTensor, EtType};
use vsf::WorldCoord;

fn main() -> Result<(), String> {
    // Example: 17x163 11-bit single plane RGGB bayer
    let width = 17;
    let height = 163;
    let planes = 1;
    let bits = 11;
    let cfa = vec![b'R', b'G', b'C', b'Y']; // Red Green Cyan Yellow Bayer pattern
    let blackpoint = 499;
    let whitepoint = 8047;
    let total_samples = width * height * planes;
    let mut samples = Vec::with_capacity(total_samples);
    let mut rng = 0usize;
    for sample in 0..total_samples {
        rng ^= rng.rotate_left(13).wrapping_add(sample);
        let value = rng as u8 as u16 + blackpoint; // Simulate RAW image data
        samples.push(value);
    }

    let image = BitPackedTensor::pack(bits, vec![width, height], &samples);

    // Create complete RAW image with full metadata
    let raw_bytes = build_raw_image(
        image,
        Some(RawMetadata {
            cfa_pattern: Some(cfa), // RGGB Bayer pattern
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

    let lumis_pixel_count = 4096 * 3072;
    let lumis_samples: Vec<u64> = vec![2048; lumis_pixel_count]; // Mid-gray (12-bit)

    let lumis_bytes = lumis_raw_capture(
        lumis_samples,
        800.0,
        1. / 60.,  // 1/60 second shutter
        987654321, // Numeric device serial
        vec![0xAB; 32],
        vec![0xCD; 64],
        EtType::f6(1234567890.123456),
        Some(WorldCoord::from_lat_lon(47.6062, -122.3321)),
    )?;

    Ok(())
}
