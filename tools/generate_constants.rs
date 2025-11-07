// Path up and into the module
use vsf_codegen::spectral_data::{LMS_2000_10DEG_SO, XYZ_1931_2DEG_SO};
use std::fs;
use std::path::Path;

/// Invert a 3x3 matrix stored in column-major format
///
/// Matrix format: [col0_r, col0_g, col0_b, col1_r, col1_g, col1_b, col2_r, col2_g, col2_b]
fn invert_matrix_3x3(m: &[f64; 9]) -> [f64; 9] {
    let d = 1.0
        / (m[0] * (m[4] * m[8] - m[5] * m[7]) - m[1] * (m[3] * m[8] - m[5] * m[6])
            + m[2] * (m[3] * m[7] - m[4] * m[6]));
    [
        (m[4] * m[8] - m[5] * m[7]) * d,
        (m[2] * m[7] - m[1] * m[8]) * d,
        (m[1] * m[5] - m[2] * m[4]) * d,
        (m[5] * m[6] - m[3] * m[8]) * d,
        (m[0] * m[8] - m[2] * m[6]) * d,
        (m[2] * m[3] - m[0] * m[5]) * d,
        (m[3] * m[7] - m[4] * m[6]) * d,
        (m[1] * m[6] - m[0] * m[7]) * d,
        (m[0] * m[4] - m[1] * m[3]) * d,
    ]
}

/// Apply a 3x3 transformation matrix to a colour
///
/// Matrix is in column-major format: [col0_r, col0_g, col0_b, col1_r, col1_g, col1_b, col2_r, col2_g, col2_b]
///
/// This means:
/// - out[0] = colour[0] * cmx[0] + colour[1] * cmx[1] + colour[2] * cmx[2]
/// - out[1] = colour[0] * cmx[3] + colour[1] * cmx[4] + colour[2] * cmx[5]
/// - out[2] = colour[0] * cmx[6] + colour[1] * cmx[7] + colour[2] * cmx[8]
/// Apply 3×3 transformation matrix to colour vector (column-major)
///
/// Computes: result = matrix * colour
fn apply_matrix_3x3(cmx: &[f64], colour: &[f64; 3]) -> [f64; 3] {
    [
        cmx[0] * colour[0] + cmx[3] * colour[1] + cmx[6] * colour[2],  // Row 0
        cmx[1] * colour[0] + cmx[4] * colour[1] + cmx[7] * colour[2],  // Row 1
        cmx[2] * colour[0] + cmx[5] * colour[1] + cmx[8] * colour[2],  // Row 2
    ]
}

/// Multiply two 3x3 matrices (matrix multiplication: result = a * b)
///
/// Multiply two 3×3 matrices: C = A * B (column-major)
///
/// Column j of C = A * column j of B
fn convert_matrix_3x3(a: &[f64], b: &[f64]) -> [f64; 9] {
    [
        // Column 0 of result = A * column 0 of B
        a[0]*b[0] + a[3]*b[1] + a[6]*b[2],  // C[0]
        a[1]*b[0] + a[4]*b[1] + a[7]*b[2],  // C[1]
        a[2]*b[0] + a[5]*b[1] + a[8]*b[2],  // C[2]
        // Column 1 of result = A * column 1 of B
        a[0]*b[3] + a[3]*b[4] + a[6]*b[5],  // C[3]
        a[1]*b[3] + a[4]*b[4] + a[7]*b[5],  // C[4]
        a[2]*b[3] + a[5]*b[4] + a[8]*b[5],  // C[5]
        // Column 2 of result = A * column 2 of B
        a[0]*b[6] + a[3]*b[7] + a[6]*b[8],  // C[6]
        a[1]*b[6] + a[4]*b[7] + a[7]*b[8],  // C[7]
        a[2]*b[6] + a[5]*b[7] + a[8]*b[8],  // C[8]
    ]
}

/// VSF RGB monochromatic primaries (wavelengths in nm)
const VSF_RED_NM: f64 = 703.0;
const VSF_GREEN_NM: f64 = 523.0;
const VSF_BLUE_NM: f64 = 462.0;

/// Rec.2020 primaries (wavelengths in nm)
/// ITU-R BT.2020 specifies these monochromatic primaries
const REC2020_RED_NM: f64 = 630.0;
const REC2020_GREEN_NM: f64 = 532.0;
const REC2020_BLUE_NM: f64 = 467.0;

/// sRGB primaries (CIE xy chromaticity coordinates with D65 white point)
/// These are the standard sRGB primaries from IEC 61966-2-1:1999
const SRGB_RED_XY: [f64; 2] = [0.6400, 0.3300];
const SRGB_GREEN_XY: [f64; 2] = [0.3000, 0.6000];
const SRGB_BLUE_XY: [f64; 2] = [0.1500, 0.0600];

/// Adobe RGB (1998) primaries (CIE xy chromaticity coordinates with D65 white point)
const ADOBE_RGB_RED_XY: [f64; 2] = [0.6400, 0.3300];
const ADOBE_RGB_GREEN_XY: [f64; 2] = [0.2100, 0.7100];
const ADOBE_RGB_BLUE_XY: [f64; 2] = [0.1500, 0.0600];

/// Stockman & Sharpe 2000 10° data parameters
const SS2000_START_NM: f64 = 390.0;
const SS2000_STEP_NM: f64 = 5.0;
const SS2000_SAMPLES: usize = 89; // 390nm to 830nm

/// CIE 1931 2° XYZ data parameters
const XYZ1931_START_NM: f64 = 380.0;
const XYZ1931_STEP_NM: f64 = 5.0;
const XYZ1931_SAMPLES: usize = 81; // 380nm to 780nm

/// Configuration for building matrices from observer data
struct ObserverConfig {
    name: &'static str,
    source_start_nm: f64,
    source_step_nm: f64,
    source_samples: usize,
    output_start_nm: usize,
    output_end_nm: usize,
    use_log_interpolation: bool,
    normalize_channels: bool,
}

/// Parse Stockman & Sharpe 2000 10° cone fundamentals from log10 format
/// Just converts from log10 to linear - normalization happens after interpolation
fn parse_lms_data() -> Vec<[f64; 3]> {
    let mut lms_values = Vec::with_capacity(SS2000_SAMPLES);

    // Convert from log10 to linear (raw L, M, S cone responses)
    for i in 0..SS2000_SAMPLES {
        let l_log = LMS_2000_10DEG_SO[i * 3];
        let m_log = LMS_2000_10DEG_SO[i * 3 + 1];
        let s_log = LMS_2000_10DEG_SO[i * 3 + 2];

        let l = if l_log.is_nan() { 0.0 } else { 10_f64.powf(l_log) };
        let m = if m_log.is_nan() { 0.0 } else { 10_f64.powf(m_log) };
        let s = if s_log.is_nan() { 0.0 } else { 10_f64.powf(s_log) };

        lms_values.push([l, m, s]);
    }

    println!("// Converted {} samples from log10 to linear", SS2000_SAMPLES);

    lms_values
}

