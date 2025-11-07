//! # VSF Colourspace Library
//!
//! This library provides conversion between VSF RGB (a spectrally-defined colourspace) and legacy colour standards (typicallly 1931 xy based)
//!
//! ## Spectral Colourspaces
//!
//! VSF RGB and other spectrally-defined colourspaces specify primaries by wavelengths rather than xy chromaticity coordinates. This provides:
//! - Physical reproducibility (703nm means 703nm in any laboratory)
//! - Observer model independence (primaries don't change when observer models improve)
//! - No accumulated transformation errors
//!
//! ## Legacy Colourspaces
//!
//! Traditional colourspaces (sRGB, Adobe RGB, etc.) specify primaries using CIE xy chromaticity coordinates derived from the 1931 Standard Observer. These spaces are permanently bound to historical observer data and cannot be updated without changing the specification itself.

pub mod convert;
pub mod legacy;
pub mod rec2020;
pub mod spectral;
pub mod spectrum;
pub mod transfer;

// Re-export constants from spectral, legacy, and rec2020 modules
pub use legacy::*;
pub use rec2020::*;
pub use spectral::*;
pub use spectrum::*;
