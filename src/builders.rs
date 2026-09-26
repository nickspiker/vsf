//! High-level builders for common VSF use cases
//!
//! This module provides constructors for complex data types that require packaging multiple VSF primitives together:
//! - GPS coordinate conversions (lat/lon → WorldCoord)
//! - RAW camera images with metadata
//!
//! For simple types, use VsfType directly:
//! - Text: `VsfType::x("Hello".to_string())`
//! - Images: `VsfType::p(BitPackedTensor::pack(12, vec![w, h], &samples))`
//! - Tensors: `VsfType::t_u3(Tensor::new(vec![w, h], data))`
//!
//! # Examples ∞
//! ```ignore
//! use vsf::builders::*; use vsf::types::*;
//!
//! // RAW camera image (12-bit sensor) let raw = raw_image(12, 4096, 3072, pixel_data);

use crate::prelude::*;
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

/// Sensor black level (digital zero point), in raw sample units (DN)
/// WHY integer: a black level is a sample value, and samples are integers; an f32 here was a float at rest for a count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlackLevel(u64);

impl BlackLevel {
    pub fn new(level: u64) -> Result<Self, String> {
        Ok(BlackLevel(level))
    }

    pub fn dn(self) -> u64 {
        self.0
    }

    pub fn to_values(self) -> Vec<VsfType> {
        vec![uint(self.0)]
    }

    pub fn from_values(values: &[VsfType]) -> Result<Self, String> {
        Self::new(one_u64(values, "black level")?)
    }
}

/// Sensor white level (saturation point), in raw sample units (DN)
/// A white level of zero is refused: it would leave no range above black, and "unknown" is an absent field, never 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WhiteLevel(u64);

impl WhiteLevel {
    pub fn new(level: u64) -> Result<Self, String> {
        if level == 0 {
            return Err("White level must be positive".to_string());
        }
        Ok(WhiteLevel(level))
    }

    pub fn dn(self) -> u64 {
        self.0
    }

    pub fn to_values(self) -> Vec<VsfType> {
        vec![uint(self.0)]
    }

    pub fn from_values(values: &[VsfType]) -> Result<Self, String> {
        Self::new(one_u64(values, "white level")?)
    }
}

/// Black must sit strictly below white when both are known, or the sensor has no usable range.
fn check_levels(black: Option<BlackLevel>, white: Option<WhiteLevel>) -> Result<(), String> {
    match (black, white) {
        (Some(b), Some(w)) if b.0 >= w.0 => Err(format!(
            "Black level {} must be below white level {}",
            b.0, w.0
        )),
        _ => Ok(()),
    }
}

/// Hash reference to a calibration frame (dark frame, flat field, etc.)
#[derive(Debug, Clone)]
pub struct CalibrationHash(VsfType);

impl CalibrationHash {
    pub fn new(algorithm: u8, hash: Vec<u8>) -> Result<Self, String> {
        use crate::crypto_algorithms::{HASH_BLAKE3, HASH_SHA256, HASH_SHA512};

        if hash.is_empty() {
            return Err("Hash cannot be empty".to_string());
        }

        let vsf_type = match algorithm {
            HASH_BLAKE3 => VsfType::hb(hash),
            HASH_SHA256 | HASH_SHA512 => VsfType::hs(hash),
            _ => return Err(format!("Unsupported hash algorithm: {}", algorithm as char)),
        };

        Ok(CalibrationHash(vsf_type))
    }

    pub fn to_vsf_type(self) -> VsfType {
        self.0
    }

    pub fn from_vsf_type(vsf: VsfType) -> Result<Self, String> {
        match vsf {
            VsfType::hb(ref hash) | VsfType::hs(ref hash) if !hash.is_empty() => {
                Ok(CalibrationHash(vsf))
            }
            VsfType::hb(_) | VsfType::hs(_) => Err("Hash cannot be empty".to_string()),
            _ => Err("Expected hash type for calibration hash".to_string()),
        }
    }
}

/// Magic 9: 3×3 colour transformation matrix (Sensor RGB → LMS) Must contain exactly 9 elements in row-major order
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

// ==================== INTEGER EXPOSURE METADATA ====================
//
// Doctrine (Nick, 2026-09-26): exposure metadata is LINEAR INTEGERS at rest, never floats, never logs, never reciprocals.
// Durations are Eagle oscillations; gain, f-number and lens lengths are reduced integer fractions; an exposure SETTING (bias, aperture, ISO) is a signed rational count of stops, shown in twelfths.
// Stops and "1/250" are display forms, derived exactly from the stored integers.
// All maths is 64-bit and checked: overflow-checks are off in every profile, so plain `*` or `+` would wrap silently.
// Every overflow or inexact conversion comes back as Err; nothing here panics on input.

/// Write a u64 as an auto-sized (EWE) unsigned.
/// PROOF: on a 32-bit target usize cannot hold every u64, so a value past usize::MAX goes out as a fixed u6 rather than being truncated by `as usize`.
fn uint(n: u64) -> VsfType {
    match usize::try_from(n) {
        Ok(v) => VsfType::u(v, false),
        Err(_) => VsfType::u6(n),
    }
}

/// Write an i64 as an auto-sized (EWE) signed, with the same 32-bit guard as [`uint`].
fn sint(n: i64) -> VsfType {
    match isize::try_from(n) {
        Ok(v) => VsfType::i(v),
        Err(_) => VsfType::i6(n),
    }
}

/// Read a field that must carry exactly one unsigned value, at any width.
fn one_u64(values: &[VsfType], what: &str) -> Result<u64, String> {
    match values {
        [v] => v
            .as_u64()
            .ok_or_else(|| format!("{} must be an unsigned integer", what)),
        _ => Err(format!("{} must carry exactly one value, got {}", what, values.len())),
    }
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

/// A non-negative integer fraction with a positive denominator, always reduced to lowest terms.
/// WHY reduced: one value gets one encoding at rest, so two files holding f/2.8 are byte-identical in that field and a reader can refuse any other spelling.
/// WHY a fraction: cameras report f-numbers, gains and lens lengths as fractions (EXIF RATIONAL); keeping the fraction is exact where a float or a fixed unit would round.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ratio {
    num: u64,
    den: u64,
}

impl Ratio {
    /// Any fraction with a nonzero denominator; reduced on the way in.
    pub fn new(num: u64, den: u64) -> Result<Self, String> {
        if den == 0 {
            return Err("Fraction denominator cannot be zero".to_string());
        }
        // PROOF: den > 0, so g >= 1 and neither division can trap; num == 0 reduces to 0/1.
        let g = gcd(num, den);
        Ok(Ratio {
            num: num / g,
            den: den / g,
        })
    }

    /// A strictly positive fraction; `what` names the quantity in the error.
    pub fn positive(num: u64, den: u64, what: &str) -> Result<Self, String> {
        if num == 0 {
            return Err(format!("{} must be positive", what));
        }
        Self::new(num, den)
    }

    pub fn num(self) -> u64 {
        self.num
    }

    pub fn den(self) -> u64 {
        self.den
    }

    /// Two values: numerator then denominator, each auto-sized.
    pub fn to_values(self) -> Vec<VsfType> {
        vec![uint(self.num), uint(self.den)]
    }

