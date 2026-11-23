//! RAW clips (`vsf::visual::clip`): written as a recorder writes them (generation 0, frames, the closing generation), read back by the index, recovered without it, repaired, and every frame bit exact.

use vsf::file_format::VsfSection;
use vsf::types::{Tensor, VsfType};
use vsf::visual::cfa_band::{self, Cfa, Work};
use vsf::types::BitPackedTensor;
use vsf::visual::clip::{self, ClipCodec, ClipSpec, FrameMeta, Levels, Mosaic, Reader, Sum, Writer};
use vsf::visual::Capture;

fn lcg(s: &mut u64) -> u64 {
    *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); // the algorithm: an LCG wraps by definition
    *s >> 33
}

/// A frame of packed RAW10 rows on a stride: a level per colour, a ramp, and noise, so every predictor and k are exercised.
fn frame(w: usize, h: usize, stride: usize, seed: u64) -> (Vec<u8>, Vec<u16>) {
    let mut s = seed;
    let mut px = vec![0u16; w * h];
    for y in 0..h {
        for x in 0..w {
            let base = 64 + ((x & 1) << 6) as u64 + ((y & 1) << 5) as u64 + ((x + y) as u64 >> 2);
            let noise = lcg(&mut s) & 31;
            px[y * w + x] = ((base + noise) & 1023) as u16;
        }
    }
    let mut packed = vec![0xA5u8; stride * h];
    for y in 0..h {
        for g in 0..w / 4 {
            let v = &px[y * w + g * 4..y * w + g * 4 + 4];
            let o = y * stride + g * 5;
            for i in 0..4 {
                packed[o + i] = (v[i] >> 2) as u8;
            }
            packed[o + 4] = (0..4).map(|i| ((v[i] & 3) << (2 * i)) as u8).fold(0, |a, b| a | b);
        }
    }
    (packed, px)
}

fn spec(w: usize, h: usize) -> ClipSpec {
    ClipSpec {
        title: "test clip".into(),
        rate_num: 30,
        rate_den: 1,
        period_ns: 33_333_333,
        width: w,
        height: h,
        cfa: Tensor::new(vec![2, 2], vec![1, 2, 0, 1]),
        codec: ClipCodec::CfaBand { mosaic: Mosaic::Bayer, band_rows: 64 },
        channel_names: vec!["R".into(), "G".into(), "B".into()],
        black: vec![64.; 3],
        white: vec![1023.; 3],
        depth: 10,
        orientation: 1,
        capture: Capture { make: "test".into(), model: "clip".into(), mode: "video".into(), ..Capture::default() },
        characterization: None,
        observer: String::new(),
        at: Some(1 << 40),
    }
}

/// Record `n` frames into one byte vector as a recorder would; returns the bytes before the closing generation, the closing generation and the frames' samples.
fn record(sp: &ClipSpec, n: usize, seed: u64, skip: u64) -> (Vec<u8>, Vec<u8>, Vec<Vec<u16>>) {
    let (w, h) = (sp.width, sp.height);
    let stride = (w * 5 / 4 + 255) & !255;
    let (mut wr, head) = Writer::new(sp).unwrap();
    let mut file = head;
    let mut work = Box::new(Work::new());
    let mut samples = Vec::new();
    for k in 0..n {
        let (packed, px) = frame(w, h, stride, seed + k as u64);
        let mut payload = Vec::new();
        let mut bands = Vec::new();
        let mut digests = Vec::new();
        for b in 0..cfa_band::bands(h) {
            let rows = cfa_band::band_rows(h, b);
            let mut out = vec![0u8; cfa_band::max_band_len(w, rows)];
            let (la, lb) = cfa_band::encode_band(&packed[cfa_band::band_src(b, h, w, stride)], stride, w, rows, Cfa::Bayer, &mut work, &mut out);
            bands.push([la as u32, lb as u32]);
            digests.push(clip::band_digest(&out[..la], &out[la..la + lb]));
            payload.extend_from_slice(&out[..la + lb]);
        }
        let meta = FrameMeta { index: k as u64 * skip, at: (1 << 40) + k as i64 * 47_347_000, exposure_s: 1. / 128., gain: 2., focus: 300 + k as u32 };
        let p = wr.frame(&meta, &bands, &digests).unwrap();
        assert_eq!(p.payload_len, payload.len());
        assert_eq!(wr.frames().last().unwrap().offset, file.len() as u64);
        file.extend_from_slice(&p.head);
        file.extend_from_slice(&payload);
        file.extend_from_slice(&p.tail);
        assert_eq!(wr.end(), file.len() as u64);
        samples.push(px);
    }
    let closing = wr.close("test", "clip close").unwrap();
    (file, closing, samples)
}