/// Linear interpolation between two wavelength samples in log space
/// For data outside the range, extrapolate linearly in log space
fn interpolate_lms_log(lms_data_log: &[[f64; 3]], wavelength_nm: f64) -> [f64; 3] {
    let index_f = (wavelength_nm - SS2000_START_NM) / SS2000_STEP_NM;

    // Handle extrapolation beyond range
    if index_f < 0.0 {
        // Extrapolate before start
        let slope_l = lms_data_log[1][0] - lms_data_log[0][0];
        let slope_m = lms_data_log[1][1] - lms_data_log[0][1];
        let slope_s = lms_data_log[1][2] - lms_data_log[0][2];

        let l_log = lms_data_log[0][0] + slope_l * index_f;
        let m_log = lms_data_log[0][1] + slope_m * index_f;
        let s_log = lms_data_log[0][2] + slope_s * index_f;

        return [
            if l_log.is_nan() { 0.0 } else { 10_f64.powf(l_log) },
            if m_log.is_nan() { 0.0 } else { 10_f64.powf(m_log) },
            if s_log.is_nan() { 0.0 } else { 10_f64.powf(s_log) },
        ];
    }

    if index_f >= (SS2000_SAMPLES - 1) as f64 {
        // Extrapolate after end - find last valid measurements for each channel
        let last_idx = SS2000_SAMPLES - 1;
        let steps_beyond = index_f - (last_idx as f64);

        // L-cone
        let l_log = if lms_data_log[last_idx][0].is_nan() {
            // Find last valid L measurement
            let mut last_valid_idx = None;
            for i in (0..=last_idx).rev() {
                if !lms_data_log[i][0].is_nan() {
                    last_valid_idx = Some(i);
                    break;
                }
            }
            if let Some(idx) = last_valid_idx {
                if idx >= 1 {
                    let last_val = lms_data_log[idx][0];
                    let prev_val = lms_data_log[idx - 1][0];
                    let slope = last_val - prev_val;
                    last_val + slope * ((wavelength_nm - (SS2000_START_NM + idx as f64 * SS2000_STEP_NM)) / SS2000_STEP_NM)
                } else {
                    f64::NAN
                }
            } else {
                f64::NAN
            }
        } else {
            let slope_l = lms_data_log[last_idx][0] - lms_data_log[last_idx - 1][0];
            lms_data_log[last_idx][0] + slope_l * steps_beyond
        };

        // M-cone
        let m_log = if lms_data_log[last_idx][1].is_nan() {
            let mut last_valid_idx = None;
            for i in (0..=last_idx).rev() {
                if !lms_data_log[i][1].is_nan() {
                    last_valid_idx = Some(i);
                    break;
                }
            }
            if let Some(idx) = last_valid_idx {
                if idx >= 1 {
                    let last_val = lms_data_log[idx][1];
                    let prev_val = lms_data_log[idx - 1][1];
                    let slope = last_val - prev_val;
                    last_val + slope * ((wavelength_nm - (SS2000_START_NM + idx as f64 * SS2000_STEP_NM)) / SS2000_STEP_NM)
                } else {
                    f64::NAN
                }
            } else {
                f64::NAN
            }
        } else {
            let slope_m = lms_data_log[last_idx][1] - lms_data_log[last_idx - 1][1];
            lms_data_log[last_idx][1] + slope_m * steps_beyond
        };

        // S-cone
        let s_log = if lms_data_log[last_idx][2].is_nan() {
            let mut last_valid_idx = None;
            for i in (0..=last_idx).rev() {
                if !lms_data_log[i][2].is_nan() {
                    last_valid_idx = Some(i);
                    break;
                }
            }
            if let Some(idx) = last_valid_idx {
                if idx >= 1 {
                    let last_val = lms_data_log[idx][2];
                    let prev_val = lms_data_log[idx - 1][2];
                    let slope = last_val - prev_val;
                    last_val + slope * ((wavelength_nm - (SS2000_START_NM + idx as f64 * SS2000_STEP_NM)) / SS2000_STEP_NM)
                } else {
                    f64::NAN
                }
            } else {
                f64::NAN
            }
        } else {
            let slope_s = lms_data_log[last_idx][2] - lms_data_log[last_idx - 1][2];
            lms_data_log[last_idx][2] + slope_s * steps_beyond
        };

        return [
            if l_log.is_nan() { 0.0 } else { 10_f64.powf(l_log) },
            if m_log.is_nan() { 0.0 } else { 10_f64.powf(m_log) },
            if s_log.is_nan() { 0.0 } else { 10_f64.powf(s_log) },
        ];
    }

    // Normal interpolation
    let index_low = index_f.floor() as usize;
    let index_high = index_low + 1;
    let t = index_f - index_low as f64;

    let low = lms_data_log[index_low];
    let high = lms_data_log[index_high];

    [
        // L-cone
        if low[0].is_nan() && high[0].is_nan() {
            0.0
        } else if high[0].is_nan() && index_low >= 1 {
            // Extrapolate: use slope from previous two points
            let prev = lms_data_log[index_low - 1][0];
            let slope = low[0] - prev;
            10_f64.powf(low[0] + slope * t)
        } else if low[0].is_nan() || high[0].is_nan() {
            0.0
        } else {
            10_f64.powf(low[0] + (high[0] - low[0]) * t)
        },

        // M-cone
        if low[1].is_nan() && high[1].is_nan() {
            0.0
        } else if high[1].is_nan() && index_low >= 1 {
            // Extrapolate: use slope from previous two points
            let prev = lms_data_log[index_low - 1][1];
            let slope = low[1] - prev;
            10_f64.powf(low[1] + slope * t)
        } else if low[1].is_nan() || high[1].is_nan() {
            0.0
        } else {
            10_f64.powf(low[1] + (high[1] - low[1]) * t)
        },

        // S-cone
        if low[2].is_nan() && high[2].is_nan() {
            // Both NaN - find last valid measurement and extrapolate from there
            if index_low >= 2 {
                // Find last two valid points
                let mut last_valid_idx = None;
                for i in (0..=index_low).rev() {
                    if !lms_data_log[i][2].is_nan() {
                        last_valid_idx = Some(i);
                        break;
                    }
                }
                if let Some(last_idx) = last_valid_idx {
                    if last_idx >= 1 {
                        let last_val = lms_data_log[last_idx][2];
                        let prev_val = lms_data_log[last_idx - 1][2];
                        let slope = last_val - prev_val;
                        let steps_beyond = (wavelength_nm - (SS2000_START_NM + last_idx as f64 * SS2000_STEP_NM)) / SS2000_STEP_NM;
                        10_f64.powf(last_val + slope * steps_beyond)
                    } else {
                        0.0
                    }
                } else {
                    0.0
                }
            } else {
                0.0
            }
        } else if high[2].is_nan() && index_low >= 1 {
            // Extrapolate: use slope from previous two points
            let prev = lms_data_log[index_low - 1][2];
            let slope = low[2] - prev;
            10_f64.powf(low[2] + slope * t)
        } else if low[2].is_nan() || high[2].is_nan() {
            0.0
        } else {
            10_f64.powf(low[2] + (high[2] - low[2]) * t)
        },
    ]
}

