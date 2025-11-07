//! Transfer functions for modern spectral colourspaces
//!
//! **Legacy transfer functions are in `src/colour/legacy/transfer.rs`**
//!
//! This module provides transfer functions for spectral colourspaces:
//! - **VSF RGB**: Simple gamma 2 (see `src/colour/convert.rs`)
//! - **Rec.2020**: Uses Rec.709 transfer function (aliases provided below)
//!
//! For legacy xy-based colourspaces (sRGB, Rec.709, Adobe RGB), use:
//! ```rust
//! use vsf::colour::legacy::transfer::*;
//! ```

// ==================== Rec.2020 TRANSFER FUNCTIONS ====================
//
// Rec.2020 is a spectral colourspace (primaries: 630nm, 532nm, 467nm)
// but it uses the Rec.709 transfer function for backwards compatibility.

/// Rec.2020 EOTF: Convert gamma-corrected Rec.2020 to linear RGB
///
/// Note: Rec.2020 uses the Rec.709 transfer function.
/// Rec.2020 is a spectral colourspace, not a legacy xy-based colourspace.
pub use crate::colour::legacy::transfer::rec709_eotf as rec2020_eotf;

/// Apply Rec.2020 EOTF to RGB triplet
pub use crate::colour::legacy::transfer::rec709_eotf_rgb as rec2020_eotf_rgb;

/// Rec.2020 OETF: Convert linear RGB to gamma-corrected Rec.2020
///
/// Note: Rec.2020 uses the Rec.709 transfer function.
/// Rec.2020 is a spectral colourspace, not a legacy xy-based colourspace.
pub use crate::colour::legacy::transfer::rec709_oetf as rec2020_oetf;

/// Apply Rec.2020 OETF to RGB triplet
pub use crate::colour::legacy::transfer::rec709_oetf_rgb as rec2020_oetf_rgb;