fn check_frames<S: vsf::container::ByteSource>(r: &Reader<S>, samples: &[Vec<u16>], skip: u64) {
    assert_eq!(r.frames().len(), samples.len());
    for (k, px) in samples.iter().enumerate() {
        assert_eq!(r.frames()[k].meta.index, k as u64 * skip);
        assert_eq!(r.frames()[k].meta.focus, 300 + k as u32);
        assert_eq!(&r.decode(k).unwrap(), px, "frame {k}");
    }
}

#[test]
fn far_with_tail_is_encode_far() {
    let payload: Vec<u8> = (0..300u32).map(|i| (i * 7) as u8).collect();
    for len in [0usize, 1, 200, 300] {
        let mut s = VsfSection::new("frame");
        s.add_field("index", VsfType::u(3, false));
        s.add_field("prev", VsfType::hb(vec![9; 32]));
        let (head, tail) = s.encode_far_with_tail("samples", b'c', len);
        let mut whole = head;
        whole.extend_from_slice(&payload[..len]);
        whole.extend_from_slice(&tail);
        s.add_field("samples", VsfType::v(b'c', payload[..len].to_vec()));
        assert_eq!(whole, s.encode_far(), "payload of {len} bytes");
    }
}

#[test]
fn closed_clip_reads_by_its_index() {
    let sp = spec(256, 200);
    let (mut file, closing, samples) = record(&sp, 5, 7, 1);
    file.extend_from_slice(&closing);
    let r = Reader::open(file.as_slice()).unwrap();
    assert!(!r.recovered());
    assert_eq!(r.spec(), &sp);
    check_frames(&r, &samples, 1);
    // Every frame as a still's sensor plane.
    let p = r.frame_plane(2).unwrap();
    assert_eq!(p.packed_samples().unwrap().unpack_u16(), samples[2]);
    // The document as a whole is VSF: generation 0 then the closing generation, chained.
    let doc = vsf::container::Document::open(file.as_slice()).unwrap();
    assert_eq!(doc.generation(), 1);
    assert_eq!(doc.generations().unwrap().len(), 2);
    assert_eq!(doc.orphaned_bytes(), 0);
}

#[test]
fn cut_short_recovers_every_whole_frame_and_repairs() {
    let sp = spec(256, 200);
    let (file, _closing, samples) = record(&sp, 6, 11, 2);
    // Power gone after the frames, before the closing generation; the blocks past the clip hold an earlier recording's bytes.
    let (old, old_closing, _) = record(&sp, 9, 99, 1);
    let mut medium = file.clone();
    medium.extend_from_slice(&old[file.len()..]);
    medium.extend_from_slice(&old_closing);
    let r = Reader::open(medium.as_slice()).unwrap();
    assert!(r.recovered());
    assert_eq!(r.end(), file.len() as u64);
    check_frames(&r, &samples, 2);
    // Repaired: the closing generation appended where the frames end, the clip whole again.
    let repair = r.repair("test").unwrap();
    let mut fixed = file.clone();
    fixed.extend_from_slice(&repair);
    let r2 = Reader::open(fixed.as_slice()).unwrap();
    assert!(!r2.recovered());
    check_frames(&r2, &samples, 2);
    // Cut inside frame 3: frames 0 to 2 come back.
    let cut = r.frames()[3].offset as usize + 100;
    let r3 = Reader::open(&file[..cut]).unwrap();
    assert!(r3.recovered());
    check_frames(&r3, &samples[..3], 2);
    // A frame's samples damaged: the clip ends before it.
    let mut bad = file.clone();
    let at = r.frames()[4].offset as usize + r.frames()[4].size as usize - 50;
    bad[at] ^= 1;
    let r4 = Reader::open(bad.as_slice()).unwrap();
    check_frames(&r4, &samples[..4], 2);
}

/// A source that counts the bytes read from it.
struct Counted<'a>(&'a [u8], std::cell::Cell<u64>);

impl vsf::container::ByteSource for Counted<'_> {
    fn len(&self) -> u64 {
        self.0.len() as u64
    }
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), vsf::container::SourceError> {
        self.1.set(self.1.get() + buf.len() as u64);
        self.0.read_at(offset, buf)
    }
}

