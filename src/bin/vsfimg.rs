//! vsfimg — convert any image (PNG / JPEG / WebP / TIFF) into a canonical VSF image file.
//!
//! Pipeline:
//! 1. Read source bytes → BLAKE3 (becomes provenance hash `hp`)
//! 2. Decode via `image` crate, extract ICC profile via `img-parts` (if present)
//! 3. Per pixel: ICC RGB (or sRGB fallback) → linear → matrix → linear VSF RGB
//! 4. Branch on output size:
//! - ≤ 256×256: gamma2-encode to u8, pack into `Tensor<u8>` shape `[h, w, 3]`, emit as `image:` section with field `data: t_u3(tensor)`
//! - > 256×256: gamma2-encode, rav1e AV1 encode (same params photon uses for avatars), emit as `image:` section with field `data: v(b'a', av1_bytes)` — the `v` (wrapped data) type's encoding-byte = `'a'` marks AV1
//! 5. `VsfBuilder` auto-computes rolling hash (`hb`); provenance hash (`hp`) is the BLAKE3 of the source bytes (not the VSF file — by design they hash different things, so `hp != hb` is expected and `hb` is required for VSF integrity)
//!
//! The conversion functions are lifted from photon/src/ui/avatar.rs verbatim where possible (ICC parsing, sRGB fallback, AV1 encode) so byte-for-byte equivalence is preserved across the ecosystem — same source image, same VSF bytes regardless of which tool produced them. Differences from photon's `encode_avatar_from_image`:
//! - no circular mask (vsfimg produces general images, not avatars)
//! - no forced resize to 256×256 (uncompressed/AV1 branch decides based on source dims)
//! - no center-crop to square (preserves source aspect)
//!
//! Usage: `vsfimg <input> <output>` — e.g. `vsfimg photo.png photo.vsf`

use std::fs;
use std::path::PathBuf;

use clap::Parser;
use image::DynamicImage;
use img_parts::jpeg::Jpeg;
use img_parts::png::Png;
use img_parts::ImageICC;
use rav1e::prelude::*;

use vsf::types::tensor::Tensor;
use vsf::types::VsfType;
use vsf::vsf_builder::VsfBuilder;

#[derive(Parser)]
#[command(name = "vsfimg")]
#[command(about = "Convert any image (PNG / JPEG / WebP / TIFF) → canonical VSF image", long_about = None)]
#[command(version)]
struct Cli {
    /// Input image file
    input: PathBuf,
    /// Output VSF file
    output: PathBuf,
}

/// Threshold (per axis) above which we switch from uncompressed Tensor<u8> to AV1. Matches photon's `AVATAR_SIZE` — photon avatars are exactly 256×256 AV1; vsfimg uses the same boundary so the format-selection rule is consistent across tools.
const UNCOMPRESSED_MAX_DIM: u32 = 256;

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(&cli) {
        eprintln!("vsfimg: {e}");
        std::process::exit(1);
    }
}

