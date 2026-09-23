//! # Legacy Colourspaces (DEPRECATED)
//!
//! **DEPRECATED**: Colourspaces in this module are defined by xy chromaticity coordinates using the CIE 1931 Standard Observer. These specifications are permanently bound to 1931 observer data and cannot be updated without changing the colourspace itself.
//!
//! **Use spectral colourspaces instead**: VSF RGB and Rec.2020 define primaries by wavelengths, making them observer-independent and physically reproducible.
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
//! Each legacy space has one matrix into VSF RGB, built from its xy primaries under the 1931 observer (the only observer an xy definition supports) with VSF's primaries placed under the same observer. Conversions between legacy spaces compose thru VSF RGB, never thru a shared XYZ hub. Spectrally defined spaces use the Stockman & Sharpe 2000 10° cone fundamentals instead; see `colour.md`.

pub mod constants;

#[deprecated(
    since = "0.2.0",
    note = "CIE 1931 xy-based colourspaces are deprecated. Use VSF RGB or Rec.2020 instead."
)]
pub mod transfer;

#[deprecated(
    since = "0.2.0",
    note = "CIE 1931 xy-based colourspaces are deprecated. Use VSF RGB or Rec.2020 instead."
)]
pub mod convert;

// Re-export all legacy functions for backward compatibility
pub use constants::*;
pub use convert::*;
pub use transfer::*;