#[test]
fn a_clip_cut_short_in_a_large_region_reads_only_its_own_bytes() {
    // A writer died: its clip at the start of a region many times its size (the rest an earlier recording's bytes, or zeros), no trailer at the region's end.
    let sp = spec(256, 200);
    let (file, _closing, samples) = record(&sp, 4, 21, 1);
    let mut region = file.clone();
    region.resize(file.len() << 6, 0);
    let src = Counted(&region, std::cell::Cell::new(0));
    let r = Reader::open(&src).unwrap();
    assert!(r.recovered());
    check_frames(&r, &samples, 1);
    // Generation 0 and each frame once for the walk, again for the check above, and a window past the end: never the whole region.
    assert!(src.1.get() < (file.len() as u64) << 3, "read {} bytes of a {}-byte region for a {}-byte clip", src.1.get(), region.len(), file.len());
}

#[test]
fn a_recovery_by_headers_reads_the_frames_headers_and_checks_only_the_tail() {
    let sp = spec(256, 200);
    let (file, _closing, samples) = record(&sp, 32, 31, 1);
    let mut region = file.clone();
    region.resize(file.len() << 2, 0);
    // Intact: every frame back, its last two frames' samples read, the rest of the samples never.
    let src = Counted(&region, std::cell::Cell::new(0));
    let mut r = Reader::open_headers(&src).unwrap();
    assert!(r.recovered());
    assert_eq!(r.frames().len(), 32);
    let walked = src.1.get();
    assert_eq!(r.check_tail(2), 2);
    assert_eq!(r.end(), file.len() as u64);
    let one = r.frames()[31].size;
    // At most generation 0's read (2^17), a header's window (2^14 bytes) and its closing bytes a frame, and the window past the last: a Pro frame is 7 MB, these 36 KB.
    let most = (1 << 17) + 33 * ((1 << 14) + 2);
    assert!(walked <= most, "the header walk read {walked} bytes (at most {most}) of a {}-byte clip", file.len());
    assert!(src.1.get() - walked < 3 * one, "the tail check read {} bytes for two frames of {one}", src.1.get() - walked);
    check_frames(&r, &samples, 1);
    // The last frames torn (samples damaged in 30, cut inside 31's samples, its header whole): the clip ends before 30, the two before it proven.
    let mut torn = file.clone();
    let f30 = r.frames()[30].offset as usize + 200;
    torn[f30] ^= 1;
    let f31 = r.frames()[31].offset as usize + 200;
    torn[f31..file.len() - 2].fill(0);
    let r2 = Reader::open_tail_checked(torn.as_slice(), 2).unwrap();
    assert_eq!(r2.frames().len(), 30);
    assert_eq!(r2.end(), r.frames()[30].offset);
    check_frames(&r2, &samples[..30], 1);
    // The same answer as the full walk, which reads every sample.
    let full = Reader::open(torn.as_slice()).unwrap();
    assert_eq!(full.end(), r2.end());
    // Repaired at that end, the clip reads by its index.
    let mut fixed = torn[..r2.end() as usize].to_vec();
    fixed.extend_from_slice(&r2.repair("test").unwrap());
    let r3 = Reader::open(fixed.as_slice()).unwrap();
    assert!(!r3.recovered());
    check_frames(&r3, &samples[..30], 1);
}

#[test]
fn a_new_clip_over_an_old_one_stops_at_its_own_frames() {
    let sp = spec(128, 64);
    let (old, _, _) = record(&sp, 8, 5, 1);
    let (new, _, samples) = record(&sp, 3, 6, 1);
    let mut medium = new.clone();
    // The old clip's frames past the new one's end: same geometry, so they parse, but they name another chain.
    medium.extend_from_slice(&old[new.len()..]);
    let r = Reader::open(medium.as_slice()).unwrap();
    check_frames(&r, &samples, 1);
}

