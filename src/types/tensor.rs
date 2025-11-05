//! Tensor types for VSF: contiguous, strided, and bitpacked

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

/// Bitpacked tensor for arbitrary bit depths (1-128 bits per sample)
///
/// Binary format: `[p][dim_count][bit_depth][shape...][packed_data...]`
/// - bit_depth stored as u8: 0x01-0x80 (1-128 bits)
/// - Samples packed MSB-first, big-endian across byte boundaries
/// - Only low `bit_depth` bits of input values are packed; high bits ignored
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
/// // Option 1: Generic "just works" (most common)
/// let samples: Vec<u16> = vec![2048; 1920 * 1080];  // 12-bit values (0-4095)
/// let tensor = BitPackedTensor::pack(12, vec![1920, 1080], &samples);
/// let unpacked = tensor.unpack().into_u64();  // Auto-sized, then promoted
///
/// // Option 2: Explicit type control (when you need guarantees)
/// let tensor = BitPackedTensor::pack_u16(12, vec![1920, 1080], &samples);
/// let unpacked: Vec<u16> = tensor.unpack_u16();  // Explicit, no enum
/// ```
#[derive(Debug, Clone)]
pub struct BitPackedTensor {
    /// Bits per sample (1-128): 0x01-0x80
    pub bit_depth: u8,
    /// Tensor dimensions (row-major)
    pub shape: Vec<usize>,
    /// Packed bytes: (total_elements * bit_depth + 7) / 8 bytes
    pub data: Vec<u8>,
}

/// Trait for types that can be packed into a BitPackedTensor
///
/// This exists solely to enable the generic pack() convenience method.
/// For explicit type handling, use pack_u8(), pack_u16(), etc. directly.
pub trait PackableUnsigned: Copy {
    fn pack_samples(bit_depth: u8, shape: Vec<usize>, samples: &[Self]) -> BitPackedTensor;
}

