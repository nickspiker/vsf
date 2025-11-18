//! VSF Schema System - Type-safe section and field validation
//!
//! This module provides schema definitions for VSF sections, enabling:
//! - Type-safe section builders with Rust-native syntax
//! - Field validation against allowed values
//! - Round-trip parsing: VSF → Struct → VSF
//! - Automatic error messages for invalid fields
//!
//! ## Architecture
//!
//! - **SectionSchema**: Defines allowed fields for a section type
//! - **FieldSchema**: Defines field name, type constraints, validation rules
//! - **SchemaRegistry**: Registry of official VSF section types
//! - **UserRegistry**: Registry for application-specific sections
//!
//! ## Design Principles
//!
//! 1. **Order Independence**: Field order doesn't matter (except z, y, b in header)
//! 2. **Parse → Modify → Encode**: Support round-trip modifications
//! 3. **Compile-time Safety**: Use Rust type system where possible
//! 4. **Runtime Validation**: Schema enforcement for dynamic sections
//!
//! ## Example
//!
//! ```ignore
//! // Define schema
//! let image_schema = SectionSchema::new("image")
//!     .field("iso", FieldType::U16)
//!     .field("shutter_speed", FieldType::F32)
//!     .field("aperture", FieldType::F32);
//!
//! // Build section with validation
//! let section = image_schema.build()
//!     .set("iso", 400)?
//!     .set("shutter_speed", 1.0/125.0)?
//!     .set("aperture", 2.8)?
//!     .encode()?;
//!
//! // Parse and validate
//! let parsed = image_schema.parse(&bytes)?;
//! assert_eq!(parsed.get::<u16>("iso")?, 400);
//! ```

pub mod constraint;
pub mod conversions;
pub mod field;
pub mod official;
pub mod registry;
pub mod section;
pub mod validate;

pub use constraint::TypeConstraint;
pub use conversions::{AsciiText, FromVsfType, IntoVsfType};
pub use field::FieldSchema;
pub use official::register_official_schemas;
pub use registry::{SchemaRegistry, UserRegistry};
pub use section::{FieldValue, SectionBuilder, SectionSchema};
pub use validate::{ValidationError, ValidationResult};

/// Prelude for common schema operations
pub mod prelude {
    pub use super::constraint::TypeConstraint;
    pub use super::conversions::{AsciiText, FromVsfType, IntoVsfType};
    pub use super::field::FieldSchema;
    pub use super::registry::{SchemaRegistry, UserRegistry};
    pub use super::section::{FieldValue, SectionBuilder, SectionSchema};
    pub use super::validate::{ValidationError, ValidationResult};
}