#[test]
fn real_frame_round_trips() {
    // The Pro main's binned frame, when this host has it (ferros target/cam).
    let path = "/mnt/Harbor/Code/ferros/target/cam/shot1.raw10";
    let Ok(raw) = std::fs::read(path) else {
        eprintln!("skipped: {path} missing");
        return;
    };
    let (w, h, stride) = (4080usize, 3072usize, 5120usize);
    let mut sp = spec(w, h);
    sp.cfa = Tensor::new(vec![2, 2], vec![1, 2, 0, 1]);
    let (mut wr, head) = Writer::new(&sp).unwrap();
    let mut file = head;
    let mut work = Box::new(Work::new());
    let mut payload = Vec::new();
    let (mut bands, mut digests) = (Vec::new(), Vec::new());
    for b in 0..cfa_band::bands(h) {
        let rows = cfa_band::band_rows(h, b);
        let mut out = vec![0u8; cfa_band::max_band_len(w, rows)];
        let (la, lb) = cfa_band::encode_band(&raw[cfa_band::band_src(b, h, w, stride)], stride, w, rows, Cfa::Bayer, &mut work, &mut out);
        bands.push([la as u32, lb as u32]);
        digests.push(clip::band_digest(&out[..la], &out[la..la + lb]));
        payload.extend_from_slice(&out[..la + lb]);
    }
    let p = wr.frame(&FrameMeta { index: 0, at: 1 << 40, exposure_s: 0.0688, gain: 1., focus: 0 }, &bands, &digests).unwrap();
    file.extend_from_slice(&p.head);
    file.extend_from_slice(&payload);
    file.extend_from_slice(&p.tail);
    file.extend_from_slice(&wr.close("test", "clip close").unwrap());
    let r = Reader::open(file.as_slice()).unwrap();
    let px = r.decode(0).unwrap();
    for y in 0..h {
        for g in 0..w / 4 {
            let o = y * stride + g * 5;
            for i in 0..4 {
                let v = ((raw[o + i] as u16) << 2) | ((raw[o + 4] >> (2 * i)) & 3) as u16;
                assert_eq!(px[y * w + g * 4 + i], v, "row {y} column {}", g * 4 + i);
            }
        }
    }
    eprintln!("real frame: {} bytes coded of {} packed, ratio {:.2}", payload.len(), w * h * 5 / 4, (w * h * 5 / 4) as f64 / payload.len() as f64);
}

#[test]
fn a_grade_appends_and_the_clip_still_reads_by_its_index() {
    use vsf::spectral_image::{IdtClass, ViewOp};
    let sp = spec(256, 200);
    let (mut file, closing, samples) = record(&sp, 4, 21, 1);
    file.extend_from_slice(&closing);
    let before = file.len();
    let ops = vec![ViewOp { name: "exposure".into(), class: IdtClass::Creative, params: vec![1.5] }];
    let doc = vsf::container::Document::open(file.as_slice()).unwrap();
    let grade = clip::edit_generation(&doc, "test", "grade", &ops).unwrap();
    drop(doc);
    file.extend_from_slice(&grade);
    let r = Reader::open(file.as_slice()).unwrap();
    assert!(!r.recovered());
    check_frames(&r, &samples, 1);
    // The grade reads back as the default view's ops; the frames' bytes never moved.
    let v = r.visual().unwrap();
    assert_eq!(v.default_graph().unwrap().to_view_transform().ops, ops);
    assert_eq!(v.title, "test clip");
    let doc = vsf::container::Document::open(file.as_slice()).unwrap();
    assert_eq!(doc.generation(), 2);
    assert_eq!(doc.generations().unwrap()[1].end, before as u64);
    // A second grade supersedes the first.
    let ops2 = vec![ViewOp { name: "exposure".into(), class: IdtClass::Creative, params: vec![-1.] }];
    let grade2 = clip::edit_generation(&doc, "test", "grade", &ops2).unwrap();
    drop(doc);
    file.extend_from_slice(&grade2);
    let r = Reader::open(file.as_slice()).unwrap();
    assert!(!r.recovered());
    assert_eq!(r.visual().unwrap().default_graph().unwrap().to_view_transform().ops, ops2);
    // A still's reader refuses the clip: it has no plane.
    assert!(vsf::visual::Visual::read(file.as_slice()).is_err());
}

#[test]
fn far_with_packed_tails_is_encode_far() {
    let (a, b) = (BitPackedTensor::pack_u64(34, vec![3, 5], &(0..15u64).map(|i| i << 30).collect::<Vec<_>>()), BitPackedTensor::pack_u16(11, vec![3, 5], &(0..15u16).map(|i| i * 100).collect::<Vec<_>>()));
    for two in [false, true] {
        let mut s = VsfSection::new("frame");
        s.add_field("index", VsfType::u(3, false));
        let mut tails = vec![("samples", a.bit_depth, a.shape.as_slice())];
        if two {
            tails.push(("motion", b.bit_depth, b.shape.as_slice()));
        }
        let pieces = s.encode_far_with_packed_tails(&tails);
        let mut whole = pieces[0].clone();
        whole.extend_from_slice(&a.data);
        whole.extend_from_slice(&pieces[1]);
        s.add_field("samples", VsfType::p(a.clone()));
        if two {
            whole.extend_from_slice(&b.data);
            whole.extend_from_slice(&pieces[2]);
            s.add_field("motion", VsfType::p(b.clone()));
        }
        assert_eq!(whole, s.encode_far(), "motion {two}");
    }
}

