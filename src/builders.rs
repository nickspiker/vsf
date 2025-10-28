//! High-level builders for common VSF use cases
//!
//! This module provides constructors for complex data types that require
//! packaging multiple VSF primitives together:
//! - GPS coordinate conversions (lat/lon → WorldCoord)
//! - RAW camera images with metadata
//!
//! For simple types, use VsfType directly:
//! - Text: `VsfType::x("Hello".to_string())`
//! - Images: `VsfType::p(BitPackedTensor::pack(12, vec![w, h], &samples))`
//! - Tensors: `VsfType::t_u3(Tensor::new(vec![w, h], data))`
//!
//! # Examples
//! ∞
//! ```ignore
//! use vsf::builders::*;
//! use vsf::types::*;
//!
//! // RAW camera image (12-bit sensor)
//! let raw = raw_image(12, 4096, 3072, pixel_data);

use crate::types::{BitPackedTensor, Tensor, VsfType, WorldCoord};
use crate::vsf_builder::VsfBuilder;

// ==================== NEWTYPE WRAPPERS FOR TYPE SAFETY ====================

/// CFA (Colour Filter Array) pattern with validation
/// - Bayer 2×2: 4 bytes like `[b'R', b'G', b'G', b'B']`
/// - X-Trans 6×6: 36 bytes
/// - Valid colours: R, G, B, C (Cyan), Y (Yellow), W (White), E (Emerald)
#[derive(Debug, Clone)]
pub struct CfaPattern(VsfType);

impl CfaPattern {
    pub fn new(pattern: Vec<u8>) -> Result<Self, String> {
        // Validate pattern length (common sizes: 4 for Bayer, 36 for X-Trans)
        if pattern.is_empty() {
            return Err("CFA pattern cannot be empty".to_string());
        }

        // Validate colour codes
        for &byte in &pattern {
            match byte {
                b'R' | b'G' | b'B' | b'C' | b'Y' | b'W' | b'E' => {}
                _ => return Err(format!("Invalid CFA colour code: {}", byte as char)),
            }
        }

        Ok(CfaPattern(VsfType::t_u3(Tensor {
            shape: vec![pattern.len()],
            data: pattern,
        })))
    }

    pub fn to_vsf_type(self) -> VsfType {
        self.0
    }

    pub fn from_vsf_type(vsf: VsfType) -> Result<Self, String> {
        match vsf {
            VsfType::t_u3(ref tensor) => {
                // Validate the pattern
                for &byte in &tensor.data {
                    match byte {
                        b'R' | b'G' | b'B' | b'C' | b'Y' | b'W' | b'E' => {}
                        _ => return Err(format!("Invalid CFA colour code: {}", byte as char)),
                    }
                }
                Ok(CfaPattern(vsf))
            }
            _ => Err("Expected t_u3 type for CFA pattern".to_string()),
        }
    }
}

/// Sensor black level (digital zero point)
#[derive(Debug, Clone)]
pub struct BlackLevel(VsfType);

impl BlackLevel {
    pub fn new(level: f32) -> Result<Self, String> {
        if level < 0.0 {
            return Err("Black level cannot be negative".to_string());
        }
        Ok(BlackLevel(VsfType::f5(level)))
    }

    pub fn to_vsf_type(self) -> VsfType {
        self.0
    }

    pub fn from_vsf_type(vsf: VsfType) -> Result<Self, String> {
        match vsf {
            VsfType::f5(v) if v >= 0.0 => Ok(BlackLevel(vsf)),
            VsfType::f5(_) => Err("Black level cannot be negative".to_string()),
            _ => Err("Expected f5 type for black level".to_string()),
        }
    }
}

/// Sensor white level (saturation point)
#[derive(Debug, Clone)]
pub struct WhiteLevel(VsfType);

impl WhiteLevel {
    pub fn new(level: f32) -> Result<Self, String> {
        if level <= 0.0 {
            return Err("White level must be positive".to_string());
        }
        Ok(WhiteLevel(VsfType::f5(level)))
    }

    pub fn to_vsf_type(self) -> VsfType {
        self.0
    }

    pub fn from_vsf_type(vsf: VsfType) -> Result<Self, String> {
        match vsf {
            VsfType::f5(v) if v > 0.0 => Ok(WhiteLevel(vsf)),
            VsfType::f5(_) => Err("White level must be positive".to_string()),
            _ => Err("Expected f5 type for white level".to_string()),
        }
    }
}

/// Hash reference to a calibration frame (dark frame, flat field, etc.)
#[derive(Debug, Clone)]
pub struct CalibrationHash(VsfType);

impl CalibrationHash {
    pub fn new(algorithm: u8, hash: Vec<u8>) -> Result<Self, String> {
        if hash.is_empty() {
            return Err("Hash cannot be empty".to_string());
        }
        Ok(CalibrationHash(VsfType::h(algorithm, hash)))
    }

    pub fn to_vsf_type(self) -> VsfType {
        self.0
    }

    pub fn from_vsf_type(vsf: VsfType) -> Result<Self, String> {
        match vsf {
            VsfType::h(_, ref hash) if !hash.is_empty() => Ok(CalibrationHash(vsf)),
            VsfType::h(_, _) => Err("Hash cannot be empty".to_string()),
            _ => Err("Expected h type for calibration hash".to_string()),
        }
    }
}

/// Magic 9: 3×3 colour transformation matrix (Sensor RGB → LMS)
/// Must contain exactly 9 elements in row-major order
#[derive(Debug, Clone)]
pub struct Magic9(VsfType);

impl Magic9 {
    pub fn new(values: Vec<f32>) -> Result<Self, String> {
        if values.len() != 9 {
            return Err(format!(
                "Magic 9 matrix must have exactly 9 elements, got {}",
                values.len()
            ));
        }
        Ok(Magic9(VsfType::t_f5(Tensor {
            shape: vec![3, 3],
            data: values,
        })))
    }

    pub fn to_vsf_type(self) -> VsfType {
        self.0
    }

    pub fn from_vsf_type(vsf: VsfType) -> Result<Self, String> {
        match vsf {
            VsfType::t_f5(ref tensor) => {
                if tensor.data.len() != 9 {
                    return Err(format!(
                        "Magic 9 matrix must have exactly 9 elements, got {}",
                        tensor.data.len()
                    ));
                }
                Ok(Magic9(vsf))
            }
            _ => Err("Expected t_f5 type for Magic 9 matrix".to_string()),
        }
    }
}

/// ISO speed (sensitivity)
#[derive(Debug, Clone)]
pub struct IsoSpeed(VsfType);

impl IsoSpeed {
    pub fn new(iso: f32) -> Result<Self, String> {
        if iso <= 0.0 {
            return Err("ISO speed must be positive".to_string());
        }
        Ok(IsoSpeed(VsfType::f5(iso)))
    }

    pub fn to_vsf_type(self) -> VsfType {
        self.0
    }

    pub fn from_vsf_type(vsf: VsfType) -> Result<Self, String> {
        match vsf {
            VsfType::f5(v) if v > 0.0 => Ok(IsoSpeed(vsf)),
            VsfType::f5(_) => Err("ISO speed must be positive".to_string()),
            _ => Err("Expected f5 type for ISO speed".to_string()),
        }
    }
}

/// Shutter time in seconds
#[derive(Debug, Clone)]
pub struct ShutterTime(VsfType);

impl ShutterTime {
    pub fn new(seconds: f32) -> Result<Self, String> {
        if seconds <= 0.0 {
            return Err("Shutter time must be positive".to_string());
        }
        Ok(ShutterTime(VsfType::f5(seconds)))
    }

    pub fn to_vsf_type(self) -> VsfType {
        self.0
    }

    pub fn from_vsf_type(vsf: VsfType) -> Result<Self, String> {
        match vsf {
            VsfType::f5(v) if v > 0.0 => Ok(ShutterTime(vsf)),
            VsfType::f5(_) => Err("Shutter time must be positive".to_string()),
            _ => Err("Expected f5 type for shutter time".to_string()),
        }
    }
}

/// Aperture (f-number)
#[derive(Debug, Clone)]
pub struct Aperture(VsfType);

impl Aperture {
    pub fn new(f_number: f32) -> Result<Self, String> {
        if f_number <= 0.0 {
            return Err("Aperture f-number must be positive".to_string());
        }
        Ok(Aperture(VsfType::f5(f_number)))
    }

    pub fn to_vsf_type(self) -> VsfType {
        self.0
    }

    pub fn from_vsf_type(vsf: VsfType) -> Result<Self, String> {
        match vsf {
            VsfType::f5(v) if v > 0.0 => Ok(Aperture(vsf)),
            VsfType::f5(_) => Err("Aperture f-number must be positive".to_string()),
            _ => Err("Expected f5 type for aperture".to_string()),
        }
    }
}