    /// Read `[num, den]`, refusing a zero denominator and any unreduced spelling.
    pub fn from_values(values: &[VsfType], what: &str) -> Result<Self, String> {
        let (num, den) = match values {
            [n, d] => (
                n.as_u64()
                    .ok_or_else(|| format!("{} numerator must be an unsigned integer", what))?,
                d.as_u64()
                    .ok_or_else(|| format!("{} denominator must be an unsigned integer", what))?,
            ),
            _ => {
                return Err(format!(
                    "{} must carry [numerator, denominator], got {} values",
                    what,
                    values.len()
                ))
            }
        };
        let r = Self::new(num, den)?;
        if r.num != num || r.den != den {
            return Err(format!("{} {}/{} is not in lowest terms", what, num, den));
        }
        Ok(r)
    }
}

/// ISO speed as a reduced positive fraction (ISO 100 = 100/1; a sub-unit ISO like 0.8 = 4/5 stays exact).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsoSpeed(Ratio);

impl IsoSpeed {
    pub fn new(num: u64, den: u64) -> Result<Self, String> {
        Ok(IsoSpeed(Ratio::positive(num, den, "ISO speed")?))
    }

    pub fn ratio(self) -> Ratio {
        self.0
    }

    pub fn to_values(self) -> Vec<VsfType> {
        self.0.to_values()
    }

    pub fn from_values(values: &[VsfType]) -> Result<Self, String> {
        let r = Ratio::from_values(values, "ISO speed")?;
        Self::new(r.num, r.den)
    }
}

/// Exposure duration in WHOLE Eagle oscillations, floored like every other Eagle count.
/// One oscillation is about 704 ps: every consumer shutter, strobe and high-speed cinema exposure resolves to a millionth or better, and a u64 spans about 412 years.
/// Zero is a real reading, not a sentinel: it means the exposure was under one oscillation (a gated-ICCD corner), exactly as `end − start` of two floored instants inside one oscillation is 0.
/// "Unknown" is an absent field; a finer remainder, if ever needed, is a separate optional field beside this count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShutterTime(u64);

impl ShutterTime {
    pub fn new(oscillations: u64) -> Result<Self, String> {
        Ok(ShutterTime(oscillations))
    }

    /// From a camera's nominal seconds as a fraction (1/250 s = `from_seconds(1, 250)`).
    /// Floors to whole oscillations, the Euclidean way: the stored count is the whole oscillations elapsed, never rounded up into one that was not.
    /// An exposure under one oscillation floors to 0, which is stored, not refused.
    /// Err only on a zero denominator or a value 64-bit maths cannot hold (never a wrap).
    pub fn from_seconds(num: u64, den: u64) -> Result<Self, String> {
        if den == 0 {
            return Err("Exposure time denominator cannot be zero".to_string());
        }
        const OPS: u64 = crate::types::eagle_time::OSCILLATIONS_PER_SECOND;
        // Split num = q·den + r so the product only overflows when the answer itself would.
        // PROOF: r < den, so r·OPS/den < OPS and the second term adds less than one second.
        let (q, r) = (num / den, num % den);
        let whole = q
            .checked_mul(OPS)
            .ok_or("Exposure time overflows 64 bits of oscillations")?;
        let frac = r
            .checked_mul(OPS)
            .ok_or("Exposure time denominator too large for 64-bit maths")?
            / den;
        let osc = whole
            .checked_add(frac)
            .ok_or("Exposure time overflows 64 bits of oscillations")?;
        Ok(ShutterTime(osc))
    }

    pub fn oscillations(self) -> u64 {
        self.0
    }

    /// The duration in seconds as an exact reduced fraction (oscillations / OPS), for display.
    pub fn seconds(self) -> Ratio {
        // PROOF: OPS is a nonzero constant, so new() cannot fail here.
        Ratio::new(self.0, crate::types::eagle_time::OSCILLATIONS_PER_SECOND)
            .expect("OPS is nonzero")
    }

    pub fn to_values(self) -> Vec<VsfType> {
        vec![uint(self.0)]
    }

    pub fn from_values(values: &[VsfType]) -> Result<Self, String> {
        Self::new(one_u64(values, "exposure time")?)
    }
}

/// Aperture as N², the f-number SQUARED, a reduced positive fraction.
/// WHY squared: exposure goes as t × ISO / N², so N² is the linear quantity the photometry multiplies, and the full-stop series lands on exact powers of two (f/1.4 → 2, f/2.8 → 8, f/11 → 128) where N itself (2√2, 8√2) is irrational and no fraction holds it.
/// Third-, half- and quarter-stop steps are irrational in every linear space; their exact home is [`StopSetting`], a rational count of stops stored beside this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aperture(Ratio);

/// The full-stop markings whose convention is √2^k, as (marking num, marking den) reduced → N² (num, den).
/// Markings that are already exact (1, 2, 4, 8, 16, 32, 64, 128) square exactly without the table.
const FULL_STOP_MARKINGS: [((u64, u64), (u64, u64)); 8] = [
    ((7, 10), (1, 2)),
    ((7, 5), (2, 1)),
    ((14, 5), (8, 1)),
    ((28, 5), (32, 1)),
    ((11, 1), (128, 1)),
    ((22, 1), (512, 1)),
    ((45, 1), (2048, 1)),
    ((90, 1), (8192, 1)),
];

impl Aperture {
    /// From N² directly (f/2.8 = `from_n_squared(8, 1)`).
    pub fn from_n_squared(num: u64, den: u64) -> Result<Self, String> {
        Ok(Aperture(Ratio::positive(num, den, "Aperture N²")?))
    }

    /// From the f-number printed on the lens or reported by the camera, as a fraction (f/2.8 = `from_marking(28, 10)`).
    /// A full-stop marking means its √2^k by convention and maps to that exact power of two; any other marking (f/1.8, f/0.95, f/3.2) is squared exactly as written.
    /// Err on zero, a zero denominator, or a square that overflows 64 bits.
    pub fn from_marking(num: u64, den: u64) -> Result<Self, String> {
        let m = Ratio::positive(num, den, "Aperture f-number")?;
        if let Some(&(_, (n2n, n2d))) = FULL_STOP_MARKINGS.iter().find(|&&(k, _)| k == (m.num, m.den)) {
            return Self::from_n_squared(n2n, n2d);
        }
        // PROOF: m is reduced, so num² / den² is reduced too (a common factor of the squares would divide num and den).
        let n2n = m.num.checked_mul(m.num).ok_or("Aperture N² overflows 64 bits")?;
        let n2d = m.den.checked_mul(m.den).ok_or("Aperture N² overflows 64 bits")?;
        Self::from_n_squared(n2n, n2d)
    }

    pub fn n_squared(self) -> Ratio {
        self.0
    }

    pub fn to_values(self) -> Vec<VsfType> {
        self.0.to_values()
    }

    pub fn from_values(values: &[VsfType]) -> Result<Self, String> {
        let r = Ratio::from_values(values, "Aperture N²")?;
        Self::from_n_squared(r.num, r.den)
    }
}

/// Focal length in metres as a reduced positive fraction (50 mm = 1/20).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocalLength(Ratio);

impl FocalLength {
    pub fn new(num: u64, den: u64) -> Result<Self, String> {
        Ok(FocalLength(Ratio::positive(num, den, "Focal length")?))
    }

    pub fn from_millimetres(mm: u64) -> Result<Self, String> {
        Self::new(mm, 1000)
    }

    pub fn metres(self) -> Ratio {
        self.0
    }

    pub fn to_values(self) -> Vec<VsfType> {
        self.0.to_values()
    }

    pub fn from_values(values: &[VsfType]) -> Result<Self, String> {
        let r = Ratio::from_values(values, "Focal length")?;
        Self::new(r.num, r.den)
    }
}

