//! Field schema definitions and type conversions

use crate::VsfType;
use std::fmt;

/// Supported field types in VSF schemas
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    // Unsigned integers
    U8,
    U16,
    U32,
    U64,
    // Signed integers
    I8,
    I16,
    I32,
    I64,
    // Floats
    F32,
    F64,
    // Eagle Time
    EagleTimeF32,  // ef5
    EagleTimeF64,  // ef6
    // Strings
    String,
    // Binary data
    Bytes,
    // Hashes
    Blake3Hash,    // 32 bytes
    // Keys
    Ed25519Key,    // 32 bytes public key
    X25519Key,     // 32 bytes public key
    // Custom (for extensibility)
    Custom(&'static str),
}

impl fmt::Display for FieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldType::U8 => write!(f, "u8"),
            FieldType::U16 => write!(f, "u16"),
            FieldType::U32 => write!(f, "u32"),
            FieldType::U64 => write!(f, "u64"),
            FieldType::I8 => write!(f, "i8"),
            FieldType::I16 => write!(f, "i16"),
            FieldType::I32 => write!(f, "i32"),
            FieldType::I64 => write!(f, "i64"),
            FieldType::F32 => write!(f, "f32"),
            FieldType::F64 => write!(f, "f64"),
            FieldType::EagleTimeF32 => write!(f, "Eagle Time (f32)"),
            FieldType::EagleTimeF64 => write!(f, "Eagle Time (f64)"),
            FieldType::String => write!(f, "String"),
            FieldType::Bytes => write!(f, "Bytes"),
            FieldType::Blake3Hash => write!(f, "BLAKE3 Hash"),
            FieldType::Ed25519Key => write!(f, "Ed25519 Key"),
            FieldType::X25519Key => write!(f, "X25519 Key"),
            FieldType::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// Field value wrapper for type-safe storage
#[derive(Debug, Clone)]
pub enum FieldValue {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    EagleTimeF32(f32),
    EagleTimeF64(f64),
    String(String),
    Bytes(Vec<u8>),
    VsfType(VsfType),
}

impl FieldValue {
    /// Get the field type of this value
    pub fn field_type(&self) -> FieldType {
        match self {
            FieldValue::U8(_) => FieldType::U8,
            FieldValue::U16(_) => FieldType::U16,
            FieldValue::U32(_) => FieldType::U32,
            FieldValue::U64(_) => FieldType::U64,
            FieldValue::I8(_) => FieldType::I8,
            FieldValue::I16(_) => FieldType::I16,
            FieldValue::I32(_) => FieldType::I32,
            FieldValue::I64(_) => FieldType::I64,
            FieldValue::F32(_) => FieldType::F32,
            FieldValue::F64(_) => FieldType::F64,
            FieldValue::EagleTimeF32(_) => FieldType::EagleTimeF32,
            FieldValue::EagleTimeF64(_) => FieldType::EagleTimeF64,
            FieldValue::String(_) => FieldType::String,
            FieldValue::Bytes(_) => FieldType::Bytes,
            FieldValue::VsfType(_) => FieldType::Custom("VsfType"),
        }
    }

    /// Convert to VsfType for encoding
    pub fn to_vsf_type(&self) -> VsfType {
        match self {
            FieldValue::U8(v) => VsfType::u(*v as usize, false),
            FieldValue::U16(v) => VsfType::u(*v as usize, false),
            FieldValue::U32(v) => VsfType::u(*v as usize, false),
            FieldValue::U64(v) => VsfType::u(*v as usize, false),
            FieldValue::I8(v) => VsfType::i(*v as isize),
            FieldValue::I16(v) => VsfType::i(*v as isize),
            FieldValue::I32(v) => VsfType::i(*v as isize),
            FieldValue::I64(v) => VsfType::i(*v as isize),
            FieldValue::F32(v) => VsfType::f5(*v),
            FieldValue::F64(v) => VsfType::f6(*v),
            FieldValue::EagleTimeF32(v) => {
                use crate::types::EtType;
                VsfType::e(EtType::f5(*v))
            }
            FieldValue::EagleTimeF64(v) => {
                use crate::types::EtType;
                VsfType::e(EtType::f6(*v))
            }
            FieldValue::String(s) => VsfType::x(s.clone()),
            FieldValue::Bytes(b) => VsfType::hb(b.clone()),
            FieldValue::VsfType(vsf) => vsf.clone(),
        }
    }
}

