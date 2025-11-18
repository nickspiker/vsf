//! Section schema and builder (stub implementation)
//!
//! TODO: Complete implementation for parse → modify → encode workflow

use super::field::{FieldSchema, FieldValue};
use super::validate::{ValidationError, ValidationResult};
use std::collections::HashMap;

/// Schema definition for a VSF section
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
    pub fn field(mut self, name: impl Into<String>, field_type: super::field::FieldType) -> Self {
        self.fields
            .push(FieldSchema::new(name, field_type));
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
#[derive(Debug)]
pub struct SectionBuilder {
    schema: SectionSchema,
    fields: HashMap<String, FieldValue>,
}

impl SectionBuilder {
    /// Create new builder from schema
    pub fn new(schema: SectionSchema) -> Self {
        Self {
            schema,
            fields: HashMap::new(),
        }
    }

    /// Set a field value (with type checking)
    pub fn set(mut self, name: impl AsRef<str>, value: impl Into<FieldValue>) -> ValidationResult<Self> {
        let name = name.as_ref();
        let value = value.into();

        // Validate field exists
        let field_schema = self.schema.validate_field(name)?;

        // Validate type
        field_schema.validate(&value)?;

        self.fields.insert(name.to_string(), value);
        Ok(self)
    }

    /// Get a field value
    pub fn get<T>(&self, name: &str) -> ValidationResult<&FieldValue> {
        self.fields.get(name).ok_or_else(|| {
            ValidationError::Custom(format!("Field '{}' not set", name))
        })
    }

    /// Encode to VSF bytes
    /// Format: [d"section_name" o(...field_values...)]
    /// Fields are ordered according to schema definition (order-independent semantics)
    pub fn encode(&self) -> ValidationResult<Vec<u8>> {
        use crate::VsfType;

        // Check all required fields are set
        for field_schema in &self.schema.fields {
            if field_schema.required && !self.fields.contains_key(&field_schema.name) {
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

        // Encode field values in schema order
        for field_schema in &self.schema.fields {
            if let Some(value) = self.fields.get(&field_schema.name) {
                bytes.extend(value.to_vsf_type().flatten());
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
            _ => return Err(ValidationError::Custom(format!("Expected section name (d), got {:?}", section_name))),
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
        for field_schema in &schema.fields {
            // Check if we've hit the closing ']'
            if ptr >= section_bytes.len() || section_bytes[ptr] == b']' {
                // No more values - remaining fields are unset
                break;
            }

            // Parse the value
            let value = parse(section_bytes, &mut ptr)
                .map_err(|e| ValidationError::Custom(format!("Failed to parse field '{}': {}", field_schema.name, e)))?;

            // Convert VsfType to FieldValue and add to builder
            let field_value = FieldValue::from_vsf_type(&value)?;
            builder = builder.set(&field_schema.name, field_value)?;
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
    use crate::schema::field::FieldType;

    #[test]
    fn test_section_builder_round_trip() {
        // Create a schema
        let schema = SectionSchema::new("test")
            .field("width", FieldType::U32)
            .field("height", FieldType::U32)
            .field("name", FieldType::String);

        // Build a section with the schema
        let builder = schema.build()
            .set("width", 1920u32).unwrap()
            .set("height", 1080u32).unwrap()
            .set("name", "test_section".to_string()).unwrap();

        // Encode it
        let encoded = builder.encode().unwrap();

        // Parse it back
        let parsed = SectionBuilder::parse(schema.clone(), &encoded).unwrap();

        // Re-encode and verify it matches
        let re_encoded = parsed.encode().unwrap();
        assert_eq!(encoded, re_encoded, "Round-trip encoding should produce identical bytes");
    }

    #[test]
    fn test_section_parser_validates_name() {
        let schema = SectionSchema::new("test")
            .field("value", FieldType::U16);

        // Create a section with wrong name
        let wrong_section = SectionSchema::new("wrong")
            .field("value", FieldType::U16);

        let built = wrong_section.build()
            .set("value", 42u16).unwrap();
        let encoded = built.encode().unwrap();

        // Should fail because names don't match
        let result = SectionBuilder::parse(schema, &encoded);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("name mismatch"));
    }

    #[test]
    fn test_section_parser_with_eagle_time() {
        use crate::schema::field::FieldType;

        let schema = SectionSchema::new("metadata")
            .field("timestamp", FieldType::EagleTimeF64)
            .field("count", FieldType::U32);

        let builder = schema.build()
            .set("timestamp", FieldValue::EagleTimeF64(1234567.89)).unwrap()
            .set("count", 42u32).unwrap();

        let encoded = builder.encode().unwrap();
        let parsed = SectionBuilder::parse(schema, &encoded).unwrap();

        // Verify round-trip
        let re_encoded = parsed.encode().unwrap();
        assert_eq!(encoded, re_encoded);
    }
}
