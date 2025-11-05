//! # Legacy Colourspaces
//!
//! Colourspaces in this module are defined by xy chromaticity coordinates using the CIE 1931 Standard Observer. These specifications are permanently bound to 1931 observer data and cannot be updated without changing the colourspace itself.
//!
//! ## sRGB
//!
//! The standard RGB colourspace for web content, consumer displays, and digital cameras.
//! - **Primaries**: Defined in 1931 xy coordinates (same as Rec.709)
//! - **White Point**: D65 (simulated north sky daylight)
//! - **Gamma**: Piecewise function (linear segment + 2.4 power)
//!
//! ## XYZ
//!
//! CIE XYZ tristimulus values represent colours in terms of the 1931 Standard Observer response. This is the foundation space for most xy-coordinate-based colour standards.
//!
//! ## Conversions
//!
//! Conversions between legacy colourspaces use published transformation matrices and go thru XYZ tristimulus space when necessary. Conversions to/from spectrally-defined spaces go thru LMS cone space using the CIE 2006 2° Standard Observer to maintain perceptual equivalence.

pub mod constants;
pub use constants::*;