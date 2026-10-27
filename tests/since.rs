//! Every type's bytes, pinned next to the version `types::since` gives it. A file's `y` is only as honest as that table, and the table is only as honest as this test: change how a type flattens and the test fails until that type's version rises with it.
//!
//! - `cargo test --test since` — compares every sample with `tests/fixtures/since.txt`.
//! - `VSF_BLESS=1 cargo test --test since` — rewrites the fixture, adding new samples and accepting changed bytes only where the type's version rose. A changed type still at its old version is refused: an old reader would misparse it while the file told it to go ahead.
//! - `VSF_BLESS=force` — accepts every change. Only for bytes no released reader has ever seen (a type added since the last publish) or a change old readers decode identically.
//!
//! Run once without and once with `--features spirix` (and `text-encode` for `x`): a configuration blesses only the samples it can build and keeps the rest of the fixture as it was.

use std::collections::BTreeMap;
use vsf::types::{BitPackedTensor, EtType, StridedTensor, Tensor, VsfType, WaAddress, WorldCell, WorldCoord, SINCE_TABLE};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/since.txt");

/// One tensor per wire form: 1-D (the compact `tn` form) and 2-D for contiguous, 2-D for strided.
macro_rules! tensors {
    ($out:ident; $($t:ident $q:ident [$($x:expr),+]);* $(;)?) => {$(
        let data = vec![$($x),+];
        $out.push((concat!(stringify!($t), "/1d").into(), VsfType::$t(Tensor::new(vec![data.len()], data.clone()))));
        $out.push((concat!(stringify!($t), "/2d").into(), VsfType::$t(Tensor::new(vec![2, 2], data.clone()))));
        $out.push((stringify!($q).into(), VsfType::$q(StridedTensor::new(vec![2, 2], vec![2, 1], data))));
    )*};
}

