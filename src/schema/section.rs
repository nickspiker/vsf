//! Section schema and builder with positional encoding
//!
//! ## Design: Vec for Building, Positional for Wire Format
//!
//! **Building phase (Vec<Option<VsfType>> indexed by field position):**
//! ```rust
//! let section = schema.builder()
//!     .set("iso", 800u32)?          // Find "iso" in schema (index 0), set fields[0] = Some(u5(800))
//!     .set("aperture", 2.8f32)?     // Find "aperture" (index 1), set fields[1] = Some(f5(2.8))
//!     .set("timestamp", time)?      // Find "timestamp" (index 2), set fields[2] = Some(e(...))
//!     .encode()?;
//! // Vec has length = schema.fields.len(), uses Option for unset fields
//! ```
//!
//! **Wire format (Positional - just the values):**
//! ```text
//! [d"camera" u5{800} f5{2.8} ef6{...}]
//!    └─name   └─value[0] └─value[1] └─value[2]
//!             (written in schema order: iso, aperture, timestamp)
//! ```
//! Values encoded positionally in schema-defined order. Vec discarded after encode().
//!
//! **Why Vec<Option<VsfType>> during building?**
//! - Ergonomic field access by name: `builder.set("field_name", value)`
//! - Order-independent construction: set fields in any order
//! - Parse → modify → re-encode workflow: `builder.set("iso", new_value)?`
//! - Compact memory: Vec length = number of fields in schema (typically 3-10)
//! - Fast linear search: Finding field index in 3-10 element schema is faster than HashMap overhead
//!
//! **Why positional encoding on wire?**
//! - Compact: No field names repeated in binary (30-50% size reduction)
//! - Fast parsing: No string matching, direct positional decode
//! - Schema-driven: Field semantics come from schema, not wire format
//! - Forward compatible: Schema can add optional fields at end
//!
//! **Concrete example with 3 fields:**
//! ```text
//! Schema defines:      ["iso", "aperture", "timestamp"]       (Vec<FieldSchema>)
//! Builder stores:      [Some(u5(800)), Some(f5(2.8)), Some(e(...))]  (Vec<Option<VsfType>>)
//! Wire bytes:          [d"camera" u5{800} f5{2.8} ef6{...}]          (no field names)
//! ```
//!
//! ## Parse → Modify → Encode Workflow
//! ```rust
//! // Parse existing section
//! let mut builder = SectionBuilder::parse(schema, section_bytes)?;
//!
//! // Modify specific fields
//! builder = builder.set("iso", 1600u32)?;      // Update ISO
//!
//! // Re-encode with changes
//! let updated_bytes = builder.encode()?;
//! ```

use super::constraint::TypeConstraint;
use super::conversions::{FromVsfType, IntoVsfType};
use super::field::FieldSchema;
use super::validate::{ValidationError, ValidationResult};
use crate::VsfType;

/// Schema definition for a VSF section
///
/// Defines the structure, field types, and validation rules for a VSF section.
/// Sections are encoded with positional field values (no field names in wire format).
///
/// # Example
/// ```rust
/// use vsf::schema::{SectionSchema, TypeConstraint};
///
/// let schema = SectionSchema::new("camera")
///     .description("Camera metadata")
///     .field("iso", TypeConstraint::AnyUnsigned)
///     .field("aperture", TypeConstraint::AnyFloat)
///     .field("timestamp", TypeConstraint::AnyEagleTime);
///
/// // Fields are encoded positionally in this order: iso, aperture, timestamp
/// ```
#[derive(Debug, Clone)]
pub struct SectionSchema {
    pub name: String,
    pub fields: Vec<FieldSchema>,
    pub description: Option<String>,
}

impl SectionSchema {
    /// Create a new section schema
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            fields: Vec::new(),
            description: None,
        }
    }

    /// Add a field to this section schema
    pub fn field(mut self, name: impl Into<String>, constraint: TypeConstraint) -> Self {
        self.fields.push(FieldSchema::new(name, constraint));
        self
    }

    /// Add description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Create a builder for this section
    pub fn build(&self) -> SectionBuilder {
        SectionBuilder::new(self.clone())
    }

    /// Get allowed field names
    pub fn allowed_fields(&self) -> Vec<String> {
        self.fields.iter().map(|f| f.name.clone()).collect()
    }

    /// Validate a field exists in this schema
    pub fn validate_field(&self, name: &str) -> ValidationResult<&FieldSchema> {
        self.fields
            .iter()
            .find(|f| f.name == name)
            .ok_or_else(|| ValidationError::UnknownField {
                section: self.name.clone(),
                field: name.to_string(),
                allowed: self.allowed_fields(),
            })
    }
}