fn run(cli: &Cli) -> Result<(), String> {
    let source_bytes = fs::read(&cli.input)
        .map_err(|e| format!("reading {}: {e}", cli.input.display()))?;

    // Provenance hash: BLAKE3 of the source file's bytes. This is the immutable identity of "the image that produced this VSF" — distinct from the VSF file's own rolling hash. Both go in the header; consumers can use `hp` to dedupe / cite the source and `hb` to verify the VSF file hasn't been tampered with.
    let hp = blake3::hash(&source_bytes);

    let icc_profile_bytes = extract_icc_profile(&source_bytes)?;
    let icc_converter = match icc_profile_bytes {
        Some(profile) => Some(parse_icc_converter(&profile)?),
        None => None,
    };

    let img = image::load_from_memory(&source_bytes)
        .map_err(|e| format!("decoding source image: {e}"))?;
    let width = img.width();
    let height = img.height();

    // Convert source pixels to linear VSF RGB (f32). Whole image, no crop, no resize.
    let linear_vsf = decode_to_linear_vsf(&img, &icc_converter)?;

    // Format-selection rule: ≤ UNCOMPRESSED_MAX_DIM on BOTH axes → uncompressed tensor; otherwise → AV1. Square AV1 is what rav1e + photon's pipeline are tuned for, so for non-square sources we currently still hand them to rav1e at native dims — rav1e accepts arbitrary widths/heights (subject to its own minimums).
    let use_av1 = width > UNCOMPRESSED_MAX_DIM || height > UNCOMPRESSED_MAX_DIM;

    let pixels_field: VsfType = if use_av1 {
        let av1 = encode_av1(&linear_vsf, width as usize, height as usize)?;
        // Wrap the AV1 byte payload with encoding-byte 'a' (= AV1) via `v(u8, Vec<u8>)`. Consumers dispatch on the encoding byte to pick the right decoder.
        VsfType::v(b'a', av1)
    } else {
        // Gamma2-encode each linear channel into u8, pack interleaved [h, w, 3].
        let mut bytes = Vec::with_capacity((width as usize) * (height as usize) * 3);
        for &lin in &linear_vsf {
            // `.max(0.)` defends against negative linear values that out-of-gamut ICC matrices can produce — sqrt of negative would be NaN.
            bytes.push(vsf::colour::convert::delinearize_gamma2_u8_f32(lin.max(0.)));
        }
        let tensor = Tensor::new(vec![height as usize, width as usize, 3], bytes);
        VsfType::t_u3(tensor)
    };

    let vsf_bytes = VsfBuilder::new()
        .provenance_hash(*hp.as_bytes())
        .add_section("image", vec![("data".to_string(), pixels_field)])
        .build()
        .map_err(|e| format!("building VSF: {e}"))?;

    fs::write(&cli.output, &vsf_bytes)
        .map_err(|e| format!("writing {}: {e}", cli.output.display()))?;

    let kind = if use_av1 { "AV1" } else { "uncompressed" };
    eprintln!(
        "vsfimg: {} ({}×{}, {}) → {} ({} bytes)",
        cli.input.display(),
        width,
        height,
        kind,
        cli.output.display(),
        vsf_bytes.len(),
    );
    Ok(())
}

/// Decode a `DynamicImage` to linear VSF RGB f32, one channel-triple per pixel in row-major order. Uses the ICC converter when present; falls back to sRGB-assumption (the realistic default for the wild, where most images either have no profile or have a profile tagged as sRGB).
fn decode_to_linear_vsf(
    img: &DynamicImage,
    icc_converter: &Option<IccColourConverter>,
) -> Result<Vec<f32>, String> {
    let width = img.width() as usize;
    let height = img.height() as usize;
    let mut linear_vsf = vec![0.0f32; width * height * 3];

    match img {
        DynamicImage::ImageRgb16(_) | DynamicImage::ImageRgba16(_) => {
            // 16-bit source: /65536 normalize, route thru u16 conversion path.
            let rgb16_img = img.to_rgb16();
            let rgb_pixels = rgb16_img.as_raw();
            for i in 0..(width * height) {
                let src = i * 3;
                let r = rgb_pixels[src];
                let g = rgb_pixels[src + 1];
                let b = rgb_pixels[src + 2];
                let vsf = if let Some(converter) = icc_converter {
                    convert_pixel_linear_u16(r, g, b, converter)
                } else {
                    srgb_fallback_u16(r, g, b)
                };
                linear_vsf[src] = vsf[0];
                linear_vsf[src + 1] = vsf[1];
                linear_vsf[src + 2] = vsf[2];
            }
        }
        _ => {
            // 8-bit source (or downcast to 8-bit for other formats).
            let rgb_img = img.to_rgb8();
            let rgb_pixels = rgb_img.as_raw();
            for i in 0..(width * height) {
                let src = i * 3;
                let r = rgb_pixels[src];
                let g = rgb_pixels[src + 1];
                let b = rgb_pixels[src + 2];
                let vsf = if let Some(converter) = icc_converter {
                    convert_pixel_linear_u8(r, g, b, converter)
                } else {
                    srgb_fallback_u8(r, g, b)
                };
                linear_vsf[src] = vsf[0];
                linear_vsf[src + 1] = vsf[1];
                linear_vsf[src + 2] = vsf[2];
            }
        }
    }

    Ok(linear_vsf)
}

