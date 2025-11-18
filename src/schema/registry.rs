//! Schema registries for official and user-defined sections (stub implementation)
//!
//! TODO: Implement full registry with official VSF schemas

use super::section::SectionSchema;
use super::validate::{ValidationError, ValidationResult};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Registry of official VSF section schemas
#[derive(Debug, Clone)]
pub struct SchemaRegistry {
    schemas: Arc<RwLock<HashMap<String, SectionSchema>>>,
}

impl SchemaRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            schemas: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the global registry instance with official schemas pre-registered
    pub fn global() -> &'static Self {
        use std::sync::OnceLock;
        static INSTANCE: OnceLock<SchemaRegistry> = OnceLock::new();
        INSTANCE.get_or_init(|| {
            let registry = SchemaRegistry::new();
            super::official::register_official_schemas(&registry);
            registry
        })
    }

    /// Register a schema
    pub fn register(&self, schema: SectionSchema) {
        let mut schemas = self.schemas.write().unwrap();
        schemas.insert(schema.name.clone(), schema);
    }

    /// Get a schema by name
    pub fn get(&self, name: &str) -> ValidationResult<SectionSchema> {
        let schemas = self.schemas.read().unwrap();
        schemas
            .get(name)
            .cloned()
            .ok_or_else(|| ValidationError::UnknownSection {
                name: name.to_string(),
            })
    }

    /// List all registered schema names
    pub fn list(&self) -> Vec<String> {
        let schemas = self.schemas.read().unwrap();
        schemas.keys().cloned().collect()
    }
}

impl Default for SchemaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry for user-defined section schemas
pub type UserRegistry = SchemaRegistry;
