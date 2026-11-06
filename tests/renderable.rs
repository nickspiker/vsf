//! The renderable objects (`ro*`, v11): every object reads back as it was built, in any numeric type, and malformed or hostile bodies are refused by name.

use vsf::types::{Align, Button, ButtonVariant, Cap, Ellipse, FillRule, Gradient, GradientKind, Grid, Image, Input, Join, Node, Num, Nums, Nurbs, Paint, Path, PerAxis, Polyline, Rect, Stroke, Surface, Text, Verb};
use vsf::types::{Tensor, VsfType};
use vsf::EncodeNumber;

fn round_trip(v: &VsfType) -> VsfType {
    let bytes = v.flatten();
    let mut ptr = 0;
    let back = vsf::parse(&bytes, &mut ptr).unwrap_or_else(|e| panic!("{v:?} did not read back: {e:?}"));
    assert_eq!(ptr, bytes.len(), "{v:?} left bytes unread");
    assert_eq!(&back, v);
    back
}

fn parse_err(bytes: &[u8]) -> String {
    let mut ptr = 0;
    format!("{:?}", vsf::parse(bytes, &mut ptr).expect_err("parsed"))
}

/// `ro<letter>`, the body length in bits, the body.
fn object(letter: u8, body: &[u8]) -> Vec<u8> {
    let mut out = vec![b'r', b'o', letter];
    out.extend_from_slice(&(body.len() << 3).encode_number());
    out.extend_from_slice(body);
    out
}

fn red() -> Paint {
    Paint::colour(VsfType::rcr).unwrap()
}

fn stroke() -> Stroke {
    Stroke { width: Num::from(0.0625f32), paint: Paint::colour(VsfType::rcb).unwrap(), join: Some(Join::Bevel), cap: Some(Cap::Round), miter: Some(Num::from(4u8)) }
}

fn circle() -> VsfType {
    VsfType::roe(Box::new(Ellipse { geometry: Nums::centre_half_f32([0., 0.], [1., 1.]), rotation: None, start: None, sweep: None, fill: Some(red()), stroke: None }))
}

