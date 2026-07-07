//! Type conversions between Rust primitives and VsfType
//!
//! Provides ergonomic trait-based conversions for working with VsfType values without needing intermediate wrapper types.
//!
//! # Example: IntoVsfType (Rust → VsfType)
//!
//!
//! ```rust
//! use vsf::schema::IntoVsfType; use vsf::VsfType;
//!
//! // Automatic conversion from Rust primitives let u: VsfType = 42u32.into_vsf_type();          // VsfType::u5(42) let i: VsfType = (-100i32).into_vsf_type();      // VsfType::i5(-100) let f: VsfType = 3.14f64.into_vsf_type();        // VsfType::f6(3.14) let b: VsfType = true.into_vsf_type();           // VsfType::u0(true) let s: VsfType = "hello".into_vsf_type();        // VsfType::x("hello")
//!
//! // Used automatically in SectionBuilder::set() let section = schema.builder() .set("count", 42u32)?      // Calls into_vsf_type() internally .set("name", "test")?      // Calls into_vsf_type() internally .build()?;
//! ```
//!
//! # Example: FromVsfType (VsfType → Rust)
//!
//!
//! ```rust
//! use vsf::schema::FromVsfType; use vsf::VsfType;
//!
//! let vsf = VsfType::u5(42);
//!
//! // Extract to specific Rust type let n: u32 = u32::from_vsf_type(&vsf)?;          // Ok(42) let n2: u64 = u64::from_vsf_type(&vsf)?;         // Ok(42u64) - cross-size works
//!
//! // Type mismatch produces clear error let vsf = VsfType::f6(3.14); let result = u32::from_vsf_type(&vsf); assert!(result.is_err());  // Cannot convert f6 to u32
//!
//! // Used automatically in SectionBuilder::get_value() let count: u32 = section.get_value("count")?;    // Calls from_vsf_type() internally
//! ```
//!
//! # Cross-Size Conversions
//!
//! The traits support automatic widening conversions with bounds checking:
//!
//!
//! ```rust
//! use vsf::schema::FromVsfType; use vsf::VsfType;
//!
//! // Widen u8 → u32 (always safe) let small = VsfType::u3(100u8); let big: u32 = u32::from_vsf_type(&small)?;      // Ok(100u32)
//!
//! // Narrow u32 → u8 (bounds checked) let vsf = VsfType::u5(1000u32); let result = u8::from_vsf_type(&vsf); assert!(result.is_err());  // 1000 doesn't fit in u8
//! ```

use super::constraint::vsf_type_name;
use super::validate::{ValidationError, ValidationResult};
use crate::prelude::*;
use crate::types::{EtType, VsfType};

/// Convert Rust types to VsfType
///
/// This trait provides automatic conversion from Rust primitives to the appropriate VsfType variant, making schema APIs ergonomic.
pub trait IntoVsfType {
    fn into_vsf_type(self) -> VsfType;
}

/// Extract Rust types from VsfType
///
/// This trait provides type-safe extraction of Rust primitives from VsfType values, with appropriate error handling for type mismatches.
pub trait FromVsfType: Sized {
    fn from_vsf_type(vsf: &VsfType) -> ValidationResult<Self>;
}

// === UNSIGNED INTEGERS ===

impl IntoVsfType for u8 {
    fn into_vsf_type(self) -> VsfType {
        VsfType::u3(self)
    }
}

impl FromVsfType for u8 {
    fn from_vsf_type(vsf: &VsfType) -> ValidationResult<Self> {
        match vsf {
            VsfType::u3(v) => Ok(*v),
            VsfType::u(v, _) if *v <= u8::MAX as usize => Ok(*v as u8),
            VsfType::u0(v) => Ok(if *v { 1 } else { 0 }),
            _ => Err(ValidationError::Custom(format!(
                "Cannot convert {} to u8",
                vsf_type_name(vsf)
            ))),
        }
    }
}

impl IntoVsfType for u16 {
    fn into_vsf_type(self) -> VsfType {
        VsfType::u4(self)
    }
}

