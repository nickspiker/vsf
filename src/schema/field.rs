//! Field schema definitions using TypeConstraint
//!
//! Defines how fields within VSF sections are validated using type constraints
//! rather than parallel type enums. This ensures complete coverage of all VsfType
//! variants without duplication.

use super::constraint::{vsf_type_name, TypeConstraint};
use super::validate::ValidationResult;
use crate::VsfType;

/// Schema definition for a field within a section
///
/// Fields are validated using TypeConstraint which pattern-matches against
/// VsfType variants rather than duplicating the type system.
#[derive(Clone)]
pub struct FieldSchema {
    pub name: String,
    pub constraint: TypeConstraint,
    pub required: bool,
    pub description: Option<String>,
    pub default: Option<VsfType>,
}

impl FieldSchema {
    /// Create a new field schema with a type constraint
    pub fn new(name: impl Into<String>, constraint: TypeConstraint) -> Self {
        Self {
            name: name.into(),
            constraint,
            required: false,
            description: None,
            default: None,
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

    /// Set default value (automatically marks field as optional)
    pub fn default(mut self, value: VsfType) -> Self {
        self.default = Some(value);
        self.required = false;
        self
    }

    /// Validate a VsfType against this field schema
    pub fn validate(&self, value: &VsfType) -> ValidationResult<()> {
        self.constraint.validate(value)
    }
}

impl std::fmt::Debug for FieldSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FieldSchema")
            .field("name", &self.name)
            .field("constraint", &self.constraint)
            .field("required", &self.required)
            .field("description", &self.description)
            .field("default", &self.default.as_ref().map(vsf_type_name))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_schema_creation() {
        let field = FieldSchema::new("width", TypeConstraint::AnyUnsigned)
            .required()
            .description("Image width in pixels");

        assert_eq!(field.name, "width");
        assert!(field.required);
        assert_eq!(field.description, Some("Image width in pixels".to_string()));
    }

    #[test]
    fn test_field_validation() {
        let field = FieldSchema::new("count", TypeConstraint::AnyUnsigned);

        // Should accept unsigned integers
        assert!(field.validate(&VsfType::u5(42)).is_ok());
        assert!(field.validate(&VsfType::u3(10)).is_ok());

        // Should reject non-unsigned
        assert!(field.validate(&VsfType::f6(3.14)).is_err());
        assert!(field.validate(&VsfType::x("hello".to_string())).is_err());
    }

    #[test]
    fn test_field_with_default() {
        let field =
            FieldSchema::new("timeout", TypeConstraint::AnyUnsigned).default(VsfType::u5(30));

        assert!(!field.required); // default() makes it optional
        assert!(field.default.is_some());
    }
}