/// Linear interpolation for already-converted linear data
fn interpolate_lms(lms_data: &[[f64; 3]], wavelength_nm: f64) -> [f64; 3] {
    let index_f = (wavelength_nm - SS2000_START_NM) / SS2000_STEP_NM;
    let index_low = index_f.floor() as usize;
    let index_high = (index_low + 1).min(SS2000_SAMPLES - 1);
    let t = index_f - index_low as f64;

    let low = lms_data[index_low];
    let high = lms_data[index_high];

    [
        low[0] + (high[0] - low[0]) * t,
        low[1] + (high[1] - low[1]) * t,
        low[2] + (high[2] - low[2]) * t,
    ]
}

/// Parse CIE 1931 2° XYZ colour matching functions (already in linear format)
fn parse_xyz_data() -> Vec<[f64; 3]> {
    let mut xyz_values = Vec::with_capacity(XYZ1931_SAMPLES);

    for i in 0..XYZ1931_SAMPLES {
        let x = XYZ_1931_2DEG_SO[i * 3];
        let y = XYZ_1931_2DEG_SO[i * 3 + 1];
        let z = XYZ_1931_2DEG_SO[i * 3 + 2];

        xyz_values.push([x, y, z]);
    }

    println!("// Loaded {} XYZ samples (380nm-780nm)", XYZ1931_SAMPLES);
    xyz_values
}

/// Linear interpolation for XYZ colour matching functions
fn interpolate_xyz(xyz_data: &[[f64; 3]], wavelength_nm: f64) -> [f64; 3] {
    let index_f = (wavelength_nm - XYZ1931_START_NM) / XYZ1931_STEP_NM;

    // Clamp to valid range
    if index_f < 0.0 {
        return [0.0, 0.0, 0.0];
    }
    if index_f >= (XYZ1931_SAMPLES - 1) as f64 {
        return [0.0, 0.0, 0.0];
    }

    let index_low = index_f.floor() as usize;
    let index_high = (index_low + 1).min(XYZ1931_SAMPLES - 1);
    let t = index_f - index_low as f64;

    let low = xyz_data[index_low];
    let high = xyz_data[index_high];

    [
        low[0] + (high[0] - low[0]) * t,
        low[1] + (high[1] - low[1]) * t,
        low[2] + (high[2] - low[2]) * t,
    ]
}

/// Calculate Illuminant E (equal energy) in lms by integrating across normalized spectrum
fn calculate_illuminant_e_lms(lms_data: &[[f64; 3]]) -> [f64; 3] {
    let mut l_sum = 0.0;
    let mut m_sum = 0.0;
    let mut s_sum = 0.0;

    // Simple sum - data is already normalized so each channel sums to 1.0
    for lms in lms_data {
        l_sum += lms[0];
        m_sum += lms[1];
        s_sum += lms[2];
    }

    // Data is normalized, so this should be [1.0, 1.0, 1.0]
    [l_sum, m_sum, s_sum]
}

/// Build VSF RGB → lms transformation matrix using geometric mean normalization
/// Each ROW is one primary's geometric mean normalized [l, m, s] triplet
/// Uses normalized 1nm data (uppercase LMS where each channel sums to 1.0)
fn build_vsf_to_lms_matrix(lms_1nm: &[f64]) -> [f64; 9] {
    // Extract LMS values at VSF primaries from 1nm data
    // Data starts at 390nm, so index = wavelength - 390
    let red_idx = (VSF_RED_NM as usize - 390) * 3;
    let green_idx = (VSF_GREEN_NM as usize - 390) * 3;
    let blue_idx = (VSF_BLUE_NM as usize - 390) * 3;

    let red_LMS = [lms_1nm[red_idx], lms_1nm[red_idx + 1], lms_1nm[red_idx + 2]];
    let green_LMS = [lms_1nm[green_idx], lms_1nm[green_idx + 1], lms_1nm[green_idx + 2]];
    let blue_LMS = [lms_1nm[blue_idx], lms_1nm[blue_idx + 1], lms_1nm[blue_idx + 2]];

    // Apply geometric mean normalization: l = L/(L+M+S), m = M/(L+M+S), s = S/(L+M+S)
    let red_sum = red_LMS[0] + red_LMS[1] + red_LMS[2];
    let red_lms = [red_LMS[0] / red_sum, red_LMS[1] / red_sum, red_LMS[2] / red_sum];

    let green_sum = green_LMS[0] + green_LMS[1] + green_LMS[2];
    let green_lms = [green_LMS[0] / green_sum, green_LMS[1] / green_sum, green_LMS[2] / green_sum];

    let blue_sum = blue_LMS[0] + blue_LMS[1] + blue_LMS[2];
    let blue_lms = [blue_LMS[0] / blue_sum, blue_LMS[1] / blue_sum, blue_LMS[2] / blue_sum];

    println!("// VSF primaries - uppercase LMS (normalized across spectrum):");
    println!("//   Red (703nm):   L={}, M={}, S={}", red_LMS[0], red_LMS[1], red_LMS[2]);
    println!("//   Green (523nm): L={}, M={}, S={}", green_LMS[0], green_LMS[1], green_LMS[2]);
    println!("//   Blue (462nm):  L={}, M={}, S={}", blue_LMS[0], blue_LMS[1], blue_LMS[2]);

    println!("// VSF primaries - lowercase lms (geometric mean normalized):");
    println!("//   Red (703nm):   l={}, m={}, s={} (sum={})", red_lms[0], red_lms[1], red_lms[2], red_lms[0] + red_lms[1] + red_lms[2]);
    println!("//   Green (523nm): l={}, m={}, s={} (sum={})", green_lms[0], green_lms[1], green_lms[2], green_lms[0] + green_lms[1] + green_lms[2]);
    println!("//   Blue (462nm):  l={}, m={}, s={} (sum={})", blue_lms[0], blue_lms[1], blue_lms[2], blue_lms[0] + blue_lms[1] + blue_lms[2]);
    println!("//   Column sums (unscaled): l={}, m={}, s={} (total={})",
             red_lms[0] + green_lms[0] + blue_lms[0],
             red_lms[1] + green_lms[1] + blue_lms[1],
             red_lms[2] + green_lms[2] + blue_lms[2],
             red_lms[0] + red_lms[1] + red_lms[2] + green_lms[0] + green_lms[1] + green_lms[2] + blue_lms[0] + blue_lms[1] + blue_lms[2]);

    // Build unscaled matrix in column-major format
    // Each column is a primary's full [L, M, S] response
    let unscaled = [
        red_lms[0], red_lms[1], red_lms[2],         // Column 0: Red primary's [L, M, S]
        green_lms[0], green_lms[1], green_lms[2],   // Column 1: Green primary's [L, M, S]
        blue_lms[0], blue_lms[1], blue_lms[2],      // Column 2: Blue primary's [L, M, S]
    ];

    // Invert the matrix
    let unscaled_inv = invert_matrix_3x3(&unscaled);

    // Multiply our target Illuminant E [1,1,1] thru the INVERSE
    // This tells us what RGB input we need to produce lms=[1,1,1]
    let illum_e = [1.0, 1.0, 1.0];
    let rgb_scale_factors = apply_matrix_3x3(&unscaled_inv, &illum_e);

    println!("// RGB brightness needed to produce Illuminant E:");
    println!("//   RGB = [{}, {}, {}]", rgb_scale_factors[0], rgb_scale_factors[1], rgb_scale_factors[2]);

    // Scale each column (primary) by MULTIPLYING by the corresponding RGB scaling factor
    // This ensures equal brightness primaries (RGB=[1,1,1]) produce Illuminant E
    // Matrix is in column-major format, so each group of 3 values is a PRIMARY's full LMS response
    let scaled = [
        red_lms[0] * rgb_scale_factors[0], red_lms[1] * rgb_scale_factors[0], red_lms[2] * rgb_scale_factors[0],        // Red primary scaled
        green_lms[0] * rgb_scale_factors[1], green_lms[1] * rgb_scale_factors[1], green_lms[2] * rgb_scale_factors[1],  // Green primary scaled
        blue_lms[0] * rgb_scale_factors[2], blue_lms[1] * rgb_scale_factors[2], blue_lms[2] * rgb_scale_factors[2],     // Blue primary scaled
    ];

    // Verify the scaled matrix
    let rgb_white = [1.0, 1.0, 1.0];
    let scaled_white = apply_matrix_3x3(&scaled, &rgb_white);
    println!("// Verification - RGB=[1,1,1] thru scaled matrix:");
    println!("//   lms = [{}, {}, {}]", scaled_white[0], scaled_white[1], scaled_white[2]);

    scaled
}