// ============================================================================
// ICC profile extraction + parsing (lifted from photon/src/ui/avatar.rs — verbatim where possible so byte-for-byte behavior is identical across the ecosystem)
// ============================================================================

/// Extract ICC profile bytes from an image file. Returns `None` if no profile present.
fn extract_icc_profile(image_data: &[u8]) -> Result<Option<Vec<u8>>, String> {
    if let Ok(jpeg) = Jpeg::from_bytes(image_data.to_vec().into()) {
        if let Some(icc) = jpeg.icc_profile() {
            return Ok(Some(icc.to_vec()));
        }
    }
    if let Ok(png) = Png::from_bytes(image_data.to_vec().into()) {
        if let Some(icc) = png.icc_profile() {
            return Ok(Some(icc.to_vec()));
        }
    }
    if let Some(icc) = extract_tiff_icc(image_data) {
        return Ok(Some(icc));
    }
    Ok(None)
}

/// TIFF stores ICC in tag 34675 (0x8773, InterColorProfile). The `image` crate doesn't expose this; parse the TIFF IFD manually.
fn extract_tiff_icc(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 8 {
        return None;
    }
    let (big_endian, magic) = match &data[0..4] {
        [b'I', b'I', 0x2A, 0x00] => (false, true),
        [b'M', b'M', 0x00, 0x2A] => (true, true),
        _ => (false, false),
    };
    if !magic {
        return None;
    }
    let read_u16 = |offset: usize| -> u16 {
        if big_endian {
            u16::from_be_bytes([data[offset], data[offset + 1]])
        } else {
            u16::from_le_bytes([data[offset], data[offset + 1]])
        }
    };
    let read_u32 = |offset: usize| -> u32 {
        if big_endian {
            u32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ])
        } else {
            u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ])
        }
    };
    let ifd_offset = read_u32(4) as usize;
    if ifd_offset + 2 > data.len() {
        return None;
    }
    let num_entries = read_u16(ifd_offset) as usize;
    let entries_start = ifd_offset + 2;
    for i in 0..num_entries {
        let entry_offset = entries_start + i * 12;
        if entry_offset + 12 > data.len() {
            break;
        }
        let tag = read_u16(entry_offset);
        if tag == 34675 {
            let count = read_u32(entry_offset + 4) as usize;
            let value_offset = read_u32(entry_offset + 8) as usize;
            if value_offset + count <= data.len() {
                return Some(data[value_offset..value_offset + count].to_vec());
            }
        }
    }
    None
}

/// Tone Reproduction Curve from an ICC profile — encodes the per-channel transfer function (gamma, LUT, parametric, or linear).
#[derive(Clone)]
enum TrcCurve {
    Linear,
    Gamma(f32),
    Lut(Vec<f32>),
    Parametric { function_type: u16, vals: Vec<f32> },
}

/// Pre-parsed ICC converter for fast per-pixel use. Stores the ICC RGB → XYZ matrix + the static XYZ → VSF RGB matrix, plus the three TRC curves.
struct IccColourConverter {
    icc_to_xyz: [f32; 9],
    xyz_to_vsf: [f32; 9],
    r_trc: TrcCurve,
    g_trc: TrcCurve,
    b_trc: TrcCurve,
}