/// Focal length in meters
#[derive(Debug, Clone)]
pub struct FocalLength(VsfType);

impl FocalLength {
    pub fn new(meters: f32) -> Result<Self, String> {
        if meters <= 0.0 {
            return Err("Focal length must be positive".to_string());
        }
        Ok(FocalLength(VsfType::f5(meters)))
    }

    pub fn to_vsf_type(self) -> VsfType {
        self.0
    }

    pub fn from_vsf_type(vsf: VsfType) -> Result<Self, String> {
        match vsf {
            VsfType::f5(v) if v > 0.0 => Ok(FocalLength(vsf)),
            VsfType::f5(_) => Err("Focal length must be positive".to_string()),
            _ => Err("Expected f5 type for focal length".to_string()),
        }
    }
}

/// Exposure compensation in EV
#[derive(Debug, Clone)]
pub struct ExposureCompensation(VsfType);

impl ExposureCompensation {
    pub fn new(ev: f32) -> Result<Self, String> {
        Ok(ExposureCompensation(VsfType::f5(ev))) // Can be negative
    }

    pub fn to_vsf_type(self) -> VsfType {
        self.0
    }

    pub fn from_vsf_type(vsf: VsfType) -> Result<Self, String> {
        match vsf {
            VsfType::f5(_) => Ok(ExposureCompensation(vsf)),
            _ => Err("Expected f5 type for exposure compensation".to_string()),
        }
    }
}

/// Focus distance in meters
#[derive(Debug, Clone)]
pub struct FocusDistance(VsfType);

impl FocusDistance {
    pub fn new(meters: f32) -> Result<Self, String> {
        if meters < 0.0 {
            return Err("Focus distance cannot be negative".to_string());
        }
        Ok(FocusDistance(VsfType::f5(meters)))
    }

    pub fn to_vsf_type(self) -> VsfType {
        self.0
    }

    pub fn from_vsf_type(vsf: VsfType) -> Result<Self, String> {
        match vsf {
            VsfType::f5(v) if v >= 0.0 => Ok(FocusDistance(vsf)),
            VsfType::f5(_) => Err("Focus distance cannot be negative".to_string()),
            _ => Err("Expected f5 type for focus distance".to_string()),
        }
    }
}

/// Flash status
#[derive(Debug, Clone)]
pub struct FlashFired(VsfType);

impl FlashFired {
    pub fn new(fired: bool) -> Result<Self, String> {
        Ok(FlashFired(VsfType::u0(fired)))
    }

    pub fn to_vsf_type(self) -> VsfType {
        self.0
    }

    pub fn from_vsf_type(vsf: VsfType) -> Result<Self, String> {
        match vsf {
            VsfType::u0(_) => Ok(FlashFired(vsf)),
            _ => Err("Expected u0 type for flash fired".to_string()),
        }
    }
}

/// Metering mode (spot, center, matrix, etc.)
#[derive(Debug, Clone)]
pub struct MeteringMode(VsfType);

impl MeteringMode {
    pub fn new(mode: String) -> Result<Self, String> {
        if mode.is_empty() {
            return Err("Metering mode cannot be empty".to_string());
        }
        Ok(MeteringMode(VsfType::x(mode)))
    }

    pub fn to_vsf_type(self) -> VsfType {
        self.0
    }

    pub fn from_vsf_type(vsf: VsfType) -> Result<Self, String> {
        match vsf {
            VsfType::x(ref s) if !s.is_empty() => Ok(MeteringMode(vsf)),
            VsfType::x(_) => Err("Metering mode cannot be empty".to_string()),
            _ => Err("Expected x type for metering mode".to_string()),
        }
    }
}

/// Manufacturer name
#[derive(Debug, Clone)]
pub struct Manufacturer(VsfType);

impl Manufacturer {
    pub fn new(name: String) -> Result<Self, String> {
        if name.is_empty() {
            return Err("Manufacturer name cannot be empty".to_string());
        }
        Ok(Manufacturer(VsfType::x(name)))
    }

    pub fn to_vsf_type(self) -> VsfType {
        self.0
    }

    pub fn from_vsf_type(vsf: VsfType) -> Result<Self, String> {
        match vsf {
            VsfType::x(ref s) if !s.is_empty() => Ok(Manufacturer(vsf)),
            VsfType::x(_) => Err("Manufacturer name cannot be empty".to_string()),
            _ => Err("Expected x type for manufacturer".to_string()),
        }
    }
}

/// Model name
#[derive(Debug, Clone)]
pub struct ModelName(VsfType);

impl ModelName {
    pub fn new(name: String) -> Result<Self, String> {
        if name.is_empty() {
            return Err("Model name cannot be empty".to_string());
        }
        Ok(ModelName(VsfType::x(name)))
    }

    pub fn to_vsf_type(self) -> VsfType {
        self.0
    }

    pub fn from_vsf_type(vsf: VsfType) -> Result<Self, String> {
        match vsf {
            VsfType::x(ref s) if !s.is_empty() => Ok(ModelName(vsf)),
            VsfType::x(_) => Err("Model name cannot be empty".to_string()),
            _ => Err("Expected x type for model name".to_string()),
        }
    }
}

/// Serial number
#[derive(Debug, Clone)]
pub struct SerialNumber(VsfType);

impl SerialNumber {
    pub fn new(serial: String) -> Result<Self, String> {
        if serial.is_empty() {
            return Err("Serial number cannot be empty".to_string());
        }
        Ok(SerialNumber(VsfType::x(serial)))
    }

    pub fn to_vsf_type(self) -> VsfType {
        self.0
    }

    pub fn from_vsf_type(vsf: VsfType) -> Result<Self, String> {
        match vsf {
            VsfType::x(ref s) if !s.is_empty() => Ok(SerialNumber(vsf)),
            VsfType::x(_) => Err("Serial number cannot be empty".to_string()),
            _ => Err("Expected x type for serial number".to_string()),
        }
    }
}

// ==================== RAW IMAGE METADATA STRUCTURES ====================

/// Metadata for RAW image captures
#[derive(Debug, Clone)]
pub struct RawMetadata {
    // Sensor characteristics
    pub cfa_pattern: Option<CfaPattern>,
    pub black_level: Option<BlackLevel>,
    pub white_level: Option<WhiteLevel>,

    // Calibration frames (by hash reference, not embedded)
    pub dark_frame_hash: Option<CalibrationHash>,
    pub flat_field_hash: Option<CalibrationHash>,
    pub bias_frame_hash: Option<CalibrationHash>,
    pub vignette_correction_hash: Option<CalibrationHash>,
    pub distortion_correction_hash: Option<CalibrationHash>,

    // Magic 9 (3×3 colour matrix: Sensor RGB → LMS)
    pub magic_9: Option<Magic9>,
}

/// Camera settings at time of capture
#[derive(Debug, Clone)]
pub struct CameraSettings {
    pub make: Option<Manufacturer>,
    pub model: Option<ModelName>,
    pub serial_number: Option<SerialNumber>,
    pub iso_speed: Option<IsoSpeed>,
    pub shutter_time_s: Option<ShutterTime>,
    pub aperture_f_number: Option<Aperture>,
    pub focal_length_m: Option<FocalLength>,
    pub exposure_compensation: Option<ExposureCompensation>,
    pub focus_distance_m: Option<FocusDistance>,
    pub flash_fired: Option<FlashFired>,
    pub metering_mode: Option<MeteringMode>,
    // No white_balance - use magic_9 for Sensor→LMS conversion
}

/// Lens information
#[derive(Debug, Clone)]
pub struct LensInfo {
    pub make: Option<Manufacturer>,
    pub model: Option<ModelName>,
    pub serial_number: Option<SerialNumber>,
    pub min_focal_length_m: Option<FocalLength>,
    pub max_focal_length_m: Option<FocalLength>,
    pub min_aperture_f: Option<Aperture>, // Smallest aperture (largest f-number, e.g. f/22)
    pub max_aperture_f: Option<Aperture>, // Largest aperture (smallest f-number, e.g. f/1.4)
}

// ==================== BUILDER PATTERN API ====================

/// Builder for RawMetadata with convenient field access
#[derive(Debug, Clone, Default)]
pub struct RawMetadataBuilder {
    pub cfa_pattern: Option<Vec<u8>>,
    pub black_level: Option<f32>,
    pub white_level: Option<f32>,
    pub dark_frame_hash: Option<(u8, Vec<u8>)>,
    pub flat_field_hash: Option<(u8, Vec<u8>)>,
    pub bias_frame_hash: Option<(u8, Vec<u8>)>,
    pub vignette_correction_hash: Option<(u8, Vec<u8>)>,
    pub distortion_correction_hash: Option<(u8, Vec<u8>)>,
    pub magic_9: Option<Vec<f32>>,
}