#[test]
fn every_object_reads_back() {
    let gradient = Gradient { kind: GradientKind::Conic, geometry: Nums::f32(vec![2], vec![0., 0.]).unwrap(), offsets: Nums::f32(vec![3], vec![0., 0.5, 1.]).unwrap(), colours: vec![VsfType::rcr, VsfType::rcn, VsfType::rcb], angle: Some(Num::from(0.25f32)) };
    let mut text = Text::new(VsfType::a("octopus".into()), Nums::f32(vec![2], vec![0., 0.]).unwrap(), Num::from(0.0625f32), red());
    text.font = Some(Box::new(VsfType::hb(vec![7; 32])));
    text.leading = Some(Num::from(1.25f32));
    text.spacing = Some(Num::from(0.0078125f32));
    text.weight = Some(Num::from(700u16));
    text.tilt = Some(Num::from(-0.03125f32));
    text.wrap = Some(Num::from(2u8));
    text.align = Some(Align::Right);
    text.rotation = Some(Num::from(0.5f32));
    for v in [
        circle(),
        VsfType::rob(Box::new(Rect { geometry: Nums::centre_half_f32([0., 0.], [2., 1.]), rotation: None, corners: Some(Nums::f32(vec![4, 2], vec![0.5, 0.25, 0.25, 0.125, 0.5, 0.25, 0.25, 0.125]).unwrap()), exponent: Some(PerAxis::each(Nums::f32(vec![2], vec![3., 24.]).unwrap()).unwrap()), fill: Some(Paint::Gradient(Box::new(gradient.clone()))), stroke: Some(stroke()) })),
        VsfType::roe(Box::new(Ellipse { geometry: Nums::f64(vec![3, 2], vec![0., 0., 2., 1., 1., 0.5]).unwrap(), rotation: Some(Num::from(0.125f64)), start: Some(Num::from(0.)), sweep: Some(Num::from(0.75)), fill: None, stroke: Some(stroke()) })),
        VsfType::rop(Box::new(Path { points: Nums::points_f32(&[[0., 0.], [1., 1.], [2., 0.], [3., 1.], [4., 0.], [5., 5.], [1., 1.]]), verbs: vec![Verb::Move, Verb::Cubic, Verb::Close, Verb::Arc, Verb::Line], angles: Some(Nums::f32(vec![3], vec![0., 0., 0.5]).unwrap()), rule: Some(FillRule::NonZero), fill: Some(red()), stroke: None })),
        VsfType::rol(Box::new(Polyline { points: Nums::i16(vec![2, 2], vec![-4, 4, 4, -4]).unwrap(), closed: false, fill: None, stroke: Some(stroke()) })),
        VsfType::ror(Box::new(Nurbs { points: Nums::points_f32(&[[0., 0.], [1., 2.], [2., 0.], [3., 2.]]), degree: Num::from(3u8), knots: Nums::f32(vec![8], vec![0., 0., 0., 0., 1., 1., 1., 1.]).unwrap(), weights: None, fill: None, stroke: Some(stroke()) })),
        VsfType::ron(Box::new(Node { view: None, translate: None, rotation: None, scale: Some(PerAxis::each(Nums::f32(vec![2], vec![0.0625, -0.0625]).unwrap()).unwrap()), mask: None, children: vec![circle()] })),
        VsfType::ron(Box::new(Node::new())),
        VsfType::roa(Box::new(Grid { cols: Num::from(1u8), rows: Num::from(1u8), children: vec![] })),
        VsfType::rot(Box::new(text)),
        VsfType::rou(Box::new(Button { geometry: Nums::centre_half_f32([0., 0.], [1., 0.5]), label: Box::new(VsfType::a("go".into())), variant: ButtonVariant::Text, paint: red() })),
        VsfType::roq(Box::new(Input { geometry: Nums::centre_half_f32([0., 0.], [1., 0.5]), placeholder: Box::new(VsfType::a("".into())), paint: red() })),
        VsfType::roi(Box::new(Image { geometry: Nums::centre_half_f32([0., 0.], [1., 1.]), handle: Num::uint(u64::MAX), tint: None })),
        VsfType::rof(Box::new(Surface { geometry: Nums::centre_half_f32([0., 0.], [1., 1.]), handle: Num::uint(0) })),
        VsfType::rog(Box::new(gradient)),
        VsfType::rok(Box::new(stroke())),
    ] {
        round_trip(&v);
        assert_eq!(v.since().version, 11, "{v:?}");
    }
}

/// Precision is the writer's choice: the same circle at 1-bit packed, i3, f6 and (with spirix) s77 all read back exactly, each in its own type.
#[test]
fn any_numeric_type_carries_geometry() {
    let mut geometries = vec![
        Nums::packed(1, vec![2, 2], &[0, 0, 1, 1]),
        Nums::new(VsfType::t_i3(Tensor::new(vec![2, 2], vec![-1, 1, 3, 3]))).unwrap(),
        Nums::new(VsfType::t_u7(Tensor::new(vec![2, 2], vec![0, 0, u128::MAX, 1]))).unwrap(),
        Nums::f64(vec![2, 2], vec![0., 0., 1e-300, 1e300]).unwrap(),
    ];
    #[cfg(feature = "spirix")]
    geometries.push(Nums::new(VsfType::t_s77(Tensor::new(vec![2, 2], vec![spirix::ScalarF7E7::from(0.5); 4]))).unwrap());
    for geometry in geometries.drain(..) {
        let v = VsfType::roe(Box::new(Ellipse { geometry, rotation: Some(Num::from(1u8)), start: None, sweep: None, fill: Some(red()), stroke: None }));
        round_trip(&v);
    }
    assert_eq!(Nums::packed(1, vec![2, 2], &[0, 0, 1, 1]).to_f64(), vec![0., 0., 1., 1.]);
    assert_eq!(Num::from(-3i8).to_f64(), -3.);
}

