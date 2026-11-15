//! AV1 planes: a plane's samples wrapped under codec `a` ([`super::CODEC_AV1`], VSF's code for AV1 in `VsfType::v`), written with rav1e and read with rav1d.
//!
//! The stream is one key frame of exactly the plane's geometry and depth, and it carries nothing a reader needs that the plane does not already say: `width`, `height`, `depth`, `channel_count`, `channel_names` and `layout` give the picture, `encoding` and `primaries` what its codes mean (an 8-bit `g2t` plane in VSF RGB is Lumis's VSF 8B). The bitstream's own colour description is left unspecified, since VSF's transform is not one of AV1's, and its range is full.
//!
//! How the channels travel:
//! - three planar channels (R, G, B) as VSF YCbCr 4:2:0, the house transform Photon's avatars and `vsfimg` already write and `vsf::image` reads: on codes normalised by the top code, Y = (R + 2G + B) / 4, Cb = (B - Y) / 2 + 1/2, Cr = (R - Y) / 2 + 1/2, the chroma of each 2 x 2 block from its mean colour; the inverse is R = Y + 2(Cr - 1/2), B = Y + 2(Cb - 1/2), G = (4Y - R - B) / 2;
//! - one channel as AV1 monochrome (4:0:0), the codes as Y.
//!
//! Depth 8 or 10: AV1's main profile, which every decoder handles. The transform is exact in integers apart from the halving of Cb and Cr, so even a lossless stream comes back within a code or two; the loss is the quantizer's.
//!
//! Features: `av1-decode` (rav1d) reads, `av1-encode` (rav1e) writes, `av1` is both. `av1-threads` lets rav1e use the rayon pool, `av1-asm` rav1e's hand-written assembly (x86-64 and aarch64; NASM for x86-64). Without `av1-decode`, a wrapped plane reads as a clean error naming av1 (`Plane::packed_samples`).

#[cfg(any(feature = "av1-encode", feature = "av1-decode"))]
use super::{bad, VisualError};
#[cfg(any(feature = "av1-encode", feature = "av1-decode"))]
use crate::prelude::*;

/// How hard rav1e works and how much it keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Av1Settings {
    /// rav1e's base quantizer, 0 (finest) to 255: higher is smaller and coarser.
    pub quantizer: u8,
    /// rav1e's speed preset, 0 (slowest, smallest) to 10 (fastest).
    pub speed: u8,
    /// Tiles to split the frame into, coded in parallel with `av1-threads` (0: rav1e's choice, the fewest the level allows).
    pub tiles: usize,
    /// Encoder threads with `av1-threads` (0: rayon's global pool).
    pub threads: usize,
}

impl Av1Settings {
    /// Photon's avatar settings: quantizer 32, speed 6.
    pub const AVATAR: Av1Settings = Av1Settings { quantizer: 32, speed: 6, tiles: 0, threads: 0 };
    /// A camera still: about half a megabyte for a 12.5 megapixel 8-bit g2t frame (Lumis's VSF 8B, the owner's target, 2026-11-15), speed 10 so a device's cores save it in seconds, sixteen tiles for them to share.
    pub const STILL: Av1Settings = Av1Settings { quantizer: 96, speed: 10, tiles: 1 << 4, threads: 0 };
}

impl Default for Av1Settings {
    fn default() -> Self {
        Av1Settings::STILL
    }
}

#[cfg(any(feature = "av1-encode", feature = "av1-decode"))]
/// The geometry a stream can have, from the plane's own fields: the sample count `channels * width * height`.
fn check_shape(channels: usize, width: usize, height: usize, depth: u8) -> Result<usize, VisualError> {
    if channels != 1 && channels != 3 {
        return Err(bad(format!("av1: a plane of {channels} channels; AV1 carries 1 (monochrome) or 3 (R, G, B as VSF YCbCr 4:2:0)")));
    }
    if depth != 8 && depth != 10 {
        return Err(bad(format!("av1: a {depth}-bit plane; AV1's main profile carries 8 or 10 bits")));
    }
    // WHY/PROOF: AV1 codes each dimension in 16 bits (1 to 65536); rav1e refuses outside that, and a zero dimension has no picture to code.
    if width == 0 || height == 0 || width > 1 << 16 || height > 1 << 16 {
        return Err(bad(format!("av1: a {width} x {height} plane; AV1 frames are 1 to 65536 on a side")));
    }
    // WHY/PROOF: the dimensions are read from a file, and on a 32-bit target (Android's armeabi-v7a) 3 x 65536 x 65536 overflows usize; rav1d's frame size limit is a u32 of pixels besides.
    let pixels = (width as u64) * (height as u64);
    if pixels > u32::MAX as u64 || (channels as u64) * pixels > usize::MAX as u64 {
        return Err(bad(format!("av1: a {width} x {height} plane is too large for this target")));
    }
    Ok(channels * width * height)
}