/// A camera SETTING in log space: a signed count of stops from a stated reference, as an exact reduced fraction (num/den stops, den > 0).
/// WHY a fraction of stops: grid positions are powers of 2^(1/den), irrational in every linear space, but exact as a rational exponent; any grid (half, third, quarter, or one nobody has built yet) lands without rounding.
/// Display in twelfths: 12 divides by 2, 3, 4 and 6, so every real camera grid shows as whole twelfths (−1⅓ stops = −16); [`twelfths`](Self::twelfths) is None only for a grid twelfths cannot show.
/// Used for exposure bias (reference: no bias), aperture setting (reference: f/1, so f/2.8 = 3 stops) and ISO setting (reference: ISO 100, so ISO 50 = −1 stop, ISO 125 = 1/3).
/// Durations never use this: they stay linear oscillations, since exposure maths and row timing need them linear.
/// Zero is a real setting (the reference itself), not a sentinel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StopSetting {
    num: i64,
    den: u64,
}

/// The exposure bias is a [`StopSetting`] from no bias.
pub type ExposureCompensation = StopSetting;

impl StopSetting {
    /// `num/den` stops, reduced on the way in so one setting has one encoding at rest.
    pub fn from_stops(num: i64, den: u64) -> Result<Self, String> {
        if den == 0 {
            return Err("Stop setting denominator cannot be zero".to_string());
        }
        // PROOF: den > 0 so g >= 1; g divides |num|, so the reduced magnitude is at most 2^63, which is only reached when num == i64::MIN and g == 1.
        let g = gcd(num.unsigned_abs(), den);
        let mag = num.unsigned_abs() / g;
        let num = if num >= 0 {
            // PROOF: num >= 0 means mag <= i64::MAX.
            mag as i64
        } else if mag == 1u64 << 63 {
            i64::MIN
        } else {
            -(mag as i64)
        };
        Ok(StopSetting { num, den: den / g })
    }

    /// Whole twelfths of a stop (f/3.2 from f/1 = `from_twelfths(40)`).
    pub fn from_twelfths(twelfths: i64) -> Result<Self, String> {
        Self::from_stops(twelfths, 12)
    }

    pub fn num(self) -> i64 {
        self.num
    }

    pub fn den(self) -> u64 {
        self.den
    }

    /// The setting in whole twelfths for display, or None when its grid does not divide 12 (fifths, sevenths) or the count would not fit 64 bits.
    pub fn twelfths(self) -> Option<i64> {
        if 12 % self.den != 0 {
            return None;
        }
        // PROOF: den divides 12, so den is 1, 2, 3, 4, 6 or 12 and 12/den is a small positive integer.
        self.num.checked_mul((12 / self.den) as i64)
    }

    /// Two values: signed numerator then unsigned denominator, each auto-sized.
    pub fn to_values(self) -> Vec<VsfType> {
        vec![sint(self.num), uint(self.den)]
    }

    /// Read `[num, den]`, refusing a zero denominator and any unreduced spelling.
    pub fn from_values(values: &[VsfType]) -> Result<Self, String> {
        let (num, den) = match values {
            [n, d] => (
                n.as_i64().ok_or("Stop setting numerator must be a signed integer")?,
                d.as_u64().ok_or("Stop setting denominator must be an unsigned integer")?,
            ),
            _ => {
                return Err(format!(
                    "Stop setting must carry [numerator, denominator], got {} values",
                    values.len()
                ))
            }
        };
        let s = Self::from_stops(num, den)?;
        if s.num != num || s.den != den {
            return Err(format!("Stop setting {}/{} is not in lowest terms", num, den));
        }
        Ok(s)
    }
}

/// Focus distance in metres as a reduced positive fraction.
/// Zero is refused (nothing focuses at the lens plane) and "unknown" is an absent field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusDistance(Ratio);

impl FocusDistance {
    pub fn new(num: u64, den: u64) -> Result<Self, String> {
        Ok(FocusDistance(Ratio::positive(num, den, "Focus distance")?))
    }

    pub fn metres(self) -> Ratio {
        self.0
    }

    pub fn to_values(self) -> Vec<VsfType> {
        self.0.to_values()
    }

    pub fn from_values(values: &[VsfType]) -> Result<Self, String> {
        let r = Ratio::from_values(values, "Focus distance")?;
        Self::new(r.num, r.den)
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
    pub exposure_osc: Option<ShutterTime>,
    pub aperture_n2: Option<Aperture>,
    /// Stop setting from f/1, when the camera works on a grid.
    pub aperture_setting_stops: Option<StopSetting>,
    /// Stop setting from ISO 100, when the camera works on a grid.
    pub iso_setting_stops: Option<StopSetting>,
    pub focal_length_m: Option<FocalLength>,
    pub exposure_bias_stops: Option<ExposureCompensation>,
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
    pub min_aperture_n2: Option<Aperture>, // Smallest aperture (largest f-number, e.g. f/22)
    pub max_aperture_n2: Option<Aperture>, // Largest aperture (smallest f-number, e.g. f/1.4)
}

// ==================== BUILDER PATTERN API ====================

/// Builder for RawMetadata with convenient field access
#[derive(Debug, Clone, Default)]
pub struct RawMetadataBuilder {
    pub cfa_pattern: Option<Vec<u8>>,
    pub black_level: Option<u64>,
    pub white_level: Option<u64>,
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

        let black_level = self.black_level.map(BlackLevel::new).transpose()?;
        let white_level = self.white_level.map(WhiteLevel::new).transpose()?;
        check_levels(black_level, white_level)?;
        Ok(Some(RawMetadata {
            cfa_pattern: self.cfa_pattern.map(CfaPattern::new).transpose()?,
            black_level,
            white_level,
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
            magic_9: self.magic_9.map(Magic9::new).transpose()?,
        }))
    }
}

/// Builder for CameraSettings with convenient field access
#[derive(Debug, Clone, Default)]
pub struct CameraBuilder {
    pub make: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    /// (num, den), e.g. `(800, 1)`
    pub iso_speed: Option<(u64, u64)>,
    /// Eagle oscillations; see [`ShutterTime::from_seconds`] to convert a nominal "1/250"
    pub exposure_osc: Option<u64>,
    /// e.g. f/2.8 = `Aperture::from_marking(28, 10)?` (stored as N² = 8)
    pub aperture_n2: Option<Aperture>,
    /// stops from f/1 as (num, den), e.g. f/3.2 = `(10, 3)`
    pub aperture_setting_stops: Option<(i64, u64)>,
    /// stops from ISO 100 as (num, den), e.g. ISO 125 = `(1, 3)`
    pub iso_setting_stops: Option<(i64, u64)>,
    /// metres as (num, den), e.g. 50 mm = `(50, 1000)`
    pub focal_length_m: Option<(u64, u64)>,
    /// stops as (num, den), e.g. −½ stop = `(-1, 2)`
    pub exposure_bias_stops: Option<(i64, u64)>,
    /// metres as (num, den)
    pub focus_distance_m: Option<(u64, u64)>,
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
            && self.exposure_osc.is_none()
            && self.aperture_n2.is_none()
            && self.aperture_setting_stops.is_none()
            && self.iso_setting_stops.is_none()
            && self.focal_length_m.is_none()
            && self.exposure_bias_stops.is_none()
            && self.focus_distance_m.is_none()
            && self.flash_fired.is_none()
            && self.metering_mode.is_none()
        {
            return Ok(None);
        }