fn parse_icc_converter(icc_profile: &[u8]) -> Result<IccColourConverter, String> {
    use icc_profile::{Data, DecodedICCProfile, ICCNumber};
    use vsf::colour::XYZ2VSF_RGB;

    let icc_vec = icc_profile.to_vec();
    let profile = DecodedICCProfile::new(&icc_vec)
        .map_err(|e| format!("parsing ICC profile: {e:?}"))?;

    let extract_xyz = |tag: &str| -> Result<[f32; 3], String> {
        match profile.tags.get(tag) {
            Some(Data::XYZNumber(xyz)) => Ok([xyz.x.as_f32(), xyz.y.as_f32(), xyz.z.as_f32()]),
            Some(Data::XYZNumberArray(arr)) if !arr.is_empty() => {
                let xyz = &arr[0];
                Ok([xyz.x.as_f32(), xyz.y.as_f32(), xyz.z.as_f32()])
            }
            _ => Err(format!("ICC profile missing {tag} tag")),
        }
    };

    let r_xyz = extract_xyz("rXYZ")?;
    let g_xyz = extract_xyz("gXYZ")?;
    let b_xyz = extract_xyz("bXYZ")?;

    let icc_to_xyz = [
        r_xyz[0], r_xyz[1], r_xyz[2], g_xyz[0], g_xyz[1], g_xyz[2], b_xyz[0], b_xyz[1], b_xyz[2],
    ];
    let xyz_to_vsf = XYZ2VSF_RGB;
    let r_trc = parse_trc_curve(profile.tags.get("rTRC"))?;
    let g_trc = parse_trc_curve(profile.tags.get("gTRC"))?;
    let b_trc = parse_trc_curve(profile.tags.get("bTRC"))?;

    Ok(IccColourConverter {
        icc_to_xyz,
        xyz_to_vsf,
        r_trc,
        g_trc,
        b_trc,
    })
}

fn parse_trc_curve(trc: Option<&icc_profile::Data>) -> Result<TrcCurve, String> {
    use icc_profile::{Data, ICCNumber};
    match trc {
        Some(Data::Curve(curve)) => {
            if curve.is_empty() {
                Ok(TrcCurve::Linear)
            } else if curve.len() == 1 {
                let gamma = curve[0] as f32 / 256.0;
                Ok(TrcCurve::Gamma(gamma))
            } else {
                let normalized: Vec<f32> = curve.iter().map(|&v| v as f32 / 65535.0).collect();
                Ok(TrcCurve::Lut(normalized))
            }
        }
        Some(Data::ParametricCurve(param)) => {
            let vals: Vec<f32> = param.vals.iter().map(|v| v.as_f32()).collect();
            Ok(TrcCurve::Parametric {
                function_type: param.funtion_type,
                vals,
            })
        }
        None => Ok(TrcCurve::Gamma(2.2)),
        _ => Err("Unsupported TRC type in ICC profile".to_string()),
    }
}

#[inline]
fn apply_trc_normalized(normalized: f32, trc: &TrcCurve) -> f32 {
    match trc {
        TrcCurve::Linear => normalized,
        TrcCurve::Gamma(gamma) => normalized.powf(*gamma),
        TrcCurve::Lut(lut) => {
            let index = normalized * (lut.len() - 1) as f32;
            let i0 = index.floor() as usize;
            let i1 = (i0 + 1).min(lut.len() - 1);
            let frac = index - i0 as f32;
            lut[i0] + (lut[i1] - lut[i0]) * frac
        }
        TrcCurve::Parametric { function_type, vals } => {
            // ICC parametric curve formulas (function type 0–4). Falls back to identity for unrecognized function types; gives the best-effort answer rather than erroring on exotic profiles.
            match function_type {
                0x0000 => normalized.powf(vals[0]),
                0x0001 => {
                    let (gamma, a, b) = (vals[0], vals[1], vals[2]);
                    if normalized >= -b / a {
                        (a * normalized + b).powf(gamma)
                    } else {
                        0.0
                    }
                }
                0x0002 => {
                    let (gamma, a, b, c) = (vals[0], vals[1], vals[2], vals[3]);
                    if normalized >= -b / a {
                        (a * normalized + b).powf(gamma) + c
                    } else {
                        c
                    }
                }
                0x0003 => {
                    let (gamma, a, b, c, d) = (vals[0], vals[1], vals[2], vals[3], vals[4]);
                    if normalized >= d {
                        (a * normalized + b).powf(gamma)
                    } else {
                        c * normalized
                    }
                }
                0x0004 => {
                    let (gamma, a, b, c, d, e, f) =
                        (vals[0], vals[1], vals[2], vals[3], vals[4], vals[5], vals[6]);
                    if normalized >= d {
                        (a * normalized + b).powf(gamma) + e
                    } else {
                        c * normalized + f
                    }
                }
                _ => normalized,
            }
        }
    }
}

