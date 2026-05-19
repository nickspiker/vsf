//! Spectral data structures for representing wavelength-based colour information

use num_traits::Float;
use crate::prelude::*;

/// A spectrum represents spectral data at regular wavelength intervals
///
/// This structure encapsulates spectral power distributions, colour matching functions, or other wavelength-dependent data with a defined range and spacing.
#[derive(Debug, Clone)]
pub struct Spectrum {
    /// Starting wavelength in nanometers
    pub start_nm: f32,

    /// Ending wavelength in nanometers
    pub stop_nm: f32,

    /// Spacing between samples in nanometers
    pub spacing_nm: f32,

    /// Spectral data values
    /// For single-channel data (like SPD), this is one value per wavelength
    /// For multi-channel data (like LMS), this is N values per wavelength (interleaved)
    pub data: Vec<f32>,
}

impl Spectrum {
    /// Create a new spectrum with the given parameters
    pub fn new(start_nm: f32, stop_nm: f32, spacing_nm: f32, data: Vec<f32>) -> Self {
        Self {
            start_nm,
            stop_nm,
            spacing_nm,
            data,
        }
    }

    /// Get the number of wavelength samples in this spectrum
    pub fn num_samples(&self) -> usize {
        ((self.stop_nm - self.start_nm) / self.spacing_nm).round() as usize + 1
    }

    /// Get the wavelength for a given sample index
    pub fn wavelength_at(&self, index: usize) -> f32 {
        self.start_nm + (index as f32 * self.spacing_nm)
    }

    /// Check if a wavelength falls within this spectrum's range
    pub fn contains_wavelength(&self, wavelength_nm: f32) -> bool {
        wavelength_nm >= self.start_nm && wavelength_nm <= self.stop_nm
    }
}

/// A const-compatible spectrum for reference spectral data
///
/// This structure is designed for const declarations of reference spectral data (like CIE colour matching functions) that live in the binary's .rodata section. Uses a static slice instead of Vec for const compatibility.
#[derive(Debug, Clone, Copy)]
pub struct ConstSpectrum {
    /// Starting wavelength in nanometers
    pub start_nm: f32,

    /// Ending wavelength in nanometers
    pub stop_nm: f32,

    /// Spacing between samples in nanometers
    pub spacing_nm: f32,

    /// Number of channels (e.g., 1 for SPD, 3 for LMS)
    pub num_channels: usize,

    /// Spectral data values (static slice for const compatibility)
    /// For single-channel data (like SPD), this is one value per wavelength
    /// For multi-channel data (like LMS), this is N values per wavelength (interleaved)
    pub data: &'static [f32],
}

impl ConstSpectrum {
    /// Get the number of wavelength samples in this spectrum
    pub const fn num_samples(&self) -> usize {
        ((self.stop_nm - self.start_nm) / self.spacing_nm) as usize + 1
    }

    /// Get the wavelength for a given sample index
    pub const fn wavelength_at(&self, index: usize) -> f32 {
        self.start_nm + (index as f32 * self.spacing_nm)
    }

    /// Check if a wavelength falls within this spectrum's range
    pub const fn contains_wavelength(&self, wavelength_nm: f32) -> bool {
        wavelength_nm >= self.start_nm && wavelength_nm <= self.stop_nm
    }
}

/// Convert a ConstSpectrum reference to an owned Spectrum
///
/// Note: This converts multi-channel ConstSpectrum to single-channel Spectrum. Use the appropriate conversion method if you need to preserve channel information.
impl From<&ConstSpectrum> for Spectrum {
    fn from(const_spectrum: &ConstSpectrum) -> Self {
        Self {
            start_nm: const_spectrum.start_nm,
            stop_nm: const_spectrum.stop_nm,
            spacing_nm: const_spectrum.spacing_nm,
            data: const_spectrum.data.to_vec(),
        }
    }
}

/// A three-channel spectrum representing LMS cone fundamentals or similar data
#[derive(Debug, Clone)]
pub struct Spectrum3 {
    /// Starting wavelength in nanometers
    pub start_nm: f32,

    /// Ending wavelength in nanometers
    pub stop_nm: f32,

    /// Spacing between samples in nanometers
    pub spacing_nm: f32,

    /// Spectral data values as [L, M, S] triplets
    /// Format: [l0, m0, s0, l1, m1, s1, ..., ln, mn, sn]
    pub data: Vec<f32>,
}

impl Spectrum3 {
    /// Create a new 3-channel spectrum
    pub fn new(start_nm: f32, stop_nm: f32, spacing_nm: f32, data: Vec<f32>) -> Self {
        Self {
            start_nm,
            stop_nm,
            spacing_nm,
            data,
        }
    }

    /// Get the number of wavelength samples
    pub fn num_samples(&self) -> usize {
        self.data.len() / 3
    }

    /// Get the wavelength for a given sample index
    pub fn wavelength_at(&self, index: usize) -> f32 {
        self.start_nm + (index as f32 * self.spacing_nm)
    }

    /// Get the three values at a given sample index
    pub fn get_at(&self, index: usize) -> Option<[f32; 3]> {
        if index * 3 + 2 < self.data.len() {
            Some([
                self.data[index * 3],
                self.data[index * 3 + 1],
                self.data[index * 3 + 2],
            ])
        } else {
            None
        }
    }

    /// Check if a wavelength falls within this spectrum's range
    pub fn contains_wavelength(&self, wavelength_nm: f32) -> bool {
        wavelength_nm >= self.start_nm && wavelength_nm <= self.stop_nm
    }
}