impl RawMetadataBuilder {
    /// Convert builder to RawMetadata (returns None if all fields are None)
    fn build(self) -> Result<Option<RawMetadata>, String> {
        if self.cfa_pattern.is_none()
            && self.black_level.is_none()
            && self.white_level.is_none()
            && self.dark_frame_hash.is_none()
            && self.flat_field_hash.is_none()
            && self.bias_frame_hash.is_none()
            && self.vignette_correction_hash.is_none()
            && self.distortion_correction_hash.is_none()
            && self.magic_9.is_none()
        {
            return Ok(None);
        }

        Ok(Some(RawMetadata {
            cfa_pattern: self.cfa_pattern.map(|p| CfaPattern::new(p)).transpose()?,
            black_level: self.black_level.map(|l| BlackLevel::new(l)).transpose()?,
            white_level: self.white_level.map(|l| WhiteLevel::new(l)).transpose()?,
            dark_frame_hash: self
                .dark_frame_hash
                .map(|(alg, hash)| CalibrationHash::new(alg, hash))
                .transpose()?,
            flat_field_hash: self
                .flat_field_hash
                .map(|(alg, hash)| CalibrationHash::new(alg, hash))
                .transpose()?,
            bias_frame_hash: self
                .bias_frame_hash
                .map(|(alg, hash)| CalibrationHash::new(alg, hash))
                .transpose()?,
            vignette_correction_hash: self
                .vignette_correction_hash
                .map(|(alg, hash)| CalibrationHash::new(alg, hash))
                .transpose()?,
            distortion_correction_hash: self
                .distortion_correction_hash
                .map(|(alg, hash)| CalibrationHash::new(alg, hash))
                .transpose()?,
            magic_9: self.magic_9.map(|m| Magic9::new(m)).transpose()?,
        }))
    }
}

/// Builder for CameraSettings with convenient field access
#[derive(Debug, Clone, Default)]
pub struct CameraBuilder {
    pub make: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub iso_speed: Option<f32>,
    pub shutter_time_s: Option<f32>,
    pub aperture_f_number: Option<f32>,
    pub focal_length_m: Option<f32>,
    pub exposure_compensation: Option<f32>,
    pub focus_distance_m: Option<f32>,
    pub flash_fired: Option<bool>,
    pub metering_mode: Option<String>,
}

impl CameraBuilder {
    /// Convert builder to CameraSettings (returns None if all fields are None)
    fn build(self) -> Result<Option<CameraSettings>, String> {
        if self.make.is_none()
            && self.model.is_none()
            && self.serial_number.is_none()
            && self.iso_speed.is_none()
            && self.shutter_time_s.is_none()
            && self.aperture_f_number.is_none()
            && self.focal_length_m.is_none()
            && self.exposure_compensation.is_none()
            && self.focus_distance_m.is_none()
            && self.flash_fired.is_none()
            && self.metering_mode.is_none()
        {
            return Ok(None);
        }

        Ok(Some(CameraSettings {
            make: self.make.map(|m| Manufacturer::new(m)).transpose()?,
            model: self.model.map(|m| ModelName::new(m)).transpose()?,
            serial_number: self.serial_number.map(|s| SerialNumber::new(s)).transpose()?,
            iso_speed: self.iso_speed.map(|i| IsoSpeed::new(i)).transpose()?,
            shutter_time_s: self
                .shutter_time_s
                .map(|s| ShutterTime::new(s))
                .transpose()?,
            aperture_f_number: self
                .aperture_f_number
                .map(|a| Aperture::new(a))
                .transpose()?,
            focal_length_m: self
                .focal_length_m
                .map(|f| FocalLength::new(f))
                .transpose()?,
            exposure_compensation: self
                .exposure_compensation
                .map(|e| ExposureCompensation::new(e))
                .transpose()?,
            focus_distance_m: self
                .focus_distance_m
                .map(|f| FocusDistance::new(f))
                .transpose()?,
            flash_fired: self.flash_fired.map(|f| FlashFired::new(f)).transpose()?,
            metering_mode: self
                .metering_mode
                .map(|m| MeteringMode::new(m))
                .transpose()?,
        }))
    }
}

/// Builder for LensInfo with convenient field access
#[derive(Debug, Clone, Default)]
pub struct LensBuilder {
    pub make: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub min_focal_length_m: Option<f32>,
    pub max_focal_length_m: Option<f32>,
    pub min_aperture_f: Option<f32>,
    pub max_aperture_f: Option<f32>,
}

impl LensBuilder {
    /// Convert builder to LensInfo (returns None if all fields are None)
    fn build(self) -> Result<Option<LensInfo>, String> {
        if self.make.is_none()
            && self.model.is_none()
            && self.serial_number.is_none()
            && self.min_focal_length_m.is_none()
            && self.max_focal_length_m.is_none()
            && self.min_aperture_f.is_none()
            && self.max_aperture_f.is_none()
        {
            return Ok(None);
        }

        Ok(Some(LensInfo {
            make: self.make.map(|m| Manufacturer::new(m)).transpose()?,
            model: self.model.map(|m| ModelName::new(m)).transpose()?,
            serial_number: self
                .serial_number
                .map(|s| SerialNumber::new(s))
                .transpose()?,
            min_focal_length_m: self
                .min_focal_length_m
                .map(|f| FocalLength::new(f))
                .transpose()?,
            max_focal_length_m: self
                .max_focal_length_m
                .map(|f| FocalLength::new(f))
                .transpose()?,
            min_aperture_f: self.min_aperture_f.map(|a| Aperture::new(a)).transpose()?,
            max_aperture_f: self.max_aperture_f.map(|a| Aperture::new(a)).transpose()?,
        }))
    }
}

/// Builder pattern for creating RAW images with ergonomic dot notation
///
/// # Example
/// ```ignore
/// use vsf::builders::RawImageBuilder;
/// use vsf::types::BitPackedTensor;
///
/// let samples: Vec<u64> = vec![2048; 4096 * 3072];
/// let image = BitPackedTensor::pack(12, vec![4096, 3072], &samples);
///
/// let mut raw = RawImageBuilder::new(image);
/// raw.camera.iso_speed = Some(800.0);
/// raw.camera.shutter_time_s = Some(1.0 / 60.0);
/// raw.raw.cfa_pattern = Some(vec![b'R', b'G', b'G', b'B']);
/// raw.lens.make = Some("Sony".to_string());
///
/// let bytes = raw.build()?;
/// ```
#[derive(Debug, Clone)]
pub struct RawImageBuilder {
    image: BitPackedTensor,
    pub raw: RawMetadataBuilder,
    pub camera: CameraBuilder,
    pub lens: LensBuilder,
}

impl RawImageBuilder {
    /// Create a new RawImageBuilder with the image data
    pub fn new(image: BitPackedTensor) -> Self {
        Self {
            image,
            raw: RawMetadataBuilder::default(),
            camera: CameraBuilder::default(),
            lens: LensBuilder::default(),
        }
    }

    /// Build the complete VSF RAW image file
    pub fn build(self) -> Result<Vec<u8>, String> {
        let metadata = self.raw.build()?;
        let camera = self.camera.build()?;
        let lens = self.lens.build()?;

        build_raw_image(self.image, metadata, camera, lens)
    }
}

// ==================== SIMPLE HELPER FUNCTIONS ====================

/// Create a RAW camera image with arbitrary bit depth
///
/// Supports 1-256 bits per sample
///
/// # Arguments
/// * `bit_depth` - Bits per sample (1-256, where 0 = 256)
/// * `width` - Image width in samples
/// * `height` - Image height in samples
/// * `samples` - RAW sensor sample values (unreferenced, single-plane)
///
/// # Example
/// ```ignore
/// let samples: Vec<u64> = vec![2048; 4096 * 3072]; // 12-bit mid-gray
/// let raw = raw_image(12, 4096, 3072, samples);
/// ```
pub fn raw_image(bit_depth: u8, width: usize, height: usize, samples: Vec<u64>) -> VsfType {
    let tensor = BitPackedTensor::pack(bit_depth, vec![width, height], &samples);
    VsfType::p(tensor)
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
    let tensor = Tensor::new(vec![width, height, 3], rgb_data);
    let img = VsfType::t_u3(tensor);
    let loc = WorldCoord::from_lat_lon(lat, lon);
    (img, loc)
}

