//! Type constraints for VSF schema validation
//!
//! TypeConstraint provides a flexible way to validate VsfType values without duplicating the VsfType enum. Instead of parallel type systems, we use pattern-based constraints that can match categories, specific types, or apply custom validation logic.

use crate::prelude::*;
use super::validate::{ValidationError, ValidationResult};
use crate::types::{EtType, VsfType};
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc;
use core::fmt;

/// Type constraint for VSF schema fields
///
/// Constraints validate VsfType values using pattern matching rather than creating a parallel type system. This ensures complete coverage of all
/// VsfType variants including future additions.
#[derive(Clone)]
pub enum TypeConstraint {
    // === CATEGORY CONSTRAINTS (most common) ===
    /// Any unsigned integer type (u0, u, u3-u7)
    AnyUnsigned,
    /// Any signed integer type (i, i3-i7)
    AnySigned,
    /// Any integer type (unsigned or signed)
    AnyInteger,
    /// Any float type (f5, f6)
    AnyFloat,
    /// Any numeric type (integer or float)
    AnyNumeric,
    /// Any complex number (j5, j6)
    AnyComplex,

    // === EAGLE TIME VARIANTS ===
    /// Any Eagle Time type (e(EtType::*))
    AnyEagleTime,
    /// Eagle Time with specific backing type constraint
    EagleTime(Box<TypeConstraint>),

    // === STRING TYPES ===
    /// Any string type (x, d, l)
    AnyString,
    /// UTF-8 text (x) - User-facing Unicode text
    Utf8Text,
    /// Dictionary key (d) - Internal naming (section names, field names, keys)
    DictKey,
    /// ASCII text (l) - User-facing ASCII-only text
    AsciiText,

    // === HASH TYPES ===
    /// Any hash type (hp, hb, hs, hm, hg)
    AnyHash,
    /// BLAKE3 provenance hash (hp)
    Blake3Provenance,
    /// BLAKE3 rolling hash (hb)
    Blake3Rolling,
    /// SHA hash (hs) - SHA-256, SHA-512, etc.
    ShaHash,
    /// Smear hash (hm) - XOR of BLAKE3 + SHA3-256 + SHA-512
    Smear,
    /// Spaghetti hash (hg) - domain-separated, maximally weird mixing
    Spaghetti,

    // === KEY TYPES ===
    /// Any cryptographic key (ke, kx, kp, kc, ka)
    AnyKey,
    /// Ed25519 public key (ke)
    Ed25519Key,
    /// X25519 key (kx)
    X25519Key,
    /// ECDSA P-256 key (kp)
    EcdsaP256Key,
    /// Curve25519 key (kc)
    Curve25519Key,
    /// AES key (ka)
    AesKey,

    // === SIGNATURE TYPES ===
    /// Any signature type (ge, gp)
    AnySignature,
    /// Ed25519 signature (ge)
    Ed25519Sig,
    /// ECDSA P-256 signature (gp)
    EcdsaP256Sig,

    // === TENSOR TYPES ===
    /// Any contiguous tensor with element constraint (t_*)
    Tensor(Box<TypeConstraint>),
    /// Any strided tensor with element constraint (q_*)
    StridedTensor(Box<TypeConstraint>),

    // === SPIRIX TYPES ===
    #[cfg(feature = "spirix")]
    /// Any Spirix scalar (s33-s77)
    AnyScalar,
    #[cfg(feature = "spirix")]
    /// Any Spirix circle (c33-c77)
    AnyCircle,

    // === COLOUR TYPES ===
    /// Any colour type (r*)
    AnyColour,

    // === WRAPPED DATA ===
    /// Wrapped data with specific encoding byte
    Wrapped(u8),
    /// Any wrapped data (v)
    AnyWrapped,

    // === COMPOSITE CONSTRAINTS ===
    /// Value must match ANY of these constraints (OR)
    OneOf(Vec<TypeConstraint>),
    /// Value must match ALL of these constraints (AND)
    AllOf(Vec<TypeConstraint>),

    // === CUSTOM VALIDATION ===
    /// Custom predicate function for complex validation.
    /// Requires `Arc`, which requires `target_has_atomic = "ptr"`; gated out on single-core embedded targets (e.g., Cortex-M0+).
    #[cfg(target_has_atomic = "ptr")]
    Custom {
        name: &'static str,
        description: &'static str,
        validator: Arc<dyn Fn(&VsfType) -> bool + Send + Sync>,
    },