/// A frame of exact sums of `n` exposures: wide enough that a u16 would not hold them.
fn sums(w: usize, h: usize, n: u64, seed: u64) -> Vec<u64> {
    let mut s = seed;
    (0..w * h).map(|_| n * 64 + (lcg(&mut s) & 1023) * n / 2).collect()
}

fn packed_tensor(px: &[u64], w: usize, h: usize) -> BitPackedTensor {
    let peak = px.iter().copied().max().unwrap_or(0);
    BitPackedTensor::pack_u64((64 - peak.leading_zeros()).max(1) as u8, vec![h, w], px)
}

/// Record `n` packed frames, each summing more exposures than the last; returns the bytes before the closing generation, the closing generation, and each frame's sums and motion sums.
fn record_packed(sp: &ClipSpec, n: usize, seed: u64) -> (Vec<u8>, Vec<u8>, Vec<(Vec<u64>, Option<Vec<u64>>)>) {
    let (w, h) = (sp.width, sp.height);
    let motion = matches!(sp.codec, ClipCodec::Packed { motion: true });
    let (mut wr, head) = Writer::new(sp).unwrap();
    let mut file = head;
    let mut planes = Vec::new();
    for k in 0..n {
        let e = 1u64 << (8 * k as u32 + 3);
        let px = sums(w, h, e, seed + k as u64);
        let m = motion.then(|| sums(w, h, e >> 1, seed + 100 + k as u64));
        let (t, mt) = (packed_tensor(&px, w, h), m.as_ref().map(|m| packed_tensor(m, w, h)));
        let sum = Sum { exposures: e as u32, levels: Levels { black: vec![(e * 64) as f32; 3], white: vec![(e * 1023) as f32; 3] }, motion: motion.then(|| Levels { black: vec![0.; 3], white: vec![(e * 1023) as f32; 3] }) };
        let meta = FrameMeta { index: k as u64, at: (1 << 40) + k as i64 * (1 << 33), exposure_s: 1. / 64., gain: 1., focus: 300 + k as u32 };
        let pieces = wr.packed_frame(&meta, &sum, &t, mt.as_ref()).unwrap();
        assert_eq!(wr.frames().last().unwrap().offset, file.len() as u64);
        file.extend_from_slice(&pieces[0]);
        file.extend_from_slice(&t.data);
        file.extend_from_slice(&pieces[1]);
        if let Some(mt) = &mt {
            file.extend_from_slice(&mt.data);
            file.extend_from_slice(&pieces[2]);
        }
        assert_eq!(wr.end(), file.len() as u64);
        planes.push((px, m));
    }
    let closing = wr.close("test", "clip close").unwrap();
    (file, closing, planes)
}

fn packed_spec(w: usize, h: usize, motion: bool) -> ClipSpec {
    let mut sp = spec(w, h);
    sp.codec = ClipCodec::Packed { motion };
    sp.title = "time-lapse".into();
    sp
}

fn check_packed<S: vsf::container::ByteSource>(r: &Reader<S>, planes: &[(Vec<u64>, Option<Vec<u64>>)]) {
    assert_eq!(r.frames().len(), planes.len());
    for (k, (px, m)) in planes.iter().enumerate() {
        let e = 1u32 << (8 * k as u32 + 3);
        assert_eq!(r.frames()[k].exposures, e);
        let (sum, t, mt) = r.packed(k).unwrap();
        assert_eq!(sum.exposures, e);
        assert_eq!(sum.levels.white, vec![(e as u64 * 1023) as f32; 3]);
        assert_eq!(&t.unpack_u64(), px, "frame {k}");
        assert_eq!(mt.map(|t| t.unpack_u64()), *m, "frame {k} motion");
    }
}