fn format_matrix(m: &[f64; 9], name: &str) -> String {
    format!(
        "pub const {}: [f32; 9] = [\n    {}f32, {}f32, {}f32,\n    {}f32, {}f32, {}f32,\n    {}f32, {}f32, {}f32,\n];",
        name,
        m[0], m[1], m[2],
        m[3], m[4], m[5],
        m[6], m[7], m[8]
    )
}

/// Build VSF RGB → XYZ transformation matrix
/// Each ROW is one primary's normalized [x, y, z] triplet (sum normalization)
/// Uses normalized 1nm data where Y (luminance) is scaled to peak at 1.0
fn build_vsf_to_xyz_matrix(xyz_1nm: &[f64]) -> [f64; 9] {
    // Extract XYZ values at VSF primaries from 1nm data
    // Data starts at 380nm, so index = wavelength - 380
    let red_idx = (VSF_RED_NM as usize - 380) * 3;
    let green_idx = (VSF_GREEN_NM as usize - 380) * 3;
    let blue_idx = (VSF_BLUE_NM as usize - 380) * 3;

    let red_XYZ = [xyz_1nm[red_idx], xyz_1nm[red_idx + 1], xyz_1nm[red_idx + 2]];
    let green_XYZ = [xyz_1nm[green_idx], xyz_1nm[green_idx + 1], xyz_1nm[green_idx + 2]];
    let blue_XYZ = [xyz_1nm[blue_idx], xyz_1nm[blue_idx + 1], xyz_1nm[blue_idx + 2]];

    // Apply sum normalization: x = X/(X+Y+Z), y = Y/(X+Y+Z), z = Z/(X+Y+Z)
    let red_sum = red_XYZ[0] + red_XYZ[1] + red_XYZ[2];
    let red_xyz = [red_XYZ[0] / red_sum, red_XYZ[1] / red_sum, red_XYZ[2] / red_sum];

    let green_sum = green_XYZ[0] + green_XYZ[1] + green_XYZ[2];
    let green_xyz = [green_XYZ[0] / green_sum, green_XYZ[1] / green_sum, green_XYZ[2] / green_sum];

    let blue_sum = blue_XYZ[0] + blue_XYZ[1] + blue_XYZ[2];
    let blue_xyz = [blue_XYZ[0] / blue_sum, blue_XYZ[1] / blue_sum, blue_XYZ[2] / blue_sum];

    println!("// VSF primaries - uppercase XYZ (from CIE 1931 2° CMF):");
    println!("//   Red (703nm):   X={}, Y={}, Z={}", red_XYZ[0], red_XYZ[1], red_XYZ[2]);
    println!("//   Green (523nm): X={}, Y={}, Z={}", green_XYZ[0], green_XYZ[1], green_XYZ[2]);
    println!("//   Blue (462nm):  X={}, Y={}, Z={}", blue_XYZ[0], blue_XYZ[1], blue_XYZ[2]);

    println!("// VSF primaries - lowercase xyz (sum normalized):");
    println!("//   Red (703nm):   x={}, y={}, z={} (sum={})", red_xyz[0], red_xyz[1], red_xyz[2], red_xyz[0] + red_xyz[1] + red_xyz[2]);
    println!("//   Green (523nm): x={}, y={}, z={} (sum={})", green_xyz[0], green_xyz[1], green_xyz[2], green_xyz[0] + green_xyz[1] + green_xyz[2]);
    println!("//   Blue (462nm):  x={}, y={}, z={} (sum={})", blue_xyz[0], blue_xyz[1], blue_xyz[2], blue_xyz[0] + blue_xyz[1] + blue_xyz[2]);

    // Build unscaled matrix with each primary as a COLUMN (column-major format)
    let unscaled = [
        red_xyz[0], red_xyz[1], red_xyz[2],         // Column 0: red primary's [X,Y,Z] contribution
        green_xyz[0], green_xyz[1], green_xyz[2],   // Column 1: green primary's [X,Y,Z] contribution
        blue_xyz[0], blue_xyz[1], blue_xyz[2],      // Column 2: blue primary's [X,Y,Z] contribution
    ];

    // Invert the matrix
    let unscaled_inv = invert_matrix_3x3(&unscaled);

    // Multiply our target Illuminant E [1,1,1] thru the INVERSE
    // This tells us what RGB input we need to produce XYZ=[1,1,1]
    let illum_e = [1.0, 1.0, 1.0];
    let rgb_scale_factors = apply_matrix_3x3(&unscaled_inv, &illum_e);

    println!("// RGB brightness needed to produce Illuminant E in XYZ:");
    println!("//   RGB = [{}, {}, {}]", rgb_scale_factors[0], rgb_scale_factors[1], rgb_scale_factors[2]);

    // Scale each column (primary) by multiplying by the corresponding RGB scaling factor
    // Matrix is in column-major format, so each column (group of 3 consecutive values) is a PRIMARY
    let scaled = [
        red_xyz[0] * rgb_scale_factors[0], red_xyz[1] * rgb_scale_factors[0], red_xyz[2] * rgb_scale_factors[0],  // Red column scaled
        green_xyz[0] * rgb_scale_factors[1], green_xyz[1] * rgb_scale_factors[1], green_xyz[2] * rgb_scale_factors[1],  // Green column scaled
        blue_xyz[0] * rgb_scale_factors[2], blue_xyz[1] * rgb_scale_factors[2], blue_xyz[2] * rgb_scale_factors[2],  // Blue column scaled
    ];

    // Verify the scaled matrix
    let rgb_white = [1.0, 1.0, 1.0];
    let scaled_white = apply_matrix_3x3(&scaled, &rgb_white);
    println!("// Verification - RGB=[1,1,1] thru scaled XYZ matrix:");
    println!("//   XYZ = [{}, {}, {}]", scaled_white[0], scaled_white[1], scaled_white[2]);

    scaled
}

