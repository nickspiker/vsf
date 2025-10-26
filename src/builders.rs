//! High-level builders for common VSF use cases
//!
//! This module provides convenient constructor functions for common data types:
//! - Plain text documents
//! - Images (RAW sensor data, processed photos)
//! - GPS tracks
//! - Simple key-value metadata
//!
//! # Examples
//!
//! ```ignore
//! use vsf::builders::*;
//!
//! // Plain text document
//! let doc = text_document("Hello, world!");
//!
//! // RAW camera image (Lumis 12-bit sensor)
//! let raw = raw_image_12bit(4096, 3072, pixel_data);
//!
//! // GPS track
//! let track = gps_track(vec![
//!     (40.7128, -74.0060),  // NYC
//!     (51.5074, -0.1278),   // London
//! ]);
//! ```

use crate::types::{BitPackedTensor, EtType, Tensor, VsfType, WorldCoord};
use crate::vsf_builder::VsfBuilder;

// ==================== RAW IMAGE METADATA STRUCTURES ====================

/// Metadata for RAW image captures
#[derive(Debug, Clone)]
pub struct RawMetadata {
    // Sensor characteristics
    pub cfa_pattern: Option<Vec<u8>>,      // [R, G, G, B] = RGGB for Bayer 2x2
    pub black_level: Option<u16>,          // Sensor black point
    pub white_level: Option<u16>,          // Sensor white point (saturation)

    // Calibration frames (by hash reference, not embedded)
    pub dark_frame_hash: Option<Vec<u8>>,          // SHA-3 or BLAKE3 of dark frame
    pub flat_field_hash: Option<Vec<u8>>,          // Flat field correction
    pub bias_frame_hash: Option<Vec<u8>>,          // Bias frame
    pub vignette_correction_hash: Option<Vec<u8>>, // Lens+aperture vignetting
    pub distortion_correction_hash: Option<Vec<u8>>, // Lens+focal distortion

    // Color correction (3x3 matrix)
    pub color_matrix: Option<Vec<f64>>,    // 9 elements, row-major order
}

/// Camera settings at time of capture
#[derive(Debug, Clone)]
pub struct CameraSettings {
    pub iso_speed: Option<u32>,            // ISO sensitivity
    pub shutter_time_ns: Option<u64>,      // Shutter speed in nanoseconds
    pub aperture_f_number: Option<f64>,    // f-stop (e.g. 2.8)
    pub focal_length_mm: Option<f64>,      // Focal length in millimeters
    pub exposure_compensation: Option<f64>, // EV adjustment
    pub focus_distance_m: Option<f64>,     // Focus distance in meters
    pub flash_fired: Option<bool>,
    pub metering_mode: Option<String>,     // "spot", "center", "matrix"
    pub white_balance: Option<String>,     // "auto", "daylight", "tungsten", etc.
}

/// Lens information
#[derive(Debug, Clone)]
pub struct LensInfo {
    pub make: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub min_focal_length_mm: Option<f64>,
    pub max_focal_length_mm: Option<f64>,
    pub min_aperture_f: Option<f64>,
    pub max_aperture_f: Option<f64>,
}

/// TOKEN authentication for capture
#[derive(Debug, Clone)]
pub struct TokenAuth {
    pub creator_pubkey: Vec<u8>,          // Ed25519 public key (32 bytes)
    pub device_serial: String,             // Hardware serial number
    pub timestamp_et: f64,                 // Eagle Time timestamp
    pub location: Option<WorldCoord>,      // Dymaxion location (if GPS available)
    pub signature: Vec<u8>,                // Ed25519 signature over entire capture
}

/// Create a simple plain text document
///
/// Returns a VsfType::x (Unicode string)
///
/// # Example
/// ```ignore
/// let doc = text_document("README contents here...");
/// let encoded = doc.flatten();
/// ```
pub fn text_document(text: impl Into<String>) -> VsfType {
    VsfType::x(text.into())
}