/// Lists were counted in one byte before v11, so a 256th point wrapped the count and corrupted everything after it. Counts are tensor shapes now.
#[test]
fn long_lists_survive() {
    let points: Vec<[f32; 2]> = (0..1 << 12).map(|i| [i as f32, -(i as f32)]).collect();
    let mut verbs = vec![Verb::Move];
    verbs.extend(core::iter::repeat(Verb::Line).take(points.len() - 1));
    round_trip(&VsfType::rop(Box::new(Path { points: Nums::points_f32(&points), verbs, angles: None, rule: None, fill: None, stroke: Some(stroke()) })));
    round_trip(&VsfType::rol(Box::new(Polyline { points: Nums::points_f32(&points), closed: true, fill: Some(red()), stroke: None })));
    let stops = 1 << 9;
    let gradient = Gradient { kind: GradientKind::Linear, geometry: Nums::f32(vec![2, 2], vec![0., 0., 1., 0.]).unwrap(), offsets: Nums::f32(vec![stops], (0..stops).map(|i| i as f32 / stops as f32).collect()).unwrap(), colours: vec![VsfType::rcr; stops], angle: None };
    round_trip(&VsfType::rog(Box::new(gradient)));
}

/// A reader steps over an object it has no use for by its length alone.
#[test]
fn an_object_can_be_skipped_by_its_length() {
    let bytes = circle().flatten();
    let mut p = 3;
    let bits = vsf::decoding::helpers::decode_usize(&bytes, &mut p).unwrap();
    assert_eq!(p + (bits >> 3), bytes.len());
}

#[test]
fn malformed_bodies_are_refused_by_name() {
    let geometry = Nums::centre_half_f32([0., 0.], [1., 1.]).value().flatten();
    let with = |extra: &[u8]| {
        let mut body = geometry.clone();
        body.extend_from_slice(extra);
        object(b'e', &body)
    };
    // An attribute the object does not have.
    assert!(parse_err(&with(b"q")).contains("no attribute"));
    // An attribute twice.
    let rot = VsfType::f5(0.25).flatten();
    let twice = [&[b'a'][..], &rot, &[b'a'], &rot].concat();
    assert!(parse_err(&with(&twice)).contains("repeats"));
    // A start with no sweep.
    let start = [&[b's'][..], &rot].concat();
    assert!(parse_err(&with(&start)).contains("start without a sweep"));
    // A fill that is not paint: a renderable object where a colour belongs.
    let not_paint = [&[b'f'][..], &circle().flatten()].concat();
    assert!(parse_err(&with(&not_paint)).contains("colour or a 'rog'"));
    // Geometry of the wrong shape.
    let flat = Nums::f32(vec![4], vec![0., 0., 1., 1.]).unwrap().value().flatten();
    assert!(parse_err(&object(b'e', &flat)).contains("ellipse geometry"));
    // A non-number where numbers belong.
    assert!(parse_err(&object(b'e', &VsfType::a("x".into()).flatten())).contains("is a real-number tensor"));
    // A body that runs past the input, and one that is not whole bytes.
    let mut short = with(b"");
    short.pop();
    assert!(vsf::parse(&short, &mut 0).is_err());
    let mut ragged = b"roe".to_vec();
    ragged.extend_from_slice(&13usize.encode_number());
    ragged.extend_from_slice(&[0, 0]);
    assert!(parse_err(&ragged).contains("whole bytes"));
    // An unknown object letter.
    assert!(parse_err(&object(b'z', &[])).contains("unknown renderable object"));
    // A node child that draws nothing.
    let colour_child = [&[b'c'][..], &VsfType::rcr.flatten()].concat();
    assert!(parse_err(&object(b'n', &colour_child)).contains("not a drawable"));
}

#[test]
fn path_verbs_must_account_for_every_point_and_angle() {
    let path = |points: &[[f32; 2]], verbs: Vec<Verb>, angles: Option<Vec<f32>>| Path { points: Nums::points_f32(points), verbs, angles: angles.map(|a| Nums::f32(vec![a.len()], a).unwrap()), rule: None, fill: None, stroke: None };
    assert!(path(&[[0., 0.], [1., 1.]], vec![Verb::Move, Verb::Line], None).check().is_ok());
    assert!(path(&[[0., 0.], [1., 1.], [2., 2.]], vec![Verb::Move, Verb::Line], None).check().unwrap_err().contains("take 2 points"));
    assert!(path(&[[0., 0.], [1., 1.]], vec![Verb::Arc], None).check().unwrap_err().contains("take 3 angles"));
    assert!(path(&[[0., 0.]], vec![Verb::Line], None).check().unwrap_err().contains("no current point"));
    assert!(path(&[[0., 0.], [1., 1.]], vec![Verb::Move, Verb::Close, Verb::Line], None).check().unwrap_err().contains("no current point"));
    assert!(path(&[], vec![], None).check().unwrap_err().contains("at least one verb"));
    // An unknown verb code on the wire.
    let points = Nums::points_f32(&[[0., 0.]]).value().flatten();
    let verbs = VsfType::p(vsf::types::BitPackedTensor::pack(3, vec![1], &[7u8])).flatten();
    assert!(parse_err(&object(b'p', &[points, verbs].concat())).contains("unknown code 7"));
}