/// Build RGB → XYZ transformation matrix from xy chromaticity coordinates
/// Uses CIE xy coordinates with D65 white point
fn build_rgb_from_xy_to_xyz_matrix(
    red_xy: [f64; 2],
    green_xy: [f64; 2],
    blue_xy: [f64; 2],
    space_name: &str,
) -> [f64; 9] {
    // Convert xy to XYZ (assume Y=1 for each primary)
    // X = x * Y / y
    // Y = Y (normalized to 1)
    // Z = (1 - x - y) * Y / y

    let red_XYZ = [red_xy[0] / red_xy[1], 1.0, (1.0 - red_xy[0] - red_xy[1]) / red_xy[1]];
    let green_XYZ = [green_xy[0] / green_xy[1], 1.0, (1.0 - green_xy[0] - green_xy[1]) / green_xy[1]];
    let blue_XYZ = [blue_xy[0] / blue_xy[1], 1.0, (1.0 - blue_xy[0] - blue_xy[1]) / blue_xy[1]];

    println!("// {} primaries - CIE xy chromaticity:", space_name);
    println!("//   Red:   x={:.4}, y={:.4}", red_xy[0], red_xy[1]);
    println!("//   Green: x={:.4}, y={:.4}", green_xy[0], green_xy[1]);
    println!("//   Blue:  x={:.4}, y={:.4}", blue_xy[0], blue_xy[1]);

    println!("// {} primaries - XYZ (Y=1 normalized):", space_name);
    println!("//   Red:   X={}, Y={}, Z={}", red_XYZ[0], red_XYZ[1], red_XYZ[2]);
    println!("//   Green: X={}, Y={}, Z={}", green_XYZ[0], green_XYZ[1], green_XYZ[2]);
    println!("//   Blue:  X={}, Y={}, Z={}", blue_XYZ[0], blue_XYZ[1], blue_XYZ[2]);

    // Build unscaled matrix with each primary as a COLUMN (column-major format)
    let unscaled = [
        red_XYZ[0], red_XYZ[1], red_XYZ[2],         // Column 0: red primary's [X,Y,Z] contribution
        green_XYZ[0], green_XYZ[1], green_XYZ[2],   // Column 1: green primary's [X,Y,Z] contribution
        blue_XYZ[0], blue_XYZ[1], blue_XYZ[2],      // Column 2: blue primary's [X,Y,Z] contribution
    ];

    // Invert the matrix
    let unscaled_inv = invert_matrix_3x3(&unscaled);

    // D65 white point in XYZ (normalized so Y=1.0)
    let d65_xyz = [0.95047, 1.0, 1.08883];
    let rgb_scale_factors = apply_matrix_3x3(&unscaled_inv, &d65_xyz);

    println!("// RGB brightness needed to produce D65 white point in XYZ:");
    println!("//   RGB = [{}, {}, {}]", rgb_scale_factors[0], rgb_scale_factors[1], rgb_scale_factors[2]);

    // Scale each column (primary) by multiplying by the corresponding RGB scaling factor
    // Matrix is in column-major format, so each column (group of 3 consecutive values) is a PRIMARY
    let scaled = [
        red_XYZ[0] * rgb_scale_factors[0], red_XYZ[1] * rgb_scale_factors[0], red_XYZ[2] * rgb_scale_factors[0],  // Red column scaled
        green_XYZ[0] * rgb_scale_factors[1], green_XYZ[1] * rgb_scale_factors[1], green_XYZ[2] * rgb_scale_factors[1],  // Green column scaled
        blue_XYZ[0] * rgb_scale_factors[2], blue_XYZ[1] * rgb_scale_factors[2], blue_XYZ[2] * rgb_scale_factors[2],  // Blue column scaled
    ];

    // Verify the scaled matrix
    let rgb_white = [1.0, 1.0, 1.0];
    let scaled_white = apply_matrix_3x3(&scaled, &rgb_white);
    println!("// Verification - {} RGB=[1,1,1] thru scaled XYZ matrix:", space_name);
    println!("//   XYZ = [{}, {}, {}] (should be D65)", scaled_white[0], scaled_white[1], scaled_white[2]);

    scaled
}

/// Build Rec.2020 RGB → XYZ transformation matrix
/// Uses monochromatic primaries (630nm, 532nm, 467nm) with D65 white point
fn build_rec2020_to_xyz_matrix(xyz_1nm: &[f64]) -> [f64; 9] {
    // Extract XYZ values at Rec.2020 primaries from 1nm data
    // Data starts at 380nm, so index = wavelength - 380
    let red_idx = (REC2020_RED_NM as usize - 380) * 3;
    let green_idx = (REC2020_GREEN_NM as usize - 380) * 3;
    let blue_idx = (REC2020_BLUE_NM as usize - 380) * 3;

    let red_XYZ = [xyz_1nm[red_idx], xyz_1nm[red_idx + 1], xyz_1nm[red_idx + 2]];
    let green_XYZ = [xyz_1nm[green_idx], xyz_1nm[green_idx + 1], xyz_1nm[green_idx + 2]];
    let blue_XYZ = [xyz_1nm[blue_idx], xyz_1nm[blue_idx + 1], xyz_1nm[blue_idx + 2]];

    // Normalize to sum=1.0 for each primary
    let red_sum = red_XYZ[0] + red_XYZ[1] + red_XYZ[2];
    let green_sum = green_XYZ[0] + green_XYZ[1] + green_XYZ[2];
    let blue_sum = blue_XYZ[0] + blue_XYZ[1] + blue_XYZ[2];

    let red_xyz = [red_XYZ[0] / red_sum, red_XYZ[1] / red_sum, red_XYZ[2] / red_sum];
    let green_xyz = [green_XYZ[0] / green_sum, green_XYZ[1] / green_sum, green_XYZ[2] / green_sum];
    let blue_xyz = [blue_XYZ[0] / blue_sum, blue_XYZ[1] / blue_sum, blue_XYZ[2] / blue_sum];

    println!("// Rec.2020 primaries - uppercase XYZ (raw CIE 1931 values):");
    println!("//   Red (630nm):   X={}, Y={}, Z={}", red_XYZ[0], red_XYZ[1], red_XYZ[2]);
    println!("//   Green (532nm): X={}, Y={}, Z={}", green_XYZ[0], green_XYZ[1], green_XYZ[2]);
    println!("//   Blue (467nm):  X={}, Y={}, Z={}", blue_XYZ[0], blue_XYZ[1], blue_XYZ[2]);

    println!("// Rec.2020 primaries - lowercase xyz (sum normalized):");
    println!("//   Red (630nm):   x={}, y={}, z={} (sum={})", red_xyz[0], red_xyz[1], red_xyz[2], red_xyz[0] + red_xyz[1] + red_xyz[2]);
    println!("//   Green (532nm): x={}, y={}, z={} (sum={})", green_xyz[0], green_xyz[1], green_xyz[2], green_xyz[0] + green_xyz[1] + green_xyz[2]);
    println!("//   Blue (467nm):  x={}, y={}, z={} (sum={})", blue_xyz[0], blue_xyz[1], blue_xyz[2], blue_xyz[0] + blue_xyz[1] + blue_xyz[2]);

    // Build unscaled matrix with each primary as a COLUMN (column-major format)
    let unscaled = [
        red_xyz[0], red_xyz[1], red_xyz[2],         // Column 0: red primary's [X,Y,Z] contribution
        green_xyz[0], green_xyz[1], green_xyz[2],   // Column 1: green primary's [X,Y,Z] contribution
        blue_xyz[0], blue_xyz[1], blue_xyz[2],      // Column 2: blue primary's [X,Y,Z] contribution
    ];

    // Invert the matrix
    let unscaled_inv = invert_matrix_3x3(&unscaled);

    // D65 white point in XYZ (normalized so Y=1.0)
    // CIE Standard Illuminant D65
    let d65_xyz = [0.95047, 1.0, 1.08883];
    let rgb_scale_factors = apply_matrix_3x3(&unscaled_inv, &d65_xyz);

    println!("// RGB brightness needed to produce D65 white point in XYZ:");
    println!("//   RGB = [{}, {}, {}]", rgb_scale_factors[0], rgb_scale_factors[1], rgb_scale_factors[2]);

    // Scale each column (primary) by multiplying by the corresponding RGB scaling factor
    // Matrix is in column-major format, so each column (group of 3 consecutive values) is a PRIMARY
    let scaled = [
        red_xyz[0] * rgb_scale_factors[0], red_xyz[1] * rgb_scale_factors[0], red_xyz[2] * rgb_scale_factors[0],  // Red column scaled
        green_xyz[0] * rgb_scale_factors[1], green_xyz[1] * rgb_scale_factors[1], green_xyz[2] * rgb_scale_factors[1],  // Green column scaled
        blue_xyz[0] * rgb_scale_factors[2], blue_xyz[1] * rgb_scale_factors[2], blue_xyz[2] * rgb_scale_factors[2],  // Blue column scaled
    ];

    // Verify the scaled matrix
    let rgb_white = [1.0, 1.0, 1.0];
    let scaled_white = apply_matrix_3x3(&scaled, &rgb_white);
    println!("// Verification - Rec.2020 RGB=[1,1,1] thru scaled XYZ matrix:");
    println!("//   XYZ = [{}, {}, {}] (should be D65)", scaled_white[0], scaled_white[1], scaled_white[2]);

    scaled
}