/// Type-safe conversions from Rust primitives to FieldValue
impl From<u8> for FieldValue {
    fn from(v: u8) -> Self {
        FieldValue::U8(v)
    }
}

impl From<u16> for FieldValue {
    fn from(v: u16) -> Self {
        FieldValue::U16(v)
    }
}

impl From<u32> for FieldValue {
    fn from(v: u32) -> Self {
        FieldValue::U32(v)
    }
}

impl From<u64> for FieldValue {
    fn from(v: u64) -> Self {
        FieldValue::U64(v)
    }
}

impl From<i8> for FieldValue {
    fn from(v: i8) -> Self {
        FieldValue::I8(v)
    }
}

impl From<i16> for FieldValue {
    fn from(v: i16) -> Self {
        FieldValue::I16(v)
    }
}

impl From<i32> for FieldValue {
    fn from(v: i32) -> Self {
        FieldValue::I32(v)
    }
}

impl From<i64> for FieldValue {
    fn from(v: i64) -> Self {
        FieldValue::I64(v)
    }
}

impl From<f32> for FieldValue {
    fn from(v: f32) -> Self {
        FieldValue::F32(v)
    }
}

impl From<f64> for FieldValue {
    fn from(v: f64) -> Self {
        FieldValue::F64(v)
    }
}

impl From<String> for FieldValue {
    fn from(v: String) -> Self {
        FieldValue::String(v)
    }
}

impl From<&str> for FieldValue {
    fn from(v: &str) -> Self {
        FieldValue::String(v.to_string())
    }
}

impl From<Vec<u8>> for FieldValue {
    fn from(v: Vec<u8>) -> Self {
        FieldValue::Bytes(v)
    }
}

impl From<VsfType> for FieldValue {
    fn from(v: VsfType) -> Self {
        FieldValue::VsfType(v)
    }
}

/// Schema definition for a field within a section
#[derive(Debug, Clone)]
pub struct FieldSchema {
    pub name: String,
    pub field_type: FieldType,
    pub required: bool,
    pub description: Option<String>,
}

impl FieldSchema {
    /// Create a new field schema
    pub fn new(name: impl Into<String>, field_type: FieldType) -> Self {
        Self {
            name: name.into(),
            field_type,
            required: false,
            description: None,
        }
    }

    /// Mark field as required
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Add description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Validate a value against this field schema
    pub fn validate(&self, value: &FieldValue) -> Result<(), String> {
        let value_type = value.field_type();

        // Type matching with some flexibility for numeric types
        let valid = match (self.field_type, value_type) {
            // Exact match
            (a, b) if a == b => true,
            // Allow any unsigned integer for unsigned field
            (FieldType::U8 | FieldType::U16 | FieldType::U32 | FieldType::U64,
             FieldType::U8 | FieldType::U16 | FieldType::U32 | FieldType::U64) => true,
            // Allow any signed integer for signed field
            (FieldType::I8 | FieldType::I16 | FieldType::I32 | FieldType::I64,
             FieldType::I8 | FieldType::I16 | FieldType::I32 | FieldType::I64) => true,
            // Allow any float for float field
            (FieldType::F32 | FieldType::F64, FieldType::F32 | FieldType::F64) => true,
            // VsfType can be anything
            (_, FieldType::Custom("VsfType")) => true,
            _ => false,
        };

        if valid {
            Ok(())
        } else {
            Err(format!(
                "Field '{}' expects type {}, got {}",
                self.name, self.field_type, value_type
            ))
        }
    }
}
