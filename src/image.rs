//! Decode the canonical VSF image format into an α + darkness pixel buffer — the
//! counterpart to the `vsfimg` encoder and [`crate::builders::compressed_image`].
//!
//! A VSF image file carries a section labelled `image` whose image field is either an
//! uncompressed `Tensor<u8>` (`t_u3`, shape `[h, w, 3]`, VSF RGB gamma2) for sources
//! ≤ 256×256, or a `v(b'a', av1_bytes)` AV1 wrapper for larger sources. Both land in the
//! same [`DecodedImage`] so callers don't care which path the file took. The field is
//! matched by *type*, not name, so both the `vsfimg`/orb convention (`data`) and the
//! [`crate::builders::compressed_image`] convention (`pixels`) decode.
//!
//! Output is packed α + darkness `u32` per the fluor/toka pixel convention: α = `0xFF`
//! (opaque — alpha shaping lives in the rasteriser's mask), darkness = `255 − visible_RGB`
//! so the buffer drops straight into an `under()` composite with no per-pixel inversion.
//! Both fluor's `Icon` and toka's `Canvas` use this exact convention, so one decoder feeds
//! both with no repack.
//!
//! VSF YCbCr inverse (matches `vsfimg`'s encoder math): `R = Y + 2(Cr − 0.5)`,
//! `B = Y + 2(Cb − 0.5)`, `G = (4Y − R − B) / 2`. Files are VSF RGB by definition — no
//! legacy colour tagging.

use rav1d::include::dav1d::data::Dav1dData;
use rav1d::include::dav1d::dav1d::{Dav1dContext, Dav1dSettings};
use rav1d::include::dav1d::picture::Dav1dPicture;
use rav1d::src::lib::{
    dav1d_close, dav1d_data_create, dav1d_default_settings, dav1d_get_picture, dav1d_open,
    dav1d_picture_unref, dav1d_send_data,
};
use std::ptr::NonNull;

use crate::decoding::parse::parse;
use crate::file_format::{VsfField, VsfHeader};
use crate::types::VsfType;

/// Decoded image ready to composite. Square or rectangular; the caller crops/masks at draw time.
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    /// Packed α + darkness pixels, row-major, `width * height` long. α byte = `0xFF`
    /// (opaque); RGB bytes = `255 − visible_RGB` (VSF RGB gamma2 darkness).
    pub pixels: Vec<u32>,
}

#[derive(Debug)]
pub enum ImageError {
    /// VSF parse or structural problem (header, section, field).
    Parse(String),
    /// File parsed but doesn't carry an `image` section.
    MissingImageSection,
    /// `image` section exists but carried no tensor/AV1 image field.
    MissingImageData,
    /// Image field carried a type this decoder doesn't handle (only `t_u3` tensor and
    /// `v(b'a', ...)` AV1 are supported).
    UnsupportedDataType,
    /// Tensor shape isn't `[h, w, 3]`.
    BadTensorShape(Vec<usize>),
    /// rav1d returned an error code or no picture for an AV1 payload.
    Av1Decode(String),
}

impl core::fmt::Display for ImageError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ImageError::Parse(s) => write!(f, "VSF parse: {}", s),
            ImageError::MissingImageSection => write!(f, "no 'image' section in file"),
            ImageError::MissingImageData => write!(f, "'image' section has no tensor/AV1 field"),
            ImageError::UnsupportedDataType => {
                write!(f, "image field has unsupported VSF type (expected t_u3 or v(b'a', ...))")
            }
            ImageError::BadTensorShape(s) => write!(f, "tensor shape {:?} not [h, w, 3]", s),
            ImageError::Av1Decode(s) => write!(f, "AV1 decode: {}", s),
        }
    }
}

impl std::error::Error for ImageError {}