/// Generate 1nm-spaced XYZ data from 380nm to 780nm (401 samples)
/// Interpolates linearly, keeps raw CIE 1931 2° values (Y peaks at ~1.0)
fn generate_1nm_xyz_data() -> Vec<f64> {
    let xyz_data = parse_xyz_data();
    let mut xyz_1nm = Vec::new();

    for wavelength in 380..=780 {
        let xyz = interpolate_xyz(&xyz_data, wavelength as f64);
        xyz_1nm.push(xyz[0]);
        xyz_1nm.push(xyz[1]);
        xyz_1nm.push(xyz[2]);
    }

    println!("// Interpolated to {} XYZ samples (380nm-780nm)", (780 - 380 + 1));

    xyz_1nm
}

/// Generate 1nm-spaced LMS data from 390nm to 830nm (441 samples)
/// Interpolates in log space, then normalizes so each channel sums to 1.0
fn generate_1nm_lms_data() -> Vec<f64> {
    // First, interpolate to 1nm spacing in log space
    let mut lms_1nm = Vec::new();

    // Keep log data for interpolation
    let mut lms_data_log: Vec<[f64; 3]> = Vec::with_capacity(SS2000_SAMPLES);
    for i in 0..SS2000_SAMPLES {
        lms_data_log.push([
            LMS_2000_10DEG_SO[i * 3],
            LMS_2000_10DEG_SO[i * 3 + 1],
            LMS_2000_10DEG_SO[i * 3 + 2],
        ]);
    }

    for wavelength in 390..=830 {
        let lms = interpolate_lms_log(&lms_data_log, wavelength as f64);
        lms_1nm.push(lms);
    }

    println!("// Interpolated to {} 1nm samples", lms_1nm.len());

    // Calculate sums for each channel
    let mut l_sum = 0.0;
    let mut m_sum = 0.0;
    let mut s_sum = 0.0;

    for lms in &lms_1nm {
        l_sum += lms[0];
        m_sum += lms[1];
        s_sum += lms[2];
    }

    println!("// 1nm raw LMS sums: L={}, M={}, S={}", l_sum, m_sum, s_sum);
    println!("// 1nm raw LMS total: {}", l_sum + m_sum + s_sum);

    // Normalize: reciprocate and multiply
    let l_factor = 1.0 / l_sum;
    let m_factor = 1.0 / m_sum;
    let s_factor = 1.0 / s_sum;

    println!("// 1nm normalization factors: L={}, M={}, S={}", l_factor, m_factor, s_factor);

    // Apply normalization and flatten to Vec<f64>
    let mut result = Vec::new();
    for lms in &mut lms_1nm {
        result.push(lms[0] * l_factor);
        result.push(lms[1] * m_factor);
        result.push(lms[2] * s_factor);
    }

    // Verify normalization
    let mut l_check = 0.0;
    let mut m_check = 0.0;
    let mut s_check = 0.0;
    for i in (0..result.len()).step_by(3) {
        l_check += result[i];
        m_check += result[i + 1];
        s_check += result[i + 2];
    }

    println!("// 1nm normalized lms sums: l={}, m={}, s={}", l_check, m_check, s_check);
    println!("// 1nm normalized lms total: {}", l_check + m_check + s_check);

    result
}

fn format_lms_1nm_array(data: &[f64]) -> String {
    let mut output = String::from("const LMS_2000_10DEG_1NM_DATA: [f32; 1323] = [\n");

    for (i, &value) in data.iter().enumerate() {
        if i % 3 == 0 {
            output.push_str("    ");
        }
        output.push_str(&format!("{}f32", value));
        if i < data.len() - 1 {
            output.push_str(", ");
        }
        if i % 3 == 2 {
            output.push('\n');
        }
    }

    output.push_str("];\n\n");
    output.push_str("/// Stockman & Sharpe (2000) 10° cone fundamentals at 1nm spacing\n");
    output.push_str("///\n");
    output.push_str("/// Linear interpolation from 5nm data, normalized so each channel sums to 1.0.\n");
    output.push_str("/// Data spans 390nm to 830nm with 1nm spacing (441 wavelength samples).\n");
    output.push_str("/// 3-channel interleaved format: [L, M, S] at each wavelength.\n");
    output.push_str("pub const LMS_2000_10DEG_1NM: ConstSpectrum = ConstSpectrum {\n");
    output.push_str("    start_nm: 390.0,\n");
    output.push_str("    stop_nm: 830.0,\n");
    output.push_str("    spacing_nm: 1.0,\n");
    output.push_str("    num_channels: 3,\n");
    output.push_str("    data: &LMS_2000_10DEG_1NM_DATA,\n");
    output.push_str("};\n");
    output
}