/// A caller's samples against the geometry they claim.
#[cfg(feature = "av1-encode")]
fn check_samples(channels: usize, width: usize, height: usize, depth: u8, samples: usize) -> Result<(), VisualError> {
    let want = check_shape(channels, width, height, depth)?;
    if samples != want {
        return Err(bad(format!("av1: {samples} samples for {channels} x {width} x {height}")));
    }
    Ok(())
}

#[cfg(feature = "av1-encode")]
mod encode {
    use super::*;
    use rav1e::prelude::*;

    /// Planar codes `[channels, height, width]` at `depth` bits (8 or 10) to an AV1 stream.
    pub fn encode(samples: &[u16], channels: usize, width: usize, height: usize, depth: u8, settings: &Av1Settings) -> Result<Vec<u8>, VisualError> {
        check_samples(channels, width, height, depth, samples.len())?;
        // WHY/PROOF: a code past the top is a caller's bug that would otherwise come back as another colour (the 8-bit copy into rav1e keeps only the low byte); refused by name instead.
        if let Some(c) = samples.iter().find(|&&c| c >> depth != 0) {
            return Err(bad(format!("av1: code {c} does not fit {depth} bits")));
        }
        encode_with(channels, width, height, depth, settings, |i| samples[i] as u32)
    }

    /// [`encode`] for 8-bit codes held as bytes, the VSF 8B case.
    pub fn encode_u8(samples: &[u8], channels: usize, width: usize, height: usize, settings: &Av1Settings) -> Result<Vec<u8>, VisualError> {
        check_samples(channels, width, height, 8, samples.len())?;
        encode_with(channels, width, height, 8, settings, |i| samples[i] as u32)
    }

    /// Y (and Cb, Cr) planes from the planar codes, then rav1e at the plane's depth.
    fn encode_with(channels: usize, width: usize, height: usize, depth: u8, settings: &Av1Settings, code: impl Fn(usize) -> u32) -> Result<Vec<u8>, VisualError> {
        let n = width * height;
        let top = ((1u32 << depth) - 1) as f32;
        let at = |ch: usize, i: usize| code(ch * n + i) as f32 / top;
        let q = |v: f32| (v * top + 0.5) as u16;
        let (cw, chh) = ((width + 1) / 2, (height + 1) / 2);
        let mut y_plane = vec![0u16; n];
        let mut cb_plane = Vec::new();
        let mut cr_plane = Vec::new();
        if channels == 1 {
            for i in 0..n {
                y_plane[i] = q(at(0, i));
            }
        } else {
            for i in 0..n {
                y_plane[i] = q((at(0, i) + 2. * at(1, i) + at(2, i)) / 4.);
            }
            cb_plane = vec![0u16; cw * chh];
            cr_plane = vec![0u16; cw * chh];
            for cy in 0..chh {
                for cx in 0..cw {
                    // WHY/PROOF: an odd width or height leaves a last block one pixel wide or tall; the min repeats that edge pixel instead of reading the next row (or past the plane), as AV1's own 4:2:0 subsampling of an odd frame does.
                    let (x0, y0) = (2 * cx, 2 * cy);
                    let (x1, y1) = ((x0 + 1).min(width - 1), (y0 + 1).min(height - 1));
                    let px = [y0 * width + x0, y0 * width + x1, y1 * width + x0, y1 * width + x1];
                    let mean = |ch: usize| px.iter().map(|&i| at(ch, i)).sum::<f32>() / 4.;
                    let (r, g, b) = (mean(0), mean(1), mean(2));
                    let y = (r + 2. * g + b) / 4.;
                    cb_plane[cy * cw + cx] = q((b - y) / 2. + 0.5);
                    cr_plane[cy * cw + cx] = q((r - y) / 2. + 0.5);
                }
            }
        }
        let cfg = EncoderConfig {
            width,
            height,
            bit_depth: depth as usize,
            chroma_sampling: if channels == 1 { ChromaSampling::Cs400 } else { ChromaSampling::Cs420 },
            pixel_range: PixelRange::Full,
            still_picture: true,
            time_base: Rational::new(1, 1),
            low_latency: true,
            speed_settings: SpeedSettings::from_preset(settings.speed),
            quantizer: settings.quantizer as usize,
            min_quantizer: 0,
            tiles: settings.tiles,
            ..Default::default()
        };
        let cfg = Config::new().with_encoder_config(cfg).with_threads(settings.threads);
        let planes = [(y_plane, width), (cb_plane, cw), (cr_plane, cw)];
        if depth == 8 {
            run::<u8>(&cfg, &planes)
        } else {
            run::<u16>(&cfg, &planes)
        }
    }

