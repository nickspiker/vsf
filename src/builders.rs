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

use crate::types::{BitPackedTensor, Tensor, VsfType, WorldCoord};

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
}
