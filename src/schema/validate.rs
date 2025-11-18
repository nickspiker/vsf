//! Validation error types and results

use std::fmt;

/// Validation error for schema enforcement
#[derive(Debug, Clone)]
pub enum ValidationError {
    /// Field not allowed in this section
    UnknownField {
        section: String,
        field: String,
        allowed: Vec<String>,
    },
    /// Required field missing
    MissingField { section: String, field: String },
    /// Type mismatch
    TypeMismatch {
        section: String,
        field: String,
        expected: String,
        got: String,
    },
    /// Invalid value
    InvalidValue {
        section: String,
        field: String,
        reason: String,
    },
    /// Section not found in registry
    UnknownSection { name: String },
    /// Type constraint violation
    TypeConstraintViolation { constraint: String, got: String },
    /// Custom validation error
    Custom(String),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::UnknownField {
                section,
                field,
                allowed,
            } => {
                write!(
                    f,
                    "Field '{}' not valid for section '{}'. Allowed fields: {}",
                    field,
                    section,
                    allowed.join(", ")
                )
            }
            ValidationError::MissingField { section, field } => {
                write!(
                    f,
                    "Required field '{}' missing in section '{}'",
                    field, section
                )
            }
            ValidationError::TypeMismatch {
                section,
                field,
                expected,
                got,
            } => {
                write!(
                    f,
                    "Type mismatch in section '{}', field '{}': expected {}, got {}",
                    section, field, expected, got
                )
            }
            ValidationError::InvalidValue {
                section,
                field,
                reason,
            } => {
                write!(
                    f,
                    "Invalid value for section '{}', field '{}': {}",
                    section, field, reason
                )
            }
            ValidationError::UnknownSection { name } => {
                write!(f, "Unknown section type '{}'", name)
            }
            ValidationError::TypeConstraintViolation { constraint, got } => {
                write!(f, "Type constraint violation: expected {}, got {}", constraint, got)
            }
            ValidationError::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for ValidationError {}

impl From<String> for ValidationError {
    fn from(msg: String) -> Self {
        ValidationError::Custom(msg)
    }
}

impl From<&str> for ValidationError {
    fn from(msg: &str) -> Self {
        ValidationError::Custom(msg.to_string())
    }
}

/// Validation result type
pub type ValidationResult<T> = Result<T, ValidationError>;