#[test]
fn box_corner_rules() {
    let rect = |corners: Option<Nums>, exponent: Option<PerAxis>| Rect { geometry: Nums::centre_half_f32([0., 0.], [1., 1.]), rotation: None, corners, exponent, fill: None, stroke: None };
    for shape in [vec![1], vec![2], vec![4], vec![4, 2]] {
        let n: usize = shape.iter().product();
        assert!(rect(Some(Nums::f32(shape, vec![0.5; n]).unwrap()), None).check().is_ok());
    }
    assert!(rect(Some(Nums::f32(vec![3], vec![0.5; 3]).unwrap()), None).check().is_err());
    assert!(rect(None, Some(PerAxis::Both(Num::from(3u8)))).check().unwrap_err().contains("without corner radii"));
}

#[test]
fn numbers_refuse_what_is_not_a_number() {
    assert!(Num::new(VsfType::a("1".into())).is_err());
    assert!(Num::new(VsfType::u(1, false)).is_err(), "auto-sized u reads back as a sized type");
    assert!(Num::new(VsfType::u0(true)).is_err());
    assert!(Nums::new(VsfType::f5(1.)).is_err());
    assert!(Nums::new(VsfType::t_u0(Tensor::new(vec![1], vec![true]))).is_err());
    assert!(Paint::colour(circle()).is_err());
}

/// The colour constraint used to accept anything whose name began with `r`, renderable objects and the colour matrix included.
#[test]
fn colour_means_colour() {
    use vsf::schema::TypeConstraint;
    assert!(TypeConstraint::AnyColour.matches(&VsfType::rcr));
    assert!(TypeConstraint::AnyColour.matches(&VsfType::ra([1, 2, 3, 4])));
    assert!(!TypeConstraint::AnyColour.matches(&circle()));
    assert!(!TypeConstraint::AnyColour.matches(&VsfType::rm(1, 1, vec![1.], 1.)));
}

/// A chain one level short of the limit reads on a modest stack: the bound refuses hostile input, never a real scene.
#[test]
fn nesting_up_to_the_limit_reads_on_a_small_stack() {
    let mut v = circle();
    for _ in 0..vsf::types::renderable::MAX_DEPTH - 1 {
        v = VsfType::ron(Box::new(Node { view: None, translate: None, rotation: None, scale: None, mask: None, children: vec![v] }));
    }
    let bytes = v.flatten();
    let reader = std::thread::Builder::new().stack_size(1 << 19).spawn(move || vsf::parse(&bytes, &mut 0).map(|_| ()));
    reader.unwrap().join().expect("stack overflowed").expect("did not parse");
    let deeper = VsfType::ron(Box::new(Node { view: None, translate: None, rotation: None, scale: None, mask: None, children: vec![v] }));
    assert!(parse_err(&deeper.flatten()).contains("nested deeper"));
}

#[test]
fn language_round_trips() {
    use vsf::types::Language;
    for lang in [Language::new(*b"eng"), Language::new(*b"spa").with_script(215).with_region(724), Language::new(*b"tur").with_region(792)] {
        let v = VsfType::wl(lang);
        assert_eq!(v.flatten().len(), 8);
        round_trip(&v);
        assert_eq!(v.since().version, 11);
    }
    assert!(parse_err(b"wl\x44\xa8\0\0\0\0").contains("past 17575"));
    assert!(vsf::parse(b"wl\0\0\0", &mut 0).is_err());
}