    /// One key frame thru rav1e: the planes copied in at the pixel's width, every packet collected.
    fn run<T: Pixel>(cfg: &Config, planes: &[(Vec<u16>, usize); 3]) -> Result<Vec<u8>, VisualError> {
        let mut ctx: Context<T> = cfg.new_context().map_err(|e| bad(format!("av1: rav1e context: {e}")))?;
        let mut frame = ctx.new_frame();
        let bytewidth = core::mem::size_of::<T>();
        for (p, (codes, stride)) in planes.iter().enumerate() {
            if codes.is_empty() {
                continue;
            }
            let raw: Vec<u8> = if bytewidth == 1 { codes.iter().map(|&c| c as u8).collect() } else { codes.iter().flat_map(|c| c.to_le_bytes()).collect() };
            frame.planes[p].copy_from_raw_u8(&raw, stride * bytewidth, bytewidth);
        }
        ctx.send_frame(frame).map_err(|e| bad(format!("av1: send_frame: {e}")))?;
        ctx.flush();
        let mut out = Vec::new();
        loop {
            match ctx.receive_packet() {
                Ok(packet) => out.extend_from_slice(&packet.data),
                Err(EncoderStatus::LimitReached) => break,
                Err(EncoderStatus::Encoded | EncoderStatus::NeedMoreData) => continue,
                Err(e) => return Err(bad(format!("av1: encoding: {e:?}"))),
            }
        }
        if out.is_empty() {
            return Err(bad("av1: rav1e produced no output"));
        }
        Ok(out)
    }
}

#[cfg(feature = "av1-encode")]
pub use encode::{encode, encode_u8};

#[cfg(feature = "av1-decode")]
mod decode {
    use super::*;
    use core::ptr::NonNull;
    use rav1d::include::dav1d::data::Dav1dData;
    use rav1d::include::dav1d::dav1d::{Dav1dContext, Dav1dSettings};
    use rav1d::include::dav1d::headers::{DAV1D_PIXEL_LAYOUT_I400, DAV1D_PIXEL_LAYOUT_I420};
    use rav1d::include::dav1d::picture::Dav1dPicture;
    use rav1d::src::lib::{dav1d_close, dav1d_data_create, dav1d_data_unref, dav1d_default_settings, dav1d_get_picture, dav1d_open, dav1d_picture_unref, dav1d_send_data};

    /// An AV1 stream back to planar codes `[channels, height, width]` at `depth` bits: the geometry the plane declares, which the picture must match exactly.
    pub fn decode(stream: &[u8], channels: usize, width: usize, height: usize, depth: u8) -> Result<Vec<u16>, VisualError> {
        check_shape(channels, width, height, depth)?;
        // check_shape held width x height within a u32.
        with_picture(stream, (width * height) as u32, |pic| read_picture(pic, channels, width, height, depth))
    }

    /// The planes of a decoded picture, checked against the plane's geometry, thru the inverse transform.
    fn read_picture(pic: &Dav1dPicture, channels: usize, width: usize, height: usize, depth: u8) -> Result<Vec<u16>, VisualError> {
        let layout = if channels == 1 { DAV1D_PIXEL_LAYOUT_I400 } else { DAV1D_PIXEL_LAYOUT_I420 };
        if pic.p.w as usize != width || pic.p.h as usize != height || pic.p.bpc != depth as i32 || pic.p.layout != layout {
            return Err(bad(format!("av1: the stream holds a {} x {} {}-bit picture in layout {}; the plane says {width} x {height} {depth}-bit, {}", pic.p.w, pic.p.h, pic.p.bpc, pic.p.layout, if channels == 1 { "monochrome" } else { "4:2:0" })));
        }
        let plane_ptr = |p: usize| pic.data[p].map(|d| d.as_ptr() as *const u8).ok_or_else(|| bad("av1: a plane missing from the picture"));
        let (stride_y, stride_c) = (pic.stride[0] as usize, pic.stride[1] as usize);
        let y_ptr = plane_ptr(0)?;
        // SAFETY: rav1d's picture holds `h` rows of `stride` bytes for Y, and (h + 1) / 2 rows for each chroma plane of a 4:2:0 picture, each row at least `w` (or (w + 1) / 2) samples of `bpc` bits; x < width and y < height were checked equal to the picture's size above.
        let sample = |ptr: *const u8, stride: usize, x: usize, y: usize| -> i32 {
            unsafe {
                if depth == 8 {
                    *ptr.add(y * stride + x) as i32
                } else {
                    (ptr.add(y * stride + 2 * x) as *const u16).read_unaligned() as i32
                }
            }
        };
        let n = width * height;
        let top = (1i32 << depth) - 1;
        let mut out = vec![0u16; channels * n];
        if channels == 1 {
            for y in 0..height {
                for x in 0..width {
                    out[y * width + x] = sample(y_ptr, stride_y, x, y) as u16;
                }
            }
            return Ok(out);
        }
        let (u_ptr, v_ptr) = (plane_ptr(1)?, plane_ptr(2)?);
        for y in 0..height {
            for x in 0..width {
                let yv = sample(y_ptr, stride_y, x, y);
                let cb = sample(u_ptr, stride_c, x / 2, y / 2);
                let cr = sample(v_ptr, stride_c, x / 2, y / 2);
                // In codes: R = Y + 2Cr - top, B = Y + 2Cb - top, G = 2Y - (R + B) / 2.
                let r = yv + 2 * cr - top;
                let b = yv + 2 * cb - top;
                let g = 2 * yv - (r + b + 1).div_euclid(2);
                // WHY/PROOF: a lossy stream reconstructs colours a little outside the cube (a saturated edge after 4:2:0), and a code past the top would not fit the plane's depth when packed (its high bits dropped, white wrapping to black): the clamp holds each code in 0 to 2^depth - 1.
                let i = y * width + x;
                out[i] = r.clamp(0, top) as u16;
                out[n + i] = g.clamp(0, top) as u16;
                out[2 * n + i] = b.clamp(0, top) as u16;
            }
        }
        Ok(out)
    }