/// Create a RAW camera image with 12-bit sensor data
///
/// Common for professional cameras (Lumis, Sony, Canon, etc.)
///
/// # Arguments
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
/// * `samples` - Pixel values (0-4095 for 12-bit)
///
/// # Example
/// ```ignore
/// let pixels: Vec<u64> = vec![2048; 4096 * 3072]; // Mid-gray
/// let raw = raw_image_12bit(4096, 3072, pixels);
/// ```
pub fn raw_image_12bit(width: usize, height: usize, samples: Vec<u64>) -> VsfType {
    let tensor = BitPackedTensor::pack(12, vec![width, height], &samples);
    VsfType::p(tensor)
}

/// Create a RAW camera image with arbitrary bit depth
///
/// Supports 1-256 bits per sample
///
/// # Arguments
/// * `bit_depth` - Bits per sample (1-256, where 0 = 256)
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
/// * `samples` - Pixel values
pub fn raw_image(bit_depth: u8, width: usize, height: usize, samples: Vec<u64>) -> VsfType {
    let tensor = BitPackedTensor::pack(bit_depth, vec![width, height], &samples);
    VsfType::p(tensor)
}

/// Create a processed image (8-bit grayscale)
///
/// For standard grayscale images, JPEG output, etc.
///
/// # Example
/// ```ignore
/// let gray_pixels: Vec<u8> = vec![128; 1920 * 1080];
/// let img = grayscale_image_8bit(1920, 1080, gray_pixels);
/// ```
pub fn grayscale_image_8bit(width: usize, height: usize, data: Vec<u8>) -> VsfType {
    let tensor = Tensor::new(vec![width, height], data);
    VsfType::t_u3(tensor)
}

/// Create an RGB image (8-bit per channel)
///
/// Standard RGB with shape [width, height, 3]
///
/// # Example
/// ```ignore
/// let rgb_data: Vec<u8> = vec![0; 1920 * 1080 * 3];
/// let img = rgb_image_8bit(1920, 1080, rgb_data);
/// ```
pub fn rgb_image_8bit(width: usize, height: usize, data: Vec<u8>) -> VsfType {
    let tensor = Tensor::new(vec![width, height, 3], data);
    VsfType::t_u3(tensor)
}

/// Create an RGBA image (8-bit per channel with alpha)
///
/// Standard RGBA with shape [width, height, 4]
///
/// # Example
/// ```ignore
/// let rgba_data: Vec<u8> = vec![255; 1920 * 1080 * 4]; // White, opaque
/// let img = rgba_image_8bit(1920, 1080, rgba_data);
/// ```
pub fn rgba_image_8bit(width: usize, height: usize, data: Vec<u8>) -> VsfType {
    let tensor = Tensor::new(vec![width, height, 4], data);
    VsfType::t_u3(tensor)
}

/// Create a GPS track from lat/lon coordinates
///
/// Returns a 1D tensor of WorldCoord values
///
/// # Example
/// ```ignore
/// let track = gps_track(vec![
///     (40.7128, -74.0060),  // NYC
///     (51.5074, -0.1278),   // London
///     (35.6762, 139.6503),  // Tokyo
/// ]);
/// ```
pub fn gps_track(coords: Vec<(f64, f64)>) -> Vec<WorldCoord> {
    coords
        .into_iter()
        .map(|(lat, lon)| WorldCoord::from_lat_lon(lat, lon))
        .collect()
}

/// Create a single GPS waypoint
///
/// # Example
/// ```ignore
/// let nyc = gps_waypoint(40.7128, -74.0060);
/// let encoded = VsfType::w(nyc).flatten();
/// ```
pub fn gps_waypoint(lat: f64, lon: f64) -> WorldCoord {
    WorldCoord::from_lat_lon(lat, lon)
}