        Ok(Some(CameraSettings {
            make: self.make.map(Manufacturer::new).transpose()?,
            model: self.model.map(ModelName::new).transpose()?,
            serial_number: self.serial_number.map(SerialNumber::new).transpose()?,
            iso_speed: self.iso_speed.map(|(n, d)| IsoSpeed::new(n, d)).transpose()?,
            exposure_osc: self.exposure_osc.map(ShutterTime::new).transpose()?,
            aperture_n2: self.aperture_n2,
            aperture_setting_stops: self
                .aperture_setting_stops
                .map(|(n, d)| StopSetting::from_stops(n, d))
                .transpose()?,
            iso_setting_stops: self
                .iso_setting_stops
                .map(|(n, d)| StopSetting::from_stops(n, d))
                .transpose()?,
            focal_length_m: self
                .focal_length_m
                .map(|(n, d)| FocalLength::new(n, d))
                .transpose()?,
            exposure_bias_stops: self
                .exposure_bias_stops
                .map(|(n, d)| ExposureCompensation::from_stops(n, d))
                .transpose()?,
            focus_distance_m: self
                .focus_distance_m
                .map(|(n, d)| FocusDistance::new(n, d))
                .transpose()?,
            flash_fired: self.flash_fired.map(FlashFired::new).transpose()?,
            metering_mode: self.metering_mode.map(MeteringMode::new).transpose()?,
        }))
    }
}

/// Builder for LensInfo with convenient field access
#[derive(Debug, Clone, Default)]
pub struct LensBuilder {
    pub make: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    /// metres as (num, den)
    pub min_focal_length_m: Option<(u64, u64)>,
    pub max_focal_length_m: Option<(u64, u64)>,
    /// e.g. `Aperture::from_marking(22, 1)?`
    pub min_aperture_n2: Option<Aperture>,
    pub max_aperture_n2: Option<Aperture>,
}

impl LensBuilder {
    /// Convert builder to LensInfo (returns None if all fields are None)
    fn build(self) -> Result<Option<LensInfo>, String> {
        if self.make.is_none()
            && self.model.is_none()
            && self.serial_number.is_none()
            && self.min_focal_length_m.is_none()
            && self.max_focal_length_m.is_none()
            && self.min_aperture_n2.is_none()
            && self.max_aperture_n2.is_none()
        {
            return Ok(None);
        }

        Ok(Some(LensInfo {
            make: self.make.map(Manufacturer::new).transpose()?,
            model: self.model.map(ModelName::new).transpose()?,
            serial_number: self.serial_number.map(SerialNumber::new).transpose()?,
            min_focal_length_m: self
                .min_focal_length_m
                .map(|(n, d)| FocalLength::new(n, d))
                .transpose()?,
            max_focal_length_m: self
                .max_focal_length_m
                .map(|(n, d)| FocalLength::new(n, d))
                .transpose()?,
            min_aperture_n2: self.min_aperture_n2,
            max_aperture_n2: self.max_aperture_n2,
        }))
    }
}

