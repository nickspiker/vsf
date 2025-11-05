//! VSF Encoding Module
//!
//! This module handles encoding Rust types into VSF binary format.

pub mod flatten;
pub mod primitives;
pub mod traits;

// Re-export main traits
pub use traits::{EncodeNumber, EncodeNumberInclusive};