#[test]
fn a_packed_clip_holds_exact_sums_at_each_frames_own_width() {
    for motion in [false, true] {
        let sp = packed_spec(96, 40, motion);
        let (mut file, closing, planes) = record_packed(&sp, 4, 3);
        file.extend_from_slice(&closing);
        let r = Reader::open(file.as_slice()).unwrap();
        assert!(!r.recovered());
        assert_eq!(r.spec(), &sp);
        check_packed(&r, &planes);
        // Each frame's width follows its sums: the last frame's past 32 bits.
        let widths: Vec<u8> = (0..4).map(|k| r.packed(k).unwrap().1.bit_depth).collect();
        assert!(widths.windows(2).all(|w| w[0] < w[1]) && widths[3] > 32, "{widths:?}");
        // As a still's planes: the sensor plane at the frame's own width and levels, the motion plane beside it.
        let (p, mp) = r.frame_planes(3).unwrap();
        assert_eq!(p.depth, widths[3]);
        assert_eq!(p.black, vec![((1u64 << 27) * 64) as f32; 3]);
        assert_eq!(p.packed_samples().unwrap().unpack_u64(), planes[3].0);
        assert_eq!(mp.is_some(), motion);
        if let Some(mp) = mp {
            assert_eq!(mp.role, vsf::visual::PlaneRole::Motion);
            assert_eq!(mp.packed_samples().unwrap().unpack_u64(), *planes[3].1.as_ref().unwrap());
        }
        // A cfa_band reading of a packed frame refuses.
        assert!(r.decode(0).is_err());
    }
}

#[test]
fn a_packed_clip_cut_short_recovers_and_repairs() {
    let sp = packed_spec(64, 32, true);
    let (file, _closing, planes) = record_packed(&sp, 4, 9);
    let r = Reader::open(file.as_slice()).unwrap();
    assert!(r.recovered());
    check_packed(&r, &planes);
    let mut fixed = file.clone();
    fixed.extend_from_slice(&r.repair("test").unwrap());
    let r2 = Reader::open(fixed.as_slice()).unwrap();
    assert!(!r2.recovered());
    check_packed(&r2, &planes);
    // Cut inside frame 2's motion plane: frames 0 and 1 come back.
    let cut = (r.frames()[2].offset + r.frames()[2].size) as usize - 10;
    let r3 = Reader::open(&file[..cut]).unwrap();
    check_packed(&r3, &planes[..2]);
    // A motion sample damaged: its frame fails its hash, the clip ends before it.
    let mut bad = file.clone();
    bad[(r.frames()[3].offset + r.frames()[3].size) as usize - 20] ^= 1;
    let r4 = Reader::open(bad.as_slice()).unwrap();
    check_packed(&r4, &planes[..3]);
    // Headers only, then the tail checked: the same ending.
    let r5 = Reader::open_tail_checked(bad.as_slice(), 2).unwrap();
    check_packed(&r5, &planes[..3]);
}

#[test]
fn a_packed_writer_refuses_what_its_clip_does_not_hold() {
    let sp = packed_spec(32, 16, false);
    let (mut wr, _) = Writer::new(&sp).unwrap();
    let meta = FrameMeta { index: 0, at: 1 << 40, exposure_s: 1., gain: 1., focus: 0 };
    let t = packed_tensor(&sums(32, 16, 4, 1), 32, 16);
    let levels = Levels { black: vec![256.; 3], white: vec![4092.; 3] };
    // A motion plane in a clip without one, a wrong shape, levels for the wrong channel count, a band frame.
    assert!(wr.packed_frame(&meta, &Sum { exposures: 4, levels: levels.clone(), motion: Some(levels.clone()) }, &t, Some(&t)).is_err());
    assert!(wr.packed_frame(&meta, &Sum { exposures: 4, levels: levels.clone(), motion: None }, &packed_tensor(&sums(16, 32, 4, 1), 16, 32), None).is_err());
    assert!(wr.packed_frame(&meta, &Sum { exposures: 4, levels: Levels { black: vec![256.], white: vec![4092.] }, motion: None }, &t, None).is_err());
    assert!(wr.frame(&meta, &[], &[]).is_err());
    assert!(wr.packed_frame(&meta, &Sum { exposures: 4, levels, motion: None }, &t, None).is_ok());
    // And a cfa_band clip takes no packed frame.
    let (mut wb, _) = Writer::new(&spec(64, 64)).unwrap();
    assert!(wb.packed_frame(&meta, &Sum { exposures: 1, levels: Levels { black: vec![64.; 3], white: vec![1023.; 3] }, motion: None }, &packed_tensor(&sums(64, 64, 1, 1), 64, 64), None).is_err());
}