fn samples() -> Vec<(String, VsfType)> {
    let mut s: Vec<(String, VsfType)> = Vec::new();
    let mut add = |label: &str, v: VsfType| s.push((label.to_string(), v));

    // Structure and header
    add("d", VsfType::d("octopus".into()));
    add("o", VsfType::o(1234));
    add("b", VsfType::b(70000, false));
    add("b/inclusive", VsfType::b(70000, true));
    add("l", VsfType::l(300, false));
    add("l/inclusive", VsfType::l(300, true));
    add("n", VsfType::n(7));
    add("z", VsfType::z(10));
    add("y", VsfType::y(9));

    // Network
    add("na", VsfType::na(0, "example.org".into(), Some(8443)));
    add("na/noport", VsfType::na(1, "example.org".into(), None));
    add("nh", VsfType::nh("example.org".into()));
    add("ni", VsfType::ni([192, 0, 2, 1]));
    add("nj", VsfType::nj([0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]));
    add("nc", VsfType::nc("10.0.0.0".into(), 8));
    add("nm", VsfType::nm([0x02, 0x00, 0x5e, 0x10, 0x00, 0x01]));
    add("np", VsfType::np(443));
    add("ns", VsfType::ns("example.org".into(), 443));
    add("nu", VsfType::nu("https://example.org/a".into()));
    add("nn", VsfType::nn("octopus".into()));

    // Hashes, signatures, keys, MACs: bytes behind a two-letter marker and a length (RSA `gr` included: deprecated for new writers, still read)
    let bytes = |n: usize| (0..n).map(|i| (i * 37 + 11) as u8).collect::<Vec<u8>>();
    #[allow(deprecated)]
    let crypto = [
        ("hp", VsfType::hp as fn(Vec<u8>) -> VsfType), ("hb", VsfType::hb), ("hs", VsfType::hs), ("hm", VsfType::hm), ("hg", VsfType::hg), ("hc", VsfType::hc), ("hk", VsfType::hk),
        ("hP", VsfType::hP), ("hR", VsfType::hR), ("hI", VsfType::hI), ("hV", VsfType::hV),
        ("ge", VsfType::ge), ("gm", VsfType::gm), ("gp", VsfType::gp), ("gd", VsfType::gd), ("gs", VsfType::gs), ("gf", VsfType::gf), ("gH", VsfType::gH), ("gr", VsfType::gr),
        ("ke", VsfType::ke), ("kx", VsfType::kx), ("kp", VsfType::kp), ("kk", VsfType::kk), ("kc", VsfType::kc), ("ka", VsfType::ka), ("km", VsfType::km), ("kf", VsfType::kf),
        ("kl", VsfType::kl), ("kn", VsfType::kn), ("kh", VsfType::kh), ("kd", VsfType::kd), ("kb", VsfType::kb),
        ("kvf", VsfType::kvf), ("kvs", VsfType::kvs), ("ksx", VsfType::ksx), ("ksp", VsfType::ksp), ("ksk", VsfType::ksk), ("ksf", VsfType::ksf), ("ksn", VsfType::ksn),
        ("ksl", VsfType::ksl), ("ksh", VsfType::ksh), ("ksm", VsfType::ksm),
        ("mh", VsfType::mh), ("mp", VsfType::mp), ("mb", VsfType::mb), ("mc", VsfType::mc),
    ];
    for (label, make) in crypto {
        add(label, make(bytes(32)));
    }

    // Numbers
    add("u0", VsfType::u0(true));
    add("u", VsfType::u(300, false));
    add("u/wide", VsfType::u(5_000_000_000, false));
    add("u3", VsfType::u3(200));
    add("u4", VsfType::u4(60000));
    add("u5", VsfType::u5(4_000_000_000));
    add("u6", VsfType::u6(0x0123_4567_89ab_cdef));
    add("u7", VsfType::u7(0x0123_4567_89ab_cdef_fedc_ba98_7654_3210));
    add("i", VsfType::i(-300));
    add("i3", VsfType::i3(-100));
    add("i4", VsfType::i4(-30000));
    add("i5", VsfType::i5(-2_000_000_000));
    add("i6", VsfType::i6(-0x0123_4567_89ab_cdef));
    add("i7", VsfType::i7(-0x0123_4567_89ab_cdef_fedc_ba98_7654_3210));
    add("f5", VsfType::f5(-1.5));
    add("f6", VsfType::f6(std::f64::consts::PI));
    add("j5", VsfType::j5(num_complex::Complex::new(1.5, -2.5)));
    add("j6", VsfType::j6(num_complex::Complex::new(-0.25, 4.0)));

    // Text, time, place
    #[cfg(any(feature = "text", feature = "text-encode"))]
    add("x", VsfType::x("café octopus".into()));
    add("a", VsfType::a("octopus".into()));
    #[allow(deprecated)]
    for (label, et) in [("e/e5", EtType::e5(1_000_000)), ("e/e6", EtType::e6(-123_456_789_012)), ("e/e7", EtType::e7(1 << 100)), ("e/f5", EtType::f5(1.5e9)), ("e/f6", EtType::f6(-2.25e12))] {
        add(label, VsfType::e(et));
    }
    add("w", VsfType::w(WorldCoord::from_lat_lon(-36.8485, 174.7633)));
    add("wm", VsfType::wm(WorldCell::from_lat_lon(-36.8485, 174.7633)));
    add("wa", VsfType::wa(WaAddress::new().with_line1("1 Queen Street").with_city("Auckland")));

    // Wrapped data
    add("v", VsfType::v(b'b', bytes(40)));

    // Colour
    add("r", VsfType::r(3, 4, bytes(6)));
    for (label, c) in [
        ("rcb", VsfType::rcb), ("rcc", VsfType::rcc), ("rcg", VsfType::rcg), ("rcj", VsfType::rcj), ("rck", VsfType::rck), ("rcl", VsfType::rcl), ("rcn", VsfType::rcn),
        ("rco", VsfType::rco), ("rcq", VsfType::rcq), ("rcr", VsfType::rcr), ("rcv", VsfType::rcv), ("rcw", VsfType::rcw), ("rcy", VsfType::rcy),
    ] {
        add(label, c);
    }
    add("re", VsfType::re(200));
    add("rx", VsfType::rx(60000));
    add("rz", VsfType::rz(0.5));
    add("ri", VsfType::ri(100));
    add("rp", VsfType::rp(40000));
    add("ru", VsfType::ru([10, 20, 30]));
    add("rs", VsfType::rs([1000, 2000, 3000]));
    add("rf", VsfType::rf([0.25, 0.5, 0.75]));
    add("ra", VsfType::ra([10, 20, 30, 255]));
    add("rt", VsfType::rt([1000, 2000, 3000, 65535]));
    add("rh", VsfType::rh([0.25, 0.5, 0.75, 1.0]));
    add("rm", VsfType::rm(3, 3, vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0], 2.0));

    // Bit-packed tensors, opcodes
    add("p", VsfType::p(BitPackedTensor::pack_u16(12, vec![2, 3], &[0, 1, 4095, 7, 8, 9])));
    add("op", VsfType::op(b'p', b's'));

    tensors!(s;
        t_u0 q_u0 [true, false, true, true];
        t_u3 q_u3 [1u8, 2, 254, 255];
        t_u4 q_u4 [1u16, 2, 65534, 65535];
        t_u5 q_u5 [1u32, 2, 4_000_000_000, 7];
        t_u6 q_u6 [1u64, 2, u64::MAX, 7];
        t_u7 q_u7 [1u128, 2, u128::MAX, 7];
        t_i3 q_i3 [-1i8, 2, -128, 127];
        t_i4 q_i4 [-1i16, 2, -32768, 32767];
        t_i5 q_i5 [-1i32, 2, i32::MIN, i32::MAX];
        t_i6 q_i6 [-1i64, 2, i64::MIN, i64::MAX];
        t_i7 q_i7 [-1i128, 2, i128::MIN, i128::MAX];
        t_f5 q_f5 [-1.5f32, 0.25, 1e-7, 3.0e9];
        t_f6 q_f6 [-1.5f64, 0.25, 1e-300, 3.0e300];
        t_j5 q_j5 [num_complex::Complex::new(1.0f32, -1.0), num_complex::Complex::new(0.5, 0.25), num_complex::Complex::new(-2.0, 0.0), num_complex::Complex::new(0.0, 3.0)];
        t_j6 q_j6 [num_complex::Complex::new(1.0f64, -1.0), num_complex::Complex::new(0.5, 0.25), num_complex::Complex::new(-2.0, 0.0), num_complex::Complex::new(0.0, 3.0)];
    );

    #[cfg(feature = "spirix")]
    spirix_samples(&mut s);
    s
}

