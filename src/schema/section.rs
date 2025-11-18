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
    pub fn parse(schema: SectionSchema, section_bytes: &[u8]) -> ValidationResult<Self> {
        // TODO: Implement parsing - extract fields from VSF section bytes
        // This will use vsf::parse() to decode the section structure
        Err(ValidationError::Custom(
            "SectionBuilder::parse() not yet implemented".to_string(),
        ))
    }
}