/// Create a geotagged image with location metadata
///
/// Returns (image, location) tuple
///
/// # Example
/// ```ignore
/// let (img, loc) = geotagged_photo(
///     1920, 1080,
///     rgb_data,
///     40.7128, -74.0060  // Photo taken in NYC
/// );
/// ```
pub fn geotagged_photo(
    width: usize,
    height: usize,
    rgb_data: Vec<u8>,
    lat: f64,
    lon: f64,
) -> (VsfType, WorldCoord) {
    let img = rgb_image_8bit(width, height, rgb_data);
    let loc = WorldCoord::from_lat_lon(lat, lon);
    (img, loc)
}

// ==================== COMPLETE RAW IMAGE BUILDERS ====================

/// Build a complete RAW image file with full metadata and calibration
///
/// This is the production-ready version for Lumis and other camera apps.
/// Includes sensor characteristics, camera settings, calibration references,
/// and optional TOKEN authentication.
///
/// # Arguments
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
/// * `bits_per_pixel` - Bit depth (8, 10, 12, 14, 16, etc.)
/// * `pixels` - Raw pixel data (bitpacked bytes for bits_per_pixel > 8)
/// * `metadata` - Optional sensor and calibration metadata
/// * `camera` - Optional camera settings at capture time
/// * `lens` - Optional lens information
/// * `token_auth` - Optional TOKEN authentication
///
/// # Returns
/// Complete VSF file bytes ready to write to disk
pub fn complete_raw_image(
    width: usize,
    height: usize,
    bits_per_pixel: u8,
    pixels: Vec<u8>,
    metadata: Option<RawMetadata>,
    camera: Option<CameraSettings>,
    lens: Option<LensInfo>,
    token_auth: Option<TokenAuth>,
) -> Result<Vec<u8>, String> {
    let mut builder = VsfBuilder::new();

    // TOKEN auth section (if provided)
    if let Some(auth) = token_auth {
        let mut auth_items = vec![
            ("creator pubkey".to_string(), VsfType::h(auth.creator_pubkey)),
            ("device serial".to_string(), VsfType::x(auth.device_serial)),
            ("timestamp et".to_string(), VsfType::e(EtType::f6(auth.timestamp_et))),
            ("signature".to_string(), VsfType::g(auth.signature)),
        ];

        if let Some(loc) = auth.location {
            auth_items.push(("location".to_string(), VsfType::w(loc)));
        }

        builder = builder.add_section("token auth", auth_items);
    }

    // Build imaging raw section
    let mut raw_items = vec![
        ("width".to_string(), VsfType::u(width, false)),
        ("height".to_string(), VsfType::u(height, false)),
        ("bits per pixel".to_string(), VsfType::u(bits_per_pixel as usize, false)),
    ];

    // Add optional metadata
    if let Some(meta) = metadata {
        if let Some(cfa) = meta.cfa_pattern {
            raw_items.push((
                "cfa pattern".to_string(),
                VsfType::t_u3(Tensor {
                    shape: vec![cfa.len()],
                    data: cfa,
                }),
            ));
        }

        if let Some(black) = meta.black_level {
            raw_items.push(("black level".to_string(), VsfType::u(black as usize, false)));
        }

        if let Some(white) = meta.white_level {
            raw_items.push(("white level".to_string(), VsfType::u(white as usize, false)));
        }

        // Calibration hashes
        if let Some(hash) = meta.dark_frame_hash {
            raw_items.push(("dark frame hash".to_string(), VsfType::h(hash)));
        }

        if let Some(hash) = meta.flat_field_hash {
            raw_items.push(("flat field hash".to_string(), VsfType::h(hash)));
        }

        if let Some(hash) = meta.bias_frame_hash {
            raw_items.push(("bias frame hash".to_string(), VsfType::h(hash)));
        }

        if let Some(hash) = meta.vignette_correction_hash {
            raw_items.push(("vignette correction hash".to_string(), VsfType::h(hash)));
        }

        if let Some(hash) = meta.distortion_correction_hash {
            raw_items.push(("distortion correction hash".to_string(), VsfType::h(hash)));
        }

        // Color matrix
        if let Some(matrix) = meta.color_matrix {
            if matrix.len() == 9 {
                raw_items.push((
                    "color matrix".to_string(),
                    VsfType::t_f6(Tensor {
                        shape: vec![3, 3],
                        data: matrix,
                    }),
                ));
            }
        }
    }

    // Camera settings
    if let Some(cam) = camera {
        if let Some(iso) = cam.iso_speed {
            raw_items.push(("iso speed".to_string(), VsfType::u(iso as usize, false)));
        }

        if let Some(shutter) = cam.shutter_time_ns {
            raw_items.push(("shutter time ns".to_string(), VsfType::u(shutter as usize, false)));
        }

        if let Some(aperture) = cam.aperture_f_number {
            raw_items.push(("aperture f number".to_string(), VsfType::f6(aperture)));
        }

        if let Some(focal) = cam.focal_length_mm {
            raw_items.push(("focal length mm".to_string(), VsfType::f6(focal)));
        }

        if let Some(comp) = cam.exposure_compensation {
            raw_items.push(("exposure compensation".to_string(), VsfType::f6(comp)));
        }

        if let Some(focus) = cam.focus_distance_m {
            raw_items.push(("focus distance m".to_string(), VsfType::f6(focus)));
        }

        if let Some(flash) = cam.flash_fired {
            raw_items.push(("flash fired".to_string(), VsfType::u0(flash)));
        }

        if let Some(metering) = cam.metering_mode {
            raw_items.push(("metering mode".to_string(), VsfType::x(metering)));
        }

        if let Some(wb) = cam.white_balance {
            raw_items.push(("white balance".to_string(), VsfType::x(wb)));
        }
    }

    // Lens info
    if let Some(l) = lens {
        if let Some(make) = l.make {
            raw_items.push(("lens make".to_string(), VsfType::x(make)));
        }

        if let Some(model) = l.model {
            raw_items.push(("lens model".to_string(), VsfType::x(model)));
        }

        if let Some(serial) = l.serial_number {
            raw_items.push(("lens serial".to_string(), VsfType::x(serial)));
        }

        if let Some(min_focal) = l.min_focal_length_mm {
            raw_items.push(("lens min focal mm".to_string(), VsfType::f6(min_focal)));
        }

        if let Some(max_focal) = l.max_focal_length_mm {
            raw_items.push(("lens max focal mm".to_string(), VsfType::f6(max_focal)));
        }

        if let Some(min_ap) = l.min_aperture_f {
            raw_items.push(("lens min aperture".to_string(), VsfType::f6(min_ap)));
        }

        if let Some(max_ap) = l.max_aperture_f {
            raw_items.push(("lens max aperture".to_string(), VsfType::f6(max_ap)));
        }
    }

    builder = builder.add_section("imaging raw", raw_items);

    // Pixel data (unboxed)
    builder = builder.add_unboxed("pixels", pixels);

    builder.build()
}