#[cfg(feature = "spirix")]
fn spirix_samples(s: &mut Vec<(String, VsfType)>) {
    use spirix::*;
    use vsf::types::{ButtonVariant, Fill, GradientStop, GradientVariant, PathCommand, SplineType, Stroke, StrokeCap, StrokeJoin, TextStyle, Transform};

    let sc = |x: f64| ScalarF4E4::from(x);
    let ci = |r: f64, i: f64| CircleF4E4::from((r, i));
    let colour = || Box::new(VsfType::rcr);
    let fill = || Fill::Solid(colour());
    let stroke = || Some(Stroke { width: sc(0.01), colour: Box::new(VsfType::rcb), join: StrokeJoin::Round, cap: StrokeCap::Square });
    let leaf = || VsfType::roc(ci(0.0, 0.0), sc(0.1), fill(), None);
    let mut add = |label: &str, v: VsfType| s.push((label.to_string(), v));

    // Spirix colour
    add("rd", VsfType::rd(sc(0.5)));
    add("rb", VsfType::rb([sc(0.25), sc(0.5), sc(0.75)]));
    add("rw", VsfType::rw([sc(0.25), sc(0.5), sc(0.75), sc(1.0)]));
    add("rq", VsfType::rq(4, 4, 3, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]));

    // Renderable objects
    add("rob", VsfType::rob(ci(0.0, 0.0), ci(0.5, 0.5), fill(), stroke(), vec![leaf()]));
    add("roc", leaf());
    add("roe", VsfType::roe(ci(0.1, 0.2), ci(0.3, 0.4), fill(), stroke()));
    add("rol", VsfType::rol(ci(0.0, 0.0), ci(1.0, 1.0), sc(0.01), colour()));
    add("rop", VsfType::rop(vec![PathCommand::MoveTo(ci(0.0, 0.0)), PathCommand::LineTo(ci(1.0, 0.0)), PathCommand::QuadraticTo(ci(1.0, 1.0), ci(0.0, 1.0)), PathCommand::CubicTo(ci(0.1, 0.1), ci(0.2, 0.2), ci(0.3, 0.3)), PathCommand::Close], fill(), stroke()));
    add("roo", VsfType::roo(vec![ci(0.0, 0.0), ci(0.5, 0.5), ci(1.0, 0.0)], sc(0.02), colour(), true));
    add("ror", VsfType::ror(vec![ci(0.0, 0.0), ci(0.5, 1.0), ci(1.0, 0.0)], vec![sc(0.0), sc(0.0), sc(0.0), sc(1.0), sc(1.0), sc(1.0)], 2, fill(), None));
    add("rox", VsfType::rox(vec![ci(0.0, 0.0), ci(0.5, 1.0), ci(1.0, 0.0)], SplineType::CatmullRom, fill(), stroke()));
    let mut style = TextStyle::default();
    style.weight = Some(sc(700.0));
    style.align = Some(1);
    add("rot", VsfType::rot(ci(0.0, 0.0), Box::new(VsfType::a("octopus".into())), sc(0.05), colour(), Some(style)));
    add("rot/nostyle", VsfType::rot(ci(0.0, 0.0), Box::new(VsfType::a("octopus".into())), sc(0.05), colour(), None));
    add("rou", VsfType::rou(ci(0.0, 0.0), ci(0.2, 0.1), "OK".into(), ButtonVariant::Outlined, colour()));
    add("roq", VsfType::roq(ci(0.0, 0.0), ci(0.4, 0.1), "name".into(), colour()));
    add("roa", VsfType::roa(2, 1, vec![leaf(), leaf()]));
    add("roi", VsfType::roi(ci(0.0, 0.0), ci(0.5, 0.5), 42, colour()));
    add("rof", VsfType::rof(ci(0.0, 0.0), ci(0.5, 0.5), 7));
    add("ron", VsfType::ron(ci(0.0, 0.0), ci(1.0, 1.0), vec![leaf()]));
    add("rom", VsfType::rom(Box::new(leaf()), vec![leaf()]));
    add("row", VsfType::row(Transform { translate: Some(ci(0.1, 0.2)), rotate: Some(sc(0.5)), scale: None, origin: None }, vec![leaf()]));
    add("rog", VsfType::rog(GradientVariant::Linear { start: ci(0.0, 0.0), end: ci(1.0, 1.0) }, vec![GradientStop { offset: sc(0.0), colour: [sc(1.0), sc(0.0), sc(0.0), sc(1.0)] }, GradientStop { offset: sc(1.0), colour: [sc(0.0), sc(0.0), sc(1.0), sc(1.0)] }]));
    add("rok", VsfType::rok(sc(0.01), colour(), StrokeJoin::Miter, StrokeCap::Butt));

    // Scalars, circles, and their tensors at every fraction × exponent width
    macro_rules! spirix_family {
        ($($s:ident $c:ident $ts:ident $tc:ident $qs:ident $qc:ident $S:ident $C:ident);* $(;)?) => {$(
            let sv = vec![$S::from(-1.5), $S::from(0.25), $S::from(3.0), $S::from(0.0)];
            let cv = vec![$C::from((1.5, -0.5)), $C::from((0.25, 0.0)), $C::from((-3.0, 2.0)), $C::from((0.0, 0.0))];
            add(stringify!($s), VsfType::$s($S::from(-1.5)));
            add(stringify!($c), VsfType::$c($C::from((1.5, -0.5))));
            add(concat!(stringify!($ts), "/1d"), VsfType::$ts(Tensor::new(vec![4], sv.clone())));
            add(concat!(stringify!($ts), "/2d"), VsfType::$ts(Tensor::new(vec![2, 2], sv.clone())));
            add(concat!(stringify!($tc), "/1d"), VsfType::$tc(Tensor::new(vec![4], cv.clone())));
            add(concat!(stringify!($tc), "/2d"), VsfType::$tc(Tensor::new(vec![2, 2], cv.clone())));
            add(stringify!($qs), VsfType::$qs(StridedTensor::new(vec![2, 2], vec![2, 1], sv)));
            add(stringify!($qc), VsfType::$qc(StridedTensor::new(vec![2, 2], vec![2, 1], cv)));
        )*};
    }
    spirix_family!(
        s33 c33 t_s33 t_c33 q_s33 q_c33 ScalarF3E3 CircleF3E3; s34 c34 t_s34 t_c34 q_s34 q_c34 ScalarF3E4 CircleF3E4;
        s35 c35 t_s35 t_c35 q_s35 q_c35 ScalarF3E5 CircleF3E5; s36 c36 t_s36 t_c36 q_s36 q_c36 ScalarF3E6 CircleF3E6;
        s37 c37 t_s37 t_c37 q_s37 q_c37 ScalarF3E7 CircleF3E7;
        s43 c43 t_s43 t_c43 q_s43 q_c43 ScalarF4E3 CircleF4E3; s44 c44 t_s44 t_c44 q_s44 q_c44 ScalarF4E4 CircleF4E4;
        s45 c45 t_s45 t_c45 q_s45 q_c45 ScalarF4E5 CircleF4E5; s46 c46 t_s46 t_c46 q_s46 q_c46 ScalarF4E6 CircleF4E6;
        s47 c47 t_s47 t_c47 q_s47 q_c47 ScalarF4E7 CircleF4E7;
        s53 c53 t_s53 t_c53 q_s53 q_c53 ScalarF5E3 CircleF5E3; s54 c54 t_s54 t_c54 q_s54 q_c54 ScalarF5E4 CircleF5E4;
        s55 c55 t_s55 t_c55 q_s55 q_c55 ScalarF5E5 CircleF5E5; s56 c56 t_s56 t_c56 q_s56 q_c56 ScalarF5E6 CircleF5E6;
        s57 c57 t_s57 t_c57 q_s57 q_c57 ScalarF5E7 CircleF5E7;
        s63 c63 t_s63 t_c63 q_s63 q_c63 ScalarF6E3 CircleF6E3; s64 c64 t_s64 t_c64 q_s64 q_c64 ScalarF6E4 CircleF6E4;
        s65 c65 t_s65 t_c65 q_s65 q_c65 ScalarF6E5 CircleF6E5; s66 c66 t_s66 t_c66 q_s66 q_c66 ScalarF6E6 CircleF6E6;
        s67 c67 t_s67 t_c67 q_s67 q_c67 ScalarF6E7 CircleF6E7;
        s73 c73 t_s73 t_c73 q_s73 q_c73 ScalarF7E3 CircleF7E3; s74 c74 t_s74 t_c74 q_s74 q_c74 ScalarF7E4 CircleF7E4;
        s75 c75 t_s75 t_c75 q_s75 q_c75 ScalarF7E5 CircleF7E5; s76 c76 t_s76 t_c76 q_s76 q_c76 ScalarF7E6 CircleF7E6;
        s77 c77 t_s77 t_c77 q_s77 q_c77 ScalarF7E7 CircleF7E7;
    );
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// `label version hex`, one per line, sorted by label.
fn read_fixture() -> BTreeMap<String, (usize, String)> {
    let text = std::fs::read_to_string(FIXTURE).unwrap_or_default();
    text.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| {
            let mut p = l.split_whitespace();
            let label = p.next().unwrap().to_string();
            let version = p.next().and_then(|v| v.parse().ok()).expect("fixture line: label version hex");
            (label, (version, p.next().unwrap_or("").to_string()))
        })
        .collect()
}