    // === PERMISSIVE ===
    /// Accept any VsfType
    Any,
}

impl TypeConstraint {
    /// Validate a VsfType against this constraint
    pub fn validate(&self, value: &VsfType) -> ValidationResult<()> {
        if self.matches(value) {
            Ok(())
        } else {
            Err(ValidationError::TypeConstraintViolation {
                constraint: self.description(),
                got: vsf_type_name(value),
            })
        }
    }

    /// Check if a VsfType matches this constraint
    pub fn matches(&self, value: &VsfType) -> bool {
        match self {
            TypeConstraint::AnyUnsigned => matches!(
                value,
                VsfType::u0(_)
                    | VsfType::u(_, _)
                    | VsfType::u3(_)
                    | VsfType::u4(_)
                    | VsfType::u5(_)
                    | VsfType::u6(_)
                    | VsfType::u7(_)
            ),

            TypeConstraint::AnySigned => matches!(
                value,
                VsfType::i(_)
                    | VsfType::i3(_)
                    | VsfType::i4(_)
                    | VsfType::i5(_)
                    | VsfType::i6(_)
                    | VsfType::i7(_)
            ),

            TypeConstraint::AnyInteger => {
                TypeConstraint::AnyUnsigned.matches(value)
                    || TypeConstraint::AnySigned.matches(value)
            }

            TypeConstraint::AnyFloat => matches!(value, VsfType::f5(_) | VsfType::f6(_)),

            TypeConstraint::AnyNumeric => {
                TypeConstraint::AnyInteger.matches(value) || TypeConstraint::AnyFloat.matches(value)
            }

            TypeConstraint::AnyComplex => matches!(value, VsfType::j5(_) | VsfType::j6(_)),

            TypeConstraint::AnyEagleTime => matches!(value, VsfType::e(_)),

            TypeConstraint::EagleTime(inner) => {
                if let VsfType::e(et) = value {
                    #[allow(deprecated)]
                    match et {
                        EtType::e5(_) => inner.matches(&VsfType::i5(0)),
                        EtType::e6(_) => inner.matches(&VsfType::i6(0)),
                        EtType::e7(_) => inner.matches(&VsfType::i6(0)),
                        EtType::f5(_) => inner.matches(&VsfType::f5(0.0)),
                        EtType::f6(_) => inner.matches(&VsfType::f6(0.0)),
                    }
                } else {
                    false
                }
            }

            TypeConstraint::AnyString => {
                matches!(value, VsfType::x(_) | VsfType::d(_) | VsfType::a(_))
            }

            TypeConstraint::Utf8Text => matches!(value, VsfType::x(_)),
            TypeConstraint::DictKey => matches!(value, VsfType::d(_)),
            TypeConstraint::AsciiText => matches!(value, VsfType::a(_)),

            TypeConstraint::AnyHash => {
                matches!(
                    value,
                    VsfType::hp(_)
                        | VsfType::hb(_)
                        | VsfType::hs(_)
                        | VsfType::hm(_)
                        | VsfType::hg(_)
                        | VsfType::hP(_)
                )
            }

            TypeConstraint::Blake3Provenance => matches!(value, VsfType::hp(_)),
            TypeConstraint::Blake3Rolling => matches!(value, VsfType::hb(_)),
            TypeConstraint::ShaHash => matches!(value, VsfType::hs(_)),
            TypeConstraint::Smear => matches!(value, VsfType::hm(_)),
            TypeConstraint::Spaghetti => matches!(value, VsfType::hg(_)),

            TypeConstraint::AnyKey => matches!(
                value,
                VsfType::ke(_) | VsfType::kx(_) | VsfType::kp(_) | VsfType::kc(_) | VsfType::ka(_)
            ),

            TypeConstraint::Ed25519Key => matches!(value, VsfType::ke(_)),
            TypeConstraint::X25519Key => matches!(value, VsfType::kx(_)),
            TypeConstraint::EcdsaP256Key => matches!(value, VsfType::kp(_)),
            TypeConstraint::Curve25519Key => matches!(value, VsfType::kc(_)),
            TypeConstraint::AesKey => matches!(value, VsfType::ka(_)),

            TypeConstraint::AnySignature => matches!(value, VsfType::ge(_) | VsfType::gp(_)),

            TypeConstraint::Ed25519Sig => matches!(value, VsfType::ge(_)),
            TypeConstraint::EcdsaP256Sig => matches!(value, VsfType::gp(_)),

            TypeConstraint::Tensor(_elem_constraint) => {
                // Check if it's any t_* variant
                // TODO: Also validate element type with elem_constraint
                matches!(
                    value,
                    VsfType::t_u0(_)
                        | VsfType::t_u3(_)
                        | VsfType::t_u4(_)
                        | VsfType::t_u5(_)
                        | VsfType::t_u6(_)
                        | VsfType::t_u7(_)
                        | VsfType::t_i3(_)
                        | VsfType::t_i4(_)
                        | VsfType::t_i5(_)
                        | VsfType::t_i6(_)
                        | VsfType::t_i7(_)
                        | VsfType::t_f5(_)
                        | VsfType::t_f6(_)
                        | VsfType::t_j5(_)
                        | VsfType::t_j6(_)
                )
            }

            TypeConstraint::StridedTensor(_elem_constraint) => {
                // Check if it's any q_* variant
                matches!(
                    value,
                    VsfType::q_u0(_)
                        | VsfType::q_u3(_)
                        | VsfType::q_u4(_)
                        | VsfType::q_u5(_)
                        | VsfType::q_u6(_)
                        | VsfType::q_u7(_)
                        | VsfType::q_i3(_)
                        | VsfType::q_i4(_)
                        | VsfType::q_i5(_)
                        | VsfType::q_i6(_)
                        | VsfType::q_i7(_)
                        | VsfType::q_f5(_)
                        | VsfType::q_f6(_)
                        | VsfType::q_j5(_)
                        | VsfType::q_j6(_)
                )
            }

            #[cfg(feature = "spirix")]
            TypeConstraint::AnyScalar => {
                matches!(
                    value,
                    VsfType::s33(_)
                        | VsfType::s34(_)
                        | VsfType::s35(_)
                        | VsfType::s36(_)
                        | VsfType::s37(_)
                        | VsfType::s43(_)
                        | VsfType::s44(_)
                        | VsfType::s45(_)
                        | VsfType::s46(_)
                        | VsfType::s47(_)
                        | VsfType::s53(_)
                        | VsfType::s54(_)
                        | VsfType::s55(_)
                        | VsfType::s56(_)
                        | VsfType::s57(_)
                        | VsfType::s63(_)
                        | VsfType::s64(_)
                        | VsfType::s65(_)
                        | VsfType::s66(_)
                        | VsfType::s67(_)
                        | VsfType::s73(_)
                        | VsfType::s74(_)
                        | VsfType::s75(_)
                        | VsfType::s76(_)
                        | VsfType::s77(_)
                )
            }

            #[cfg(feature = "spirix")]
            TypeConstraint::AnyCircle => {
                matches!(
                    value,
                    VsfType::c33(_)
                        | VsfType::c34(_)
                        | VsfType::c35(_)
                        | VsfType::c36(_)
                        | VsfType::c37(_)
                        | VsfType::c43(_)
                        | VsfType::c44(_)
                        | VsfType::c45(_)
                        | VsfType::c46(_)
                        | VsfType::c47(_)
                        | VsfType::c53(_)
                        | VsfType::c54(_)
                        | VsfType::c55(_)
                        | VsfType::c56(_)
                        | VsfType::c57(_)
                        | VsfType::c63(_)
                        | VsfType::c64(_)
                        | VsfType::c65(_)
                        | VsfType::c66(_)
                        | VsfType::c67(_)
                        | VsfType::c73(_)
                        | VsfType::c74(_)
                        | VsfType::c75(_)
                        | VsfType::c76(_)
                        | VsfType::c77(_)
                )
            }

            TypeConstraint::AnyColour => vsf_type_name(value).starts_with('r'),

            TypeConstraint::Wrapped(encoding) => {
                if let VsfType::v(enc, _) = value {
                    enc == encoding
                } else {
                    false
                }
            }

            TypeConstraint::AnyWrapped => matches!(value, VsfType::v(_, _)),

            TypeConstraint::OneOf(constraints) => constraints.iter().any(|c| c.matches(value)),

            TypeConstraint::AllOf(constraints) => constraints.iter().all(|c| c.matches(value)),

            #[cfg(target_has_atomic = "ptr")]
            TypeConstraint::Custom { validator, .. } => validator(value),

            TypeConstraint::Any => true,
        }
    }

