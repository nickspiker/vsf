//! Section schema and builder with named field encoding
//!
//! ## Design: Multi-Valued Fields with Named Encoding
//!
//! **Building phase (Vec<FieldValue> with multi-valued support):**
//! ```rust
//! let section = schema.builder()
//!     .set("iso", 800u32)?                 // Single value: FieldValue { name: "iso", values: [u5(800)] }
//!     .set_multi("size", vec![3u8, 4, 5])? // Multi-value: FieldValue { name: "size", values: [u3(3), u3(4), u3(5)] }
//!     .set_empty("cloudy")?                // Empty: FieldValue { name: "cloudy", values: [] }
//!     .encode()?;
//! ```
//!
//! **Wire format (Named fields with d-type keys):**
//! ```text
//! [d"camera" (d"iso":u5{800}) (d"size":u3{3},u3{4},u3{5}) (d"cloudy")]
//!    └─name   └─single value   └─multiple values         └─empty vector
//! ```
//! Each field is encoded as:
//! - `(d"field_name":val1,val2,val3)` for multi-valued fields
//! - `(d"field_name":value)` for single value
//! - `(d"field_name")` for empty vector (no colon!)
//!
//! **Why FieldValue with Vec<VsfType>?**
//! - Multi-valued fields: Support multiple values per field like `(d"size":u3{3},u3{4},u3{5})`
//! - Empty vectors: Fields can be present but empty like `(d"cloudy")`
//! - Type flexibility: Each value can be different VsfType (validated against constraint)
//! - Compressible: Field names compress well with dictionary compression
//!
//! **Why named field encoding on wire?**
//! - Type flexibility: Different sections can have different field types (u5, f6, etc.)
//! - Compressible: Field names compress well (3-15 chars become 1-2 bytes with dictionary compression)
//! - Order-independent parsing: Fields can be read in any order
//! - Optional fields: Missing fields are simply omitted from encoding
//! - Future-compatible: New fields can be added without breaking existing parsers
//!
//! **Concrete example with multi-valued fields:**
//! ```text
//! Schema defines:      ["iso", "size", "cloudy"]       (Vec<FieldSchema>)
//! Builder stores:      [FieldValue { name: "iso", values: [u5(800)] },
//!                       FieldValue { name: "size", values: [u3(3), u3(4), u3(5)] },
//!                       FieldValue { name: "cloudy", values: [] }]
//! Wire bytes:          [d"camera" (d"iso":u5{800}) (d"size":u3{3},u3{4},u3{5}) (d"cloudy")]
//! ```
//!
//! ## Parse → Modify → Encode Workflow
//! ```rust
//! // Parse existing section
//! let mut builder = SectionBuilder::parse(schema, section_bytes)?;
//!
//! // Modify specific fields
//! builder = builder.set("iso", 1600u32)?;      // Update ISO
//! builder = builder.add_value("size", 6u8)?;   // Add value to existing field
//!
//! // Re-encode with changes
//! let updated_bytes = builder.encode()?;
//! ```

use super::constraint::TypeConstraint;
use super::conversions::{FromVsfType, IntoVsfType};
use super::field::FieldSchema;
use super::validate::{ValidationError, ValidationResult};
use crate::VsfType;

/// A field with a name and vector of VsfType values
///
/// Fields can have:
/// - 0 values (empty): `(d"cloudy")` - field present but empty
/// - 1 value: `(d"port":u4{41641})` - single value
/// - N values: `(d"size":u3{3},u3{4},u3{5})` - multiple comma-separated values
#[derive(Debug, Clone)]
pub struct FieldValue {
    pub name: String,
    pub values: Vec<VsfType>,
}

impl FieldValue {
    /// Create a new field with the given name and values
    pub fn new(name: impl Into<String>, values: Vec<VsfType>) -> Self {
        Self {
            name: name.into(),
            values,
        }
    }