fn write_spectral_constants(vsf_to_lms: &[f64; 9], lms_to_vsf: &[f64; 9], lms_1nm: &[f64]) -> std::io::Result<()> {
    let content = format!(
r#"//! Spectral colourspace transformation matrices and constants
//!
//! **Auto-generated - do not edit directly!**
//! Generated by tools/src/bin/generate_constants.rs
//!
//! All matrices in this module are derived from wavelength-based primaries and
//! observer models (Stockman & Sharpe 2000 10° cone fundamentals).
//! These are independent of the CIE 1931 xy coordinate system.
//!
//! VSF uses uppercase 'LMS' to denote normalized cone space where each
//! channel independently sums to 1.0 (total = 3.0).

use crate::colour::spectrum::ConstSpectrum;

/// Raw spectral data for Stockman & Sharpe (2000) 10° cone fundamentals at 1nm spacing
///
/// Linear interpolation from 5nm data, normalized so each channel sums to 1.0.
/// Data spans 390nm to 830nm (441 samples × 3 channels = 1323 values).
/// Format: [L390, M390, S390, L391, M391, S391, ..., L830, M830, S830]
{}
/// VSF RGB → lms transformation matrix
///
/// Converts linear VSF RGB (703nm, 523nm, 462nm primaries) to normalized lms cone space.
/// Derived from Stockman & Sharpe (2000) 10° cone fundamentals with white point
/// scaling so that RGB=[1,1,1] maps to Illuminant E (equal energy spectrum).
///
/// Matrix layout (column-major):
/// - Indices 0-2: Red primary's [L, M, S] cone responses
/// - Indices 3-5: Green primary's [L, M, S] cone responses
/// - Indices 6-8: Blue primary's [L, M, S] cone responses
{}

/// lms → VSF RGB transformation matrix
///
/// Converts normalized lms cone space to linear VSF RGB.
/// Inverse of VSF_RGB2LMS.
{}

/// lms → Photopic luminance weights
///
/// Standard photopic luminosity function weights for converting lms to luminance.
pub const LMS2PHOTOPIC: [f32; 3] = [
    1.0,  // l cone weight (placeholder - needs actual photopic weights)
    1.0,  // m cone weight
    0.0,  // s cone weight (S-cones don't contribute to photopic luminance)
];

// Rec.2020 transformation matrices are now in the rec2020 module
// See: src/colour/rec2020/constants.rs
"#,
        format_lms_1nm_array(lms_1nm),
        format_matrix(vsf_to_lms, "VSF_RGB2LMS"),
        format_matrix(lms_to_vsf, "LMS2VSF_RGB")
    );

    let path = Path::new("../src/colour/spectral/constants.rs");
    fs::write(path, content)?;
    println!("Wrote spectral constants to {}", path.display());
    Ok(())
}

fn format_xyz_1nm_array(data: &[f64]) -> String {
    let mut output = String::from("const XYZ_1931_2DEG_1NM_DATA: [f32; 1203] = [\n");

    for (i, &value) in data.iter().enumerate() {
        if i % 3 == 0 {
            output.push_str("    ");
        }
        output.push_str(&format!("{}f32", value));
        if i < data.len() - 1 {
            output.push_str(", ");
        }
        if i % 3 == 2 {
            output.push('\n');
        }
    }

    output.push_str("];\n\n");
    output.push_str("/// CIE 1931 2° Standard Observer XYZ colour matching functions at 1nm spacing\n");
    output.push_str("///\n");
    output.push_str("/// Linear interpolation from 5nm data.\n");
    output.push_str("/// Data spans 380nm to 780nm with 1nm spacing (401 wavelength samples).\n");
    output.push_str("/// 3-channel interleaved format: [X, Y, Z] at each wavelength.\n");
    output.push_str("pub const XYZ_1931_2DEG_1NM: ConstSpectrum = ConstSpectrum {\n");
    output.push_str("    start_nm: 380.0,\n");
    output.push_str("    stop_nm: 780.0,\n");
    output.push_str("    spacing_nm: 1.0,\n");
    output.push_str("    num_channels: 3,\n");
    output.push_str("    data: &XYZ_1931_2DEG_1NM_DATA,\n");
    output.push_str("};\n");
    output
}

fn write_xyz_constants(
    vsf_to_xyz: &[f64; 9],
    xyz_to_vsf: &[f64; 9],
    vsf_to_srgb: &[f64; 9],
    srgb_to_vsf: &[f64; 9],
    vsf_to_adobe_rgb: &[f64; 9],
    adobe_rgb_to_vsf: &[f64; 9],
    xyz_1nm: &[f64],
) -> std::io::Result<()> {
    let content = format!(
r#"//! Legacy colourspace transformation matrices and constants
//!
//! **Auto-generated - do not edit directly!**
//! Generated by tools/src/bin/generate_constants.rs
//!
//! # ⚠️ Legacy Warning
//!
//! **All matrices in this module are defined using CIE 1931 xy chromaticity coordinates and are permanently bound to the 1931 2° Standard Observer.**
//!
//! The CIE 1931 XYZ system is based on colour matching experiments from the 1920s with only ~17 observers. It has known flaws and introduces accumulated errors thru multiple transformation steps.
//!
//! **VSF prefers spectral/wavelength-based definitions** (see `spectral` module) which use modern Stockman & Sharpe 2000 10° cone fundamentals and avoid CIE 1931 entirely.
//!
//! ## When to use this module
//!
//! - **sRGB/Rec.709/Adobe 1998**: Required for compatibility (primaries defined in xy)
//! - **XYZ conversions**: Only when absolutely necessary for legacy workflows (DNG)
//!
//! For new colour work, use spectral definitions (Rec.2020, VSF RGB) whenever possible.

use crate::colour::spectrum::ConstSpectrum;

/// Raw spectral data for CIE 1931 2° Standard Observer XYZ colour matching functions at 1nm spacing
///
/// Linear interpolation from 5nm data.
/// Data spans 380nm to 780nm (401 samples × 3 channels = 1203 values).
/// Format: [X380, Y380, Z380, X381, Y381, Z381, ..., X780, Y780, Z780]
{}
/// VSF RGB → XYZ transformation matrix
///
/// Converts linear VSF RGB (703nm, 523nm, 462nm primaries) to CIE 1931 XYZ colourspace. Derived from CIE 1931 2° Standard Observer colour matching functions with white point scaling so that RGB=[1,1,1] maps to Illuminant E (equal energy spectrum).
///
/// Matrix layout (column-major):
/// - Indices 0-2: X channel contributions from [red, green, blue]
/// - Indices 3-5: Y channel contributions from [red, green, blue]
/// - Indices 6-8: Z channel contributions from [red, green, blue]
{}

/// XYZ → VSF RGB transformation matrix
///
/// Converts CIE 1931 XYZ colourspace to linear VSF RGB.
/// Inverse of VSF_RGB2XYZ.
{}

/// VSF RGB → sRGB transformation matrix
///
/// Converts linear VSF RGB (703nm, 523nm, 462nm primaries, Illuminant E white) to linear sRGB (IEC 61966-2-1:1999 primaries, D65 white point).
///
/// Matrix layout (column-major):
/// - Indices 0-2: Red channel contributions from [red, green, blue]
/// - Indices 3-5: Green channel contributions from [red, green, blue]
/// - Indices 6-8: Blue channel contributions from [red, green, blue]
{}

/// sRGB → VSF RGB transformation matrix
///
/// Converts linear sRGB to linear VSF RGB.
/// Inverse of VSF_RGB2SRGB.
{}

/// VSF RGB → Adobe RGB (1998) transformation matrix
///
/// Converts linear VSF RGB (703nm, 523nm, 462nm primaries, Illuminant E white) to linear Adobe RGB (1998 specification primaries, D65 white point).
///
/// Matrix layout (column-major):
/// - Indices 0-2: Red channel contributions from [red, green, blue]
/// - Indices 3-5: Green channel contributions from [red, green, blue]
/// - Indices 6-8: Blue channel contributions from [red, green, blue]
{}

/// Adobe RGB (1998) → VSF RGB transformation matrix
///
/// Converts linear Adobe RGB (1998) to linear VSF RGB.
/// Inverse of VSF_RGB2ADOBE_RGB.
{}
"#,
        format_xyz_1nm_array(xyz_1nm),
        format_matrix(vsf_to_xyz, "VSF_RGB2XYZ"),
        format_matrix(xyz_to_vsf, "XYZ2VSF_RGB"),
        format_matrix(vsf_to_srgb, "VSF_RGB2SRGB"),
        format_matrix(srgb_to_vsf, "SRGB2VSF_RGB"),
        format_matrix(vsf_to_adobe_rgb, "VSF_RGB2ADOBE_RGB"),
        format_matrix(adobe_rgb_to_vsf, "ADOBE_RGB2VSF_RGB")
    );

    let path = Path::new("../src/colour/legacy/constants.rs");
    fs::write(path, content)?;
    println!("Wrote XYZ and RGB constants to {}", path.display());
    Ok(())
}