/// Builder pattern for creating RAW images with ergonomic dot notation
///
/// # Example
/// ```ignore
/// use vsf::builders::RawImageBuilder; use vsf::types::BitPackedTensor;
///
/// let samples: Vec<u64> = vec![2048; 4096 * 3072]; let image = BitPackedTensor::pack(12, vec![4096, 3072], &samples);
///
/// let mut raw = RawImageBuilder::new(image); raw.camera.iso_speed = Some((800, 1)); raw.camera.exposure_osc = Some(ShutterTime::from_seconds(1, 60)?.oscillations()); raw.raw.cfa_pattern = Some(vec![b'R', b'G', b'G', b'B']); raw.lens.make = Some("Sony".to_string());
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
/// * `bit_depth` - Bits per sample (1-256, where 0 = 256) * `width` - Image width in samples * `height` - Image height in samples * `samples` - RAW sensor sample values (unreferenced, single-plane)
///
/// # Example
/// ```ignore
/// let samples: Vec<u64> = vec![2048; 4096 * 3072]; // 12-bit mid-gray let raw = raw_image(12, 4096, 3072, samples);
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
/// let track = gps_track(vec![ (40.7128, -74.0060),  // NYC (51.5074, -0.1278),   // London (35.6762, 139.6503),  // Tokyo ]);
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
/// let nyc = gps_waypoint(40.7128, -74.0060); let encoded = VsfType::w(nyc).flatten();
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
/// let (img, loc) = geotagged_photo( 1920, 1080, rgb_data, 40.7128, -74.0060  // Photo taken in NYC );
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
/// **IMPORTANT:** The `image` parameter is a `BitPackedTensor` which is SELF-DESCRIBING. It already contains:
/// - `bit_depth` (8, 10, 12, 14, 16, etc.)
/// - `shape` ([width, height] like [4096, 3072])
/// - `data` (the actual bitpacked pixels)
///
/// **DO NOT** add redundant width/height/bits_per_pixel fields! The `p` type has it all.
///
/// # VSF Structure Created
/// ```text
/// RÅ<...n1 or n2 labels...> [(dimage:p[bitdepth, shape, pixels])    ← Image is FIRST field (self-describing!) (diso speed:u...)                      ← Optional metadata follows (dexposure_osc:u...) (dcfa pattern:t_u3['R','G','G','B'])   ← ASCII characters for readability (dcolour matrix:t_f6[...])]
/// ```
///
/// If TOKEN auth is provided, creates TWO labels: "token auth" and "raw" If no TOKEN auth, creates ONE label: "raw" only
///
/// # Arguments
/// * `image` - BitPackedTensor (use `BitPackedTensor::pack(bit_depth, shape, samples)`) * `metadata` - Optional sensor metadata (CFA pattern, black/white levels, calibration hashes) * `camera` - Optional camera settings (ISO, shutter, aperture, etc.)
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
    // Each item is (field name, values): the integer fractions ride as two values in one field.
    let mut raw_items: Vec<(String, Vec<VsfType>)> = vec![("image".to_string(), vec![VsfType::p(image)])];

    // Add optional metadata
    if let Some(meta) = metadata {
        if let Some(cfa) = meta.cfa_pattern {
            raw_items.push(("cfa_pattern".to_string(), vec![cfa.to_vsf_type()]));
        }

        if let Some(black) = meta.black_level {
            raw_items.push(("black_level".to_string(), black.to_values()));
        }

        if let Some(white) = meta.white_level {
            raw_items.push(("white_level".to_string(), white.to_values()));
        }

        // Calibration hashes (algorithm + hash bytes)
        if let Some(hash) = meta.dark_frame_hash {
            raw_items.push(("dark_frame_hash".to_string(), vec![hash.to_vsf_type()]));
        }

        if let Some(hash) = meta.flat_field_hash {
            raw_items.push(("flat_field_hash".to_string(), vec![hash.to_vsf_type()]));
        }

        if let Some(hash) = meta.bias_frame_hash {
            raw_items.push(("bias_frame_hash".to_string(), vec![hash.to_vsf_type()]));
        }

        if let Some(hash) = meta.vignette_correction_hash {
            raw_items.push(("vignette_correction_hash".to_string(), vec![hash.to_vsf_type()]));
        }

        if let Some(hash) = meta.distortion_correction_hash {
            raw_items.push(("distortion_correction_hash".to_string(), vec![hash.to_vsf_type()]));
        }

        // Magic 9 (3×3 colour matrix: Sensor RGB → LMS)
        if let Some(matrix) = meta.magic_9 {
            raw_items.push(("magic_9".to_string(), vec![matrix.to_vsf_type()]));
        }
    }

    // Camera settings
    if let Some(cam) = camera {
        if let Some(make) = cam.make {
            raw_items.push(("camera_make".to_string(), vec![make.to_vsf_type()]));
        }

        if let Some(model) = cam.model {
            raw_items.push(("camera_model".to_string(), vec![model.to_vsf_type()]));
        }

        if let Some(serial) = cam.serial_number {
            raw_items.push(("camera_serial".to_string(), vec![serial.to_vsf_type()]));
        }

        if let Some(iso) = cam.iso_speed {
            raw_items.push(("iso_speed".to_string(), iso.to_values()));
        }

        if let Some(shutter) = cam.exposure_osc {
            raw_items.push(("exposure_osc".to_string(), shutter.to_values()));
        }

        if let Some(aperture) = cam.aperture_n2 {
            raw_items.push(("aperture_n2".to_string(), aperture.to_values()));
        }

        if let Some(setting) = cam.aperture_setting_stops {
            raw_items.push(("aperture_setting_stops".to_string(), setting.to_values()));
        }

        if let Some(setting) = cam.iso_setting_stops {
            raw_items.push(("iso_setting_stops".to_string(), setting.to_values()));
        }

        if let Some(focal) = cam.focal_length_m {
            raw_items.push(("focal_length_m".to_string(), focal.to_values()));
        }

        if let Some(comp) = cam.exposure_bias_stops {
            raw_items.push(("exposure_bias_stops".to_string(), comp.to_values()));
        }

        if let Some(focus) = cam.focus_distance_m {
            raw_items.push(("focus_distance_m".to_string(), focus.to_values()));
        }

        if let Some(flash) = cam.flash_fired {
            raw_items.push(("flash_fired".to_string(), vec![flash.to_vsf_type()]));
        }

        if let Some(metering) = cam.metering_mode {
            raw_items.push(("metering_mode".to_string(), vec![metering.to_vsf_type()]));
        }
    }

    // Lens info
    if let Some(l) = lens {
        if let Some(make) = l.make {
            raw_items.push(("lens_make".to_string(), vec![make.to_vsf_type()]));
        }

        if let Some(model) = l.model {
            raw_items.push(("lens_model".to_string(), vec![model.to_vsf_type()]));
        }

        if let Some(serial) = l.serial_number {
            raw_items.push(("lens_serial".to_string(), vec![serial.to_vsf_type()]));
        }

        if let Some(min_focal) = l.min_focal_length_m {
            raw_items.push(("lens_min_focal_m".to_string(), min_focal.to_values()));
        }

        if let Some(max_focal) = l.max_focal_length_m {
            raw_items.push(("lens_max_focal_m".to_string(), max_focal.to_values()));
        }

        if let Some(min_ap) = l.min_aperture_n2 {
            raw_items.push(("lens_min_aperture_n2".to_string(), min_ap.to_values()));
        }

        if let Some(max_ap) = l.max_aperture_n2 {
            raw_items.push(("lens_max_aperture_n2".to_string(), max_ap.to_values()));
        }
    }

    let mut section = crate::file_format::VsfSection::new("raw");
    for (name, values) in raw_items {
        section.add_field_multi(name, values);
    }
    builder = builder.add_section_direct(section);

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
/// 1. Creates a `BitPackedTensor::pack(12, [4096, 3072], samples)` - this packs your 12-bit samples into the minimal bitpacked representation
/// 2. Adds sensor metadata (CFA pattern, black/white levels)
/// 3. Adds camera settings (ISO, shutter speed)
///
/// **The resulting p type contains EVERYTHING about the image:**
/// - No separate width field (shape has it: [4096, 3072])
/// - No separate bit_depth field (p encoding has it: 12)
/// - No separate sample data section (p has the bitpacked bytes)
///
/// # Arguments
/// * `samples` - RAW sensor sample values as u64 (0-4095 for 12-bit), will be bitpacked * `iso` - whole ISO speed (e.g., 100, 200, 400, 800, 1600, 3200) * `exposure_osc` - exposure time in Eagle oscillations (`ShutterTime::from_seconds(1, 60)?.oscillations()` for 1/60 second)
pub fn lumis_raw_capture(samples: Vec<u64>, iso: u64, exposure_osc: u64) -> Result<Vec<u8>, String> {
    // Create BitPackedTensor for 12-bit Lumis sensor
    let image = BitPackedTensor::pack(12, vec![4096, 3072], &samples);

    build_raw_image(
        image,
        Some(RawMetadata {
            cfa_pattern: Some(CfaPattern::new(vec![b'R', b'G', b'G', b'B'])?), // RGGB Bayer pattern
            black_level: Some(BlackLevel::new(64)?),
            white_level: Some(WhiteLevel::new(4095)?),
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
            iso_speed: Some(IsoSpeed::new(iso, 1)?),
            exposure_osc: Some(ShutterTime::new(exposure_osc)?),
            aperture_n2: None,
            aperture_setting_stops: None,
            iso_setting_stops: None,
            focal_length_m: None,
            exposure_bias_stops: None,
            focus_distance_m: None,
            flash_fired: Some(FlashFired::new(false)?),
            metering_mode: None,
        }),
        None, // No lens info (phone camera)
    )
}

// ==================== COMPRESSED IMAGE ====================

/// AV1 encoding marker for v-wrapped data
pub const ENCODING_AV1: u8 = b'a';

/// Build a compressed image (AV1 payload in VSF RGB colourspace)
///
/// Creates a minimal VSF file with:
/// - Provenance hash only (no rolling hash, no signature)
/// - AV1-compressed pixel data wrapped in v type (`va`)
///
/// The `va` encoding tells us it's AV1, and AV1 bitstream contains dimensions. Provenance hash ensures integrity. No redundant metadata needed.
///
/// Assumes VSF RGB colourspace (gamma 2, Rec.2020 primaries).
///
/// # Arguments
/// * `av1_data` - AV1-encoded pixel data
///
/// # Returns
/// Complete VSF file bytes ready to write to disk
pub fn compressed_image(av1_data: Vec<u8>) -> Result<Vec<u8>, String> {
    VsfBuilder::new()
        .provenance_only()
        .add_section(
            "image",
            vec![("pixels".to_string(), VsfType::v(ENCODING_AV1, av1_data))],
        )
        .build()
}

/// Parsed compressed image from a VSF file
pub struct ParsedCompressedImage {
    pub encoding: u8,
    pub data: Vec<u8>,
}

/// Parse a compressed image VSF file
///
/// Extracts encoding type and compressed pixel data. Dimensions come from decoding the AV1 bitstream.
///
/// # Arguments
/// * `data` - Complete VSF file bytes
///
/// # Returns
/// ParsedCompressedImage or error
pub fn parse_compressed_image(data: &[u8]) -> Result<ParsedCompressedImage, String> {
    use crate::file_format::{VsfHeader, VsfSection};

    // Parse header using library function
    let (header, _) = VsfHeader::decode(data)?;

    // Find the "image" section
    let image_field = header
        .fields
        .iter()
        .find(|f| f.name == "image")
        .ok_or("Required 'image' section not found")?;

    // Parse section at offset
    let mut ptr = image_field.offset_bytes;
    let section = VsfSection::parse(data, &mut ptr)?;

    // Extract pixels field
    let pixels_field = section
        .get_field("pixels")
        .ok_or("Missing 'pixels' field in image section")?;

    // Get first value (the v-wrapped data)
    let value = pixels_field.values.first().ok_or("Empty 'pixels' field")?;

    match value {
        VsfType::v(encoding, pixel_data) => Ok(ParsedCompressedImage {
            encoding: *encoding,
            data: pixel_data.clone(),
        }),
        _ => Err("Expected v type for pixels field".to_string()),
    }
}