// Macro to generate explicit pack_u* methods on BitPackedTensor
macro_rules! impl_bitpack {
    ($fn_name:ident, $t:ty, $work_t:ty) => {
        impl BitPackedTensor {
            #[doc = concat!("Pack ", stringify!($t), " samples into bitpacked tensor\n\n")]
            #[doc = "# Arguments\n"]
            #[doc = "* `bit_depth` - Bits per sample (1-128 supported, 0 reserved for future 256-bit)\n"]
            #[doc = "* `shape` - Tensor dimensions\n"]
            #[doc = "* `samples` - Sample values (only low `bit_depth` bits are packed, high bits ignored)\n\n"]
            #[doc = "# Panics\n"]
            #[doc = "* If bit_depth exceeds the type's bit width\n"]
            #[doc = "* If bit_depth > 128 (256-bit not yet supported)\n"]
            #[doc = "* If samples.len() doesn't match shape product\n"]
            pub fn $fn_name(bit_depth: u8, shape: Vec<usize>, samples: &[$t]) -> Self {
                let total_elements: usize = shape.iter().product();
                assert_eq!(
                    samples.len(),
                    total_elements,
                    "Sample count {} doesn't match shape {:?} (expected {})",
                    samples.len(),
                    shape,
                    total_elements
                );

                let bits_per_sample = if bit_depth == 0 {
                    panic!("bit_depth=0 (256-bit) not yet supported - use 1-128");
                } else {
                    bit_depth as usize
                };

                // Reject >128 bit depths until hardware support
                if bits_per_sample > 128 {
                    panic!("bit_depth > 128 not yet supported (waiting for native u256 support)");
                }

                // Type-level check: can this type hold bit_depth bits?
                if bits_per_sample > <$t>::BITS as usize {
                    panic!(
                        "Cannot pack {}-bit values into {}-bit type {}",
                        bits_per_sample,
                        <$t>::BITS,
                        std::any::type_name::<$t>()
                    );
                }

                // Calculate total bits and bytes needed
                let total_bits = total_elements * bits_per_sample;
                let byte_count = (total_bits + 7) / 8;
                let mut data = vec![0u8; byte_count];

                // Pack samples MSB-first, big-endian
                // Only low bit_depth bits are read; high bits are ignored
                let mut bit_offset = 0;
                for &sample in samples {
                    let value = sample as $work_t;
                    for bit_idx in (0..bits_per_sample).rev() {
                        let bit = if (value >> bit_idx) & 1 == 1 { 1u8 } else { 0u8 };
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
        }
    };
}

// Generate pack_* methods for each unsigned type
// Use u64 as work type for u8/u16/u32/u64 (native 64-bit ops)
// Use u128 for u128 (unavoidably emulated until hardware u256 support)
impl_bitpack!(pack_u8, u8, u64);
impl_bitpack!(pack_u16, u16, u64);
impl_bitpack!(pack_u32, u32, u64);
impl_bitpack!(pack_u64, u64, u64);
impl_bitpack!(pack_u128, u128, u128);
impl_bitpack!(pack_usize, usize, u64);

// Trait implementations that delegate to explicit pack_u* methods
impl PackableUnsigned for u8 {
    fn pack_samples(bit_depth: u8, shape: Vec<usize>, samples: &[Self]) -> BitPackedTensor {
        BitPackedTensor::pack_u8(bit_depth, shape, samples)
    }
}

impl PackableUnsigned for u16 {
    fn pack_samples(bit_depth: u8, shape: Vec<usize>, samples: &[Self]) -> BitPackedTensor {
        BitPackedTensor::pack_u16(bit_depth, shape, samples)
    }
}

impl PackableUnsigned for u32 {
    fn pack_samples(bit_depth: u8, shape: Vec<usize>, samples: &[Self]) -> BitPackedTensor {
        BitPackedTensor::pack_u32(bit_depth, shape, samples)
    }
}

impl PackableUnsigned for u64 {
    fn pack_samples(bit_depth: u8, shape: Vec<usize>, samples: &[Self]) -> BitPackedTensor {
        BitPackedTensor::pack_u64(bit_depth, shape, samples)
    }
}

impl PackableUnsigned for u128 {
    fn pack_samples(bit_depth: u8, shape: Vec<usize>, samples: &[Self]) -> BitPackedTensor {
        BitPackedTensor::pack_u128(bit_depth, shape, samples)
    }
}

impl PackableUnsigned for usize {
    fn pack_samples(bit_depth: u8, shape: Vec<usize>, samples: &[Self]) -> BitPackedTensor {
        BitPackedTensor::pack_usize(bit_depth, shape, samples)
    }
}

/// Unpacked samples from a BitPackedTensor
///
/// The enum variant matches the minimal type needed for the bit depth.
/// For explicit type control, use unpack_u8(), unpack_u16(), etc. directly.
#[derive(Debug, Clone, PartialEq)]
pub enum UnpackedSamples {
    U8(Vec<u8>),     // 1-8 bit depths
    U16(Vec<u16>),   // 9-16 bit depths
    U32(Vec<u32>),   // 17-32 bit depths
    U64(Vec<u64>),   // 33-64 bit depths
    U128(Vec<u128>), // 65-128 bit depths
}

impl UnpackedSamples {
    /// Convert to u64, promoting smaller types (panics for >64 bit)
    pub fn into_u64(self) -> Vec<u64> {
        match self {
            UnpackedSamples::U8(v) => v.into_iter().map(|x| x as u64).collect(),
            UnpackedSamples::U16(v) => v.into_iter().map(|x| x as u64).collect(),
            UnpackedSamples::U32(v) => v.into_iter().map(|x| x as u64).collect(),
            UnpackedSamples::U64(v) => v,
            UnpackedSamples::U128(_) => {
                panic!("Cannot convert >64 bit samples to u64 (would truncate)")
            }
        }
    }

    /// Convert to u128, promoting all types
    pub fn into_u128(self) -> Vec<u128> {
        match self {
            UnpackedSamples::U8(v) => v.into_iter().map(|x| x as u128).collect(),
            UnpackedSamples::U16(v) => v.into_iter().map(|x| x as u128).collect(),
            UnpackedSamples::U32(v) => v.into_iter().map(|x| x as u128).collect(),
            UnpackedSamples::U64(v) => v.into_iter().map(|x| x as u128).collect(),
            UnpackedSamples::U128(v) => v,
        }
    }

    /// Get the number of samples
    pub fn len(&self) -> usize {
        match self {
            UnpackedSamples::U8(v) => v.len(),
            UnpackedSamples::U16(v) => v.len(),
            UnpackedSamples::U32(v) => v.len(),
            UnpackedSamples::U64(v) => v.len(),
            UnpackedSamples::U128(v) => v.len(),
        }
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl BitPackedTensor {
    /// Pack samples with automatic type dispatch (convenience wrapper)
    ///
    /// Calls the appropriate pack_u* method based on the input type.
    /// Use explicit pack_u8(), pack_u16() etc. methods if you need compile-time guarantees.
    ///
    /// # Examples
    /// ```ignore
    /// // Generic - works with any unsigned type
    /// let samples: Vec<u16> = vec![2048; 1920 * 1080];
    /// let tensor = BitPackedTensor::pack(12, vec![1920, 1080], &samples);
    /// ```
    pub fn pack<T: PackableUnsigned>(bit_depth: u8, shape: Vec<usize>, samples: &[T]) -> Self {
        T::pack_samples(bit_depth, shape, samples)
    }

    /// Unpack to the minimal type that fits bit_depth
    ///
    /// Returns:
    /// - Vec<u8> for 1-8 bit depths
    /// - Vec<u16> for 9-16 bit depths
    /// - Vec<u32> for 17-32 bit depths
    /// - Vec<u64> for 33-64 bit depths
    /// - Vec<u128> for 65-128 bit depths
    ///
    /// Use explicit unpack_u8(), unpack_u16() etc. if you need a specific type.
    ///
    /// # Examples
    /// ```ignore
    /// let tensor = BitPackedTensor::pack(12, vec![100, 100], &samples);
    /// // Auto-sized, then promoted to u64
    /// let unpacked = tensor.unpack().into_u64();
    /// ```
    pub fn unpack(&self) -> UnpackedSamples {
        let bits = self.bit_depth as usize;
        match bits {
            1..=8 => UnpackedSamples::U8(self.unpack_to_u8()),
            9..=16 => UnpackedSamples::U16(self.unpack_to_u16()),
            17..=32 => UnpackedSamples::U32(self.unpack_to_u32()),
            33..=64 => UnpackedSamples::U64(self.unpack_to_u64()),
            65..=128 => UnpackedSamples::U128(self.unpack_to_u128()),
            _ => panic!("bit_depth {} not supported (max 128)", self.bit_depth),
        }
    }

    /// Unpack to u8 samples
    ///
    /// # Panics
    /// Panics if bit_depth > 8 (data wouldn't fit)
    pub fn unpack_u8(&self) -> Vec<u8> {
        if self.bit_depth > 8 {
            panic!(
                "Cannot unpack {}-bit data into u8 (would truncate)",
                self.bit_depth
            );
        }
        self.unpack_to_u8()
    }

    /// Unpack to u16 samples
    ///
    /// # Panics
    /// Panics if bit_depth > 16 (data wouldn't fit)
    pub fn unpack_u16(&self) -> Vec<u16> {
        if self.bit_depth > 16 {
            panic!(
                "Cannot unpack {}-bit data into u16 (would truncate)",
                self.bit_depth
            );
        }
        self.unpack_to_u16()
    }

    /// Unpack to u32 samples
    ///
    /// # Panics
    /// Panics if bit_depth > 32 (data wouldn't fit)
    pub fn unpack_u32(&self) -> Vec<u32> {
        if self.bit_depth > 32 {
            panic!(
                "Cannot unpack {}-bit data into u32 (would truncate)",
                self.bit_depth
            );
        }
        self.unpack_to_u32()
    }

    /// Unpack to u64 samples
    ///
    /// # Panics
    /// Panics if bit_depth > 64 (data wouldn't fit)
    pub fn unpack_u64(&self) -> Vec<u64> {
        if self.bit_depth > 64 {
            panic!(
                "Cannot unpack {}-bit data into u64 (would truncate)",
                self.bit_depth
            );
        }
        self.unpack_to_u64()
    }

    /// Unpack to u128 samples (works for all current bit depths 1-128)
    pub fn unpack_u128(&self) -> Vec<u128> {
        self.unpack_to_u128()
    }

    // Private unpack helpers for each type
    fn unpack_to_u8(&self) -> Vec<u8> {
        let total_elements: usize = self.shape.iter().product();
        let bits_per_sample = self.bit_depth as usize;
        let mut samples = Vec::with_capacity(total_elements);

        let mut bit_offset = 0;
        for _ in 0..total_elements {
            let mut sample = 0u8;
            for _ in 0..bits_per_sample {
                let byte_idx = bit_offset / 8;
                let bit_pos = 7 - (bit_offset % 8);
                let bit = (self.data[byte_idx] >> bit_pos) & 1;
                sample = (sample << 1) | bit;
                bit_offset += 1;
            }
            samples.push(sample);
        }
        samples
    }

    fn unpack_to_u16(&self) -> Vec<u16> {
        let total_elements: usize = self.shape.iter().product();
        let bits_per_sample = self.bit_depth as usize;
        let mut samples = Vec::with_capacity(total_elements);

        let mut bit_offset = 0;
        for _ in 0..total_elements {
            let mut sample = 0u16;
            for _ in 0..bits_per_sample {
                let byte_idx = bit_offset / 8;
                let bit_pos = 7 - (bit_offset % 8);
                let bit = (self.data[byte_idx] >> bit_pos) & 1;
                sample = (sample << 1) | (bit as u16);
                bit_offset += 1;
            }
            samples.push(sample);
        }
        samples
    }

    fn unpack_to_u32(&self) -> Vec<u32> {
        let total_elements: usize = self.shape.iter().product();
        let bits_per_sample = self.bit_depth as usize;
        let mut samples = Vec::with_capacity(total_elements);

        let mut bit_offset = 0;
        for _ in 0..total_elements {
            let mut sample = 0u32;
            for _ in 0..bits_per_sample {
                let byte_idx = bit_offset / 8;
                let bit_pos = 7 - (bit_offset % 8);
                let bit = (self.data[byte_idx] >> bit_pos) & 1;
                sample = (sample << 1) | (bit as u32);
                bit_offset += 1;
            }
            samples.push(sample);
        }
        samples
    }

    fn unpack_to_u64(&self) -> Vec<u64> {
        let total_elements: usize = self.shape.iter().product();
        let bits_per_sample = self.bit_depth as usize;
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

    fn unpack_to_u128(&self) -> Vec<u128> {
        let total_elements: usize = self.shape.iter().product();
        let bits_per_sample = self.bit_depth as usize;
        let mut samples = Vec::with_capacity(total_elements);

        let mut bit_offset = 0;
        for _ in 0..total_elements {
            let mut sample = 0u128;
            for _ in 0..bits_per_sample {
                let byte_idx = bit_offset / 8;
                let bit_pos = 7 - (bit_offset % 8);
                let bit = (self.data[byte_idx] >> bit_pos) & 1;
                sample = (sample << 1) | (bit as u128);
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