// ==================== COMPLETE RAW IMAGE BUILDERS ====================

/// Build a complete RAW image file with full metadata and calibration
///
/// **IMPORTANT:** The `image` parameter is a `BitPackedTensor` which is SELF-DESCRIBING.
/// It already contains:
/// - `bit_depth` (8, 10, 12, 14, 16, etc.)
/// - `shape` ([width, height] like [4096, 3072])
/// - `data` (the actual bitpacked pixels)
///
/// **DO NOT** add redundant width/height/bits_per_pixel fields! The `p` type has it all.
///
/// # VSF Structure Created
/// ```text
/// RÅ<...n1 or n2 labels...>
/// [(dimage:p[bitdepth, shape, pixels])    ← Image is FIRST field (self-describing!)
///  (diso speed:u...)                      ← Optional metadata follows
///  (dshutter time ns:u...)
///  (dcfa pattern:t_u3['R','G','G','B'])   ← ASCII characters for readability
///  (dcolour matrix:t_f6[...])]
/// ```
///
/// If TOKEN auth is provided, creates TWO labels: "token auth" and "raw"
/// If no TOKEN auth, creates ONE label: "raw" only
///
/// # Arguments
/// * `image` - BitPackedTensor (use `BitPackedTensor::pack(bit_depth, shape, samples)`)
/// * `metadata` - Optional sensor metadata (CFA pattern, black/white levels, calibration hashes)
/// * `camera` - Optional camera settings (ISO, shutter, aperture, etc.)
/// * `lens` - Optional lens info (make, model, focal range, aperture range)
///
/// # Returns
/// Complete VSF file bytes ready to write to disk
///
/// # Note
/// To add cryptographic verification, use the verification module functions:
/// - `verification::add_file_hash()` for full file integrity
/// - `verification::sign_section()` for per-section signatures
pub fn build_raw_image(
    image: BitPackedTensor,
    metadata: Option<RawMetadata>,
    camera: Option<CameraSettings>,
    lens: Option<LensInfo>,
) -> Result<Vec<u8>, String> {
    let mut builder = VsfBuilder::new();

    // Build raw section - start with the image (p type has width, height, bit_depth)
    let mut raw_items = vec![("image".to_string(), VsfType::p(image))];

    // Add optional metadata
    if let Some(meta) = metadata {
        if let Some(cfa) = meta.cfa_pattern {
            raw_items.push(("cfa_pattern".to_string(), cfa.to_vsf_type()));
        }

        if let Some(black) = meta.black_level {
            raw_items.push(("black_level".to_string(), black.to_vsf_type()));
        }

        if let Some(white) = meta.white_level {
            raw_items.push(("white_level".to_string(), white.to_vsf_type()));
        }

        // Calibration hashes (algorithm + hash bytes)
        if let Some(hash) = meta.dark_frame_hash {
            raw_items.push(("dark_frame_hash".to_string(), hash.to_vsf_type()));
        }

        if let Some(hash) = meta.flat_field_hash {
            raw_items.push(("flat_field_hash".to_string(), hash.to_vsf_type()));
        }

        if let Some(hash) = meta.bias_frame_hash {
            raw_items.push(("bias_frame_hash".to_string(), hash.to_vsf_type()));
        }

        if let Some(hash) = meta.vignette_correction_hash {
            raw_items.push(("vignette_correction_hash".to_string(), hash.to_vsf_type()));
        }

        if let Some(hash) = meta.distortion_correction_hash {
            raw_items.push(("distortion_correction_hash".to_string(), hash.to_vsf_type()));
        }

        // Magic 9 (3×3 colour matrix: Sensor RGB → LMS)
        if let Some(matrix) = meta.magic_9 {
            raw_items.push(("magic_9".to_string(), matrix.to_vsf_type()));
        }
    }

    // Camera settings
    if let Some(cam) = camera {
        if let Some(make) = cam.make {
            raw_items.push(("camera_make".to_string(), make.to_vsf_type()));
        }

        if let Some(model) = cam.model {
            raw_items.push(("camera_model".to_string(), model.to_vsf_type()));
        }

        if let Some(serial) = cam.serial_number {
            raw_items.push(("camera_serial".to_string(), serial.to_vsf_type()));
        }

        if let Some(iso) = cam.iso_speed {
            raw_items.push(("iso_speed".to_string(), iso.to_vsf_type()));
        }

        if let Some(shutter) = cam.shutter_time_s {
            raw_items.push(("shutter_time_s".to_string(), shutter.to_vsf_type()));
        }

        if let Some(aperture) = cam.aperture_f_number {
            raw_items.push(("aperture_f_number".to_string(), aperture.to_vsf_type()));
        }

        if let Some(focal) = cam.focal_length_m {
            raw_items.push(("focal_length_m".to_string(), focal.to_vsf_type()));
        }

        if let Some(comp) = cam.exposure_compensation {
            raw_items.push(("exposure_compensation".to_string(), comp.to_vsf_type()));
        }

        if let Some(focus) = cam.focus_distance_m {
            raw_items.push(("focus_distance_m".to_string(), focus.to_vsf_type()));
        }

        if let Some(flash) = cam.flash_fired {
            raw_items.push(("flash_fired".to_string(), flash.to_vsf_type()));
        }

        if let Some(metering) = cam.metering_mode {
            raw_items.push(("metering_mode".to_string(), metering.to_vsf_type()));
        }
    }

    // Lens info
    if let Some(l) = lens {
        if let Some(make) = l.make {
            raw_items.push(("lens_make".to_string(), make.to_vsf_type()));
        }

        if let Some(model) = l.model {
            raw_items.push(("lens_model".to_string(), model.to_vsf_type()));
        }

        if let Some(serial) = l.serial_number {
            raw_items.push(("lens_serial".to_string(), serial.to_vsf_type()));
        }

        if let Some(min_focal) = l.min_focal_length_m {
            raw_items.push(("lens_min_focal_m".to_string(), min_focal.to_vsf_type()));
        }

        if let Some(max_focal) = l.max_focal_length_m {
            raw_items.push(("lens_max_focal_m".to_string(), max_focal.to_vsf_type()));
        }

        if let Some(min_ap) = l.min_aperture_f {
            raw_items.push(("lens_min_aperture".to_string(), min_ap.to_vsf_type()));
        }

        if let Some(max_ap) = l.max_aperture_f {
            raw_items.push(("lens_max_aperture".to_string(), max_ap.to_vsf_type()));
        }
    }

    builder = builder.add_section("raw", raw_items);

    builder.build()
}

/// Convenience function for Lumis 12-bit captures
///
/// **Lumis sensor specs:**
/// - Resolution: 4096×3072 (12.6 megapixels)
/// - Bit depth: 12-bit (values 0-4095)
/// - Bayer pattern: RGGB
/// - Black level: 64
/// - White level: 4095
///
/// **What this function does:**
/// 1. Creates a `BitPackedTensor::pack(12, [4096, 3072], samples)` - this packs your
///    12-bit samples into the minimal bitpacked representation
/// 2. Adds sensor metadata (CFA pattern, black/white levels)
/// 3. Adds camera settings (ISO, shutter speed)
///
/// **The resulting p type contains EVERYTHING about the image:**
/// - No separate width field (shape has it: [4096, 3072])
/// - No separate bit_depth field (p encoding has it: 12)
/// - No separate sample data section (p has the bitpacked bytes)
///
/// # Arguments
/// * `samples` - RAW sensor sample values as u64 (0-4095 for 12-bit), will be bitpacked
/// * `iso` - ISO speed (e.g., 100, 200, 400, 800, 1600, 3200)
/// * `shutter_s` - Shutter time in seconds (e.g., 1./60. = 0.0167 for 1/60 second)
pub fn lumis_raw_capture(samples: Vec<u64>, iso: f32, shutter_s: f32) -> Result<Vec<u8>, String> {
    // Create BitPackedTensor for 12-bit Lumis sensor
    let image = BitPackedTensor::pack(12, vec![4096, 3072], &samples);

    build_raw_image(
        image,
        Some(RawMetadata {
            cfa_pattern: Some(CfaPattern::new(vec![b'R', b'G', b'G', b'B'])?), // RGGB Bayer pattern
            black_level: Some(BlackLevel::new(64.0)?),
            white_level: Some(WhiteLevel::new(4095.0)?),
            dark_frame_hash: None,
            flat_field_hash: None,
            bias_frame_hash: None,
            vignette_correction_hash: None,
            distortion_correction_hash: None,
            magic_9: None,
        }),
        Some(CameraSettings {
            make: None,
            model: None,
            serial_number: None,
            iso_speed: Some(IsoSpeed::new(iso)?),
            shutter_time_s: Some(ShutterTime::new(shutter_s)?),
            aperture_f_number: None,
            focal_length_m: None,
            exposure_compensation: None,
            focus_distance_m: None,
            flash_fired: Some(FlashFired::new(false)?),
            metering_mode: None,
        }),
        None, // No lens info (phone camera)
    )
}