    /// Create an empty field (no values)
    pub fn empty(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            values: Vec::new(),
        }
    }

    /// Create a field with a single value
    pub fn single(name: impl Into<String>, value: VsfType) -> Self {
        Self {
            name: name.into(),
            values: vec![value],
        }
    }

    /// Flatten this field to wire format:
    /// - Empty: `(d"name")`
    /// - Single: `(d"name":value)`
    /// - Multi: `(d"name":val1,val2,val3)`
    pub fn flatten(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Opening parenthesis
        bytes.push(b'(');

        // Field name as d-type
        bytes.extend(VsfType::d(self.name.clone()).flatten());

        // Values (if any)
        if !self.values.is_empty() {
            bytes.push(b':');
            for (i, value) in self.values.iter().enumerate() {
                if i > 0 {
                    bytes.push(b',');  // Comma separator between values
                }
                bytes.extend(value.flatten());
            }
        }

        // Closing parenthesis
        bytes.push(b')');

        bytes
    }
}

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
/// Uses Vec<FieldValue> internally to support multi-valued fields.
/// Field names are looked up via linear search (fast for typical 3-10 field schemas).
///
/// # Example
/// ```rust
/// use vsf::schema::{SectionSchema, TypeConstraint};
///
/// let schema = SectionSchema::new("image")
///     .field("width", TypeConstraint::AnyUnsigned)
///     .field("height", TypeConstraint::AnyUnsigned)
///     .field("cloudy", TypeConstraint::AnyUnsigned);
///
/// let section = schema.build()
///     .set("width", 1920u32)?           // Single value
///     .set("height", 1080u32)?          // Single value
///     .set_empty("cloudy")?             // Empty field
///     .encode()?;
/// // Wire: [d"image" (d"width":u5{1920}) (d"height":u5{1080}) (d"cloudy")]
/// ```
#[derive(Debug)]
pub struct SectionBuilder {
    schema: SectionSchema,
    fields: Vec<FieldValue>,  // Multi-valued fields
}

impl SectionBuilder {
    /// Create new builder from schema (empty fields)
    pub fn new(schema: SectionSchema) -> Self {
        Self {
            schema,
            fields: Vec::new(),
        }
    }

    /// Set a field with a single value (replaces existing field if present)
    pub fn set<T: IntoVsfType>(
        mut self,
        name: impl AsRef<str>,
        value: T,
    ) -> ValidationResult<Self> {
        let name = name.as_ref();
        let vsf_value = value.into_vsf_type();

        // Validate field exists in schema
        let field_schema = self.schema.validate_field(name)?;

        // Validate type constraint
        field_schema.validate(&vsf_value)?;

        // Remove existing field with this name (if any)
        self.fields.retain(|f| f.name != name);

        // Add new field with single value
        self.fields.push(FieldValue::single(name, vsf_value));

        Ok(self)
    }

    /// Set an empty field (field present but no values)
    pub fn set_empty(mut self, name: impl AsRef<str>) -> ValidationResult<Self> {
        let name = name.as_ref();

        // Validate field exists in schema
        self.schema.validate_field(name)?;

        // Remove existing field with this name (if any)
        self.fields.retain(|f| f.name != name);

        // Add empty field
        self.fields.push(FieldValue::empty(name));

        Ok(self)
    }

    /// Set a field with multiple values (replaces existing field if present)
    pub fn set_multi<T: IntoVsfType>(
        mut self,
        name: impl AsRef<str>,
        values: Vec<T>,
    ) -> ValidationResult<Self> {
        let name = name.as_ref();

        // Validate field exists in schema
        let field_schema = self.schema.validate_field(name)?;

        // Convert and validate all values
        let mut vsf_values = Vec::new();
        for value in values {
            let vsf_value = value.into_vsf_type();
            field_schema.validate(&vsf_value)?;
            vsf_values.push(vsf_value);
        }

        // Remove existing field with this name (if any)
        self.fields.retain(|f| f.name != name);

        // Add new field with multiple values
        self.fields.push(FieldValue::new(name, vsf_values));

        Ok(self)
    }

    /// Add a value to an existing field (creates field if not present)
    pub fn add_value<T: IntoVsfType>(
        mut self,
        name: impl AsRef<str>,
        value: T,
    ) -> ValidationResult<Self> {
        let name = name.as_ref();
        let vsf_value = value.into_vsf_type();

        // Validate field exists in schema
        let field_schema = self.schema.validate_field(name)?;

        // Validate type constraint
        field_schema.validate(&vsf_value)?;

        // Find existing field or create new one
        if let Some(field) = self.fields.iter_mut().find(|f| f.name == name) {
            field.values.push(vsf_value);
        } else {
            self.fields.push(FieldValue::single(name, vsf_value));
        }

        Ok(self)
    }

    /// Get a field by name (returns the vector of values)
    pub fn get(&self, name: &str) -> ValidationResult<&Vec<VsfType>> {
        self.fields
            .iter()
            .find(|f| f.name == name)
            .map(|f| &f.values)
            .ok_or_else(|| ValidationError::Custom(format!("Field '{}' not set", name)))
    }

