//! Schema registries for official and user-defined sections (stub implementation)
//!
//! TODO: Implement full registry with official VSF schemas

use crate::prelude::*;
use super::section::SectionSchema;
use super::validate::{ValidationError, ValidationResult};
use alloc::collections::BTreeMap;
use spin::{Once, RwLock};

/// Registry of official VSF section schemas. Backed by `spin::RwLock<BTreeMap>` so this works under `no_std + alloc`; the global singleton is a `spin::Once` and exposes a `&'static SchemaRegistry`.
#[derive(Debug)]
pub struct SchemaRegistry {
    schemas: RwLock<BTreeMap<String, SectionSchema>>,
}

impl SchemaRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            schemas: RwLock::new(BTreeMap::new()),
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
        schemas.insert(schema.name.clone(), schema);
    }

    /// Get a schema by name
    pub fn get(&self, name: &str) -> ValidationResult<SectionSchema> {
        let schemas = self.schemas.read();
        schemas
            .get(name)
            .cloned()
            .ok_or_else(|| ValidationError::UnknownSection {
                name: name.to_string(),
            })
    }

    /// List all registered schema names
    pub fn list(&self) -> Vec<String> {
        let schemas = self.schemas.read();
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
