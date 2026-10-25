//! Schema registries for official and user-defined sections (stub implementation)
//!
//! TODO: Implement full registry with official VSF schemas

use super::section::SectionSchema;
use super::validate::{ValidationError, ValidationResult};
use crate::prelude::*;
use spin::{Once, RwLock};

/// Registry of official VSF section schemas. Backed by `spin::RwLock` over a Vec sorted by name (a handful of schemas: binary search, no tree), so this works under `no_std + alloc`; the global singleton is a `spin::Once` and exposes a `&'static SchemaRegistry`.
#[derive(Debug)]
pub struct SchemaRegistry {
    schemas: RwLock<Vec<SectionSchema>>,
}

impl SchemaRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            schemas: RwLock::new(Vec::new()),
        }
    }

    /// Get the global registry instance with official schemas pre-registered
    pub fn global() -> &'static Self {
        static INSTANCE: Once<SchemaRegistry> = Once::new();
        INSTANCE.call_once(|| {
            let registry = SchemaRegistry::new();
            super::official::register_official_schemas(&registry);
            registry
        })
    }

    /// Register a schema
    pub fn register(&self, schema: SectionSchema) {
        let mut schemas = self.schemas.write();
        match schemas.binary_search_by(|s| s.name.as_str().cmp(schema.name.as_str())) {
            Ok(i) => schemas[i] = schema,
            Err(i) => schemas.insert(i, schema),
        }
    }

    /// Get a schema by name
    pub fn get(&self, name: &str) -> ValidationResult<SectionSchema> {
        let schemas = self.schemas.read();
        schemas
            .binary_search_by(|s| s.name.as_str().cmp(name))
            .map(|i| schemas[i].clone())
            .map_err(|_| ValidationError::UnknownSection {
                name: name.to_string(),
            })
    }

    /// List all registered schema names
    pub fn list(&self) -> Vec<String> {
        let schemas = self.schemas.read();
        schemas.iter().map(|s| s.name.clone()).collect()
    }
}

impl Default for SchemaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry for user-defined section schemas
pub type UserRegistry = SchemaRegistry;
