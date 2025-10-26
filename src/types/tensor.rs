/// Layout order for tensor data
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutOrder {
    RowMajor,    // C-style (default for Tensor)
    ColumnMajor, // Fortran-style
}

/// Contiguous tensor with row-major layout (no stride stored)
///
/// Binary format: `[t][dim_count][type][shape...][data...]`
/// - Always row-major, contiguous in memory
/// - No stride information stored (implicitly computed)
/// - 95% of use cases (normal arrays, images, ML tensors)
/// - Dynamic dimensionality (1D, 2D, 3D, 4D, or more)
///
/// # Examples
/// ```
/// use vsf::Tensor;
///
/// // 2D image: 1920×1080 u16 pixels
/// let img = Tensor::new(
///     vec![1920, 1080],
///     vec![0u16; 1920 * 1080]
/// );
///
/// // 3D tensor: 100×200×3 RGB
/// let rgb = Tensor::new(
///     vec![100, 200, 3],
///     vec![0u8; 100 * 200 * 3]
/// );
/// ```
#[derive(Debug, Clone)]
pub struct Tensor<T> {
    pub shape: Vec<usize>,
    pub data: Vec<T>,
}

/// Tensor with explicit stride for non-contiguous layouts
///
/// Binary format: `[q][dim_count][type][shape...][stride...][data...]`
/// - Supports arbitrary memory layouts (column-major, slices, views)
/// - Stores explicit stride information
/// - Use for: slices, transposed views, column-major matrices
///
/// # Examples
/// ```
/// use vsf::StridedTensor;
///
/// // Column-major 1000×1000 matrix
/// let mat = StridedTensor::new(
///     vec![1000, 1000],
///     vec![1, 1000],  // Column-major stride
///     vec![0.0f64; 1_000_000]
/// );
///
/// // 2D slice with custom stride
/// let slice = StridedTensor::new(
///     vec![100, 50],
///     vec![200, 2],  // Every other element
///     vec![0u8; 10_000]
/// );
/// ```
#[derive(Debug, Clone)]
pub struct StridedTensor<T> {
    pub shape: Vec<usize>,
    pub stride: Vec<usize>,
    pub data: Vec<T>,
}

impl<T> Tensor<T> {
    /// Create a new contiguous tensor with given shape and data
    pub fn new(shape: Vec<usize>, data: Vec<T>) -> Self {
        let expected_len: usize = shape.iter().product();
        assert_eq!(
            data.len(),
            expected_len,
            "Data length {} doesn't match shape {:?} (expected {})",
            data.len(),
            shape,
            expected_len
        );
        Tensor { shape, data }
    }

    /// Get number of dimensions
    pub fn ndim(&self) -> usize {
        self.shape.len()
    }

    /// Calculate total number of elements
    pub fn len(&self) -> usize {
        self.shape.iter().product()
    }

    /// Check if tensor is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T> StridedTensor<T> {
    /// Create a new strided tensor with given shape, stride, and data
    pub fn new(shape: Vec<usize>, stride: Vec<usize>, data: Vec<T>) -> Self {
        assert_eq!(
            shape.len(),
            stride.len(),
            "Shape and stride must have same number of dimensions"
        );
        StridedTensor {
            shape,
            stride,
            data,
        }
    }

    /// Get number of dimensions
    pub fn ndim(&self) -> usize {
        self.shape.len()
    }

    /// Calculate total number of elements
    pub fn len(&self) -> usize {
        self.shape.iter().product()
    }

    /// Check if tensor is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Check if tensor is contiguous (row-major)
    pub fn is_contiguous(&self) -> bool {
        let ndim = self.ndim();
        let mut expected_stride = 1;
        for i in (0..ndim).rev() {
            if self.stride[i] != expected_stride {
                return false;
            }
            expected_stride *= self.shape[i];
        }
        true
    }
}