#[inline]
fn convert_pixel_linear_u8(r: u8, g: u8, b: u8, converter: &IccColourConverter) -> [f32; 3] {
    use vsf::colour::convert::apply_matrix_3x3_f32;
    let r_lin = apply_trc_normalized(r as f32 / 255.0, &converter.r_trc);
    let g_lin = apply_trc_normalized(g as f32 / 255.0, &converter.g_trc);
    let b_lin = apply_trc_normalized(b as f32 / 255.0, &converter.b_trc);
    let xyz = apply_matrix_3x3_f32(&converter.icc_to_xyz, &[r_lin, g_lin, b_lin]);
    let vsf = apply_matrix_3x3_f32(&converter.xyz_to_vsf, &xyz);
    [vsf[0].max(0.), vsf[1].max(0.), vsf[2].max(0.)]
}

#[inline]
fn convert_pixel_linear_u16(r: u16, g: u16, b: u16, converter: &IccColourConverter) -> [f32; 3] {
    use vsf::colour::convert::apply_matrix_3x3_f32;
    let r_lin = apply_trc_normalized(r as f32 / 65536.0, &converter.r_trc);
    let g_lin = apply_trc_normalized(g as f32 / 65536.0, &converter.g_trc);
    let b_lin = apply_trc_normalized(b as f32 / 65536.0, &converter.b_trc);
    let xyz = apply_matrix_3x3_f32(&converter.icc_to_xyz, &[r_lin, g_lin, b_lin]);
    let vsf = apply_matrix_3x3_f32(&converter.xyz_to_vsf, &xyz);
    [vsf[0].max(0.), vsf[1].max(0.), vsf[2].max(0.)]
}

/// sRGB fallback when no ICC profile is present. Most "untagged" images in the wild are sRGB by convention, so this is the right default — but it's a guess, and that guess being wrong is exactly why ICC profiles exist. Using `#[allow(deprecated)]` because the sRGB helpers in vsf::colour::legacy are marked deprecated by design (they're for compat with legacy 1931-based standards, which is precisely what this fallback is for).
#[allow(deprecated)]
fn srgb_fallback_u8(r: u8, g: u8, b: u8) -> [f32; 3] {
    use vsf::colour::convert::apply_matrix_3x3_f32;
    use vsf::colour::legacy::convert::linearize_srgb_u8;
    use vsf::colour::SRGB2VSF_RGB;
    let r_lin = linearize_srgb_u8(r);
    let g_lin = linearize_srgb_u8(g);
    let b_lin = linearize_srgb_u8(b);
    apply_matrix_3x3_f32(&SRGB2VSF_RGB, &[r_lin, g_lin, b_lin])
}

#[allow(deprecated)]
fn srgb_fallback_u16(r: u16, g: u16, b: u16) -> [f32; 3] {
    use vsf::colour::convert::apply_matrix_3x3_f32;
    use vsf::colour::legacy::convert::linearize_srgb;
    use vsf::colour::SRGB2VSF_RGB;
    let r_lin = linearize_srgb(r as f32 / 65536.0);
    let g_lin = linearize_srgb(g as f32 / 65536.0);
    let b_lin = linearize_srgb(b as f32 / 65536.0);
    apply_matrix_3x3_f32(&SRGB2VSF_RGB, &[r_lin, g_lin, b_lin])
}

// ============================================================================
// AV1 encoding via rav1e (lifted from photon/src/ui/avatar.rs::encode_av1 — same encoder params, same VSF YCbCr math, so a 256×256-output photon avatar and an arbitrary-size vsfimg AV1 produce identical encodes for the same input)
// ============================================================================

