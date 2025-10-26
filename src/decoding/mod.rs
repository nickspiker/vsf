//! VSF Decoding Module
//!
//! This module handles decoding VSF binary format back into Rust types.

pub mod helpers;
pub mod metadata;
pub mod parse;
pub mod primitives;
pub mod spirix;
pub mod tensors;

// Re-export the main parse function
pub use parse::parse;