/// Builder for creating section instances with validation
///
/// Uses Vec<Option<VsfType>> internally, indexed by field position in schema.
/// Field names are looked up via linear search (fast for typical 3-10 field schemas).
///
/// # Example
/// ```rust
/// use vsf::schema::{SectionSchema, TypeConstraint};
///
/// let schema = SectionSchema::new("image")
///     .field("width", TypeConstraint::AnyUnsigned)
///     .field("height", TypeConstraint::AnyUnsigned);
///
/// let section = schema.builder()
///     .set("width", 1920u32)?     // Finds "width" at index 0, sets fields[0]
///     .set("height", 1080u32)?    // Finds "height" at index 1, sets fields[1]
///     .encode()?;                 // Encodes positionally: [d"image" u5{1920} u5{1080}]
/// ```
#[derive(Debug)]
pub struct SectionBuilder {
    schema: SectionSchema,
    fields: Vec<Option<VsfType>>,  // Indexed by schema field position
}

impl SectionBuilder {
    /// Create new builder from schema
    pub fn new(schema: SectionSchema) -> Self {
        let field_count = schema.fields.len();
        Self {
            schema,
            fields: vec![None; field_count],
        }
    }

    /// Set a field value with automatic conversion and validation
    pub fn set<T: IntoVsfType>(
        mut self,
        name: impl AsRef<str>,
        value: T,
    ) -> ValidationResult<Self> {
        let name = name.as_ref();
        let vsf_value = value.into_vsf_type();

        // Find field index in schema (linear search - fast for 3-10 fields)
        let field_index = self
            .schema
            .fields
            .iter()
            .position(|f| f.name == name)
            .ok_or_else(|| ValidationError::UnknownField {
                section: self.schema.name.clone(),
                field: name.to_string(),
                allowed: self.schema.allowed_fields(),
            })?;

        // Validate type constraint
        self.schema.fields[field_index].validate(&vsf_value)?;

        // Set value at field position
        self.fields[field_index] = Some(vsf_value);
        Ok(self)
    }

    /// Get a field value by name
    pub fn get(&self, name: &str) -> ValidationResult<&VsfType> {
        // Find field index
        let field_index = self
            .schema
            .fields
            .iter()
            .position(|f| f.name == name)
            .ok_or_else(|| ValidationError::Custom(format!("Field '{}' not in schema", name)))?;

        // Get value at that index
        self.fields[field_index]
            .as_ref()
            .ok_or_else(|| ValidationError::Custom(format!("Field '{}' not set", name)))
    }

    /// Get and extract a specific Rust type
    pub fn get_value<T: FromVsfType>(&self, name: &str) -> ValidationResult<T> {
        let vsf = self.get(name)?;
        T::from_vsf_type(vsf)
    }

    /// Encode to VSF bytes
    /// Format: [d"section_name" o(...field_values...)]
    /// Fields are ordered according to schema definition (order-independent semantics)
    pub fn encode(&self) -> ValidationResult<Vec<u8>> {
        use crate::VsfType;

        // Check all required fields are set
        for (i, field_schema) in self.schema.fields.iter().enumerate() {
            if field_schema.required && self.fields[i].is_none() {
                return Err(ValidationError::MissingField {
                    section: self.schema.name.clone(),
                    field: field_schema.name.clone(),
                });
            }
        }

        // Build the section bytes
        let mut bytes = Vec::new();

        // Section start marker
        bytes.push(b'[');

        // Section name as dictionary key
        bytes.extend(VsfType::d(self.schema.name.clone()).flatten());

        // Encode field values in schema order (positional)
        for field_value in &self.fields {
            if let Some(value) = field_value {
                bytes.extend(value.flatten());
            }
        }

        // Section end marker
        bytes.push(b']');

        Ok(bytes)
    }

