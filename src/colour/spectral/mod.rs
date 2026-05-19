//! # Spectrally-Defined Colourspaces
//!
//! Colourspaces in this module are defined by wavelength primaries rather than xy chromaticity coordinates.
//!
//! ## VSF RGB
//!
//! VSF RGB uses three monochromatic primaries at 703nm, 523nm, and 462nm, derived from the AGB geometric mean model of human cone perception. These wavelengths represent the points where each geometric mean ratio achieves maximum value.
//!
//! - **Red (R)**: 703nm - Peak of Alpha ratio (L/(L+M))
//! - **Green (G)**: 523nm - Peak of Gamma ratio (M/(L+S))
//! - **Blue (B)**: 462nm - Peak of Beta ratio (S/(S+M))
//!
//! **White Point**: Illuminant E (equal energy spectrum) **Gamma**: 2.0 (pure square/square root operations)
//!
//! ## Rec.2020
//!
//! Rec.2020 specifies wavelength primaries at 630nm, 532nm, and 467nm. We use these wavelength specifications and ignore the published xy coordinates, which don't match the wavelengths using any standard observer.
//!
//! ## LMS2006
//!
//! CIE 2006 2° cone fundamentals provide the transformation between wavelengths and human perception. These represent the spectral sensitivity of L (long), M (medium), and S (short) wavelength cone types in the human retina.

pub mod constants;
pub use constants::*;