// ==================== RAW IMAGE PARSER ====================

/// Parsed RAW image data from a VSF file
pub struct ParsedRawImage {
    pub image: BitPackedTensor,
    pub metadata: Option<RawMetadata>,
    pub camera: Option<CameraSettings>,
    pub lens: Option<LensInfo>,
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
    use crate::file_format::{VsfHeader, VsfSection};

    // Parse header using library function
    let (header, _) = VsfHeader::decode(data)?;

    // Find the "raw" section
    let raw_field = header
        .fields
        .iter()
        .find(|f| f.name == "raw")
        .ok_or("Required 'raw' section not found")?;

    // Parse section at offset
    let mut ptr = raw_field.offset_bytes;
    let section = VsfSection::parse(data, &mut ptr)?;

    // Helper to get first value from a field
    fn get_first_value<'a>(section: &'a VsfSection, name: &str) -> Option<&'a VsfType> {
        section.get_field(name)?.values.first()
    }

    // Extract image (required)
    let image = match get_first_value(&section, "image") {
        Some(VsfType::p(tensor)) => tensor.clone(),
        _ => return Err("Missing required 'image' field".to_string()),
    };

    // Initialize metadata fields
    let mut cfa_pattern: Option<CfaPattern> = None;
    let mut black_level: Option<BlackLevel> = None;
    let mut white_level: Option<WhiteLevel> = None;
    let mut dark_frame_hash: Option<CalibrationHash> = None;
    let mut flat_field_hash: Option<CalibrationHash> = None;
    let mut bias_frame_hash: Option<CalibrationHash> = None;
    let mut vignette_correction_hash: Option<CalibrationHash> = None;
    let mut distortion_correction_hash: Option<CalibrationHash> = None;
    let mut magic_9: Option<Magic9> = None;

    let mut camera_make: Option<Manufacturer> = None;
    let mut camera_model: Option<ModelName> = None;
    let mut camera_serial: Option<SerialNumber> = None;
    let mut iso_speed: Option<IsoSpeed> = None;
    let mut exposure_osc: Option<ShutterTime> = None;
    let mut aperture_n2: Option<Aperture> = None;
    let mut aperture_setting_stops: Option<StopSetting> = None;
    let mut iso_setting_stops: Option<StopSetting> = None;
    let mut focal_length_m: Option<FocalLength> = None;
    let mut exposure_bias_stops: Option<ExposureCompensation> = None;
    let mut focus_distance_m: Option<FocusDistance> = None;
    let mut flash_fired: Option<FlashFired> = None;
    let mut metering_mode: Option<MeteringMode> = None;

    let mut lens_make: Option<Manufacturer> = None;
    let mut lens_model: Option<ModelName> = None;
    let mut lens_serial: Option<SerialNumber> = None;
    let mut lens_min_focal_m: Option<FocalLength> = None;
    let mut lens_max_focal_m: Option<FocalLength> = None;
    let mut lens_min_aperture_n2: Option<Aperture> = None;
    let mut lens_max_aperture_n2: Option<Aperture> = None;

    // A single-value field's first value; the integer metadata reads the whole value list.
    fn first(values: &[VsfType], name: &str) -> Result<VsfType, String> {
        values
            .first()
            .cloned()
            .ok_or_else(|| format!("Field '{}' has no value", name))
    }

    // Every known field that is present must be well-formed: a malformed one refuses the file rather than being skipped.
    for field in &section.fields {
        let v = &field.values[..];
        let name = field.name.as_str();
        match name {
            // Raw metadata
            "cfa_pattern" => cfa_pattern = Some(CfaPattern::from_vsf_type(first(v, name)?)?),
            "black_level" => black_level = Some(BlackLevel::from_values(v)?),
            "white_level" => white_level = Some(WhiteLevel::from_values(v)?),
            "dark_frame_hash" => dark_frame_hash = Some(CalibrationHash::from_vsf_type(first(v, name)?)?),
            "flat_field_hash" => flat_field_hash = Some(CalibrationHash::from_vsf_type(first(v, name)?)?),
            "bias_frame_hash" => bias_frame_hash = Some(CalibrationHash::from_vsf_type(first(v, name)?)?),
            "vignette_correction_hash" => {
                vignette_correction_hash = Some(CalibrationHash::from_vsf_type(first(v, name)?)?)
            }
            "distortion_correction_hash" => {
                distortion_correction_hash = Some(CalibrationHash::from_vsf_type(first(v, name)?)?)
            }
            "magic_9" => magic_9 = Some(Magic9::from_vsf_type(first(v, name)?)?),
            // Camera settings
            "camera_make" => camera_make = Some(Manufacturer::from_vsf_type(first(v, name)?)?),
            "camera_model" => camera_model = Some(ModelName::from_vsf_type(first(v, name)?)?),
            "camera_serial" => camera_serial = Some(SerialNumber::from_vsf_type(first(v, name)?)?),
            "iso_speed" => iso_speed = Some(IsoSpeed::from_values(v)?),
            "exposure_osc" => exposure_osc = Some(ShutterTime::from_values(v)?),
            "aperture_n2" => aperture_n2 = Some(Aperture::from_values(v)?),
            "aperture_setting_stops" => {
                aperture_setting_stops = Some(StopSetting::from_values(v)?)
            }
            "iso_setting_stops" => iso_setting_stops = Some(StopSetting::from_values(v)?),
            "focal_length_m" => focal_length_m = Some(FocalLength::from_values(v)?),
            "exposure_bias_stops" => {
                exposure_bias_stops = Some(ExposureCompensation::from_values(v)?)
            }
            "focus_distance_m" => focus_distance_m = Some(FocusDistance::from_values(v)?),
            "flash_fired" => flash_fired = Some(FlashFired::from_vsf_type(first(v, name)?)?),
            "metering_mode" => metering_mode = Some(MeteringMode::from_vsf_type(first(v, name)?)?),
            // Lens info
            "lens_make" => lens_make = Some(Manufacturer::from_vsf_type(first(v, name)?)?),
            "lens_model" => lens_model = Some(ModelName::from_vsf_type(first(v, name)?)?),
            "lens_serial" => lens_serial = Some(SerialNumber::from_vsf_type(first(v, name)?)?),
            "lens_min_focal_m" => lens_min_focal_m = Some(FocalLength::from_values(v)?),
            "lens_max_focal_m" => lens_max_focal_m = Some(FocalLength::from_values(v)?),
            "lens_min_aperture_n2" => lens_min_aperture_n2 = Some(Aperture::from_values(v)?),
            "lens_max_aperture_n2" => lens_max_aperture_n2 = Some(Aperture::from_values(v)?),
            _ => {} // Unknown field, skip
        }
    }
    check_levels(black_level, white_level)?;

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
            cfa_pattern,
            black_level,
            white_level,
            dark_frame_hash,
            flat_field_hash,
            bias_frame_hash,
            vignette_correction_hash,
            distortion_correction_hash,
            magic_9,
        })
    } else {
        None
    };

    let camera_settings = if camera_make.is_some()
        || camera_model.is_some()
        || camera_serial.is_some()
        || iso_speed.is_some()
        || exposure_osc.is_some()
        || aperture_n2.is_some()
        || aperture_setting_stops.is_some()
        || iso_setting_stops.is_some()
        || focal_length_m.is_some()
        || exposure_bias_stops.is_some()
        || focus_distance_m.is_some()
        || flash_fired.is_some()
        || metering_mode.is_some()
    {
        Some(CameraSettings {
            make: camera_make,
            model: camera_model,
            serial_number: camera_serial,
            iso_speed,
            exposure_osc,
            aperture_n2,
            aperture_setting_stops,
            iso_setting_stops,
            focal_length_m,
            exposure_bias_stops,
            focus_distance_m,
            flash_fired,
            metering_mode,
        })
    } else {
        None
    };

    let lens_info = if lens_make.is_some()
        || lens_model.is_some()
        || lens_serial.is_some()
        || lens_min_focal_m.is_some()
        || lens_max_focal_m.is_some()
        || lens_min_aperture_n2.is_some()
        || lens_max_aperture_n2.is_some()
    {
        Some(LensInfo {
            make: lens_make,
            model: lens_model,
            serial_number: lens_serial,
            min_focal_length_m: lens_min_focal_m,
            max_focal_length_m: lens_max_focal_m,
            min_aperture_n2: lens_min_aperture_n2,
            max_aperture_n2: lens_max_aperture_n2,
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

        // Verify file is structured correctly Should have header + one "raw" section with p type
        assert!(bytes.len() > 50); // Minimal file should be small
    }

    #[cfg(feature = "text-encode")]
    #[test]
    fn test_complete_raw_image_with_metadata() {
        let samples: Vec<u64> = vec![255; 64]; // 8x8
        let image = BitPackedTensor::pack(8, vec![8, 8], &samples);

        let result = build_raw_image(
            image,
            Some(RawMetadata {
                cfa_pattern: Some(CfaPattern::new(vec![b'R', b'G', b'G', b'B']).unwrap()),
                black_level: Some(BlackLevel::new(64).unwrap()),
                white_level: Some(WhiteLevel::new(255).unwrap()),
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
                iso_speed: Some(IsoSpeed::new(800, 1).unwrap()),
                exposure_osc: Some(ShutterTime::from_seconds(1, 60).unwrap()), // 1/60 second
                aperture_n2: Some(Aperture::from_marking(28, 10).unwrap()),
                aperture_setting_stops: None,
                iso_setting_stops: None,
                focal_length_m: Some(FocalLength::from_millimetres(24).unwrap()),
                exposure_bias_stops: None,
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
        // Lumis 12-bit: 4096x3072 = 12,582,912 pixels Samples are u64 values (0-4095), will be bitpacked by the function
        let pixel_count = 4096 * 3072;
        let samples: Vec<u64> = vec![2048; pixel_count]; // Mid-gray

        let result = lumis_raw_capture(
            samples,
            800,
            ShutterTime::from_seconds(1, 60).unwrap().oscillations(),
        );

        assert!(result.is_ok());
        let bytes = result.unwrap();

        // Verify magic number (RÅ is 3 bytes in UTF-8)
        assert_eq!(&bytes[0..3], "RÅ".as_bytes());

        // File should be large (header + metadata + ~18.9MB bitpacked pixels) 12-bit × 12.6M pixels = 18.9MB
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

    #[cfg(feature = "text-encode")]
    #[test]
    fn test_roundtrip_full_metadata() {
        // Create image with full metadata
        let samples: Vec<u64> = vec![200; 64]; // 8x8
        let original_image = BitPackedTensor::pack(8, vec![8, 8], &samples);

        let original_metadata = RawMetadata {
            cfa_pattern: Some(CfaPattern::new(vec![b'R', b'G', b'G', b'B']).unwrap()), // RGGB Bayer pattern
            black_level: Some(BlackLevel::new(64).unwrap()),
            white_level: Some(WhiteLevel::new(255).unwrap()),
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
            iso_speed: Some(IsoSpeed::new(800, 1).unwrap()),
            exposure_osc: Some(ShutterTime::from_seconds(1, 60).unwrap()), // 1/60 sec
            aperture_n2: Some(Aperture::from_marking(28, 10).unwrap()),
            aperture_setting_stops: Some(StopSetting::from_twelfths(36).unwrap()),
            iso_setting_stops: Some(StopSetting::from_stops(3, 1).unwrap()),
            focal_length_m: Some(FocalLength::from_millimetres(50).unwrap()),
            exposure_bias_stops: Some(ExposureCompensation::from_stops(-1, 2).unwrap()),
            focus_distance_m: Some(FocusDistance::new(7, 2).unwrap()),
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
        let cam = parsed.camera.as_ref().unwrap();
        assert_eq!(cam.iso_speed, Some(IsoSpeed::new(800, 1).unwrap()));
        assert_eq!(cam.exposure_osc, Some(ShutterTime::from_seconds(1, 60).unwrap()));
        assert_eq!(cam.aperture_n2, Some(Aperture::from_n_squared(8, 1).unwrap()));
        assert_eq!(cam.aperture_setting_stops.and_then(|a| a.twelfths()), Some(36));
        assert_eq!(cam.iso_setting_stops.and_then(|a| a.twelfths()), Some(36));
        assert_eq!(cam.exposure_bias_stops.and_then(|c| c.twelfths()), Some(-6));
        assert_eq!(cam.focus_distance_m, Some(FocusDistance::new(7, 2).unwrap()));
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
                black_level: Some(BlackLevel::new(64).unwrap()),
                white_level: Some(WhiteLevel::new(255).unwrap()),
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

        // Find the first section (after header)
        let header_end = raw_bytes.iter().position(|&b| b == b'>').unwrap();

        // In v4 wire format, sections start after header '>' There may be no preamble, or there may be additional metadata Just verify we can find a section marker '['
        let section_start = raw_bytes[header_end..]
            .iter()
            .position(|&b| b == b'[')
            .expect("Expected to find section start '['");
        assert!(
            section_start < 200,
            "Section should start soon after header"
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

    #[cfg(feature = "text-encode")]
    #[test]
    fn test_builder_pattern_camera_settings() {
        // Test builder with camera settings
        let samples: Vec<u64> = vec![100; 64];
        let image = BitPackedTensor::pack(8, vec![8, 8], &samples);

        let mut raw = RawImageBuilder::new(image);
        raw.camera.iso_speed = Some((800, 1));
        raw.camera.exposure_osc = Some(ShutterTime::from_seconds(1, 60).unwrap().oscillations());
        raw.camera.aperture_n2 = Some(Aperture::from_marking(28, 10).unwrap());
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
        raw.raw.black_level = Some(64);
        raw.raw.white_level = Some(4095);
        raw.raw.dark_frame_hash = Some((HASH_BLAKE3, vec![0xAB; 32]));

        let result = raw.build();
        assert!(result.is_ok());
        let bytes = result.unwrap();

        // Parse and verify metadata round-tripped successfully
        let parsed = parse_raw_image(&bytes).unwrap();
        assert!(parsed.metadata.is_some());
        // Note: Can't use assert_eq on newtypes (no PartialEq), but successful parsing validates data
    }

    #[cfg(feature = "text-encode")]
    #[test]
    fn test_builder_pattern_lens_info() {
        // Test builder with lens info
        let samples: Vec<u64> = vec![100; 64];
        let image = BitPackedTensor::pack(8, vec![8, 8], &samples);

        let mut raw = RawImageBuilder::new(image);
        raw.lens.make = Some("Sony".to_string());
        raw.lens.model = Some("FE 24-70mm F2.8 GM II".to_string());
        raw.lens.serial_number = Some("ABC123456".to_string());
        raw.lens.min_focal_length_m = Some((24, 1000));
        raw.lens.max_focal_length_m = Some((70, 1000));
        raw.lens.min_aperture_n2 = Some(Aperture::from_marking(22, 1).unwrap());
        raw.lens.max_aperture_n2 = Some(Aperture::from_marking(28, 10).unwrap());

        let result = raw.build();
        assert!(result.is_ok());
        let bytes = result.unwrap();

        // Parse and verify lens info round-tripped successfully
        let parsed = parse_raw_image(&bytes).unwrap();
        assert!(parsed.lens.is_some());
        // Note: Can't use assert_eq on newtypes (no PartialEq), but successful parsing validates data
    }

    #[cfg(feature = "text-encode")]
    #[test]
    fn test_builder_pattern_full() {
        // Test builder with all fields populated
        let samples: Vec<u64> = vec![2048; 64];
        let image = BitPackedTensor::pack(12, vec![8, 8], &samples);

        let mut raw = RawImageBuilder::new(image);

        // Raw metadata
        raw.raw.cfa_pattern = Some(vec![b'R', b'G', b'G', b'B']);
        raw.raw.black_level = Some(64);
        raw.raw.white_level = Some(4095);
        raw.raw.magic_9 = Some(vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);

        // Camera settings
        raw.camera.iso_speed = Some((800, 1));
        raw.camera.exposure_osc = Some(ShutterTime::from_seconds(1, 125).unwrap().oscillations());
        raw.camera.aperture_n2 = Some(Aperture::from_marking(28, 10).unwrap());
        raw.camera.focal_length_m = Some((50, 1000));
        raw.camera.exposure_bias_stops = Some((-1, 2));
        raw.camera.focus_distance_m = Some((7, 2));
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

    #[test]
    fn exposure_integers_are_exact_and_checked() {
        let ops = crate::types::eagle_time::OSCILLATIONS_PER_SECOND;
        // Whole seconds are exact; a fraction floors to whole oscillations and never rounds up.
        assert_eq!(ShutterTime::from_seconds(2, 1).unwrap().oscillations(), 2 * ops);
        assert_eq!(ShutterTime::from_seconds(1, 250).unwrap().oscillations(), ops / 250);
        assert_eq!(ShutterTime::from_seconds(3, 2).unwrap().oscillations(), ops + ops / 2);
        // The display form is exact: oscillations over OPS, reduced.
        let s = ShutterTime::new(ops / 2).unwrap().seconds();
        assert_eq!((s.num(), s.den()), (1, 2));
        // A sub-oscillation exposure floors to 0 and is kept: 0 means "under one oscillation", not "unknown".
        assert_eq!(ShutterTime::new(0).unwrap().oscillations(), 0);
        assert_eq!(ShutterTime::from_seconds(1, ops + 1).unwrap().oscillations(), 0);
        assert_eq!(ShutterTime::from_seconds(1, ops).unwrap().oscillations(), 1);
        // A zero denominator and overflow are refused, never wrapped.
        assert!(ShutterTime::from_seconds(1, 0).is_err());
        assert!(ShutterTime::from_seconds(u64::MAX, 1).is_err());
        assert_eq!(ShutterTime::from_seconds(1, u64::MAX).unwrap().oscillations(), 0);
        // A huge denominator with a huge remainder cannot be held in 64 bits: refused, not wrapped.
        assert!(ShutterTime::from_seconds(u64::MAX - 1, u64::MAX).is_err());

        // Twelfths land exactly for half, third and quarter stops; anything else is refused.
        assert_eq!(ExposureCompensation::from_stops(-4, 3).unwrap().twelfths(), Some(-16));
        assert_eq!(ExposureCompensation::from_stops(1, 4).unwrap().twelfths(), Some(3));
        // A grid twelfths cannot show is still stored exactly; display just has no twelfths for it.
        let fifth = ExposureCompensation::from_stops(1, 5).unwrap();
        assert_eq!((fifth.num(), fifth.den(), fifth.twelfths()), (1, 5, None));
        // An unreduced setting on disk is refused.
        assert!(StopSetting::from_values(&[VsfType::i3(-8), VsfType::u3(12)]).is_err());
        // Reduced on the way in; the extremes neither trap nor wrap.
        let r = StopSetting::from_stops(-8, 12).unwrap();
        assert_eq!((r.num(), r.den(), r.twelfths()), (-2, 3, Some(-8)));
        assert_eq!(StopSetting::from_stops(i64::MIN, 1).unwrap().num(), i64::MIN);
        assert_eq!(StopSetting::from_stops(i64::MIN, 1u64 << 63).unwrap().num(), -1);
        assert_eq!(StopSetting::from_stops(i64::MAX, 1).unwrap().twelfths(), None);
        assert!(ExposureCompensation::from_stops(1, 0).is_err());

        // Fractions reduce on the way in and refuse zero where the quantity must be positive.
        // Full-stop markings land on exact powers of two in N²; other markings square as written.
        let n2 = |n, d| {
            let r = Aperture::from_marking(n, d).unwrap().n_squared();
            (r.num(), r.den())
        };
        assert_eq!(n2(28, 10), (8, 1));
        assert_eq!(n2(14, 10), (2, 1));
        assert_eq!(n2(11, 1), (128, 1));
        assert_eq!(n2(7, 10), (1, 2));
        assert_eq!(n2(4, 1), (16, 1));
        assert_eq!(n2(18, 10), (81, 25));
        assert_eq!(n2(95, 100), (361, 400));
        assert_eq!(n2(32, 10), (256, 25));
        assert!(Aperture::from_marking(0, 1).is_err());
        assert!(Aperture::from_marking(u64::MAX, 1).is_err());
        // Every manufacturer's stop grid lands exactly in twelfths.
        assert_eq!(StopSetting::from_stops(1, 3).unwrap().twelfths(), Some(4));
        assert_eq!(StopSetting::from_stops(1, 2).unwrap().twelfths(), Some(6));
        assert_eq!(StopSetting::from_stops(3, 4).unwrap().twelfths(), Some(9));
        assert!(IsoSpeed::new(100, 0).is_err());
        assert!(FocusDistance::new(0, 1).is_err());
        assert!(WhiteLevel::new(0).is_err());
    }

    #[test]
    fn exposure_readers_refuse_malformed_values() {
        // An unreduced fraction on disk is a second spelling of one value: refused.
        assert!(Ratio::from_values(&[VsfType::u(28, false), VsfType::u(10, false)], "f").is_err());
        assert!(Ratio::from_values(&[VsfType::u(14, false), VsfType::u(5, false)], "f").is_ok());
        assert!(Ratio::from_values(&[VsfType::u(1, false)], "f").is_err());
        assert!(Ratio::from_values(&[VsfType::u(1, false), VsfType::u(0, false)], "f").is_err());
        assert_eq!(ShutterTime::from_values(&[VsfType::u(0, false)]).unwrap().oscillations(), 0);
        assert!(ShutterTime::from_values(&[VsfType::f5(0.5)]).is_err());
        // Width-agnostic: a fixed-width u6 reads the same as an auto-sized value.
        assert_eq!(
            ShutterTime::from_values(&[VsfType::u6(u64::MAX)]).unwrap().oscillations(),
            u64::MAX
        );
        assert_eq!(
            ExposureCompensation::from_values(&[VsfType::i3(-4), VsfType::u3(3)]).unwrap().twelfths(),
            Some(-16)
        );

        // Black at or above white is refused at build time.
        let samples = vec![0u64; 4];
        let mut raw = RawImageBuilder::new(BitPackedTensor::pack(8, vec![2, 2], &samples));
        raw.raw.black_level = Some(255);
        raw.raw.white_level = Some(255);
        assert!(raw.build().is_err());
    }
}