/// Encode linear VSF RGB f32 → AV1 bytes. Input is `width * height * 3` f32 values in row-major order. Output is raw AV1 OBU bitstream.
///
/// VSF YCbCr derivation: Y  = (R + 2G + B) / 4 Cb = (B − Y) / 2 + 0.5 Cr = (R − Y) / 2 + 0.5 Chosen for VSF RGB primaries; differs from BT.601/BT.709 luma weights because those are tuned for the 1931-derived primaries which VSF rejects.
fn encode_av1(linear_vsf: &[f32], width: usize, height: usize) -> Result<Vec<u8>, String> {
    use vsf::colour::convert::delinearize_gamma2_f32 as delinearize_gamma2;

    // Gamma2-encode linear → display values before YCbCr (matches photon).
    let mut vsf_rgb = vec![0.0f32; width * height * 3];
    for i in 0..(width * height * 3) {
        vsf_rgb[i] = delinearize_gamma2(linear_vsf[i].max(0.0));
    }

    let enc_cfg = EncoderConfig {
        width,
        height,
        bit_depth: 8,
        chroma_sampling: ChromaSampling::Cs420,
        time_base: Rational::new(1, 1),
        low_latency: true,
        speed_settings: SpeedSettings::from_preset(6),
        quantizer: 32,
        min_quantizer: 0,
        ..Default::default()
    };
    let cfg = Config::new().with_encoder_config(enc_cfg);
    let mut ctx: Context<u8> = cfg
        .new_context()
        .map_err(|e| format!("creating rav1e context: {e}"))?;

    let mut frame = ctx.new_frame();

    // Y plane (full resolution).
    let mut y_plane = vec![0u8; width * height];
    for i in 0..(width * height) {
        let idx = i * 3;
        let r = vsf_rgb[idx];
        let g = vsf_rgb[idx + 1];
        let b = vsf_rgb[idx + 2];
        let y = (r + 2.0 * g + b) / 4.0;
        y_plane[i] = (y * 255.0).clamp(0.0, 255.0) as u8;
    }
    frame.planes[0].copy_from_raw_u8(&y_plane, width, 1);

    // Cb / Cr planes (4:2:0 — half resolution on each axis, 2×2 block average). Chroma dims round down; rav1e handles odd source dims by padding internally.
    let chroma_w = width / 2;
    let chroma_h = height / 2;
    let mut cb_plane = vec![128u8; chroma_w * chroma_h];
    let mut cr_plane = vec![128u8; chroma_w * chroma_h];
    for cy in 0..chroma_h {
        for cx in 0..chroma_w {
            let y0 = cy * 2;
            let x0 = cx * 2;
            let idx00 = (y0 * width + x0) * 3;
            let idx01 = (y0 * width + x0 + 1) * 3;
            let idx10 = ((y0 + 1) * width + x0) * 3;
            let idx11 = ((y0 + 1) * width + x0 + 1) * 3;
            let r = (vsf_rgb[idx00] + vsf_rgb[idx01] + vsf_rgb[idx10] + vsf_rgb[idx11]) / 4.0;
            let g = (vsf_rgb[idx00 + 1]
                + vsf_rgb[idx01 + 1]
                + vsf_rgb[idx10 + 1]
                + vsf_rgb[idx11 + 1])
                / 4.0;
            let b = (vsf_rgb[idx00 + 2]
                + vsf_rgb[idx01 + 2]
                + vsf_rgb[idx10 + 2]
                + vsf_rgb[idx11 + 2])
                / 4.0;
            let y = (r + 2.0 * g + b) / 4.0;
            let cb = (b - y) / 2.0 + 0.5;
            let cr = (r - y) / 2.0 + 0.5;
            cb_plane[cy * chroma_w + cx] = (cb * 255.0).clamp(0.0, 255.0) as u8;
            cr_plane[cy * chroma_w + cx] = (cr * 255.0).clamp(0.0, 255.0) as u8;
        }
    }
    frame.planes[1].copy_from_raw_u8(&cb_plane, chroma_w, 1);
    frame.planes[2].copy_from_raw_u8(&cr_plane, chroma_w, 1);

    ctx.send_frame(frame)
        .map_err(|e| format!("send_frame: {e}"))?;
    ctx.flush();

    let mut output = Vec::new();
    loop {
        match ctx.receive_packet() {
            Ok(packet) => output.extend_from_slice(&packet.data),
            Err(EncoderStatus::LimitReached) => break,
            Err(EncoderStatus::Encoded | EncoderStatus::NeedMoreData) => continue,
            Err(e) => return Err(format!("encoding: {e:?}")),
        }
    }
    if output.is_empty() {
        return Err("rav1e produced no output".to_string());
    }
    Ok(output)
}