/// Decode a full VSF image file — header + `image` section + tensor / AV1 payload — into a
/// ready-to-composite α + darkness buffer.
pub fn decode(data: &[u8]) -> Result<DecodedImage, ImageError> {
    let (header, _consumed) = VsfHeader::decode(data).map_err(ImageError::Parse)?;

    let img_field = header
        .fields
        .iter()
        .find(|f| f.name == "image")
        .ok_or(ImageError::MissingImageSection)?;

    let mut p = img_field.offset_bytes;
    if p >= data.len() {
        return Err(ImageError::Parse(format!(
            "image section offset {} beyond file length {}",
            p,
            data.len()
        )));
    }
    if data[p] == b'>' {
        p += 1;
    }
    if p >= data.len() || data[p] != b'[' {
        return Err(ImageError::Parse(format!(
            "expected '[' at section start, got byte {:02x}",
            data.get(p).copied().unwrap_or(0)
        )));
    }
    p += 1;

    if p < data.len() && data[p] != b'(' {
        let _name =
            parse(data, &mut p).map_err(|e| ImageError::Parse(format!("section name: {:?}", e)))?;
        let _count =
            parse(data, &mut p).map_err(|e| ImageError::Parse(format!("section n: {:?}", e)))?;
        let _length =
            parse(data, &mut p).map_err(|e| ImageError::Parse(format!("section b: {:?}", e)))?;
    }

    // Match the image payload by type, not field name: `vsfimg`/orb uses `data`,
    // `builders::compressed_image` uses `pixels`. Take the first tensor or AV1 wrapper.
    let mut image_value: Option<VsfType> = None;
    for _ in 0..img_field.child_count {
        let field = VsfField::parse(data, &mut p)
            .map_err(|e| ImageError::Parse(format!("field parse: {}", e)))?;
        for v in field.values {
            match v {
                VsfType::t_u3(_) => {
                    image_value = Some(v);
                    break;
                }
                VsfType::v(tag, _) if tag == b'a' => {
                    image_value = Some(v);
                    break;
                }
                _ => {}
            }
        }
        if image_value.is_some() {
            break;
        }
    }
    let image_value = image_value.ok_or(ImageError::MissingImageData)?;

    match image_value {
        VsfType::t_u3(tensor) => from_rgb_tensor(tensor.shape, tensor.data),
        VsfType::v(tag, av1_bytes) if tag == b'a' => from_av1(&av1_bytes),
        _ => Err(ImageError::UnsupportedDataType),
    }
}

/// Build directly from an uncompressed `[h, w, 3]` VSF RGB gamma2 buffer.
fn from_rgb_tensor(shape: Vec<usize>, bytes: Vec<u8>) -> Result<DecodedImage, ImageError> {
    if shape.len() != 3 || shape[2] != 3 {
        return Err(ImageError::BadTensorShape(shape));
    }
    let h = shape[0] as u32;
    let w = shape[1] as u32;
    let pixels = pack_alpha_darkness(&bytes, w, h);
    Ok(DecodedImage { width: w, height: h, pixels })
}