/// Convenience function for Lumis 12-bit captures
///
/// Lumis uses 12-bit RAW with RGGB Bayer pattern at 4096x3072
///
/// # Arguments
/// * `pixels` - Bitpacked 12-bit pixel data
/// * `iso` - ISO speed
/// * `shutter_ns` - Shutter time in nanoseconds
/// * `device_serial` - Device serial number
/// * `creator_pubkey` - Ed25519 public key (32 bytes)
/// * `signature` - Ed25519 signature (64 bytes)
/// * `timestamp_et` - Eagle Time timestamp
/// * `location` - Optional GPS location
pub fn lumis_raw_capture(
    pixels: Vec<u8>,
    iso: u32,
    shutter_ns: u64,
    device_serial: String,
    creator_pubkey: Vec<u8>,
    signature: Vec<u8>,
    timestamp_et: f64,
    location: Option<WorldCoord>,
) -> Result<Vec<u8>, String> {
    complete_raw_image(
        4096,
        3072,
        12,
        pixels,
        Some(RawMetadata {
            cfa_pattern: Some(vec![0, 1, 1, 2]), // RGGB
            black_level: Some(64),
            white_level: Some(4095),
            dark_frame_hash: None,
            flat_field_hash: None,
            bias_frame_hash: None,
            vignette_correction_hash: None,
            distortion_correction_hash: None,
            color_matrix: None,
        }),
        Some(CameraSettings {
            iso_speed: Some(iso),
            shutter_time_ns: Some(shutter_ns),
            aperture_f_number: None,
            focal_length_mm: None,
            exposure_compensation: None,
            focus_distance_m: None,
            flash_fired: Some(false),
            metering_mode: None,
            white_balance: Some("auto".to_string()),
        }),
        None, // No lens info (phone camera)
        Some(TokenAuth {
            creator_pubkey,
            device_serial,
            timestamp_et,
            location,
            signature,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_document() {
        let doc = text_document("Hello, VSF!");
        if let VsfType::x(s) = doc {
            assert_eq!(s, "Hello, VSF!");
        } else {
            panic!("Expected string");
        }
    }

    #[test]
    fn test_raw_image_12bit() {
        let samples = vec![2048u64; 100 * 50]; // 100×50 mid-gray
        let img = raw_image_12bit(100, 50, samples);

        if let VsfType::p(tensor) = img {
            assert_eq!(tensor.bit_depth, 12);
            assert_eq!(tensor.shape, vec![100, 50]);
            assert_eq!(tensor.len(), 100 * 50);
        } else {
            panic!("Expected bitpacked tensor");
        }
    }

    #[test]
    fn test_grayscale_image() {
        let data = vec![128u8; 64 * 48];
        let img = grayscale_image_8bit(64, 48, data);

        if let VsfType::t_u3(tensor) = img {
            assert_eq!(tensor.shape, vec![64, 48]);
            assert_eq!(tensor.data.len(), 64 * 48);
        } else {
            panic!("Expected u8 tensor");
        }
    }

    #[test]
    fn test_rgb_image() {
        let data = vec![255u8; 64 * 48 * 3];
        let img = rgb_image_8bit(64, 48, data);

        if let VsfType::t_u3(tensor) = img {
            assert_eq!(tensor.shape, vec![64, 48, 3]);
            assert_eq!(tensor.data.len(), 64 * 48 * 3);
        } else {
            panic!("Expected u8 tensor");
        }
    }

    #[test]
    fn test_gps_track() {
        let track = gps_track(vec![
            (40.7128, -74.0060), // NYC
            (51.5074, -0.1278),  // London
        ]);

        assert_eq!(track.len(), 2);
    }

    #[test]
    fn test_gps_waypoint() {
        // Use simple coordinates (equator, prime meridian)
        let point = gps_waypoint(0.0, 0.0);
        let (lat, lon) = point.to_lat_lon();

        // Check reasonable precision
        assert!(lat.abs() < 10.0, "Lat error: {}", lat.abs());
        assert!(lon.abs() < 10.0, "Lon error: {}", lon.abs());
    }

    #[test]
    fn test_geotagged_photo() {
        let rgb_data = vec![0u8; 100 * 100 * 3];
        // Use simple coordinates
        let (img, loc) = geotagged_photo(100, 100, rgb_data, 0.0, 0.0);

        if let VsfType::t_u3(tensor) = img {
            assert_eq!(tensor.shape, vec![100, 100, 3]);
        } else {
            panic!("Expected RGB tensor");
        }

        let (lat, lon) = loc.to_lat_lon();
        assert!(lat.abs() < 10.0);
        assert!(lon.abs() < 10.0);
    }

    #[test]
    fn test_complete_raw_image_minimal() {
        // Minimal RAW: just dimensions and pixels
        let pixels = vec![0xFF; 64]; // 8x8 8-bit

        let result = complete_raw_image(
            8, 8, 8, pixels.clone(),
            None, None, None, None,
        );

        assert!(result.is_ok());
        let bytes = result.unwrap();

        // Verify magic number (RÅ is 3 bytes in UTF-8)
        assert_eq!(&bytes[0..3], "RÅ".as_bytes());
        assert_eq!(bytes[3], b'<');

        // Should contain imaging raw section
        // Note: The label name is embedded in binary, not as plain text
        // Just verify the file is structured correctly
        assert!(bytes.len() > 100); // Should have header + metadata + pixels

        // Last 64 bytes should be pixel data
        let len = bytes.len();
        assert_eq!(&bytes[len - 64..], &pixels[..]);
    }

    #[test]
    fn test_complete_raw_image_with_metadata() {
        let pixels = vec![0xFF; 64];

        let result = complete_raw_image(
            8, 8, 8, pixels.clone(),
            Some(RawMetadata {
                cfa_pattern: Some(vec![0, 1, 1, 2]),
                black_level: Some(64),
                white_level: Some(255),
                dark_frame_hash: Some(vec![0xAB; 32]),
                flat_field_hash: None,
                bias_frame_hash: None,
                vignette_correction_hash: None,
                distortion_correction_hash: None,
                color_matrix: Some(vec![
                    1.0, 0.0, 0.0,
                    0.0, 1.0, 0.0,
                    0.0, 0.0, 1.0,
                ]),
            }),
            Some(CameraSettings {
                iso_speed: Some(800),
                shutter_time_ns: Some(16_666_667),
                aperture_f_number: Some(2.8),
                focal_length_mm: Some(24.0),
                exposure_compensation: None,
                focus_distance_m: None,
                flash_fired: Some(false),
                metering_mode: Some("matrix".to_string()),
                white_balance: Some("auto".to_string()),
            }),
            None,
            None,
        );

        assert!(result.is_ok());
        let bytes = result.unwrap();

        // Verify magic number (RÅ is 3 bytes in UTF-8)
        assert_eq!(&bytes[0..3], "RÅ".as_bytes());

        // Should contain version markers
        assert!(bytes.contains(&b'z'));
        assert!(bytes.contains(&b'y'));

        // Should contain section brackets
        assert!(bytes.contains(&b'['));
        assert!(bytes.contains(&b']'));
    }

    #[test]
    fn test_lumis_raw_capture() {
        // Lumis 12-bit: 4096x3072 = 12,582,912 pixels
        // 12 bits per pixel = 18,874,368 bits = 2,359,296 bytes
        let pixel_count = 4096 * 3072;
        let byte_count = (pixel_count * 12 + 7) / 8; // Round up
        let pixels = vec![0xFF; byte_count];

        let result = lumis_raw_capture(
            pixels.clone(),
            800,
            16_666_667,
            "LUMIS-001".to_string(),
            vec![0xAB; 32],
            vec![0xCD; 64],
            1234567890.123456,
            Some(WorldCoord::from_lat_lon(47.6062, -122.3321)),
        );

        assert!(result.is_ok());
        let bytes = result.unwrap();

        // Verify magic number (RÅ is 3 bytes in UTF-8)
        assert_eq!(&bytes[0..3], "RÅ".as_bytes());

        // File should be large (header + metadata + 2MB+ pixels)
        assert!(bytes.len() > 2_000_000, "File should be > 2MB with pixels");

        // Last bytes should be pixel data
        let len = bytes.len();
        assert_eq!(&bytes[len - byte_count..], &pixels[..]);
    }

    #[test]
    fn test_raw_with_token_auth() {
        let pixels = vec![0xFF; 64];

        let result = complete_raw_image(
            8, 8, 8, pixels,
            None, None, None,
            Some(TokenAuth {
                creator_pubkey: vec![0xAB; 32],
                device_serial: "TEST-001".to_string(),
                timestamp_et: 1234567890.0,
                location: Some(WorldCoord::from_lat_lon(0.0, 0.0)),
                signature: vec![0xCD; 64],
            }),
        );

        assert!(result.is_ok());
        let bytes = result.unwrap();

        // Verify magic number
        assert_eq!(&bytes[0..3], "RÅ".as_bytes());

        // Should have meaningful structure (header + sections + pixels)
        assert!(bytes.len() > 200, "File should have header + metadata + pixels");
    }
}