/// Bitpacked tensor for arbitrary bit depths (1-256 bits per sample)
///
/// Binary format: `[p][dim_count][bit_depth][shape...][packed_data...]`
/// - bit_depth stored as u8: 0x01-0xFF (0x00 = 256-bit)
/// - Samples packed MSB-first, big-endian across byte boundaries
/// - Final byte zero-padded to align to 8-bit boundary
/// - Row-major storage (like Tensor<T>)
///
/// # Use Cases
/// - Camera RAW data (10-bit, 12-bit, 14-bit sensors)
/// - Compressed representations
/// - Scientific instruments with non-standard bit depths
///
/// # Examples
/// ```
/// use vsf::BitPackedTensor;
///
/// // Lumis 12-bit RAW: 1920×1080 camera sensor
/// let samples: Vec<u64> = vec![2048; 1920 * 1080];  // 12-bit values (0-4095)
/// let tensor = BitPackedTensor::pack(12, vec![1920, 1080], &samples);
/// assert_eq!(tensor.bit_depth, 12);
///
/// // Unpack back to samples
/// let unpacked = tensor.unpack();
/// assert_eq!(unpacked[0], 2048);
/// ```
#[derive(Debug, Clone)]
pub struct BitPackedTensor {
    /// Bits per sample (1-256): 0x01-0xFF, where 0x00 represents 256-bit
    pub bit_depth: u8,
    /// Tensor dimensions (row-major)
    pub shape: Vec<usize>,
    /// Packed bytes: (total_elements * bit_depth + 7) / 8 bytes
    pub data: Vec<u8>,
}

impl BitPackedTensor {
    /// Pack samples into bitpacked tensor
    ///
    /// # Arguments
    /// * `bit_depth` - Bits per sample (1-256, where 0 = 256)
    /// * `shape` - Tensor dimensions
    /// * `samples` - Sample values (must fit in bit_depth bits)
    ///
    /// # Panics
    /// Panics if any sample exceeds max value for bit_depth
    pub fn pack(bit_depth: u8, shape: Vec<usize>, samples: &[u64]) -> Self {
        let total_elements: usize = shape.iter().product();
        assert_eq!(
            samples.len(),
            total_elements,
            "Sample count {} doesn't match shape {:?} (expected {})",
            samples.len(),
            shape,
            total_elements
        );

        let bits_per_sample = if bit_depth == 0 { 256 } else { bit_depth as usize };
        let max_value = if bits_per_sample == 256 {
            u64::MAX
        } else {
            (1u64 << bits_per_sample) - 1
        };

        // Validate all samples fit in bit_depth
        for (i, &sample) in samples.iter().enumerate() {
            assert!(
                sample <= max_value,
                "Sample {} at index {} exceeds max value {} for {}-bit depth",
                sample,
                i,
                max_value,
                bits_per_sample
            );
        }

        // Calculate total bits and bytes needed
        let total_bits = total_elements * bits_per_sample;
        let byte_count = (total_bits + 7) / 8;
        let mut data = vec![0u8; byte_count];

        // Pack samples MSB-first, big-endian
        let mut bit_offset = 0;
        for &sample in samples {
            for bit_idx in (0..bits_per_sample).rev() {
                let bit = ((sample >> bit_idx) & 1) as u8;
                let byte_idx = bit_offset / 8;
                let bit_pos = 7 - (bit_offset % 8);
                data[byte_idx] |= bit << bit_pos;
                bit_offset += 1;
            }
        }

        BitPackedTensor {
            bit_depth,
            shape,
            data,
        }
    }

    /// Unpack bitpacked tensor back to samples
    ///
    /// # Returns
    /// Vector of sample values (u64)
    pub fn unpack(&self) -> Vec<u64> {
        let total_elements: usize = self.shape.iter().product();
        let bits_per_sample = if self.bit_depth == 0 { 256 } else { self.bit_depth as usize };
        let mut samples = Vec::with_capacity(total_elements);

        let mut bit_offset = 0;
        for _ in 0..total_elements {
            let mut sample = 0u64;
            for _ in 0..bits_per_sample {
                let byte_idx = bit_offset / 8;
                let bit_pos = 7 - (bit_offset % 8);
                let bit = (self.data[byte_idx] >> bit_pos) & 1;
                sample = (sample << 1) | (bit as u64);
                bit_offset += 1;
            }
            samples.push(sample);
        }

        samples
    }

    /// Get number of dimensions
    pub fn ndim(&self) -> usize {
        self.shape.len()
    }

    /// Calculate total number of elements
    pub fn len(&self) -> usize {
        self.shape.iter().product()
    }

    /// Check if tensor is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