    /// Send the stream to rav1d, take its one picture and hand it to `read` while the decoder is still open, then release both. The frame size limit is the plane's own, so a stream naming a larger frame is refused before its planes are reserved.
    fn with_picture<R>(stream: &[u8], max_pixels: u32, read: impl FnOnce(&Dav1dPicture) -> Result<R, VisualError>) -> Result<R, VisualError> {
        let mut pic = Dav1dPicture::default();
        let mut settings = core::mem::MaybeUninit::<Dav1dSettings>::uninit();
        unsafe { dav1d_default_settings(NonNull::new(settings.as_mut_ptr()).unwrap()) };
        let mut settings = unsafe { settings.assume_init() };
        // Every core on a device with threads (0: rav1d's choice); one on wasm32, which cannot spawn them. One frame, so no frame delay.
        settings.n_threads = if cfg!(target_arch = "wasm32") { 1 } else { 0 };
        settings.max_frame_delay = 1;
        settings.frame_size_limit = max_pixels;
        let mut ctx: Option<Dav1dContext> = None;
        let open = unsafe { dav1d_open(NonNull::new(&mut ctx as *mut _), NonNull::new(&settings as *const _ as *mut _)) };
        if open.0 < 0 {
            return Err(bad(format!("av1: dav1d_open: {}", open.0)));
        }
        let ctx = ctx.ok_or_else(|| bad("av1: dav1d_open gave no context"))?;
        let mut d = Dav1dData::default();
        let data_ptr = unsafe { dav1d_data_create(NonNull::new(&mut d), stream.len()) };
        if data_ptr.is_null() {
            unsafe { dav1d_close(NonNull::new(&mut Some(ctx) as *mut _)) };
            return Err(bad("av1: dav1d_data_create failed"));
        }
        unsafe { core::ptr::copy_nonoverlapping(stream.as_ptr(), data_ptr, stream.len()) };
        // dav1d answers with a negated errno; EAGAIN is 35 on the BSD family and 11 elsewhere, and rav1d passes the host's value thru.
        #[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
        const EAGAIN: i32 = -35;
        #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd")))]
        const EAGAIN: i32 = -11;
        // Send/drain rounds before giving up: a one-picture stream needs two or three; a stream that never yields one ends here instead of spinning (vsf::image's loop, security audit VSF-12).
        const MAX_ROUNDS: u32 = 1 << 6;
        let mut outcome: Result<(), VisualError> = Err(bad("av1: no picture in the stream"));
        for _ in 0..MAX_ROUNDS {
            if d.sz > 0 {
                let sent = unsafe { dav1d_send_data(Some(ctx), NonNull::new(&mut d)) };
                if sent.0 < 0 && sent.0 != EAGAIN {
                    outcome = Err(bad(format!("av1: dav1d_send_data: {} (a broken stream, or a frame larger than the plane)", sent.0)));
                    break;
                }
            }
            let got = unsafe { dav1d_get_picture(Some(ctx), NonNull::new(&mut pic)) };
            if got.0 == 0 {
                outcome = Ok(());
                break;
            }
            if got.0 != EAGAIN {
                outcome = Err(bad(format!("av1: dav1d_get_picture: {}", got.0)));
                break;
            }
            if d.sz == 0 {
                break;
            }
        }
        let out = outcome.and_then(|()| {
            let r = read(&pic);
            unsafe { dav1d_picture_unref(NonNull::new(&mut pic)) };
            r
        });
        unsafe {
            dav1d_data_unref(NonNull::new(&mut d));
            dav1d_close(NonNull::new(&mut Some(ctx) as *mut _));
        }
        out
    }
}

#[cfg(feature = "av1-decode")]
pub use decode::decode;