fn write_rec2020_constants(vsf_to_rec2020: &[f64; 9], rec2020_to_vsf: &[f64; 9]) -> std::io::Result<()> {
    let content = format!(
r#"//! Rec.2020 colourspace transformation matrices
//!
//! **Auto-generated - do not edit directly!**
//! Generated by tools/src/bin/generate_constants.rs
//!
//! ITU-R BT.2020 (Rec.2020) is a wide colour gamut standard for UHDTV.
//! These matrices use monochromatic primaries (630nm, 532nm, 467nm) with D65 white point.

/// VSF RGB → Rec.2020 transformation matrix
///
/// Converts linear VSF RGB (703nm, 523nm, 462nm primaries, Illuminant E white)
/// to linear Rec.2020 RGB (630nm, 532nm, 467nm primaries, D65 white point).
///
/// Matrix layout (column-major):
/// - Indices 0-2: Red channel contributions from [red, green, blue]
/// - Indices 3-5: Green channel contributions from [red, green, blue]
/// - Indices 6-8: Blue channel contributions from [red, green, blue]
{}

/// Rec.2020 → VSF RGB transformation matrix
///
/// Converts linear Rec.2020 RGB to linear VSF RGB.
/// Inverse of VSF_RGB2REC2020.
{}
"#,
        format_matrix(vsf_to_rec2020, "VSF_RGB2REC2020"),
        format_matrix(rec2020_to_vsf, "REC2020_2VSF_RGB")
    );

    let path = Path::new("../src/colour/rec2020/constants.rs");
    // Create directory if it doesn't exist
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    println!("Wrote Rec.2020 constants to {}", path.display());
    Ok(())
}

fn main() -> std::io::Result<()> {
    println!("VSF RGB ↔ lms/XYZ transformation matrices");
    println!("Derived from Stockman & Sharpe (2000) 10° cone fundamentals and CIE 1931 2° XYZ");
    println!("VSF RGB primaries: 703nm (R), 523nm (G), 462nm (B)");
    println!();

    // Generate 1nm-spaced normalized LMS data
    println!("// Generating 1nm-spaced LMS data...");
    let lms_1nm = generate_1nm_lms_data();

    // Build VSF → lms matrix using normalized 1nm data
    println!();
    println!("// Building LMS matrices...");
    let vsf_to_lms = build_vsf_to_lms_matrix(&lms_1nm);

    // Invert to get lms → VSF
    let lms_to_vsf = invert_matrix_3x3(&vsf_to_lms);

    // Generate 1nm-spaced XYZ data
    println!();
    println!("// Generating 1nm-spaced XYZ data...");
    let xyz_1nm = generate_1nm_xyz_data();

    // Build VSF → XYZ matrix
    println!();
    println!("// Building XYZ matrices...");
    let vsf_to_xyz = build_vsf_to_xyz_matrix(&xyz_1nm);

    // Invert to get XYZ → VSF
    let xyz_to_vsf = invert_matrix_3x3(&vsf_to_xyz);

    // Build Rec.2020 → XYZ matrix
    println!();
    println!("// Building Rec.2020 matrices...");
    let rec2020_to_xyz = build_rec2020_to_xyz_matrix(&xyz_1nm);
    let xyz_to_rec2020 = invert_matrix_3x3(&rec2020_to_xyz);

    // Compose VSF → Rec.2020 transformation (VSF → XYZ → Rec.2020)
    let vsf_to_rec2020 = convert_matrix_3x3(&xyz_to_rec2020, &vsf_to_xyz);
    let rec2020_to_vsf = invert_matrix_3x3(&vsf_to_rec2020);

    println!();
    println!("// VSF → Rec.2020 composed transformation:");
    println!("//   VSF RGB → XYZ → Rec.2020 RGB");

    // Build sRGB → XYZ matrix
    println!();
    println!("// Building sRGB matrices...");
    let srgb_to_xyz = build_rgb_from_xy_to_xyz_matrix(SRGB_RED_XY, SRGB_GREEN_XY, SRGB_BLUE_XY, "sRGB");
    let xyz_to_srgb = invert_matrix_3x3(&srgb_to_xyz);

    // Compose VSF → sRGB transformation (VSF → XYZ → sRGB)
    let vsf_to_srgb = convert_matrix_3x3(&xyz_to_srgb, &vsf_to_xyz);
    let srgb_to_vsf = invert_matrix_3x3(&vsf_to_srgb);

    println!();
    println!("// VSF → sRGB composed transformation:");
    println!("//   VSF RGB → XYZ → sRGB");

    // Build Adobe RGB → XYZ matrix
    println!();
    println!("// Building Adobe RGB matrices...");
    let adobe_rgb_to_xyz = build_rgb_from_xy_to_xyz_matrix(ADOBE_RGB_RED_XY, ADOBE_RGB_GREEN_XY, ADOBE_RGB_BLUE_XY, "Adobe RGB");
    let xyz_to_adobe_rgb = invert_matrix_3x3(&adobe_rgb_to_xyz);

    // Compose VSF → Adobe RGB transformation (VSF → XYZ → Adobe RGB)
    let vsf_to_adobe_rgb = convert_matrix_3x3(&xyz_to_adobe_rgb, &vsf_to_xyz);
    let adobe_rgb_to_vsf = invert_matrix_3x3(&vsf_to_adobe_rgb);

    println!();
    println!("// VSF → Adobe RGB composed transformation:");
    println!("//   VSF RGB → XYZ → Adobe RGB");

    // Write files
    write_spectral_constants(&vsf_to_lms, &lms_to_vsf, &lms_1nm)?;
    write_xyz_constants(&vsf_to_xyz, &xyz_to_vsf, &vsf_to_srgb, &srgb_to_vsf, &vsf_to_adobe_rgb, &adobe_rgb_to_vsf, &xyz_1nm)?;
    write_rec2020_constants(&vsf_to_rec2020, &rec2020_to_vsf)?;

    println!();
    println!("Generation complete!");

    Ok(())
}
