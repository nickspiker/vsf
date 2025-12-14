//! Type conversions between Rust primitives and VsfType
//!
//! Provides ergonomic trait-based conversions for working with VsfType values
//! without needing intermediate wrapper types.
//!
//! # Example: IntoVsfType (Rust → VsfType)
//!
//! ```rust
//! use vsf::schema::IntoVsfType;
//! use vsf::VsfType;
//!
//! // Automatic conversion from Rust primitives
//! let u: VsfType = 42u32.into_vsf_type();          // VsfType::u5(42)
//! let i: VsfType = (-100i32).into_vsf_type();      // VsfType::i5(-100)
//! let f: VsfType = 3.14f64.into_vsf_type();        // VsfType::f6(3.14)
//! let b: VsfType = true.into_vsf_type();           // VsfType::u0(true)
//! let s: VsfType = "hello".into_vsf_type();        // VsfType::x("hello")
//!
//! // Used automatically in SectionBuilder::set()
//! let section = schema.builder()
//!     .set("count", 42u32)?      // Calls into_vsf_type() internally
//!     .set("name", "test")?      // Calls into_vsf_type() internally
//!     .build()?;
//! ```
//!
//! # Example: FromVsfType (VsfType → Rust)
//!
//! ```rust
//! use vsf::schema::FromVsfType;
//! use vsf::VsfType;
//!
//! let vsf = VsfType::u5(42);
//!
//! // Extract to specific Rust type
//! let n: u32 = u32::from_vsf_type(&vsf)?;          // Ok(42)
//! let n2: u64 = u64::from_vsf_type(&vsf)?;         // Ok(42u64) - cross-size works
//!
//! // Type mismatch produces clear error
//! let vsf = VsfType::f6(3.14);
//! let result = u32::from_vsf_type(&vsf);
//! assert!(result.is_err());  // Cannot convert f6 to u32
//!
//! // Used automatically in SectionBuilder::get_value()
//! let count: u32 = section.get_value("count")?;    // Calls from_vsf_type() internally
//! ```
//!
//! # Cross-Size Conversions
//!
//! The traits support automatic widening conversions with bounds checking:
//!
//! ```rust
//! use vsf::schema::FromVsfType;
//! use vsf::VsfType;
//!
//! // Widen u8 → u32 (always safe)
//! let small = VsfType::u3(100u8);
//! let big: u32 = u32::from_vsf_type(&small)?;      // Ok(100u32)
//!
//! // Narrow u32 → u8 (bounds checked)
//! let vsf = VsfType::u5(1000u32);
//! let result = u8::from_vsf_type(&vsf);
//! assert!(result.is_err());  // 1000 doesn't fit in u8
//! ```

use super::constraint::vsf_type_name;
use super::validate::{ValidationError, ValidationResult};
use crate::types::VsfType;

/// Convert Rust types to VsfType
///
/// This trait provides automatic conversion from Rust primitives to the
/// appropriate VsfType variant, making schema APIs ergonomic.
pub trait IntoVsfType {
    fn into_vsf_type(self) -> VsfType;
}

/// Extract Rust types from VsfType
///
/// This trait provides type-safe extraction of Rust primitives from VsfType
/// values, with appropriate error handling for type mismatches.
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
            VsfType::l(s) => Ok(s.clone()),
            _ => Err(ValidationError::Custom(format!(
                "Cannot convert {} to String",
                vsf_type_name(vsf)
            ))),
        }
    }
}

impl<'a> IntoVsfType for &'a str {
    fn into_vsf_type(self) -> VsfType {
        VsfType::x(self.to_string())
    }
}

/// Wrapper type for ASCII text (VsfType::l)
///
/// Use this type when you want to explicitly create ASCII-only text
/// instead of UTF-8 Unicode text. This is useful for applications
/// that need ASCII-only mode or when interoperating with systems
/// that only support ASCII.
///
/// # Example
/// ```
/// use vsf::schema::{AsciiText, IntoVsfType};
/// use vsf::VsfType;
///
/// // Create ASCII text explicitly
/// let ascii = AsciiText::new("hello");
/// let vsf_value = ascii.into_vsf_type();  // VsfType::l("hello")
///
/// // Use in schema builder
/// # use vsf::schema::{SectionSchema, FieldSchema, TypeConstraint};
/// # let schema = SectionSchema::new("test")
/// #     .add_field(FieldSchema::new("name", TypeConstraint::AsciiText));
/// let section = schema.builder()
///     .set("name", AsciiText::new("nick"))?
///     .build()?;
/// # Ok::<(), vsf::schema::ValidationError>(())
/// ```
///
/// Contrast with regular strings that create UTF-8 text:
/// ```
/// use vsf::schema::IntoVsfType;
/// use vsf::VsfType;
///
/// let utf8 = "hello".into_vsf_type();     // VsfType::x("hello")
/// let ascii = vsf::schema::AsciiText::new("hello").into_vsf_type();  // VsfType::l("hello")
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
        VsfType::l(self.0)
    }
}

impl FromVsfType for AsciiText {
    fn from_vsf_type(vsf: &VsfType) -> ValidationResult<Self> {
        match vsf {
            VsfType::l(s) => Ok(AsciiText(s.clone())),
            _ => Err(ValidationError::Custom(format!(
                "Cannot convert {} to AsciiText (expected l type)",
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
}