    /// Get the first value from a field and extract as a specific Rust type
    pub fn get_value<T: FromVsfType>(&self, name: &str) -> ValidationResult<T> {
        let values = self.get(name)?;
        let first_value = values
            .first()
            .ok_or_else(|| ValidationError::Custom(format!("Field '{}' is empty", name)))?;
        T::from_vsf_type(first_value)
    }

    /// Get all values from a field
    pub fn get_values(&self, name: &str) -> ValidationResult<Vec<VsfType>> {
        Ok(self.get(name)?.clone())
    }

    /// Encode to VSF bytes
    /// Format: [d"section_name" (d"field1":val1,val2) (d"field2") ...]
    /// Uses FieldValue.flatten() for each field
    pub fn encode(&self) -> ValidationResult<Vec<u8>> {
        use crate::VsfType;

        // Check all required fields are set
        for field_schema in &self.schema.fields {
            if field_schema.required
                && !self.fields.iter().any(|f| f.name == field_schema.name)
            {
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

        // Encode each field using FieldValue.flatten()
        for field in &self.fields {
            bytes.extend(field.flatten());
        }

        // Section end marker
        bytes.push(b']');

        Ok(bytes)
    }

    /// Parse a section from VSF bytes into this builder
    /// This enables the parse → modify → encode workflow
    ///
    /// # Format
    /// Fields can have:
    /// - Empty: `(d"field_name")`
    /// - Single: `(d"field_name":value)`
    /// - Multi: `(d"field_name":val1,val2,val3)`
    ///
    /// ```text
    /// [d"section_name" (d"field1":val1,val2) (d"field2") (d"field3":val)]
    /// ```
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

        // Parse fields: (d"field_name":val1,val2,val3) or (d"field_name")
        loop {
            // Skip whitespace
            while ptr < section_bytes.len() && section_bytes[ptr].is_ascii_whitespace() {
                ptr += 1;
            }

            // Check for end of section
            if ptr >= section_bytes.len() || section_bytes[ptr] == b']' {
                break;
            }

            // Expect '(' to start field
            if section_bytes[ptr] != b'(' {
                return Err(ValidationError::Custom(format!(
                    "Expected '(' to start field at position {}, found {:?}",
                    ptr,
                    section_bytes.get(ptr)
                )));
            }
            ptr += 1;

            // Parse field name (d"name")
            let field_name_vsf = parse(section_bytes, &mut ptr)
                .map_err(|e| ValidationError::Custom(format!("Failed to parse field name: {}", e)))?;
            let field_name = match field_name_vsf {
                crate::VsfType::d(name) => name,
                _ => {
                    return Err(ValidationError::Custom(format!(
                        "Expected field name (d), got {:?}",
                        field_name_vsf
                    )))
                }
            };

            // Check if this is an empty field (no colon)
            // Skip whitespace first
            while ptr < section_bytes.len() && section_bytes[ptr].is_ascii_whitespace() {
                ptr += 1;
            }

            let mut values = Vec::new();

            // Check for ':' - if present, parse values
            if ptr < section_bytes.len() && section_bytes[ptr] == b':' {
                ptr += 1;  // Skip ':'

                // Parse comma-separated values
                loop {
                    // Parse a value
                    let value = parse(section_bytes, &mut ptr).map_err(|e| {
                        ValidationError::Custom(format!(
                            "Failed to parse field '{}' value: {}",
                            field_name, e
                        ))
                    })?;

                    // Validate against schema
                    let field_schema = schema.validate_field(&field_name)?;
                    field_schema.validate(&value)?;

                    values.push(value);

                    // Skip whitespace
                    while ptr < section_bytes.len() && section_bytes[ptr].is_ascii_whitespace() {
                        ptr += 1;
                    }

                    // Check for comma (more values) or ')' (end of field)
                    if ptr >= section_bytes.len() {
                        return Err(ValidationError::Custom(format!(
                            "Unexpected end of input while parsing field '{}'",
                            field_name
                        )));
                    }

                    if section_bytes[ptr] == b',' {
                        ptr += 1;  // Skip comma, continue parsing values
                        continue;
                    } else if section_bytes[ptr] == b')' {
                        break;  // End of field
                    } else {
                        return Err(ValidationError::Custom(format!(
                            "Expected ',' or ')' after value in field '{}', found {:?}",
                            field_name,
                            section_bytes.get(ptr)
                        )));
                    }
                }
            }

            // Expect ')' to close field
            if ptr >= section_bytes.len() || section_bytes[ptr] != b')' {
                return Err(ValidationError::Custom(format!(
                    "Expected ')' to close field '{}', found {:?}",
                    field_name,
                    section_bytes.get(ptr)
                )));
            }
            ptr += 1;

            // Add field to builder
            builder.fields.push(FieldValue::new(field_name, values));
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

    #[test]
    fn test_multi_valued_fields() {
        // Test fields with multiple values
        let schema = SectionSchema::new("multi")
            .field("sizes", TypeConstraint::AnyUnsigned)
            .field("temperatures", TypeConstraint::AnyFloat);

        let builder = schema
            .build()
            .set_multi("sizes", vec![3u8, 4u8, 5u8])
            .unwrap()
            .set_multi("temperatures", vec![20.5f32, 21.0f32, 19.8f32])
            .unwrap();

        let encoded = builder.encode().unwrap();

        // Parse it back
        let parsed = SectionBuilder::parse(schema, &encoded).unwrap();

        // Verify we can get the values back
        let sizes = parsed.get_values("sizes").unwrap();
        assert_eq!(sizes.len(), 3);

        let temps = parsed.get_values("temperatures").unwrap();
        assert_eq!(temps.len(), 3);

        // Verify round-trip
        let re_encoded = parsed.encode().unwrap();
        assert_eq!(encoded, re_encoded);
    }

    #[test]
    fn test_empty_field() {
        // Test empty field encoding: (d"cloudy")
        let schema = SectionSchema::new("weather")
            .field("cloudy", TypeConstraint::AnyUnsigned)
            .field("temp", TypeConstraint::AnyFloat);

        let builder = schema
            .build()
            .set_empty("cloudy")
            .unwrap()
            .set("temp", 22.5f32)
            .unwrap();

        let encoded = builder.encode().unwrap();

        // Verify the wire format contains the empty field marker
        let encoded_str = String::from_utf8_lossy(&encoded);
        assert!(encoded_str.contains("cloudy"));

        // Parse it back
        let parsed = SectionBuilder::parse(schema, &encoded).unwrap();

        // Verify empty field
        let cloudy_values = parsed.get("cloudy").unwrap();
        assert_eq!(cloudy_values.len(), 0);

        // Verify round-trip
        let re_encoded = parsed.encode().unwrap();
        assert_eq!(encoded, re_encoded);
    }

    #[test]
    fn test_mixed_single_multi_empty_fields() {
        // Test a realistic mix of field types
        let schema = SectionSchema::new("data")
            .field("port", TypeConstraint::AnyUnsigned)       // Single value
            .field("sizes", TypeConstraint::AnyUnsigned)      // Multiple values
            .field("cloudy", TypeConstraint::AnyUnsigned)     // Empty
            .field("temps", TypeConstraint::AnyFloat);        // Multiple values

        let builder = schema
            .build()
            .set("port", 41641u16)
            .unwrap()
            .set_multi("sizes", vec![3u8, 4u8, 5u8])
            .unwrap()
            .set_empty("cloudy")
            .unwrap()
            .set_multi("temps", vec![20.0f32, 21.0f32])
            .unwrap();

        let encoded = builder.encode().unwrap();

        // Parse it back
        let parsed = SectionBuilder::parse(schema, &encoded).unwrap();

        // Verify all fields
        assert_eq!(parsed.get_value::<u16>("port").unwrap(), 41641);
        assert_eq!(parsed.get_values("sizes").unwrap().len(), 3);
        assert_eq!(parsed.get_values("cloudy").unwrap().len(), 0);
        assert_eq!(parsed.get_values("temps").unwrap().len(), 2);

        // Verify round-trip
        let re_encoded = parsed.encode().unwrap();
        assert_eq!(encoded, re_encoded);
    }

    #[test]
    fn test_add_value_method() {
        // Test incrementally adding values to a field
        let schema = SectionSchema::new("test")
            .field("values", TypeConstraint::AnyUnsigned);

        let builder = schema
            .build()
            .add_value("values", 1u8)
            .unwrap()
            .add_value("values", 2u8)
            .unwrap()
            .add_value("values", 3u8)
            .unwrap();

        let values = builder.get_values("values").unwrap();
        assert_eq!(values.len(), 3);

        let encoded = builder.encode().unwrap();
        let parsed = SectionBuilder::parse(schema, &encoded).unwrap();

        // Verify we got all 3 values back
        let parsed_values = parsed.get_values("values").unwrap();
        assert_eq!(parsed_values.len(), 3);
    }
}