impl FromVsfType for u16 {
    fn from_vsf_type(vsf: &VsfType) -> ValidationResult<Self> {
        match vsf {
            VsfType::u4(v) => Ok(*v),
            VsfType::u3(v) => Ok(*v as u16),
            VsfType::u(v, _) if *v <= u16::MAX as usize => Ok(*v as u16),
            VsfType::u0(v) => Ok(if *v { 1 } else { 0 }),
            _ => Err(ValidationError::Custom(format!(
                "Cannot convert {} to u16",
                vsf_type_name(vsf)
            ))),
        }
    }
}

impl IntoVsfType for u32 {
    fn into_vsf_type(self) -> VsfType {
        VsfType::u5(self)
    }
}

impl FromVsfType for u32 {
    fn from_vsf_type(vsf: &VsfType) -> ValidationResult<Self> {
        match vsf {
            VsfType::u5(v) => Ok(*v),
            VsfType::u3(v) => Ok(*v as u32),
            VsfType::u4(v) => Ok(*v as u32),
            VsfType::u(v, _) if *v <= u32::MAX as usize => Ok(*v as u32),
            VsfType::u0(v) => Ok(if *v { 1 } else { 0 }),
            _ => Err(ValidationError::Custom(format!(
                "Cannot convert {} to u32",
                vsf_type_name(vsf)
            ))),
        }
    }
}

impl IntoVsfType for u64 {
    fn into_vsf_type(self) -> VsfType {
        VsfType::u6(self)
    }
}

impl FromVsfType for u64 {
    fn from_vsf_type(vsf: &VsfType) -> ValidationResult<Self> {
        match vsf {
            VsfType::u6(v) => Ok(*v),
            VsfType::u3(v) => Ok(*v as u64),
            VsfType::u4(v) => Ok(*v as u64),
            VsfType::u5(v) => Ok(*v as u64),
            VsfType::u(v, _) => Ok(*v as u64),
            VsfType::u0(v) => Ok(if *v { 1 } else { 0 }),
            _ => Err(ValidationError::Custom(format!(
                "Cannot convert {} to u64",
                vsf_type_name(vsf)
            ))),
        }
    }
}

impl IntoVsfType for usize {
    fn into_vsf_type(self) -> VsfType {
        VsfType::u(self, false)
    }
}

impl FromVsfType for usize {
    fn from_vsf_type(vsf: &VsfType) -> ValidationResult<Self> {
        match vsf {
            VsfType::u(v, _) => Ok(*v),
            VsfType::u3(v) => Ok(*v as usize),
            VsfType::u4(v) => Ok(*v as usize),
            VsfType::u5(v) => Ok(*v as usize),
            VsfType::u6(v) => Ok(*v as usize),
            VsfType::u0(v) => Ok(if *v { 1 } else { 0 }),
            _ => Err(ValidationError::Custom(format!(
                "Cannot convert {} to usize",
                vsf_type_name(vsf)
            ))),
        }
    }
}

impl IntoVsfType for bool {
    fn into_vsf_type(self) -> VsfType {
        VsfType::u0(self)
    }
}

impl FromVsfType for bool {
    fn from_vsf_type(vsf: &VsfType) -> ValidationResult<Self> {
        match vsf {
            VsfType::u0(v) => Ok(*v),
            VsfType::u3(v) => Ok(*v != 0),
            VsfType::u(v, _) => Ok(*v != 0),
            _ => Err(ValidationError::Custom(format!(
                "Cannot convert {} to bool",
                vsf_type_name(vsf)
            ))),
        }
    }
}

// === SIGNED INTEGERS ===

impl IntoVsfType for i8 {
    fn into_vsf_type(self) -> VsfType {
        VsfType::i3(self)
    }
}