    /// Get a human-readable description of this constraint
    pub fn description(&self) -> String {
        match self {
            TypeConstraint::AnyUnsigned => "any unsigned integer".into(),
            TypeConstraint::AnySigned => "any signed integer".into(),
            TypeConstraint::AnyInteger => "any integer".into(),
            TypeConstraint::AnyFloat => "any float (f5/f6)".into(),
            TypeConstraint::AnyNumeric => "any numeric type".into(),
            TypeConstraint::AnyComplex => "any complex number".into(),
            TypeConstraint::AnyEagleTime => "Eagle Time (any backing)".into(),
            TypeConstraint::EagleTime(inner) => {
                format!("Eagle Time with {} backing", inner.description())
            }
            TypeConstraint::AnyString => "any string (x/d/l)".into(),
            TypeConstraint::Utf8Text => "UTF-8 text (x)".into(),
            TypeConstraint::DictKey => "dictionary key (d)".into(),
            TypeConstraint::AsciiText => "ASCII text (l)".into(),
            TypeConstraint::AnyHash => "any hash type".into(),
            TypeConstraint::Blake3Provenance => "BLAKE3 provenance hash (hp)".into(),
            TypeConstraint::Blake3Rolling => "BLAKE3 rolling hash (hb)".into(),
            TypeConstraint::ShaHash => "SHA hash (hs)".into(),
            TypeConstraint::Smear => "smear hash (hm)".into(),
            TypeConstraint::Spaghetti => "spaghetti hash (hg)".into(),
            TypeConstraint::AnyKey => "any cryptographic key".into(),
            TypeConstraint::Ed25519Key => "Ed25519 public key (ke)".into(),
            TypeConstraint::X25519Key => "X25519 key (kx)".into(),
            TypeConstraint::EcdsaP256Key => "ECDSA P-256 key (kp)".into(),
            TypeConstraint::Curve25519Key => "Curve25519 key (kc)".into(),
            TypeConstraint::AesKey => "AES key (ka)".into(),
            TypeConstraint::AnySignature => "any signature".into(),
            TypeConstraint::Ed25519Sig => "Ed25519 signature (ge)".into(),
            TypeConstraint::EcdsaP256Sig => "ECDSA P-256 signature (gp)".into(),
            TypeConstraint::Tensor(inner) => format!("tensor of {}", inner.description()),
            TypeConstraint::StridedTensor(inner) => {
                format!("strided tensor of {}", inner.description())
            }
            #[cfg(feature = "spirix")]
            TypeConstraint::AnyScalar => "any Spirix scalar".into(),
            #[cfg(feature = "spirix")]
            TypeConstraint::AnyCircle => "any Spirix circle".into(),
            TypeConstraint::AnyColour => "any colour type".into(),
            TypeConstraint::Wrapped(enc) => format!("wrapped data (encoding {})", enc),
            TypeConstraint::AnyWrapped => "any wrapped data".into(),
            TypeConstraint::OneOf(constraints) => {
                let descs: Vec<String> = constraints.iter().map(|c| c.description()).collect();
                format!("one of [{}]", descs.join(", "))
            }
            TypeConstraint::AllOf(constraints) => {
                let descs: Vec<String> = constraints.iter().map(|c| c.description()).collect();
                format!("all of [{}]", descs.join(", "))
            }
            #[cfg(target_has_atomic = "ptr")]
            TypeConstraint::Custom { description, .. } => (*description).into(),
            TypeConstraint::Any => "any type".into(),
        }
    }
}