    /// Parse a section from VSF bytes into this builder
    /// This enables the parse → modify → encode workflow
    ///
    /// # Format
    /// Values are encoded POSITIONALLY in schema order (no field names in bytes):
    /// ```text
    /// [d"section_name" value1 value2 value3]
    /// ```
    ///
    /// The schema defines which value corresponds to which field based on position.
    pub fn parse(schema: SectionSchema, section_bytes: &[u8]) -> ValidationResult<Self> {
        use crate::decoding::parse::parse;

        let mut ptr = 0;

        // Check for '[' start marker
        if section_bytes.is_empty() || section_bytes[ptr] != b'[' {
            return Err(ValidationError::Custom(format!(
                "Expected '[' to start section, found {:?}",
                section_bytes.get(ptr)
            )));
        }
        ptr += 1;

        // Parse section name
        let section_name = parse(section_bytes, &mut ptr)
            .map_err(|e| ValidationError::Custom(format!("Failed to parse section name: {}", e)))?;
        let section_name_str = match section_name {
            crate::VsfType::d(name) => name,
            _ => {
                return Err(ValidationError::Custom(format!(
                    "Expected section name (d), got {:?}",
                    section_name
                )))
            }
        };

        // Verify section name matches schema
        if section_name_str != schema.name {
            return Err(ValidationError::Custom(format!(
                "Section name mismatch: expected '{}', found '{}'",
                schema.name, section_name_str
            )));
        }

        let mut builder = SectionBuilder::new(schema.clone());

        // Parse field values in schema order (positional)
        for (i, field_schema) in schema.fields.iter().enumerate() {
            // Check if we've hit the closing ']'
            if ptr >= section_bytes.len() || section_bytes[ptr] == b']' {
                // No more values - remaining fields are unset
                break;
            }

            // Parse the value
            let value = parse(section_bytes, &mut ptr).map_err(|e| {
                ValidationError::Custom(format!(
                    "Failed to parse field '{}': {}",
                    field_schema.name, e
                ))
            })?;

            // Validate and store directly at position i
            field_schema.validate(&value)?;
            builder.fields[i] = Some(value);
        }

        // Expect ']' to close section
        if ptr >= section_bytes.len() || section_bytes[ptr] != b']' {
            return Err(ValidationError::Custom(format!(
                "Expected ']' to close section, found {:?}",
                section_bytes.get(ptr)
            )));
        }

        Ok(builder)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::constraint::TypeConstraint;

    #[test]
    fn test_section_builder_round_trip() {
        // Create a schema
        let schema = SectionSchema::new("test")
            .field("width", TypeConstraint::AnyUnsigned)
            .field("height", TypeConstraint::AnyUnsigned)
            .field("name", TypeConstraint::AnyString);

        // Build a section with the schema
        let builder = schema
            .build()
            .set("width", 1920u32)
            .unwrap()
            .set("height", 1080u32)
            .unwrap()
            .set("name", "test_section".to_string())
            .unwrap();

        // Encode it
        let encoded = builder.encode().unwrap();

        // Parse it back
        let parsed = SectionBuilder::parse(schema.clone(), &encoded).unwrap();

        // Re-encode and verify it matches
        let re_encoded = parsed.encode().unwrap();
        assert_eq!(
            encoded, re_encoded,
            "Round-trip encoding should produce identical bytes"
        );
    }

    #[test]
    fn test_section_parser_validates_name() {
        let schema = SectionSchema::new("test").field("value", TypeConstraint::AnyUnsigned);

        // Create a section with wrong name
        let wrong_section = SectionSchema::new("wrong").field("value", TypeConstraint::AnyUnsigned);

        let built = wrong_section.build().set("value", 42u16).unwrap();
        let encoded = built.encode().unwrap();

        // Should fail because names don't match
        let result = SectionBuilder::parse(schema, &encoded);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("name mismatch"));
    }

    #[test]
    fn test_section_parser_with_eagle_time() {
        use crate::types::EtType;

        let schema = SectionSchema::new("metadata")
            .field("timestamp", TypeConstraint::AnyEagleTime)
            .field("count", TypeConstraint::AnyUnsigned);

        let builder = schema
            .build()
            .set("timestamp", VsfType::e(EtType::f6(1234567.89)))
            .unwrap()
            .set("count", 42u32)
            .unwrap();

        let encoded = builder.encode().unwrap();
        let parsed = SectionBuilder::parse(schema, &encoded).unwrap();

        // Verify round-trip
        let re_encoded = parsed.encode().unwrap();
        assert_eq!(encoded, re_encoded);
    }
}