impl FromVsfType for i8 {
    fn from_vsf_type(vsf: &VsfType) -> ValidationResult<Self> {
        match vsf {
            VsfType::i3(v) => Ok(*v),
            VsfType::i(v) if *v >= i8::MIN as isize && *v <= i8::MAX as isize => Ok(*v as i8),
            _ => Err(ValidationError::Custom(format!(
                "Cannot convert {} to i8",
                vsf_type_name(vsf)
            ))),
        }
    }
}

impl IntoVsfType for i16 {
    fn into_vsf_type(self) -> VsfType {
        VsfType::i4(self)
    }
}

impl FromVsfType for i16 {
    fn from_vsf_type(vsf: &VsfType) -> ValidationResult<Self> {
        match vsf {
            VsfType::i4(v) => Ok(*v),
            VsfType::i3(v) => Ok(*v as i16),
            VsfType::i(v) if *v >= i16::MIN as isize && *v <= i16::MAX as isize => Ok(*v as i16),
            _ => Err(ValidationError::Custom(format!(
                "Cannot convert {} to i16",
                vsf_type_name(vsf)
            ))),
        }
    }
}

impl IntoVsfType for i32 {
    fn into_vsf_type(self) -> VsfType {
        VsfType::i5(self)
    }
}

impl FromVsfType for i32 {
    fn from_vsf_type(vsf: &VsfType) -> ValidationResult<Self> {
        match vsf {
            VsfType::i5(v) => Ok(*v),
            VsfType::i3(v) => Ok(*v as i32),
            VsfType::i4(v) => Ok(*v as i32),
            VsfType::i(v) if *v >= i32::MIN as isize && *v <= i32::MAX as isize => Ok(*v as i32),
            _ => Err(ValidationError::Custom(format!(
                "Cannot convert {} to i32",
                vsf_type_name(vsf)
            ))),
        }
    }
}

impl IntoVsfType for i64 {
    fn into_vsf_type(self) -> VsfType {
        VsfType::i6(self)
    }
}

impl FromVsfType for i64 {
    fn from_vsf_type(vsf: &VsfType) -> ValidationResult<Self> {
        match vsf {
            VsfType::i6(v) => Ok(*v),
            VsfType::i3(v) => Ok(*v as i64),
            VsfType::i4(v) => Ok(*v as i64),
            VsfType::i5(v) => Ok(*v as i64),
            VsfType::i(v) => Ok(*v as i64),
            _ => Err(ValidationError::Custom(format!(
                "Cannot convert {} to i64",
                vsf_type_name(vsf)
            ))),
        }
    }
}

impl IntoVsfType for isize {
    fn into_vsf_type(self) -> VsfType {
        VsfType::i(self)
    }
}

impl FromVsfType for isize {
    fn from_vsf_type(vsf: &VsfType) -> ValidationResult<Self> {
        match vsf {
            VsfType::i(v) => Ok(*v),
            VsfType::i3(v) => Ok(*v as isize),
            VsfType::i4(v) => Ok(*v as isize),
            VsfType::i5(v) => Ok(*v as isize),
            VsfType::i6(v) => Ok(*v as isize),
            _ => Err(ValidationError::Custom(format!(
                "Cannot convert {} to isize",
                vsf_type_name(vsf)
            ))),
        }
    }
}

// === FLOATS ===

impl IntoVsfType for f32 {
    fn into_vsf_type(self) -> VsfType {
        VsfType::f5(self)
    }
}

impl FromVsfType for f32 {
    fn from_vsf_type(vsf: &VsfType) -> ValidationResult<Self> {
        match vsf {
            VsfType::f5(v) => Ok(*v),
            VsfType::f6(v) => Ok(*v as f32),
            _ => Err(ValidationError::Custom(format!(
                "Cannot convert {} to f32",
                vsf_type_name(vsf)
            ))),
        }
    }
}

impl IntoVsfType for f64 {
    fn into_vsf_type(self) -> VsfType {
        VsfType::f6(self)
    }
}

