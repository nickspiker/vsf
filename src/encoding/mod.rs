//! VSF Encoding Module
//!
//! This module handles encoding Rust types into VSF binary format.

pub mod flatten;
pub mod primitives;
pub mod traits;

// Re-export main traits and functions
pub use flatten::hash_placeholder;
pub use traits::{EncodeNumber, EncodeNumberInclusive};