impl fmt::Debug for TypeConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TypeConstraint({})", self.description())
    }
}

impl fmt::Display for TypeConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_any_unsigned_matches() {
        let constraint = TypeConstraint::AnyUnsigned;
        assert!(constraint.matches(&VsfType::u3(42)));
        assert!(constraint.matches(&VsfType::u4(1000)));
        assert!(constraint.matches(&VsfType::u5(100000)));
        assert!(constraint.matches(&VsfType::u6(1000000000)));
        assert!(!constraint.matches(&VsfType::i3(-42)));
        assert!(!constraint.matches(&VsfType::f5(3.14)));
    }

    #[test]
    fn test_eagle_time_matches() {
        let constraint = TypeConstraint::AnyEagleTime;
        assert!(constraint.matches(&VsfType::e(EtType::e6(123456))));
        assert!(constraint.matches(&VsfType::e(EtType::e5(123))));
        assert!(constraint.matches(&VsfType::e(EtType::e7(123456789))));
        #[allow(deprecated)]
        {
            assert!(constraint.matches(&VsfType::e(EtType::f6(123.456))));
            assert!(constraint.matches(&VsfType::e(EtType::f5(123.456))));
        }
        assert!(!constraint.matches(&VsfType::f6(123.456)));
    }

    #[test]
    fn test_eagle_time_with_float_backing() {
        let constraint = TypeConstraint::EagleTime(Box::new(TypeConstraint::AnyFloat));
        #[allow(deprecated)]
        {
            assert!(constraint.matches(&VsfType::e(EtType::f6(123.456))));
            assert!(constraint.matches(&VsfType::e(EtType::f5(123.456))));
            assert!(!constraint.matches(&VsfType::e(EtType::e6(123456))));
        }
    }

    #[test]
    fn test_hash_types() {
        assert!(TypeConstraint::Blake3Provenance.matches(&VsfType::hp(vec![0u8; 32])));
        assert!(TypeConstraint::Blake3Rolling.matches(&VsfType::hb(vec![0u8; 32])));
        assert!(TypeConstraint::AnyHash.matches(&VsfType::hp(vec![0u8; 32])));
        assert!(TypeConstraint::AnyHash.matches(&VsfType::hb(vec![0u8; 32])));
    }

    #[test]
    fn test_key_types() {
        assert!(TypeConstraint::Ed25519Key.matches(&VsfType::ke(vec![0u8; 32])));
        assert!(TypeConstraint::X25519Key.matches(&VsfType::kx(vec![0u8; 32])));
        assert!(TypeConstraint::AnyKey.matches(&VsfType::ke(vec![0u8; 32])));
        assert!(TypeConstraint::AnyKey.matches(&VsfType::kx(vec![0u8; 32])));
    }

    #[test]
    fn test_one_of_constraint() {
        let constraint =
            TypeConstraint::OneOf(vec![TypeConstraint::AnyUnsigned, TypeConstraint::AnyString]);

        assert!(constraint.matches(&VsfType::u5(42)));
        assert!(constraint.matches(&VsfType::x("hello".to_string())));
        assert!(!constraint.matches(&VsfType::f6(3.14)));
    }

    #[test]
    fn test_validation_error() {
        let constraint = TypeConstraint::AnyUnsigned;
        let result = constraint.validate(&VsfType::f6(3.14));

        assert!(result.is_err());
        if let Err(ValidationError::TypeConstraintViolation { constraint: c, got }) = result {
            assert_eq!(c, "any unsigned integer");
            assert_eq!(got, "f6");
        } else {
            panic!("Expected TypeConstraintViolation error");
        }
    }
}

/// Helper function to get human-readable type name for VsfType
/// Used in error messages and debugging.
///
/// With `errors-verbose` (default-on), uses Debug formatting and extracts the
/// variant name prefix. Without it (embedded builds), returns `"?"` — this
/// kills the `<VsfType as Debug>::fmt` chain (and its ~10 KB of IEEE-754
/// float-to-string code) from being linked. Callers that depend on the
/// returned string for runtime classification (e.g. `AnyColour`'s
/// `starts_with('r')` check) only work correctly under `errors-verbose`.
#[cfg(feature = "errors-verbose")]
pub fn vsf_type_name(vsf: &VsfType) -> String {
    let debug_str = format!("{:?}", vsf);
    debug_str
        .split('(')
        .next()
        .unwrap_or(&debug_str)
        .to_string()
}

#[cfg(not(feature = "errors-verbose"))]
pub fn vsf_type_name(_vsf: &VsfType) -> alloc::string::String {
    alloc::string::String::from("?")
}