impl FromVsfType for f64 {
    fn from_vsf_type(vsf: &VsfType) -> ValidationResult<Self> {
        match vsf {
            VsfType::f6(v) => Ok(*v),
            VsfType::f5(v) => Ok(*v as f64),
            _ => Err(ValidationError::Custom(format!(
                "Cannot convert {} to f64",
                vsf_type_name(vsf)
            ))),
        }
    }
}

// === STRINGS ===

impl IntoVsfType for String {
    fn into_vsf_type(self) -> VsfType {
        VsfType::x(self)
    }
}

impl FromVsfType for String {
    fn from_vsf_type(vsf: &VsfType) -> ValidationResult<Self> {
        match vsf {
            VsfType::x(s) => Ok(s.clone()),
            VsfType::d(s) => Ok(s.clone()),
            VsfType::a(s) => Ok(s.clone()),
            _ => Err(ValidationError::Custom(format!(
                "Cannot convert {} to String",
                vsf_type_name(vsf)
            ))),
        }
    }
}

impl IntoVsfType for &str {
    fn into_vsf_type(self) -> VsfType {
        VsfType::x(self.to_string())
    }
}

/// Wrapper type for ASCII text (VsfType::a)
///
/// Use this type when you want to explicitly create ASCII-only text instead of UTF-8 Unicode text. This is useful for applications that need ASCII-only mode or when interoperating with systems that only support ASCII.
///
/// # Example
/// ```
/// use vsf::schema::{AsciiText, IntoVsfType};
///
/// // Create ASCII text explicitly
/// let ascii = AsciiText::new("hello");
/// let vsf_value = ascii.into_vsf_type(); // VsfType::a("hello")
///
/// // Use in a schema builder
/// # use vsf::schema::{SectionSchema, TypeConstraint};
/// # let schema = SectionSchema::new("test").field("name", TypeConstraint::AsciiText);
/// let section = schema.build().set("name", AsciiText::new("nick"))?.encode()?;
/// # Ok::<(), vsf::schema::ValidationError>(())
/// ```
///
/// Contrast with regular strings that create UTF-8 text:
/// ```
/// use vsf::schema::IntoVsfType; use vsf::VsfType;
///
/// let utf8 = "hello".into_vsf_type();     // VsfType::x("hello") let ascii = vsf::schema::AsciiText::new("hello").into_vsf_type();  // VsfType::a("hello")
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AsciiText(pub String);

impl AsciiText {
    /// Create a new ASCII text value
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Get the inner string as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume and return the inner string
    pub fn into_string(self) -> String {
        self.0
    }
}

impl IntoVsfType for AsciiText {
    fn into_vsf_type(self) -> VsfType {
        VsfType::a(self.0)
    }
}

impl FromVsfType for AsciiText {
    fn from_vsf_type(vsf: &VsfType) -> ValidationResult<Self> {
        match vsf {
            VsfType::a(s) => Ok(AsciiText(s.clone())),
            _ => Err(ValidationError::Custom(format!(
                "Cannot convert {} to AsciiText (expected a type)",
                vsf_type_name(vsf)
            ))),
        }
    }
}

// === BYTES ===

impl IntoVsfType for Vec<u8> {
    fn into_vsf_type(self) -> VsfType {
        VsfType::hb(self) // Default to rolling hash for raw bytes
    }
}

impl FromVsfType for Vec<u8> {
    fn from_vsf_type(vsf: &VsfType) -> ValidationResult<Self> {
        match vsf {
            VsfType::hb(bytes) => Ok(bytes.clone()),
            VsfType::hp(bytes) => Ok(bytes.clone()),
            VsfType::hs(bytes) => Ok(bytes.clone()),
            VsfType::hm(bytes) => Ok(bytes.clone()),
            VsfType::hg(bytes) => Ok(bytes.clone()),
            VsfType::ke(bytes) => Ok(bytes.clone()),
            VsfType::kx(bytes) => Ok(bytes.clone()),
            VsfType::kp(bytes) => Ok(bytes.clone()),
            VsfType::kc(bytes) => Ok(bytes.clone()),
            VsfType::ka(bytes) => Ok(bytes.clone()),
            VsfType::ge(bytes) => Ok(bytes.clone()),
            VsfType::gp(bytes) => Ok(bytes.clone()),
            VsfType::v(_, bytes) => Ok(bytes.clone()),
            _ => Err(ValidationError::Custom(format!(
                "Cannot convert {} to Vec<u8>",
                vsf_type_name(vsf)
            ))),
        }
    }
}