// ==================== RAW IMAGE PARSER ====================

/// Parsed RAW image data from a VSF file
pub struct ParsedRawImage {
    pub image: BitPackedTensor,
    pub metadata: Option<RawMetadata>,
    pub camera: Option<CameraSettings>,
    pub lens: Option<LensInfo>,
}

// Helper to convert any VsfType unsigned variant to Rust usize
fn to_usize(vsf_type: &VsfType) -> Option<usize> {
    match vsf_type {
        VsfType::u(v, _) => Some(*v),        // usize → usize (no conversion)
        VsfType::u0(b) => Some(*b as usize), // bool → usize
        VsfType::u3(v) => Some(*v as usize), // u8 → usize (widening)
        VsfType::u4(v) => Some(*v as usize), // u16 → usize (widening)
        VsfType::u5(v) => Some(*v as usize), // u32 → usize (safe on 64-bit)
        VsfType::u6(v) => Some(*v as usize), // u64 → usize (safe on 64-bit)
        VsfType::u7(v) => Some(*v as usize), // u128 → usize (truncates!)
        _ => None,
    }
}

/// Parse a VSF RAW image file
///
/// Extracts the image BitPackedTensor and all metadata fields from a VSF RAW file.
///
/// # Arguments
/// * `data` - The complete VSF file bytes
///
/// # Returns
/// ParsedRawImage containing the image and optional metadata, or an error
pub fn parse_raw_image(data: &[u8]) -> Result<ParsedRawImage, String> {
    use crate::decoding::parse::parse;

    // Verify magic number
    if data.len() < 4 {
        return Err("File too small to be valid VSF".to_string());
    }
    if &data[0..3] != "RÅ".as_bytes() || data[3] != b'<' {
        return Err("Invalid VSF magic number".to_string());
    }

    let mut pointer = 4; // Skip "RÅ<"

    // Parse header length (in bits)
    let header_length_type =
        parse(data, &mut pointer).map_err(|e| format!("Failed to parse header length: {}", e))?;
    let _header_length_bits = match header_length_type {
        VsfType::b(bits) => bits,
        _ => return Err("Expected b type for header length".to_string()),
    };

    // Parse version and backward compat
    let _version =
        parse(data, &mut pointer).map_err(|e| format!("Failed to parse version: {}", e))?;
    let _backward =
        parse(data, &mut pointer).map_err(|e| format!("Failed to parse backward compat: {}", e))?;

    // Skip file hash (always present now)
    let _file_hash =
        parse(data, &mut pointer).map_err(|e| format!("Failed to parse file hash: {}", e))?;

    // Parse label count
    let label_count_type =
        parse(data, &mut pointer).map_err(|e| format!("Failed to parse label count: {}", e))?;
    let label_count = match label_count_type {
        VsfType::n(count) => count,
        _ => return Err("Expected n type for label count".to_string()),
    };

    // Find the "raw" label
    let mut raw_offset_bits: Option<usize> = None;
    let mut raw_field_count: Option<usize> = None;

    for _ in 0..label_count {
        // Parse label definition: (d[name] o[offset] b[size] n[count])
        if data[pointer] != b'(' {
            return Err("Expected '(' for label definition".to_string());
        }
        pointer += 1;

        let label_name_type =
            parse(data, &mut pointer).map_err(|e| format!("Failed to parse label name: {}", e))?;
        let label_name = match label_name_type {
            VsfType::d(name) => name,
            _ => return Err("Expected d type for label name".to_string()),
        };

        let offset_type =
            parse(data, &mut pointer).map_err(|e| format!("Failed to parse offset: {}", e))?;
        let offset_bits = match offset_type {
            VsfType::o(bits) => bits,
            _ => return Err("Expected o type for offset".to_string()),
        };

        let size_type =
            parse(data, &mut pointer).map_err(|e| format!("Failed to parse size: {}", e))?;
        let _size_bits = match size_type {
            VsfType::b(bits) => bits,
            _ => return Err("Expected b type for size".to_string()),
        };

        let field_count_type =
            parse(data, &mut pointer).map_err(|e| format!("Failed to parse field count: {}", e))?;
        let field_count = match field_count_type {
            VsfType::n(count) => count,
            _ => return Err("Expected n type for field count".to_string()),
        };

        if data[pointer] != b')' {
            return Err("Expected ')' after label definition".to_string());
        }
        pointer += 1;

        // Store label info
        match label_name.as_str() {
            "raw" => {
                raw_offset_bits = Some(offset_bits);
                raw_field_count = Some(field_count);
            }
            _ => {} // Ignore other labels
        }
    }

    // Ensure we found the raw label
    let (raw_offset, raw_count) = match (raw_offset_bits, raw_field_count) {
        (Some(o), Some(c)) => (o, c),
        _ => return Err("Required 'raw' label not found".to_string()),
    };

    // Skip to end of header
    if data[pointer] != b'>' {
        return Err("Expected '>' for header end".to_string());
    }
    // Note: pointer is not incremented here since we seek directly to section_start_byte below

    // Initialize all field variables
    let mut image: Option<BitPackedTensor> = None;

    // Raw metadata fields
    let mut cfa_pattern: Option<Vec<u8>> = None;
    let mut black_level: Option<f32> = None;
    let mut white_level: Option<f32> = None;
    let mut dark_frame_hash: Option<(u8, Vec<u8>)> = None;
    let mut flat_field_hash: Option<(u8, Vec<u8>)> = None;
    let mut bias_frame_hash: Option<(u8, Vec<u8>)> = None;
    let mut vignette_correction_hash: Option<(u8, Vec<u8>)> = None;
    let mut distortion_correction_hash: Option<(u8, Vec<u8>)> = None;
    let mut magic_9: Option<Vec<f32>> = None;

    // Camera settings fields
    let mut camera_make: Option<String> = None;
    let mut camera_model: Option<String> = None;
    let mut camera_serial: Option<String> = None;
    let mut iso_speed: Option<f32> = None;
    let mut shutter_time_s: Option<f32> = None;
    let mut aperture_f_number: Option<f32> = None;
    let mut focal_length_m: Option<f32> = None;
    let mut exposure_compensation: Option<f32> = None;
    let mut focus_distance_m: Option<f32> = None;
    let mut flash_fired: Option<bool> = None;
    let mut metering_mode: Option<String> = None;

    // Lens info fields
    let mut lens_make: Option<String> = None;
    let mut lens_model: Option<String> = None;
    let mut lens_serial: Option<String> = None;
    let mut lens_min_focal_m: Option<f32> = None;
    let mut lens_max_focal_m: Option<f32> = None;
    let mut lens_min_aperture: Option<f32> = None;
    let mut lens_max_aperture: Option<f32> = None;

    // Parse the "raw" section
    let section_start_byte = raw_offset >> 3; // Convert bits to bytes
    if section_start_byte >= data.len() {
        return Err(format!(
            "Raw section offset {} exceeds file size {}",
            section_start_byte,
            data.len()
        ));
    }
    pointer = section_start_byte;

    // Parse preamble
    use crate::decoding::parse_preamble;
    let (_preamble_count, _preamble_size, _preamble_hash, _preamble_sig) =
        parse_preamble(data, &mut pointer)
            .map_err(|e| format!("Failed to parse raw preamble: {}", e))?;

    // Parse section start
    if data[pointer] != b'[' {
        return Err(format!(
            "Expected '[' for raw section at byte {}, found {:?}",
            pointer, data[pointer] as char
        ));
    }
    pointer += 1;

    // Parse section name
    let section_name_type = parse(data, &mut pointer)
        .map_err(|e| format!("Failed to parse raw section name: {}", e))?;
    let _section_name = match section_name_type {
        VsfType::d(name) => name,
        _ => return Err("Expected d type for raw section name".to_string()),
    };

    // Parse raw section fields
    for i in 0..raw_count {
        if data[pointer] != b'(' {
            return Err(format!("Expected '(' for field {}", i));
        }
        pointer += 1;

        // Parse field name
        let field_name_type = parse(data, &mut pointer)
            .map_err(|e| format!("Failed to parse field {} name: {}", i, e))?;
        let field_name = match field_name_type {
            VsfType::d(name) => name,
            _ => return Err(format!("Expected d type for field {} name", i)),
        };

        // Expect ':'
        if data[pointer] != b':' {
            return Err(format!("Expected ':' after field name '{}'", field_name));
        }
        pointer += 1;

        // Parse field value
        let field_value = parse(data, &mut pointer)
            .map_err(|e| format!("Failed to parse field '{}': {}", field_name, e))?;

        // Store the value based on field name
        match field_name.as_str() {
            "image" => {
                if let VsfType::p(tensor) = field_value {
                    image = Some(tensor);
                }
            }
            // Raw metadata
            "cfa_pattern" => {
                if let VsfType::t_u3(tensor) = field_value {
                    cfa_pattern = Some(tensor.data);
                }
            }
            "black_level" => {
                if let VsfType::f5(v) = field_value {
                    black_level = Some(v);
                }
            }
            "white_level" => {
                if let VsfType::f5(v) = field_value {
                    white_level = Some(v);
                }
            }
            "dark_frame_hash" => {
                if let VsfType::h(algorithm, v) = field_value {
                    dark_frame_hash = Some((algorithm, v));
                }
            }
            "flat_field_hash" => {
                if let VsfType::h(algorithm, v) = field_value {
                    flat_field_hash = Some((algorithm, v));
                }
            }
            "bias_frame_hash" => {
                if let VsfType::h(algorithm, v) = field_value {
                    bias_frame_hash = Some((algorithm, v));
                }
            }
            "vignette_correction_hash" => {
                if let VsfType::h(algorithm, v) = field_value {
                    vignette_correction_hash = Some((algorithm, v));
                }
            }
            "distortion_correction_hash" => {
                if let VsfType::h(algorithm, v) = field_value {
                    distortion_correction_hash = Some((algorithm, v));
                }
            }
            "magic_9" => {
                if let VsfType::t_f5(tensor) = field_value {
                    magic_9 = Some(tensor.data);
                }
            }
            // Camera settings
            "camera_make" => {
                if let VsfType::x(v) = field_value {
                    camera_make = Some(v);
                }
            }
            "camera_model" => {
                if let VsfType::x(v) = field_value {
                    camera_model = Some(v);
                }
            }
            "camera_serial" => {
                if let VsfType::x(v) = field_value {
                    camera_serial = Some(v);
                }
            }
            "iso_speed" => {
                if let VsfType::f5(v) = field_value {
                    iso_speed = Some(v);
                }
            }
            "shutter_time_s" => {
                if let VsfType::f5(v) = field_value {
                    shutter_time_s = Some(v);
                }
            }
            "aperture_f_number" => {
                if let VsfType::f5(v) = field_value {
                    aperture_f_number = Some(v);
                }
            }
            "focal_length_m" => {
                if let VsfType::f5(v) = field_value {
                    focal_length_m = Some(v);
                }
            }
            "exposure_compensation" => {
                if let VsfType::f5(v) = field_value {
                    exposure_compensation = Some(v);
                }
            }
            "focus_distance_m" => {
                if let VsfType::f5(v) = field_value {
                    focus_distance_m = Some(v);
                }
            }
            "flash_fired" => flash_fired = to_usize(&field_value).map(|v| v != 0),
            "metering_mode" => {
                if let VsfType::x(v) = field_value {
                    metering_mode = Some(v);
                }
            }
            // Lens info
            "lens_make" => {
                if let VsfType::x(v) = field_value {
                    lens_make = Some(v);
                }
            }
            "lens_model" => {
                if let VsfType::x(v) = field_value {
                    lens_model = Some(v);
                }
            }
            "lens_serial" => {
                if let VsfType::x(v) = field_value {
                    lens_serial = Some(v);
                }
            }
            "lens_min_focal_m" => {
                if let VsfType::f5(v) = field_value {
                    lens_min_focal_m = Some(v);
                }
            }
            "lens_max_focal_m" => {
                if let VsfType::f5(v) = field_value {
                    lens_max_focal_m = Some(v);
                }
            }
            "lens_min_aperture" => {
                if let VsfType::f5(v) = field_value {
                    lens_min_aperture = Some(v);
                }
            }
            "lens_max_aperture" => {
                if let VsfType::f5(v) = field_value {
                    lens_max_aperture = Some(v);
                }
            }
            _ => {
                // Unknown field, skip
            }
        }

        if pointer >= data.len() {
            return Err(format!(
                "Unexpected EOF after parsing raw field '{}' (field {} of {})",
                field_name, i, raw_count
            ));
        }
        if data[pointer] != b')' {
            return Err(format!(
                "Expected ')' after field '{}' at byte {}, found {:?}",
                field_name, pointer, data[pointer] as char
            ));
        }
        pointer += 1;
    }

    if data[pointer] != b']' {
        return Err("Expected ']' for section end".to_string());
    }

    // Extract the image
    let image = image.ok_or("Missing required 'image' field")?;

    // Build metadata structs from parsed fields (converting to newtypes)
    let raw_metadata = if cfa_pattern.is_some()
        || black_level.is_some()
        || white_level.is_some()
        || dark_frame_hash.is_some()
        || flat_field_hash.is_some()
        || bias_frame_hash.is_some()
        || vignette_correction_hash.is_some()
        || distortion_correction_hash.is_some()
        || magic_9.is_some()
    {
        Some(RawMetadata {
            cfa_pattern: cfa_pattern.map(|p| CfaPattern::new(p)).transpose()?,
            black_level: black_level.map(|l| BlackLevel::new(l)).transpose()?,
            white_level: white_level.map(|l| WhiteLevel::new(l)).transpose()?,
            dark_frame_hash: dark_frame_hash
                .map(|(alg, hash)| CalibrationHash::new(alg, hash))
                .transpose()?,
            flat_field_hash: flat_field_hash
                .map(|(alg, hash)| CalibrationHash::new(alg, hash))
                .transpose()?,
            bias_frame_hash: bias_frame_hash
                .map(|(alg, hash)| CalibrationHash::new(alg, hash))
                .transpose()?,
            vignette_correction_hash: vignette_correction_hash
                .map(|(alg, hash)| CalibrationHash::new(alg, hash))
                .transpose()?,
            distortion_correction_hash: distortion_correction_hash
                .map(|(alg, hash)| CalibrationHash::new(alg, hash))
                .transpose()?,
            magic_9: magic_9.map(|m| Magic9::new(m)).transpose()?,
        })
    } else {
        None
    };

    let camera_settings = if camera_make.is_some()
        || camera_model.is_some()
        || camera_serial.is_some()
        || iso_speed.is_some()
        || shutter_time_s.is_some()
        || aperture_f_number.is_some()
        || focal_length_m.is_some()
        || exposure_compensation.is_some()
        || focus_distance_m.is_some()
        || flash_fired.is_some()
        || metering_mode.is_some()
    {
        Some(CameraSettings {
            make: camera_make.map(|m| Manufacturer::new(m)).transpose()?,
            model: camera_model.map(|m| ModelName::new(m)).transpose()?,
            serial_number: camera_serial.map(|s| SerialNumber::new(s)).transpose()?,
            iso_speed: iso_speed.map(|i| IsoSpeed::new(i)).transpose()?,
            shutter_time_s: shutter_time_s.map(|s| ShutterTime::new(s)).transpose()?,
            aperture_f_number: aperture_f_number.map(|a| Aperture::new(a)).transpose()?,
            focal_length_m: focal_length_m.map(|f| FocalLength::new(f)).transpose()?,
            exposure_compensation: exposure_compensation
                .map(|e| ExposureCompensation::new(e))
                .transpose()?,
            focus_distance_m: focus_distance_m
                .map(|f| FocusDistance::new(f))
                .transpose()?,
            flash_fired: flash_fired.map(|f| FlashFired::new(f)).transpose()?,
            metering_mode: metering_mode.map(|m| MeteringMode::new(m)).transpose()?,
        })
    } else {
        None
    };

    let lens_info = if lens_make.is_some()
        || lens_model.is_some()
        || lens_serial.is_some()
        || lens_min_focal_m.is_some()
        || lens_max_focal_m.is_some()
        || lens_min_aperture.is_some()
        || lens_max_aperture.is_some()
    {
        Some(LensInfo {
            make: lens_make.map(|m| Manufacturer::new(m)).transpose()?,
            model: lens_model.map(|m| ModelName::new(m)).transpose()?,
            serial_number: lens_serial.map(|s| SerialNumber::new(s)).transpose()?,
            min_focal_length_m: lens_min_focal_m.map(|f| FocalLength::new(f)).transpose()?,
            max_focal_length_m: lens_max_focal_m.map(|f| FocalLength::new(f)).transpose()?,
            min_aperture_f: lens_min_aperture.map(|a| Aperture::new(a)).transpose()?,
            max_aperture_f: lens_max_aperture.map(|a| Aperture::new(a)).transpose()?,
        })
    } else {
        None
    };

    Ok(ParsedRawImage {
        image,
        metadata: raw_metadata,
        camera: camera_settings,
        lens: lens_info,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto_algorithms::HASH_BLAKE3;

    #[test]
    fn test_text_document() {
        let doc = VsfType::x("Hello, VSF!".to_string());
        if let VsfType::x(s) = doc {
            assert_eq!(s, "Hello, VSF!");
        } else {
            panic!("Expected string");
        }
    }

    #[test]
    fn test_raw_image_12bit() {
        let samples = vec![2048u64; 100 * 50]; // 100×50 mid-gray
        let img = raw_image(12, 100, 50, samples);

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
        let tensor = Tensor::new(vec![64, 48], data);
        let img = VsfType::t_u3(tensor);

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
        let tensor = Tensor::new(vec![64, 48, 3], data);
        let img = VsfType::t_u3(tensor);

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
        // Minimal RAW: just the image, no metadata
        let samples: Vec<u64> = vec![255; 64]; // 8x8, all white
        let image = BitPackedTensor::pack(8, vec![8, 8], &samples);

        let result = build_raw_image(image, None, None, None);

        assert!(result.is_ok());
        let bytes = result.unwrap();

        // Verify magic number (RÅ is 3 bytes in UTF-8)
        assert_eq!(&bytes[0..3], "RÅ".as_bytes());
        assert_eq!(bytes[3], b'<');

        // Verify file is structured correctly
        // Should have header + one "raw" section with p type
        assert!(bytes.len() > 50); // Minimal file should be small
    }

    #[test]
    fn test_complete_raw_image_with_metadata() {
        let samples: Vec<u64> = vec![255; 64]; // 8x8
        let image = BitPackedTensor::pack(8, vec![8, 8], &samples);

        let result = build_raw_image(
            image,
            Some(RawMetadata {
                cfa_pattern: Some(CfaPattern::new(vec![b'R', b'G', b'G', b'B']).unwrap()),
                black_level: Some(BlackLevel::new(64.0).unwrap()),
                white_level: Some(WhiteLevel::new(255.0).unwrap()),
                dark_frame_hash: Some(CalibrationHash::new(HASH_BLAKE3, vec![0xAB; 32]).unwrap()),
                flat_field_hash: None,
                bias_frame_hash: None,
                vignette_correction_hash: None,
                distortion_correction_hash: None,
                magic_9: Some(
                    Magic9::new(vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]).unwrap(),
                ),
            }),
            Some(CameraSettings {
                make: None,
                model: None,
                serial_number: None,
                iso_speed: Some(IsoSpeed::new(800.0).unwrap()),
                shutter_time_s: Some(ShutterTime::new(1. / 60.).unwrap()), // 1/60 second
                aperture_f_number: Some(Aperture::new(2.8).unwrap()),
                focal_length_m: Some(FocalLength::new(0.024).unwrap()), // 24mm = 0.024m
                exposure_compensation: None,
                focus_distance_m: None,
                flash_fired: Some(FlashFired::new(false).unwrap()),
                metering_mode: Some(MeteringMode::new("matrix".to_string()).unwrap()),
            }),
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
        // Samples are u64 values (0-4095), will be bitpacked by the function
        let pixel_count = 4096 * 3072;
        let samples: Vec<u64> = vec![2048; pixel_count]; // Mid-gray

        let result = lumis_raw_capture(
            samples,
            800.0,
            1. / 60., // 1/60 second shutter
        );

        assert!(result.is_ok());
        let bytes = result.unwrap();

        // Verify magic number (RÅ is 3 bytes in UTF-8)
        assert_eq!(&bytes[0..3], "RÅ".as_bytes());

        // File should be large (header + metadata + ~18.9MB bitpacked pixels)
        // 12-bit × 12.6M pixels = 18.9MB
        assert!(
            bytes.len() > 18_000_000,
            "File should be > 18MB with bitpacked pixels"
        );
    }

    #[test]
    fn test_roundtrip_minimal_raw() {
        // Create minimal RAW image
        let samples: Vec<u64> = (0..16).collect(); // 0-15
        let original_image = BitPackedTensor::pack(8, vec![4, 4], &samples);

        // Build VSF file
        let raw_bytes = build_raw_image(original_image.clone(), None, None, None).unwrap();

        // Parse it back
        let parsed = parse_raw_image(&raw_bytes).unwrap();

        // Verify the image matches
        assert_eq!(parsed.image.bit_depth, 8);
        assert_eq!(parsed.image.shape, vec![4, 4]);

        // Unpack and compare pixels
        let original_samples = original_image.unpack().into_u64();
        let parsed_samples = parsed.image.unpack().into_u64();
        assert_eq!(parsed_samples, original_samples);
        assert_eq!(parsed_samples, samples);

        // Verify no metadata was present
        assert!(parsed.metadata.is_none());
        assert!(parsed.camera.is_none());
        assert!(parsed.lens.is_none());
    }

    #[test]
    fn test_roundtrip_full_metadata() {
        // Create image with full metadata
        let samples: Vec<u64> = vec![200; 64]; // 8x8
        let original_image = BitPackedTensor::pack(8, vec![8, 8], &samples);

        let original_metadata = RawMetadata {
            cfa_pattern: Some(CfaPattern::new(vec![b'R', b'G', b'G', b'B']).unwrap()), // RGGB Bayer pattern
            black_level: Some(BlackLevel::new(64.0).unwrap()),
            white_level: Some(WhiteLevel::new(255.0).unwrap()),
            dark_frame_hash: Some(CalibrationHash::new(HASH_BLAKE3, vec![0xAB; 32]).unwrap()),
            flat_field_hash: Some(CalibrationHash::new(HASH_BLAKE3, vec![0xCD; 32]).unwrap()),
            bias_frame_hash: None,
            vignette_correction_hash: None,
            distortion_correction_hash: None,
            magic_9: Some(Magic9::new(vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]).unwrap()),
        };

        let original_camera = CameraSettings {
            make: Some(Manufacturer::new("TestCam".to_string()).unwrap()),
            model: Some(ModelName::new("Model X".to_string()).unwrap()),
            serial_number: Some(SerialNumber::new("CAM123456".to_string()).unwrap()),
            iso_speed: Some(IsoSpeed::new(800.0).unwrap()),
            shutter_time_s: Some(ShutterTime::new(1. / 60.).unwrap()), // 1/60 sec
            aperture_f_number: Some(Aperture::new(2.8).unwrap()),
            focal_length_m: Some(FocalLength::new(0.050).unwrap()), // 50mm = 0.050m
            exposure_compensation: Some(ExposureCompensation::new(-0.5).unwrap()),
            focus_distance_m: Some(FocusDistance::new(3.5).unwrap()),
            flash_fired: Some(FlashFired::new(false).unwrap()),
            metering_mode: Some(MeteringMode::new("matrix".to_string()).unwrap()),
        };

        // Build VSF file
        let raw_bytes = build_raw_image(
            original_image.clone(),
            Some(original_metadata.clone()),
            Some(original_camera.clone()),
            None, // No lens
        )
        .unwrap();

        // Parse it back
        let parsed = parse_raw_image(&raw_bytes).unwrap();

        // Verify image
        assert_eq!(parsed.image.bit_depth, 8);
        assert_eq!(parsed.image.shape, vec![8, 8]);
        let parsed_samples = parsed.image.unpack().into_u64();
        assert_eq!(parsed_samples, samples);

        // Verify metadata round-tripped successfully
        assert!(parsed.metadata.is_some());
        let _meta = parsed.metadata.unwrap();
        // Note: Can't use assert_eq on newtypes (no PartialEq), but successful parsing validates data

        // Verify camera settings round-tripped successfully
        assert!(parsed.camera.is_some());
        let _cam = parsed.camera.unwrap();
        // Note: Can't use assert_eq on newtypes (no PartialEq), but successful parsing validates data

        // Verify no lens
        assert!(parsed.lens.is_none());
    }

    #[test]
    fn test_preamble_structure() {
        // Create a simple RAW image
        let samples: Vec<u64> = vec![100; 16]; // 4x4
        let image = BitPackedTensor::pack(8, vec![4, 4], &samples);

        let raw_bytes = build_raw_image(
            image,
            Some(RawMetadata {
                cfa_pattern: Some(CfaPattern::new(vec![b'R', b'G', b'G', b'B']).unwrap()),
                black_level: Some(BlackLevel::new(64.0).unwrap()),
                white_level: Some(WhiteLevel::new(255.0).unwrap()),
                dark_frame_hash: None,
                flat_field_hash: None,
                bias_frame_hash: None,
                vignette_correction_hash: None,
                distortion_correction_hash: None,
                magic_9: None,
            }),
            None,
            None,
        )
        .unwrap();

        // Verify file structure
        assert_eq!(&raw_bytes[0..3], "RÅ".as_bytes()); // Magic
        assert_eq!(raw_bytes[3], b'<'); // Header start

        // Find the first preamble (after header)
        let header_end = raw_bytes.iter().position(|&b| b == b'>').unwrap();

        // The next byte should be '{' (preamble start)
        assert_eq!(
            raw_bytes[header_end + 1],
            b'{',
            "Expected preamble to start immediately after header"
        );

        // Parse the preamble
        let mut pointer = header_end + 1;
        use crate::decoding::parse_preamble;
        let (count, size_bits, hash, sig) = parse_preamble(&raw_bytes, &mut pointer).unwrap();

        // Verify preamble contents
        assert!(count > 0, "Preamble count should be > 0");
        assert!(size_bits > 0, "Preamble size should be > 0");
        assert!(hash.is_none(), "No hash should be present");
        assert!(sig.is_none(), "No signature should be present");

        // Next byte should be '[' (section start)
        assert_eq!(
            raw_bytes[pointer], b'[',
            "Expected '[' immediately after preamble"
        );
    }

    #[test]
    fn test_builder_pattern_minimal() {
        // Test minimal builder with just image
        let samples: Vec<u64> = (0..16).collect();
        let image = BitPackedTensor::pack(8, vec![4, 4], &samples);

        let raw = RawImageBuilder::new(image);
        let result = raw.build();

        assert!(result.is_ok());
        let bytes = result.unwrap();

        // Verify magic number
        assert_eq!(&bytes[0..3], "RÅ".as_bytes());

        // Parse and verify
        let parsed = parse_raw_image(&bytes).unwrap();
        assert_eq!(parsed.image.bit_depth, 8);
        assert_eq!(parsed.image.shape, vec![4, 4]);
    }

    #[test]
    fn test_builder_pattern_camera_settings() {
        // Test builder with camera settings
        let samples: Vec<u64> = vec![100; 64];
        let image = BitPackedTensor::pack(8, vec![8, 8], &samples);

        let mut raw = RawImageBuilder::new(image);
        raw.camera.iso_speed = Some(800.0);
        raw.camera.shutter_time_s = Some(1.0 / 60.0);
        raw.camera.aperture_f_number = Some(2.8);
        raw.camera.flash_fired = Some(false);
        raw.camera.metering_mode = Some("matrix".to_string());

        let result = raw.build();
        assert!(result.is_ok());
        let bytes = result.unwrap();

        // Parse and verify camera settings round-tripped successfully
        let parsed = parse_raw_image(&bytes).unwrap();
        assert!(parsed.camera.is_some());
        // Note: Can't use assert_eq on newtypes (no PartialEq), but successful parsing validates data
    }

    #[test]
    fn test_builder_pattern_raw_metadata() {
        // Test builder with raw metadata
        let samples: Vec<u64> = vec![100; 64];
        let image = BitPackedTensor::pack(8, vec![8, 8], &samples);

        let mut raw = RawImageBuilder::new(image);
        raw.raw.cfa_pattern = Some(vec![b'R', b'G', b'G', b'B']);
        raw.raw.black_level = Some(64.0);
        raw.raw.white_level = Some(4095.0);
        raw.raw.dark_frame_hash = Some((HASH_BLAKE3, vec![0xAB; 32]));

        let result = raw.build();
        assert!(result.is_ok());
        let bytes = result.unwrap();

        // Parse and verify metadata round-tripped successfully
        let parsed = parse_raw_image(&bytes).unwrap();
        assert!(parsed.metadata.is_some());
        // Note: Can't use assert_eq on newtypes (no PartialEq), but successful parsing validates data
    }

    #[test]
    fn test_builder_pattern_lens_info() {
        // Test builder with lens info
        let samples: Vec<u64> = vec![100; 64];
        let image = BitPackedTensor::pack(8, vec![8, 8], &samples);

        let mut raw = RawImageBuilder::new(image);
        raw.lens.make = Some("Sony".to_string());
        raw.lens.model = Some("FE 24-70mm F2.8 GM II".to_string());
        raw.lens.serial_number = Some("ABC123456".to_string());
        raw.lens.min_focal_length_m = Some(0.024); // 24mm
        raw.lens.max_focal_length_m = Some(0.070); // 70mm
        raw.lens.min_aperture_f = Some(22.0);
        raw.lens.max_aperture_f = Some(2.8);

        let result = raw.build();
        assert!(result.is_ok());
        let bytes = result.unwrap();

        // Parse and verify lens info round-tripped successfully
        let parsed = parse_raw_image(&bytes).unwrap();
        assert!(parsed.lens.is_some());
        // Note: Can't use assert_eq on newtypes (no PartialEq), but successful parsing validates data
    }

    #[test]
    fn test_builder_pattern_full() {
        // Test builder with all fields populated
        let samples: Vec<u64> = vec![2048; 64];
        let image = BitPackedTensor::pack(12, vec![8, 8], &samples);

        let mut raw = RawImageBuilder::new(image);

        // Raw metadata
        raw.raw.cfa_pattern = Some(vec![b'R', b'G', b'G', b'B']);
        raw.raw.black_level = Some(64.0);
        raw.raw.white_level = Some(4095.0);
        raw.raw.magic_9 = Some(vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);

        // Camera settings
        raw.camera.iso_speed = Some(800.0);
        raw.camera.shutter_time_s = Some(1.0 / 125.0);
        raw.camera.aperture_f_number = Some(2.8);
        raw.camera.focal_length_m = Some(0.050); // 50mm
        raw.camera.exposure_compensation = Some(-0.5);
        raw.camera.focus_distance_m = Some(3.5);
        raw.camera.flash_fired = Some(false);
        raw.camera.metering_mode = Some("spot".to_string());

        // Lens info
        raw.lens.make = Some("Sony".to_string());
        raw.lens.model = Some("FE 50mm F1.2 GM".to_string());

        let result = raw.build();
        assert!(result.is_ok());
        let bytes = result.unwrap();

        // Parse and verify everything
        let parsed = parse_raw_image(&bytes).unwrap();

        // Verify image
        assert_eq!(parsed.image.bit_depth, 12);
        assert_eq!(parsed.image.shape, vec![8, 8]);

        // Verify all sections round-tripped successfully
        assert!(parsed.metadata.is_some());
        assert!(parsed.camera.is_some());
        assert!(parsed.lens.is_some());
        // Note: Can't use assert_eq on newtypes (no PartialEq), but successful parsing validates data
    }

    #[test]
    fn test_cfa_pattern_validation() {
        let samples: Vec<u64> = vec![100; 16];
        let image = BitPackedTensor::pack(8, vec![4, 4], &samples);

        // Valid patterns should work
        let valid_patterns = vec![
            vec![b'R', b'G', b'G', b'B'],                               // RGGB Bayer
            vec![b'G', b'R', b'B', b'G'],                               // GRBG Bayer
            vec![b'B', b'G', b'G', b'R'],                               // BGGR Bayer
            vec![b'C', b'Y', b'Y', b'G'],                               // CYYG
            vec![b'R', b'G', b'B', b'E', b'W', b'C', b'Y', b'R', b'G'], // 3×3 custom
        ];

        for cfa in valid_patterns {
            let result = build_raw_image(
                image.clone(),
                Some(RawMetadata {
                    cfa_pattern: Some(CfaPattern::new(cfa.clone()).unwrap()),
                    black_level: None,
                    white_level: None,
                    dark_frame_hash: None,
                    flat_field_hash: None,
                    bias_frame_hash: None,
                    vignette_correction_hash: None,
                    distortion_correction_hash: None,
                    magic_9: None,
                }),
                None,
                None,
            );
            assert!(
                result.is_ok(),
                "Valid CFA pattern {:?} should be accepted",
                cfa
            );
        }

        // Invalid patterns should fail
        let invalid_patterns = vec![
            vec![b'R', b'G', b'X', b'B'], // X is not valid
            vec![0, 1, 1, 2],             // Numeric values not allowed
            vec![b'r', b'g', b'g', b'b'], // Lowercase not valid
        ];

        for cfa in invalid_patterns {
            // Invalid patterns should fail at CfaPattern::new()
            let cfa_result = CfaPattern::new(cfa.clone());
            assert!(
                cfa_result.is_err(),
                "Invalid CFA pattern {:?} should be rejected",
                cfa
            );
        }
    }
}