/// Decode an AV1 OBU bitstream → YCbCr → VSF RGB gamma2 → α + darkness. YCbCr inverse
/// mirrors `vsfimg`'s encoder math byte-for-byte. Runs single-threaded (`n_threads` left at
/// the default, but we only ever feed one frame) — safe for the OS-less wasm target where
/// thread spawning would panic.
fn from_av1(av1: &[u8]) -> Result<DecodedImage, ImageError> {
    let mut settings = std::mem::MaybeUninit::<Dav1dSettings>::uninit();
    unsafe { dav1d_default_settings(NonNull::new(settings.as_mut_ptr()).unwrap()) };
    let mut settings = unsafe { settings.assume_init() };
    // Single-threaded, single-frame: the OS-less wasm target can't spawn threads (default
    // auto-threading would panic at runtime), and an avatar/icon is one keyframe — no worker
    // pool or frame-delay buffering needed. Also keeps native decode instant for small images.
    settings.n_threads = 1;
    settings.max_frame_delay = 1;

    let mut ctx: Option<Dav1dContext> = None;
    let open = unsafe {
        dav1d_open(
            NonNull::new(&mut ctx as *mut _),
            NonNull::new(&settings as *const _ as *mut _),
        )
    };
    if open.0 < 0 {
        return Err(ImageError::Av1Decode(format!("dav1d_open: {}", open.0)));
    }
    let ctx = ctx.ok_or_else(|| ImageError::Av1Decode("dav1d_open returned null context".into()))?;

    let mut d = Dav1dData::default();
    let data_ptr = unsafe { dav1d_data_create(NonNull::new(&mut d), av1.len()) };
    if data_ptr.is_null() {
        unsafe { dav1d_close(NonNull::new(&mut Some(ctx) as *mut _)) };
        return Err(ImageError::Av1Decode("dav1d_data_create returned null".into()));
    }
    unsafe { std::ptr::copy_nonoverlapping(av1.as_ptr(), data_ptr, av1.len()) };

    loop {
        let r = unsafe { dav1d_send_data(Some(ctx), NonNull::new(&mut d)) };
        if r.0 == 0 {
            break;
        } else if r.0 == -11 {
            continue;
        } else if r.0 < 0 {
            unsafe { dav1d_close(NonNull::new(&mut Some(ctx) as *mut _)) };
            return Err(ImageError::Av1Decode(format!("dav1d_send_data: {}", r.0)));
        }
    }

    let mut pic = Dav1dPicture::default();
    loop {
        let r = unsafe { dav1d_get_picture(Some(ctx), NonNull::new(&mut pic)) };
        if r.0 == 0 {
            break;
        } else if r.0 == -11 {
            std::thread::yield_now();
            continue;
        } else {
            unsafe { dav1d_close(NonNull::new(&mut Some(ctx) as *mut _)) };
            return Err(ImageError::Av1Decode(format!("dav1d_get_picture: {}", r.0)));
        }
    }

    let width = pic.p.w as usize;
    let height = pic.p.h as usize;
    let stride_y = pic.stride[0] as usize;
    let stride_uv = pic.stride[1] as usize;
    let y_ptr = pic.data[0]
        .ok_or_else(|| ImageError::Av1Decode("missing Y plane".into()))?
        .as_ptr() as *const u8;
    let u_ptr = pic.data[1]
        .ok_or_else(|| ImageError::Av1Decode("missing U plane".into()))?
        .as_ptr() as *const u8;
    let v_ptr = pic.data[2]
        .ok_or_else(|| ImageError::Av1Decode("missing V plane".into()))?
        .as_ptr() as *const u8;

    let mut pixels = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            let y_val = unsafe { *y_ptr.add(y * stride_y + x) } as f32 / 255.0;
            let u_val = unsafe { *u_ptr.add((y / 2) * stride_uv + (x / 2)) } as f32 / 255.0;
            let v_val = unsafe { *v_ptr.add((y / 2) * stride_uv + (x / 2)) } as f32 / 255.0;
            let cb = u_val - 0.5;
            let cr = v_val - 0.5;
            let r = y_val + 2.0 * cr;
            let b = y_val + 2.0 * cb;
            let g = (4.0 * y_val - r - b) / 2.0;
            // f32 → u8 saturates (< 0 → 0, > 255 → 255, NaN → 0), so out-of-gamut maps cleanly.
            let dr = (255.0 - r * 255.0) as u8 as u32;
            let dg = (255.0 - g * 255.0) as u8 as u32;
            let db = (255.0 - b * 255.0) as u8 as u32;
            pixels.push(0xFF000000 | (dr << 16) | (dg << 8) | db);
        }
    }

    unsafe {
        dav1d_picture_unref(NonNull::new(&mut pic));
        dav1d_close(NonNull::new(&mut Some(ctx) as *mut _));
    }

    Ok(DecodedImage { width: width as u32, height: height as u32, pixels })
}

/// Pack VSF RGB gamma2 u8 triples into α + darkness u32: α=0xFF, darkness = 255 − visible.
/// The buffer length must be `w * h * 3` (caller guarantees from a verified `[h, w, 3]` shape).
fn pack_alpha_darkness(rgb: &[u8], w: u32, h: u32) -> Vec<u32> {
    let count = (w as usize) * (h as usize);
    let mut pixels = Vec::with_capacity(count);
    for chunk in rgb.chunks_exact(3) {
        let dr = (255 - chunk[0]) as u32;
        let dg = (255 - chunk[1]) as u32;
        let db = (255 - chunk[2]) as u32;
        pixels.push(0xFF000000 | (dr << 16) | (dg << 8) | db);
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_alpha_darkness_inverts_rgb_and_sets_alpha_opaque() {
        let rgb = vec![0u8, 0, 0, 255, 255, 255, 128, 64, 32];
        let p = pack_alpha_darkness(&rgb, 3, 1);
        assert_eq!(p[0], 0xFFFFFFFF);
        assert_eq!(p[1], 0xFF000000);
        assert_eq!(p[2], 0xFF000000 | (127u32 << 16) | (191u32 << 8) | 223u32);
    }
}