// === u128 (widest unsigned) ===

impl IntoVsfType for u128 {
    fn into_vsf_type(self) -> VsfType {
        VsfType::u7(self)
    }
}

impl FromVsfType for u128 {
    fn from_vsf_type(vsf: &VsfType) -> ValidationResult<Self> {
        match vsf {
            VsfType::u7(v) => Ok(*v),
            VsfType::u3(v) => Ok(*v as u128),
            VsfType::u4(v) => Ok(*v as u128),
            VsfType::u5(v) => Ok(*v as u128),
            VsfType::u6(v) => Ok(*v as u128),
            VsfType::u(v, _) => Ok(*v as u128),
            VsfType::u0(v) => Ok(if *v { 1 } else { 0 }),
            _ => Err(ValidationError::Custom(format!(
                "Cannot convert {} to u128",
                vsf_type_name(vsf)
            ))),
        }
    }
}

// === FIXED-SIZE BYTE ARRAYS ===

// 32-byte fixed arrays cover the BLAKE3-width hashes, the Photon app-specific hashes, and the 32-byte key types. The length is checked so a wrong-width value fails loudly rather than truncating.
impl FromVsfType for [u8; 32] {
    fn from_vsf_type(vsf: &VsfType) -> ValidationResult<Self> {
        let bytes: &[u8] = match vsf {
            VsfType::hp(b)
            | VsfType::hb(b)
            | VsfType::hs(b)
            | VsfType::hm(b)
            | VsfType::hg(b)
            | VsfType::hP(b)
            | VsfType::hI(b)
            | VsfType::hV(b)
            | VsfType::ke(b)
            | VsfType::kx(b)
            | VsfType::kc(b)
            | VsfType::ka(b) => b,
            _ => {
                return Err(ValidationError::Custom(format!(
                    "Cannot convert {} to [u8; 32]",
                    vsf_type_name(vsf)
                )))
            }
        };
        bytes.try_into().map_err(|_| {
            ValidationError::Custom(format!(
                "Expected 32 bytes for [u8; 32], got {}",
                bytes.len()
            ))
        })
    }
}

// 64-byte fixed arrays cover Ed25519 signatures (ge) and HMAC-SHA512 MACs (mh). The length is checked so a wrong-width value fails loudly rather than truncating.
impl FromVsfType for [u8; 64] {
    fn from_vsf_type(vsf: &VsfType) -> ValidationResult<Self> {
        let bytes: &[u8] = match vsf {
            VsfType::ge(b) | VsfType::mh(b) => b,
            _ => {
                return Err(ValidationError::Custom(format!(
                    "Cannot convert {} to [u8; 64]",
                    vsf_type_name(vsf)
                )))
            }
        };
        bytes.try_into().map_err(|_| {
            ValidationError::Custom(format!(
                "Expected 64 bytes for [u8; 64], got {}",
                bytes.len()
            ))
        })
    }
}

// === EAGLE TIME ===

impl IntoVsfType for EtType {
    fn into_vsf_type(self) -> VsfType {
        VsfType::e(self)
    }
}

impl FromVsfType for EtType {
    fn from_vsf_type(vsf: &VsfType) -> ValidationResult<Self> {
        match vsf {
            VsfType::e(et) => Ok(et.clone()),
            _ => Err(ValidationError::Custom(format!(
                "Cannot convert {} to EtType (expected e type)",
                vsf_type_name(vsf)
            ))),
        }
    }
}

// === VsfType PASSTHROUGH ===

impl IntoVsfType for VsfType {
    fn into_vsf_type(self) -> VsfType {
        self
    }
}

