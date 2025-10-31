//! Derive VSF RGB colorspace matrices from LMS2006 Standard Observer
//!
//! This tool calculates the transformation matrices for VSF's spectral-based RGB colorspace.
//!
//! Primary wavelengths:
//! - R = 703nm (L-cone dominance, pure red perception)
//! - G = 523nm (M-cone peak, clean green perception)
//! - B = 462nm (S-cone peak, minimal L/M interference)
//!
//! White point: E (equal energy) - not D65
//!
//! Run with: cargo run --bin derive_vsf_rgb

const WAVELENGTH_START: usize = 350; // nm
const WAVELENGTH_STEP: usize = 1; // nm

use num_traits::Float;
use std::ops::{Add, Div, Mul, Sub};

// Include the actual LMS2006SO data and constants from colour_constants.rs
include!("../src/colour_constants.rs");

// 3x3 matrix inversion (generic over all float types)
fn invert_3x3<T>(m: &[T; 9]) -> [T; 9]
where
    T: Float + Add<Output = T> + Sub<Output = T> + Mul<Output = T> + Div<Output = T>,
{
    let d = T::one()
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

// 3x3 matrix multiplication (generic over all float types)
fn matrix_multiply_3x3<T>(a: &[T; 9], b: &[T; 9]) -> [T; 9]
where
    T: Float + Add<Output = T> + Mul<Output = T>,
{
    [
        a[0] * b[0] + a[1] * b[3] + a[2] * b[6],
        a[0] * b[1] + a[1] * b[4] + a[2] * b[7],
        a[0] * b[2] + a[1] * b[5] + a[2] * b[8],
        a[3] * b[0] + a[4] * b[3] + a[5] * b[6],
        a[3] * b[1] + a[4] * b[4] + a[5] * b[7],
        a[3] * b[2] + a[4] * b[5] + a[5] * b[8],
        a[6] * b[0] + a[7] * b[3] + a[8] * b[6],
        a[6] * b[1] + a[7] * b[4] + a[8] * b[7],
        a[6] * b[2] + a[7] * b[5] + a[8] * b[8],
    ]
}

fn main() {
    println!("=== VSF RGB Colorspace Derivation ===\n");
    println!("Primary wavelengths:");
    println!("  R = 703nm (L-cone dominance)");
    println!("  G = 523nm (M-cone peak)");
    println!("  B = 462nm (S-cone peak)");
    println!("  White point = E (equal energy)\n");

    // Calculate indices into LMS2006SO array
    let wavelength_to_index = |wl: usize| -> usize {
        let idx = (wl - WAVELENGTH_START) / WAVELENGTH_STEP;
        idx * 3 // Each wavelength has 3 values (L, M, S)
    };

    let idx_462 = wavelength_to_index(462);
    let idx_523 = wavelength_to_index(523);
    let idx_703 = wavelength_to_index(703);

    // Extract LMS values at primary wavelengths (use f64 for precision)
    let b_lms = [
        (LMS2006SO[idx_462] * LMS2006SO_SCALE) as f64,
        (LMS2006SO[idx_462 + 1] * LMS2006SO_SCALE) as f64,
        (LMS2006SO[idx_462 + 2] * LMS2006SO_SCALE) as f64,
    ];

    let g_lms = [
        (LMS2006SO[idx_523] * LMS2006SO_SCALE) as f64,
        (LMS2006SO[idx_523 + 1] * LMS2006SO_SCALE) as f64,
        (LMS2006SO[idx_523 + 2] * LMS2006SO_SCALE) as f64,
    ];

    let r_lms = [
        (LMS2006SO[idx_703] * LMS2006SO_SCALE) as f64,
        (LMS2006SO[idx_703 + 1] * LMS2006SO_SCALE) as f64,
        (LMS2006SO[idx_703 + 2] * LMS2006SO_SCALE) as f64,
    ];

    println!("LMS values at primary wavelengths:");
    println!(
        "  462nm (B): L={:.10e}, M={:.10e}, S={:.10e}",
        b_lms[0], b_lms[1], b_lms[2]
    );
    println!(
        "  523nm (G): L={:.10e}, M={:.10e}, S={:.10e}",
        g_lms[0], g_lms[1], g_lms[2]
    );
    println!(
        "  703nm (R): L={:.10e}, M={:.10e}, S={:.10e}",
        r_lms[0], r_lms[1], r_lms[2]
    );
    println!();

    // E white point is [1, 1, 1] by definition (equal energy)
    println!("E White Point: L=1.0, M=1.0, S=1.0 (by definition)\n");

    // Normalize each primary to chromaticity coordinates (l+m+s=1 for each)
    // For each primary: divide L, M, S by (L+M+S) for that primary
    let sum_r = r_lms[0] + r_lms[1] + r_lms[2];
    let sum_g = g_lms[0] + g_lms[1] + g_lms[2];
    let sum_b = b_lms[0] + b_lms[1] + b_lms[2];

    println!(
        "Raw primary totals: R={:.10e}, G={:.10e}, B={:.10e}",
        sum_r, sum_g, sum_b
    );

    // Chromaticity normalization: L/(L+M+S), M/(L+M+S), S/(L+M+S) for each primary
    let r_lms_scaled = [r_lms[0] / sum_r, r_lms[1] / sum_r, r_lms[2] / sum_r];

    let g_lms_scaled = [g_lms[0] / sum_g, g_lms[1] / sum_g, g_lms[2] / sum_g];

    let b_lms_scaled = [b_lms[0] / sum_b, b_lms[1] / sum_b, b_lms[2] / sum_b];

    println!("\nNormalized primaries (chromaticity coordinates, l+m+s=1 for each):");
    println!(
        "  462nm (B): L={:.10e}, M={:.10e}, S={:.10e}",
        b_lms_scaled[0], b_lms_scaled[1], b_lms_scaled[2]
    );
    println!(
        "  523nm (G): L={:.10e}, M={:.10e}, S={:.10e}",
        g_lms_scaled[0], g_lms_scaled[1], g_lms_scaled[2]
    );
    println!(
        "  703nm (R): L={:.10e}, M={:.10e}, S={:.10e}",
        r_lms_scaled[0], r_lms_scaled[1], r_lms_scaled[2]
    );
    println!();

    // Verify sum
    let verify_l = r_lms_scaled[0] + g_lms_scaled[0] + b_lms_scaled[0];
    let verify_m = r_lms_scaled[1] + g_lms_scaled[1] + b_lms_scaled[1];
    let verify_s = r_lms_scaled[2] + g_lms_scaled[2] + b_lms_scaled[2];
    println!(
        "Verification - sums: L={:.10e}, M={:.10e}, S={:.10e}",
        verify_l, verify_m, verify_s
    );
    println!();

    // Build VSF_RGB2LMS matrix directly from the scaled primaries!
    // Since both VSF RGB and LMS use E white point, no chromatic adaptation needed
    // Each ROW is: L/M/S = r*(primary_r_lms) + g*(primary_g_lms) + b*(primary_b_lms)
    // The scaled values already have the normalization built in (they sum to [1,1,1])
    let vsf_rgb2lms = [
        r_lms_scaled[0],
        g_lms_scaled[0],
        b_lms_scaled[0], // L = r*0.00631 + g*0.90 + b*0.0937
        r_lms_scaled[1],
        g_lms_scaled[1],
        b_lms_scaled[1], // M = r*0.000297 + g*0.8716 + b*0.1281
        r_lms_scaled[2],
        g_lms_scaled[2],
        b_lms_scaled[2], // S = r*7.3e-9 + g*0.0289 + b*0.9711
    ];

    println!("VSF_RGB2LMS matrix (VSF RGB → LMS):");
    println!(
        "  [{:.15}, {:.15}, {:.15},",
        vsf_rgb2lms[0], vsf_rgb2lms[1], vsf_rgb2lms[2]
    );
    println!(
        "   {:.15}, {:.15}, {:.15},",
        vsf_rgb2lms[3], vsf_rgb2lms[4], vsf_rgb2lms[5]
    );
    println!(
        "   {:.15}, {:.15}, {:.15}]",
        vsf_rgb2lms[6], vsf_rgb2lms[7], vsf_rgb2lms[8]
    );
    println!();

    // Invert to get LMS2VSF_RGB
    let lms2vsf_rgb = invert_3x3(&vsf_rgb2lms);

    println!("LMS2VSF_RGB matrix (LMS → VSF RGB):");
    println!(
        "  [{:.15e}, {:.15e}, {:.15e},",
        lms2vsf_rgb[0], lms2vsf_rgb[1], lms2vsf_rgb[2]
    );
    println!(
        "   {:.15e}, {:.15e}, {:.15e},",
        lms2vsf_rgb[3], lms2vsf_rgb[4], lms2vsf_rgb[5]
    );
    println!(
        "   {:.15e}, {:.15e}, {:.15e}]",
        lms2vsf_rgb[6], lms2vsf_rgb[7], lms2vsf_rgb[8]
    );
    println!();

    // Verify the inverse is correct: forward × inverse = identity
    let identity_check = matrix_multiply_3x3(&vsf_rgb2lms, &lms2vsf_rgb);
    println!("Verification: VSF_RGB2LMS × LMS2VSF_RGB =");
    println!(
        "  [{:.15}, {:.15}, {:.15},",
        identity_check[0], identity_check[1], identity_check[2]
    );
    println!(
        "   {:.15}, {:.15}, {:.15},",
        identity_check[3], identity_check[4], identity_check[5]
    );
    println!(
        "   {:.15}, {:.15}, {:.15}]",
        identity_check[6], identity_check[7], identity_check[8]
    );

    let max_error = identity_check
        .iter()
        .enumerate()
        .map(|(i, &val)| {
            let expected = if i % 4 == 0 { 1.0 } else { 0.0 };
            (val - expected).abs()
        })
        .fold(0.0f64, f64::max);
    println!("  Max error from identity: {:.15e}", max_error);
    println!();

    // Check if the current LMS2VSF_RGB in the file is correct or transposed
    let current_lms2vsf: [f64; 9] = [
        LMS2VSF_RGB[0] as f64,
        LMS2VSF_RGB[1] as f64,
        LMS2VSF_RGB[2] as f64,
        LMS2VSF_RGB[3] as f64,
        LMS2VSF_RGB[4] as f64,
        LMS2VSF_RGB[5] as f64,
        LMS2VSF_RGB[6] as f64,
        LMS2VSF_RGB[7] as f64,
        LMS2VSF_RGB[8] as f64,
    ];

    let test_current = matrix_multiply_3x3(&vsf_rgb2lms, &current_lms2vsf);
    let current_error = test_current
        .iter()
        .enumerate()
        .map(|(i, &val)| {
            let expected = if i % 4 == 0 { 1.0 } else { 0.0 };
            (val - expected).abs()
        })
        .fold(0.0f64, f64::max);

    println!("Testing current LMS2VSF_RGB from colour_constants.rs:");
    println!("  Max error from identity: {:.15e}", current_error);

    if current_error > 1e-6 {
        // Try transpose
        let transposed: [f64; 9] = [
            current_lms2vsf[0],
            current_lms2vsf[3],
            current_lms2vsf[6],
            current_lms2vsf[1],
            current_lms2vsf[4],
            current_lms2vsf[7],
            current_lms2vsf[2],
            current_lms2vsf[5],
            current_lms2vsf[8],
        ];
        let test_transposed = matrix_multiply_3x3(&vsf_rgb2lms, &transposed);
        let transpose_error = test_transposed
            .iter()
            .enumerate()
            .map(|(i, &val)| {
                let expected = if i % 4 == 0 { 1.0 } else { 0.0 };
                (val - expected).abs()
            })
            .fold(0.0f64, f64::max);

        println!("  Trying transpose of current matrix:");
        println!("    Max error from identity: {:.15e}", transpose_error);

        if transpose_error < 1e-6 {
            println!("  *** WARNING: Current LMS2VSF_RGB appears to be TRANSPOSED! ***");
        } else {
            println!("  *** WARNING: Current LMS2VSF_RGB is INCORRECT (neither original nor transpose works) ***");
        }
    } else {
        println!("  Current LMS2VSF_RGB is correct!");
    }
    println!();

    // Get LMS2XYZ matrix from colour_constants (convert to f64)
    let lms2xyz: [f64; 9] = [
        LMS2XYZ[0] as f64,
        LMS2XYZ[1] as f64,
        LMS2XYZ[2] as f64,
        LMS2XYZ[3] as f64,
        LMS2XYZ[4] as f64,
        LMS2XYZ[5] as f64,
        LMS2XYZ[6] as f64,
        LMS2XYZ[7] as f64,
        LMS2XYZ[8] as f64,
    ];

    // Calculate VSF_RGB2XYZ = LMS2XYZ * VSF_RGB2LMS
    let vsf_rgb2xyz = matrix_multiply_3x3(&lms2xyz, &vsf_rgb2lms);

    println!("VSF_RGB2XYZ matrix (VSF RGB → CIE XYZ):");
    println!(
        "  [{:.15e}, {:.15e}, {:.15e},",
        vsf_rgb2xyz[0], vsf_rgb2xyz[1], vsf_rgb2xyz[2]
    );
    println!(
        "   {:.15e}, {:.15e}, {:.15e},",
        vsf_rgb2xyz[3], vsf_rgb2xyz[4], vsf_rgb2xyz[5]
    );
    println!(
        "   {:.15e}, {:.15e}, {:.15e}]",
        vsf_rgb2xyz[6], vsf_rgb2xyz[7], vsf_rgb2xyz[8]
    );
    println!();

    // Invert to get XYZ2VSF_RGB
    let xyz2vsf_rgb = invert_3x3(&vsf_rgb2xyz);

    println!("XYZ2VSF_RGB matrix (CIE XYZ → VSF RGB):");
    println!(
        "  [{:.15e}, {:.15e}, {:.15e},",
        xyz2vsf_rgb[0], xyz2vsf_rgb[1], xyz2vsf_rgb[2]
    );
    println!(
        "   {:.15e}, {:.15e}, {:.15e},",
        xyz2vsf_rgb[3], xyz2vsf_rgb[4], xyz2vsf_rgb[5]
    );
    println!(
        "   {:.15e}, {:.15e}, {:.15e}]",
        xyz2vsf_rgb[6], xyz2vsf_rgb[7], xyz2vsf_rgb[8]
    );
    println!();

    // Calculate VSF_RGB ↔ REC709 conversions (convert to f64)
    let lms2rec709: [f64; 9] = [
        LMS2REC709[0] as f64,
        LMS2REC709[1] as f64,
        LMS2REC709[2] as f64,
        LMS2REC709[3] as f64,
        LMS2REC709[4] as f64,
        LMS2REC709[5] as f64,
        LMS2REC709[6] as f64,
        LMS2REC709[7] as f64,
        LMS2REC709[8] as f64,
    ];

    // VSF_RGB2REC709 = LMS2REC709 * VSF_RGB2LMS
    let vsf_rgb2rec709 = matrix_multiply_3x3(&lms2rec709, &vsf_rgb2lms);

    println!("VSF_RGB2REC709 matrix (VSF RGB → Rec.709/sRGB):");
    println!(
        "  [{:.15e}, {:.15e}, {:.15e},",
        vsf_rgb2rec709[0], vsf_rgb2rec709[1], vsf_rgb2rec709[2]
    );
    println!(
        "   {:.15e}, {:.15e}, {:.15e},",
        vsf_rgb2rec709[3], vsf_rgb2rec709[4], vsf_rgb2rec709[5]
    );
    println!(
        "   {:.15e}, {:.15e}, {:.15e}]",
        vsf_rgb2rec709[6], vsf_rgb2rec709[7], vsf_rgb2rec709[8]
    );
    println!();

    let rec7092vsf_rgb = invert_3x3(&vsf_rgb2rec709);

    println!("REC7092VSF_RGB matrix (Rec.709/sRGB → VSF RGB):");
    println!(
        "  [{:.15e}, {:.15e}, {:.15e},",
        rec7092vsf_rgb[0], rec7092vsf_rgb[1], rec7092vsf_rgb[2]
    );
    println!(
        "   {:.15e}, {:.15e}, {:.15e},",
        rec7092vsf_rgb[3], rec7092vsf_rgb[4], rec7092vsf_rgb[5]
    );
    println!(
        "   {:.15e}, {:.15e}, {:.15e}]",
        rec7092vsf_rgb[6], rec7092vsf_rgb[7], rec7092vsf_rgb[8]
    );
    println!();

    // Calculate VSF_RGB ↔ REC2020 conversions (convert to f64)
    let lms2rec2020: [f64; 9] = [
        LMS2REC2020[0] as f64,
        LMS2REC2020[1] as f64,
        LMS2REC2020[2] as f64,
        LMS2REC2020[3] as f64,
        LMS2REC2020[4] as f64,
        LMS2REC2020[5] as f64,
        LMS2REC2020[6] as f64,
        LMS2REC2020[7] as f64,
        LMS2REC2020[8] as f64,
    ];

    // VSF_RGB2REC2020 = LMS2REC2020 * VSF_RGB2LMS
    let vsf_rgb2rec2020 = matrix_multiply_3x3(&lms2rec2020, &vsf_rgb2lms);

    println!("VSF_RGB2REC2020 matrix (VSF RGB → Rec.2020/BT.2020):");
    println!(
        "  [{:.15e}, {:.15e}, {:.15e},",
        vsf_rgb2rec2020[0], vsf_rgb2rec2020[1], vsf_rgb2rec2020[2]
    );
    println!(
        "   {:.15e}, {:.15e}, {:.15e},",
        vsf_rgb2rec2020[3], vsf_rgb2rec2020[4], vsf_rgb2rec2020[5]
    );
    println!(
        "   {:.15e}, {:.15e}, {:.15e}]",
        vsf_rgb2rec2020[6], vsf_rgb2rec2020[7], vsf_rgb2rec2020[8]
    );
    println!();

    let rec20202vsf_rgb = invert_3x3(&vsf_rgb2rec2020);

    println!("REC20202VSF_RGB matrix (Rec.2020/BT.2020 → VSF RGB):");
    println!(
        "  [{:.15e}, {:.15e}, {:.15e},",
        rec20202vsf_rgb[0], rec20202vsf_rgb[1], rec20202vsf_rgb[2]
    );
    println!(
        "   {:.15e}, {:.15e}, {:.15e},",
        rec20202vsf_rgb[3], rec20202vsf_rgb[4], rec20202vsf_rgb[5]
    );
    println!(
        "   {:.15e}, {:.15e}, {:.15e}]",
        rec20202vsf_rgb[6], rec20202vsf_rgb[7], rec20202vsf_rgb[8]
    );
    println!();

    // Print ready-to-paste constants for colour_constants.rs
    println!("\n=== Copy-paste for colour_constants.rs ===\n");

    println!("// VSF RGB Colorspace Constants");
    println!("// Spectral primaries: R=703nm, G=523nm, B=462nm with E white point");
    println!(
        "static E_WHITE_LM: [f32; 2] = [{:.15e}, {:.15e}];",
        1.0,
        1.0 // E white normalized to [1, 1, 1] in LMS space
    );
    println!("static VSF_RGB_PRIMARIES: [f32; 3] = [462.0, 523.0, 703.0]; // nm\n");

    println!("pub static LMS2VSF_RGB: [f32; 9] = [");
    println!(
        "    {:.15e}, {:.15e}, {:.15e},",
        lms2vsf_rgb[0], lms2vsf_rgb[1], lms2vsf_rgb[2]
    );
    println!(
        "    {:.15e}, {:.15e}, {:.15e},",
        lms2vsf_rgb[3], lms2vsf_rgb[4], lms2vsf_rgb[5]
    );
    println!(
        "    {:.15e}, {:.15e}, {:.15e},",
        lms2vsf_rgb[6], lms2vsf_rgb[7], lms2vsf_rgb[8]
    );
    println!("];\n");

    println!("pub static VSF_RGB2LMS: [f32; 9] = [");
    println!(
        "    {:.15e}, {:.15e}, {:.15e},",
        vsf_rgb2lms[0], vsf_rgb2lms[1], vsf_rgb2lms[2]
    );
    println!(
        "    {:.15e}, {:.15e}, {:.15e},",
        vsf_rgb2lms[3], vsf_rgb2lms[4], vsf_rgb2lms[5]
    );
    println!(
        "    {:.15e}, {:.15e}, {:.15e},",
        vsf_rgb2lms[6], vsf_rgb2lms[7], vsf_rgb2lms[8]
    );
    println!("];\n");

    println!("pub static VSF_RGB2XYZ: [f32; 9] = [");
    println!(
        "    {:.15e}, {:.15e}, {:.15e},",
        vsf_rgb2xyz[0], vsf_rgb2xyz[1], vsf_rgb2xyz[2]
    );
    println!(
        "    {:.15e}, {:.15e}, {:.15e},",
        vsf_rgb2xyz[3], vsf_rgb2xyz[4], vsf_rgb2xyz[5]
    );
    println!(
        "    {:.15e}, {:.15e}, {:.15e},",
        vsf_rgb2xyz[6], vsf_rgb2xyz[7], vsf_rgb2xyz[8]
    );
    println!("];\n");

    println!("pub static XYZ2VSF_RGB: [f32; 9] = [");
    println!(
        "    {:.15e}, {:.15e}, {:.15e},",
        xyz2vsf_rgb[0], xyz2vsf_rgb[1], xyz2vsf_rgb[2]
    );
    println!(
        "    {:.15e}, {:.15e}, {:.15e},",
        xyz2vsf_rgb[3], xyz2vsf_rgb[4], xyz2vsf_rgb[5]
    );
    println!(
        "    {:.15e}, {:.15e}, {:.15e},",
        xyz2vsf_rgb[6], xyz2vsf_rgb[7], xyz2vsf_rgb[8]
    );
    println!("];\n");

    println!("pub static VSF_RGB2REC709: [f32; 9] = [");
    println!(
        "    {:.15e}, {:.15e}, {:.15e},",
        vsf_rgb2rec709[0], vsf_rgb2rec709[1], vsf_rgb2rec709[2]
    );
    println!(
        "    {:.15e}, {:.15e}, {:.15e},",
        vsf_rgb2rec709[3], vsf_rgb2rec709[4], vsf_rgb2rec709[5]
    );
    println!(
        "    {:.15e}, {:.15e}, {:.15e},",
        vsf_rgb2rec709[6], vsf_rgb2rec709[7], vsf_rgb2rec709[8]
    );
    println!("];\n");

    println!("pub static REC7092VSF_RGB: [f32; 9] = [");
    println!(
        "    {:.15e}, {:.15e}, {:.15e},",
        rec7092vsf_rgb[0], rec7092vsf_rgb[1], rec7092vsf_rgb[2]
    );
    println!(
        "    {:.15e}, {:.15e}, {:.15e},",
        rec7092vsf_rgb[3], rec7092vsf_rgb[4], rec7092vsf_rgb[5]
    );
    println!(
        "    {:.15e}, {:.15e}, {:.15e},",
        rec7092vsf_rgb[6], rec7092vsf_rgb[7], rec7092vsf_rgb[8]
    );
    println!("];\n");

    println!("pub static VSF_RGB2REC2020: [f32; 9] = [");
    println!(
        "    {:.15e}, {:.15e}, {:.15e},",
        vsf_rgb2rec2020[0], vsf_rgb2rec2020[1], vsf_rgb2rec2020[2]
    );
    println!(
        "    {:.15e}, {:.15e}, {:.15e},",
        vsf_rgb2rec2020[3], vsf_rgb2rec2020[4], vsf_rgb2rec2020[5]
    );
    println!(
        "    {:.15e}, {:.15e}, {:.15e},",
        vsf_rgb2rec2020[6], vsf_rgb2rec2020[7], vsf_rgb2rec2020[8]
    );
    println!("];\n");

    println!("pub static REC20202VSF_RGB: [f32; 9] = [");
    println!(
        "    {:.15e}, {:.15e}, {:.15e},",
        rec20202vsf_rgb[0], rec20202vsf_rgb[1], rec20202vsf_rgb[2]
    );
    println!(
        "    {:.15e}, {:.15e}, {:.15e},",
        rec20202vsf_rgb[3], rec20202vsf_rgb[4], rec20202vsf_rgb[5]
    );
    println!(
        "    {:.15e}, {:.15e}, {:.15e},",
        rec20202vsf_rgb[6], rec20202vsf_rgb[7], rec20202vsf_rgb[8]
    );
    println!("];");
}
