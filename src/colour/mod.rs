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

pub mod spectral;
pub mod legacy;
pub mod convert;
pub mod spectrum;

// Re-export constants from spectral and legacy modules
pub use spectral::*;
pub use legacy::*;
pub use spectrum::*;