impl FromVsfType for VsfType {
    fn from_vsf_type(vsf: &VsfType) -> ValidationResult<Self> {
        Ok(vsf.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u32_conversion() {
        let original: u32 = 12345;
        let vsf = original.into_vsf_type();
        assert!(matches!(vsf, VsfType::u5(12345)));

        let extracted = u32::from_vsf_type(&vsf).unwrap();
        assert_eq!(extracted, original);
    }

    #[test]
    fn test_string_conversion() {
        let original = "hello world".to_string();
        let vsf = original.clone().into_vsf_type();
        assert!(matches!(vsf, VsfType::x(_)));

        let extracted = String::from_vsf_type(&vsf).unwrap();
        assert_eq!(extracted, original);
    }

    #[test]
    fn test_f64_conversion() {
        let original: f64 = 3.14159;
        let vsf = original.into_vsf_type();
        assert!(matches!(vsf, VsfType::f6(_)));

        let extracted = f64::from_vsf_type(&vsf).unwrap();
        assert_eq!(extracted, original);
    }

    #[test]
    fn test_cross_size_unsigned() {
        // u8 -> VsfType -> u16 should work
        let u8_val: u8 = 42;
        let vsf = u8_val.into_vsf_type();
        let u16_val = u16::from_vsf_type(&vsf).unwrap();
        assert_eq!(u16_val, 42u16);
    }

    #[test]
    fn test_bool_conversion() {
        let vsf_true = true.into_vsf_type();
        assert!(matches!(vsf_true, VsfType::u0(true)));
        assert_eq!(bool::from_vsf_type(&vsf_true).unwrap(), true);

        let vsf_false = false.into_vsf_type();
        assert!(matches!(vsf_false, VsfType::u0(false)));
        assert_eq!(bool::from_vsf_type(&vsf_false).unwrap(), false);
    }

    #[test]
    fn test_type_mismatch_error() {
        let vsf = VsfType::f6(3.14);
        let result = u32::from_vsf_type(&vsf);
        assert!(result.is_err());
    }

    #[test]
    fn test_u128_roundtrip_and_widening() {
        let original: u128 = 340_282_366_920_938_463_463_374_607_431_768_211_455; // u128::MAX
        let vsf = original.into_vsf_type();
        assert!(matches!(vsf, VsfType::u7(_)));
        assert_eq!(u128::from_vsf_type(&vsf).unwrap(), original);

        // Widening from a narrower unsigned still lands in u128.
        assert_eq!(u128::from_vsf_type(&VsfType::u5(42)).unwrap(), 42u128);
    }

    #[test]
    fn test_array32_len_checked() {
        let good = VsfType::hP(vec![7u8; 32]);
        assert_eq!(<[u8; 32]>::from_vsf_type(&good).unwrap(), [7u8; 32]);

        let wrong_len = VsfType::hP(vec![7u8; 31]);
        assert!(<[u8; 32]>::from_vsf_type(&wrong_len).is_err());

        let wrong_type = VsfType::u5(5);
        assert!(<[u8; 32]>::from_vsf_type(&wrong_type).is_err());
    }

    #[test]
    fn test_array64_matches_ge_and_mh() {
        let sig = VsfType::ge(vec![9u8; 64]);
        assert_eq!(<[u8; 64]>::from_vsf_type(&sig).unwrap(), [9u8; 64]);

        let mac = VsfType::mh(vec![1u8; 64]);
        assert_eq!(<[u8; 64]>::from_vsf_type(&mac).unwrap(), [1u8; 64]);

        let wrong_len = VsfType::ge(vec![9u8; 32]);
        assert!(<[u8; 64]>::from_vsf_type(&wrong_len).is_err());
    }

    #[test]
    fn test_ettype_roundtrip() {
        let et = EtType::e6(123_456);
        let vsf = et.clone().into_vsf_type();
        assert!(matches!(vsf, VsfType::e(_)));
        assert_eq!(EtType::from_vsf_type(&vsf).unwrap(), et);
    }
}