fn write_fixture(entries: &BTreeMap<String, (usize, String)>) {
    let mut out = String::from("# Every VsfType's bytes beside its version in src/types/since.rs — see tests/since.rs. Bless with VSF_BLESS=1.\n");
    for (label, (version, h)) in entries {
        out.push_str(&format!("{label} {version} {h}\n"));
    }
    std::fs::create_dir_all(std::path::Path::new(FIXTURE).parent().unwrap()).unwrap();
    std::fs::write(FIXTURE, out).unwrap();
}

#[test]
fn every_type_in_the_table_has_a_sample() {
    let sampled: std::collections::BTreeSet<&str> = samples().iter().map(|(_, v)| v.own_since().by).collect::<Vec<_>>().into_iter().collect();
    let text = cfg!(any(feature = "text", feature = "text-encode"));
    let missing: Vec<&str> = SINCE_TABLE.iter().map(|(m, _)| *m).filter(|m| !sampled.contains(m) && (text || *m != "x")).collect();
    assert!(missing.is_empty(), "types with no sample in tests/since.rs: {missing:?}");
}

#[test]
fn bytes_change_only_with_the_version() {
    let bless = std::env::var("VSF_BLESS").ok();
    let mut fixture = read_fixture();
    let mut problems = Vec::new();
    let mut refused = Vec::new();

    for (label, value) in samples() {
        let version = value.since().version;
        let now = hex(&value.flatten());
        match fixture.get(&label) {
            Some((v, h)) if *v == version && *h == now => continue,
            Some((v, h)) => {
                let what = if *h == now { format!("{label}: version {v} → {version}, bytes unchanged") } else { format!("{label}: bytes changed at version {v} → {version}\n    was {h}\n    now {now}") };
                // A type can only get newer, and its bytes can change only when it does.
                let allowed = bless.as_deref() == Some("force") || version > *v;
                if bless.is_some() && allowed {
                    fixture.insert(label, (version, now));
                } else if bless.is_some() {
                    refused.push(what);
                } else {
                    problems.push(what);
                }
            }
            None if bless.is_some() => {
                fixture.insert(label, (version, now));
            }
            None => problems.push(format!("{label}: not in the fixture (new type? bless it)")),
        }
    }

    if bless.is_some() {
        assert!(
            refused.is_empty(),
            "refused — these types' bytes or versions changed without the version rising. Raise each one's version in src/types/since.rs to the version that ships the change, then bless again:\n{}",
            refused.join("\n")
        );
        write_fixture(&fixture);
        return;
    }
    assert!(
        problems.is_empty(),
        "wire bytes differ from tests/fixtures/since.txt. If a type's bytes changed, raise its version in src/types/since.rs and run with VSF_BLESS=1:\n{}",
        problems.join("\n")
    );
}
