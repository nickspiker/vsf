//! Renderable obje        check_unsigned(&self.rows, "grid rows")ts, the `ro*` family (v11): shapes, containers, UI elements and the paint they carry, all in one tree.
//!
//! **Numbers.** Every number is a VSF value with its own type marker, so its precision is whatever the writer chose: an `f5`, an `i3`, a Spirix `s77` for extreme precision, or a bit-packed `p` grid of a few bits for almost none. A slot that holds one number takes a [`Num`]; a slot that holds several (a shape's geometry, a path's points) takes a [`Nums`], one tensor whose element type is chosen once for the slot. Readers convert to their own working type when they draw; nothing assumes a width.
//!
//! **Frames.** Coordinates and lengths are in the units of the containing [`Node`]: a node maps its own frame into its parent's by translate, rotate and scale, and the outermost frame's units are whatever the consumer holding the scene says they are, with its +y pointing down the display (image-row order, as SVG and fluor draw). There is no fixed −1…+1 window. A node may declare `view`, the region of its own frame it means to show, which is how a consumer fits a scene it did not write.
//!
//! **Angles** are in turns, everywhere: rotations, arc starts and sweeps, text tilt, conic gradient starts. Binary fractions of a turn are exact in every float and Spirix type. A positive angle turns from the frame's +x axis toward its +y axis: clockwise on the display, unless a node's scale mirrors its frame.
//!
//! **Wire.** `r o <letter>`, then the body's length in bits (an EWE number, as `v` writes its length), then the body: the type's required values in its fixed order, then any number of optional attributes, each one lowercase ASCII tag byte followed by one VSF value. Attribute order is free; an attribute appears at most once unless the type says it repeats (children, gradient colours). A reader knows each body's extent before parsing it, so it can step over an object whole. Enumerated codes (join, cap, fill rule, …) are `u3` values; a reader refuses a code it does not know.
//!
//! Nesting is bounded at [`MAX_DEPTH`] levels: every value inside a body is read thru this module, never handed back to the top-level parser, so the count cannot be reset by wrapping an object in another type.

use crate::decoding::traits::DecodeError;
use crate::prelude::*;
use crate::types::{BitPackedTensor, Tensor, VsfType};

/// How deep renderable objects may nest. Each level costs a stack frame, and a stream of nested nodes a few hundred thousand deep once ran the stack out (security audit VSF-08). No real scene nests this deep.
pub const MAX_DEPTH: u32 = 1 << 6;

// ==================== DOCUMENTS ====================

/// The section a vector document keeps its scene in.
pub const SCENE_SECTION: &str = "vector";

/// The field of [`SCENE_SECTION`] holding the scene's root: one drawable object, normally a [`Node`] whose `view` says what region to show.
pub const SCENE_FIELD: &str = "scene";

/// The scene of a vector document, or why the bytes hold none.
pub fn read_scene(bytes: &[u8]) -> Result<VsfType, String> {
    let doc = crate::container::Document::open(bytes).map_err(|e| format!("{e}"))?;
    let entry = doc.entry(SCENE_SECTION).ok_or_else(|| format!("no '{SCENE_SECTION}' section"))?;
    let section = doc.section(entry).map_err(|e| format!("{e}"))?;
    let field = section.fields.iter().find(|f| f.name == SCENE_FIELD).ok_or_else(|| format!("no '{SCENE_FIELD}' field in '{SCENE_SECTION}'"))?;
    match field.values.as_slice() {
        [v] if is_drawable(v) => Ok(v.clone()),
        [v] => Err(format!("'{SCENE_FIELD}' is {}, not a drawable object", value_name(v))),
        vs => Err(format!("'{SCENE_FIELD}' holds {} values, not one", vs.len())),
    }
}

/// A vector document's section: the scene under [`SCENE_FIELD`], ready for [`crate::vsf_builder::VsfBuilder::add_section`].
pub fn scene_section(scene: VsfType) -> (String, Vec<(String, VsfType)>) {
    (SCENE_SECTION.into(), vec![(SCENE_FIELD.into(), scene)])
}

// ==================== NUMBERS ====================

/// Calls `$m!` with every Spirix scalar marker, so one arm can be written for all 25.
#[cfg(feature = "spirix")]
macro_rules! spirix_markers {
    ($m:ident) => {
        $m! { s33 t_s33, s34 t_s34, s35 t_s35, s36 t_s36, s37 t_s37, s43 t_s43, s44 t_s44, s45 t_s45, s46 t_s46, s47 t_s47, s53 t_s53, s54 t_s54, s55 t_s55, s56 t_s56, s57 t_s57, s63 t_s63, s64 t_s64, s65 t_s65, s66 t_s66, s67 t_s67, s73 t_s73, s74 t_s74, s75 t_s75, s76 t_s76, s77 t_s77 }
    };
}

/// A real number in any sized VSF numeric type: `u3`–`u7`, `i3`–`i7`, `f5`, `f6`, or a Spirix scalar `s33`–`s77`. The marker is the precision. The auto-sized `u` and `i` are refused: they read back as a sized type, so a value built with one would not survive a round trip.
#[derive(Debug, Clone, PartialEq)]
pub struct Num(VsfType);

fn real_scalar(v: &VsfType) -> bool {
    #[cfg(feature = "spirix")]
    macro_rules! is_spirix {
        ($($s:ident $t:ident),*) => {
            matches!(v, $(VsfType::$s(_))|*)
        };
    }
    #[cfg(feature = "spirix")]
    if spirix_markers!(is_spirix) {
        return true;
    }
    matches!(
        v,
        VsfType::u3(_) | VsfType::u4(_) | VsfType::u5(_) | VsfType::u6(_) | VsfType::u7(_) | VsfType::i3(_) | VsfType::i4(_) | VsfType::i5(_) | VsfType::i6(_) | VsfType::i7(_) | VsfType::f5(_) | VsfType::f6(_)
    )
}

impl Num {
    /// `value` as a number, or why it is not one.
    pub fn new(value: VsfType) -> Result<Num, String> {
        if real_scalar(&value) {
            Ok(Num(value))
        } else {
            Err(format!("not a sized real number: {}", value_name(&value)))
        }
    }

    /// The value as written, marker and all.
    pub fn value(&self) -> &VsfType {
        &self.0
    }

    /// The value in f64, for a reader whose working type is f64. Integers wider than 53 bits and Spirix values outside f64's range round as any conversion to f64 does.
    pub fn to_f64(&self) -> f64 {
        #[cfg(feature = "spirix")]
        macro_rules! spirix_f64 {
            ($($s:ident $t:ident),*) => {
                match &self.0 {
                    $(VsfType::$s(x) => return x.to_f64(),)*
                    _ => {}
                }
            };
        }
        #[cfg(feature = "spirix")]
        spirix_markers!(spirix_f64);
        match &self.0 {
            VsfType::u3(x) => *x as f64,
            VsfType::u4(x) => *x as f64,
            VsfType::u5(x) => *x as f64,
            VsfType::u6(x) => *x as f64,
            VsfType::u7(x) => *x as f64,
            VsfType::i3(x) => *x as f64,
            VsfType::i4(x) => *x as f64,
            VsfType::i5(x) => *x as f64,
            VsfType::i6(x) => *x as f64,
            VsfType::i7(x) => *x as f64,
            VsfType::f5(x) => *x as f64,
            VsfType::f6(x) => *x,
            // Num::new admits nothing else.
            other => unreachable!("Num holds a non-number: {}", value_name(other)),
        }
    }

    /// The value when it is an unsigned integer that fits a u64; `None` for any other type. Counts and handles (a grid's columns, a curve's degree, an image handle) are read thru this.
    pub fn as_u64(&self) -> Option<u64> {
        match &self.0 {
            VsfType::u3(x) => Some(*x as u64),
            VsfType::u4(x) => Some(*x as u64),
            VsfType::u5(x) => Some(*x as u64),
            VsfType::u6(x) => Some(*x),
            VsfType::u7(x) => u64::try_from(*x).ok(),
            _ => None,
        }
    }

    /// A count or handle at the narrowest unsigned width that holds it.
    pub fn uint(value: u64) -> Num {
        Num(VsfType::uint(value))
    }
}

macro_rules! num_from {
    ($($t:ty => $v:ident),*) => {
        $(impl From<$t> for Num {
            fn from(x: $t) -> Num {
                Num(VsfType::$v(x))
            }
        })*
    };
}
num_from!(u8 => u3, u16 => u4, u32 => u5, u64 => u6, u128 => u7, i8 => i3, i16 => i4, i32 => i5, i64 => i6, i128 => i7, f32 => f5, f64 => f6);

#[cfg(feature = "spirix")]
macro_rules! num_from_spirix {
    ($($s:ident $t:ident),*) => {
        $(impl From<spirix_type!($s)> for Num {
            fn from(x: spirix_type!($s)) -> Num {
                Num(VsfType::$s(x))
            }
        })*
    };
}

#[cfg(feature = "spirix")]
macro_rules! spirix_type {
    (s33) => { spirix::ScalarF3E3 }; (s34) => { spirix::ScalarF3E4 }; (s35) => { spirix::ScalarF3E5 }; (s36) => { spirix::ScalarF3E6 }; (s37) => { spirix::ScalarF3E7 };
    (s43) => { spirix::ScalarF4E3 }; (s44) => { spirix::ScalarF4E4 }; (s45) => { spirix::ScalarF4E5 }; (s46) => { spirix::ScalarF4E6 }; (s47) => { spirix::ScalarF4E7 };
    (s53) => { spirix::ScalarF5E3 }; (s54) => { spirix::ScalarF5E4 }; (s55) => { spirix::ScalarF5E5 }; (s56) => { spirix::ScalarF5E6 }; (s57) => { spirix::ScalarF5E7 };
    (s63) => { spirix::ScalarF6E3 }; (s64) => { spirix::ScalarF6E4 }; (s65) => { spirix::ScalarF6E5 }; (s66) => { spirix::ScalarF6E6 }; (s67) => { spirix::ScalarF6E7 };
    (s73) => { spirix::ScalarF7E3 }; (s74) => { spirix::ScalarF7E4 }; (s75) => { spirix::ScalarF7E5 }; (s76) => { spirix::ScalarF7E6 }; (s77) => { spirix::ScalarF7E7 };
}

#[cfg(feature = "spirix")]
spirix_markers!(num_from_spirix);

/// Several real numbers of one type: a contiguous tensor (`t_u3`–`t_u7`, `t_i3`–`t_i7`, `t_f5`, `t_f6`, Spirix `t_s33`–`t_s77`) or a bit-packed `p` tensor of any width. The element type is chosen once for the whole slot; the shape is the slot's own rule (a point list is `[n, 2]`).
#[derive(Debug, Clone, PartialEq)]
pub struct Nums(VsfType);

fn real_tensor(v: &VsfType) -> bool {
    #[cfg(feature = "spirix")]
    macro_rules! is_spirix_tensor {
        ($($s:ident $t:ident),*) => {
            matches!(v, $(VsfType::$t(_))|*)
        };
    }
    #[cfg(feature = "spirix")]
    if spirix_markers!(is_spirix_tensor) {
        return true;
    }
    matches!(
        v,
        VsfType::t_u3(_) | VsfType::t_u4(_) | VsfType::t_u5(_) | VsfType::t_u6(_) | VsfType::t_u7(_) | VsfType::t_i3(_) | VsfType::t_i4(_) | VsfType::t_i5(_) | VsfType::t_i6(_) | VsfType::t_i7(_) | VsfType::t_f5(_) | VsfType::t_f6(_) | VsfType::p(_)
    )
}

impl Nums {
    /// `value` as a set of numbers, or why it is not one.
    pub fn new(value: VsfType) -> Result<Nums, String> {
        if real_tensor(&value) {
            Ok(Nums(value))
        } else {
            Err(format!("not a real-number tensor: {}", value_name(&value)))
        }
    }

    /// The tensor as written, marker and all.
    pub fn value(&self) -> &VsfType {
        &self.0
    }

    /// The tensor's shape.
    pub fn shape(&self) -> &[usize] {
        #[cfg(feature = "spirix")]
        macro_rules! spirix_shape {
            ($($s:ident $t:ident),*) => {
                match &self.0 {
                    $(VsfType::$t(t) => return &t.shape,)*
                    _ => {}
                }
            };
        }
        #[cfg(feature = "spirix")]
        spirix_markers!(spirix_shape);
        match &self.0 {
            VsfType::t_u3(t) => &t.shape,
            VsfType::t_u4(t) => &t.shape,
            VsfType::t_u5(t) => &t.shape,
            VsfType::t_u6(t) => &t.shape,
            VsfType::t_u7(t) => &t.shape,
            VsfType::t_i3(t) => &t.shape,
            VsfType::t_i4(t) => &t.shape,
            VsfType::t_i5(t) => &t.shape,
            VsfType::t_i6(t) => &t.shape,
            VsfType::t_i7(t) => &t.shape,
            VsfType::t_f5(t) => &t.shape,
            VsfType::t_f6(t) => &t.shape,
            VsfType::p(t) => &t.shape,
            // Nums::new admits nothing else.
            other => unreachable!("Nums holds a non-tensor: {}", value_name(other)),
        }
    }

    /// Every element in f64, row-major, for a reader whose working type is f64.
    pub fn to_f64(&self) -> Vec<f64> {
        #[cfg(feature = "spirix")]
        macro_rules! spirix_f64s {
            ($($s:ident $t:ident),*) => {
                match &self.0 {
                    $(VsfType::$t(t) => return t.data.iter().map(|x| x.to_f64()).collect(),)*
                    _ => {}
                }
            };
        }
        #[cfg(feature = "spirix")]
        spirix_markers!(spirix_f64s);
        match &self.0 {
            VsfType::t_u3(t) => t.data.iter().map(|&x| x as f64).collect(),
            VsfType::t_u4(t) => t.data.iter().map(|&x| x as f64).collect(),
            VsfType::t_u5(t) => t.data.iter().map(|&x| x as f64).collect(),
            VsfType::t_u6(t) => t.data.iter().map(|&x| x as f64).collect(),
            VsfType::t_u7(t) => t.data.iter().map(|&x| x as f64).collect(),
            VsfType::t_i3(t) => t.data.iter().map(|&x| x as f64).collect(),
            VsfType::t_i4(t) => t.data.iter().map(|&x| x as f64).collect(),
            VsfType::t_i5(t) => t.data.iter().map(|&x| x as f64).collect(),
            VsfType::t_i6(t) => t.data.iter().map(|&x| x as f64).collect(),
            VsfType::t_i7(t) => t.data.iter().map(|&x| x as f64).collect(),
            VsfType::t_f5(t) => t.data.iter().map(|&x| x as f64).collect(),
            VsfType::t_f6(t) => t.data.clone(),
            VsfType::p(t) => t.unpack_u128().into_iter().map(|x| x as f64).collect(),
            // Nums::new admits nothing else.
            other => unreachable!("Nums holds a non-tensor: {}", value_name(other)),
        }
    }

    /// An `f32` tensor of the given shape.
    pub fn f32(shape: Vec<usize>, data: Vec<f32>) -> Result<Nums, String> {
        Ok(Nums(VsfType::t_f5(Tensor::try_new(shape, data)?)))
    }

    /// An `f64` tensor of the given shape.
    pub fn f64(shape: Vec<usize>, data: Vec<f64>) -> Result<Nums, String> {
        Ok(Nums(VsfType::t_f6(Tensor::try_new(shape, data)?)))
    }

    /// An `i16` tensor of the given shape: coordinates on an integer grid, scaled by the frame that holds them.
    pub fn i16(shape: Vec<usize>, data: Vec<i16>) -> Result<Nums, String> {
        Ok(Nums(VsfType::t_i4(Tensor::try_new(shape, data)?)))
    }

    /// A bit-packed tensor of `bit_depth`-bit unsigned samples: the least precision a slot can carry.
    pub fn packed(bit_depth: u8, shape: Vec<usize>, samples: &[u64]) -> Nums {
        Nums(VsfType::p(BitPackedTensor::pack(bit_depth, shape, samples)))
    }

    /// A point list `[n, 2]` in `f32`.
    pub fn points_f32(points: &[[f32; 2]]) -> Nums {
        Nums(VsfType::t_f5(Tensor::new(vec![points.len(), 2], points.iter().flatten().copied().collect())))
    }

    /// A centre and a half-size (or radii), `[2, 2]` in `f32`: the geometry of a box, an ellipse, a button.
    pub fn centre_half_f32(centre: [f32; 2], half: [f32; 2]) -> Nums {
        Nums(VsfType::t_f5(Tensor::new(vec![2, 2], vec![centre[0], centre[1], half[0], half[1]])))
    }
}

/// One number for both axes, or one per axis (`[2]`, x then y). Which it is follows from the value: a scalar or a tensor.
#[derive(Debug, Clone, PartialEq)]
pub enum PerAxis {
    Both(Num),
    Each(Nums),
}

impl PerAxis {
    fn value(&self) -> &VsfType {
        match self {
            PerAxis::Both(n) => n.value(),
            PerAxis::Each(n) => n.value(),
        }
    }

    /// (x, y) in f64.
    pub fn to_f64(&self) -> [f64; 2] {
        match self {
            PerAxis::Both(n) => {
                let v = n.to_f64();
                [v, v]
            }
            PerAxis::Each(n) => {
                // Shape [2] is checked when the value is read or built thru `each`.
                let v = n.to_f64();
                [v[0], v[1]]
            }
        }
    }

    /// One per axis, shape `[2]` enforced.
    pub fn each(n: Nums) -> Result<PerAxis, String> {
        if n.shape() != [2] {
            return Err(format!("a per-axis value is shape [2], not {:?}", n.shape()));
        }
        Ok(PerAxis::Each(n))
    }
}

// ==================== CLOSED VOCABULARIES ====================

/// A closed vocabulary written as a `u3` code: a reader refuses a code it does not know.
macro_rules! code_enum {
    ($(#[$doc:meta])* $name:ident { $($(#[$vdoc:meta])* $variant:ident = $code:literal),+ $(,)? }) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $name { $($(#[$vdoc])* $variant),+ }
        impl $name {
            /// The wire code.
            pub fn code(self) -> u8 {
                match self { $($name::$variant => $code),+ }
            }
            /// The variant for a wire code, if there is one.
            pub fn from_code(code: u8) -> Option<Self> {
                match code { $($code => Some($name::$variant),)+ _ => None }
            }
        }
    };
}

code_enum!(
    /// How two stroked segments meet. Absent means `Miter`.
    Join { Miter = 0, Round = 1, Bevel = 2 }
);
code_enum!(
    /// How an open stroke ends. Absent means `Butt`.
    Cap { Butt = 0, Round = 1, Square = 2 }
);
code_enum!(
    /// Which points a self-crossing path fills. Absent means `NonZero`.
    FillRule { NonZero = 0, EvenOdd = 1 }
);
code_enum!(
    /// One step of a path, and what it takes from the path's lists.
    Verb {
        /// Start a subpath at the next point.
        Move = 0,
        /// A straight segment to the next point.
        Line = 1,
        /// A quadratic Bézier: control point, end point.
        Quad = 2,
        /// A cubic Bézier: two control points, end point.
        Cubic = 3,
        /// An elliptical arc: two points (centre, radii) and three angles (rotation, start, sweep). A straight segment joins the current point to the arc's start, as Canvas 2D's `ellipse()` does; with no current point the arc starts a subpath.
        Arc = 4,
        /// Close the subpath back to its first point.
        Close = 5,
    }
);
code_enum!(
    /// A gradient's geometry.
    GradientKind {
        /// `[2, 2]`: start point, end point.
        Linear = 0,
        /// `[2, 2]`: centre, radii.
        Radial = 1,
        /// `[2]`: centre; the `a` attribute sets where the sweep starts.
        Conic = 2,
    }
);
code_enum!(
    /// Where text sits against its anchor. Absent means `Centre`.
    Align { Centre = 0, Left = 1, Right = 2 }
);
code_enum!(
    /// A button's look.
    ButtonVariant { Filled = 0, Outlined = 1, Text = 2 }
);

impl Verb {
    /// Points this verb takes from the path's point list.
    pub fn points(self) -> usize {
        match self {
            Verb::Move | Verb::Line => 1,
            Verb::Quad | Verb::Arc => 2,
            Verb::Cubic => 3,
            Verb::Close => 0,
        }
    }

    /// Angles this verb takes from the path's angle list.
    pub fn angles(self) -> usize {
        match self {
            Verb::Arc => 3,
            _ => 0,
        }
    }
}

// ==================== PAINT AND STROKE ====================

/// What fills an area or a stroke: a colour (any `r` colour type) or a gradient.
#[derive(Debug, Clone, PartialEq)]
pub enum Paint {
    Colour(Box<VsfType>),
    Gradient(Box<Gradient>),
}

impl Paint {
    /// A colour paint, refused if `value` is not a colour.
    pub fn colour(value: VsfType) -> Result<Paint, String> {
        if value.is_colour() {
            Ok(Paint::Colour(Box::new(value)))
        } else {
            Err(format!("not a colour: {}", value_name(&value)))
        }
    }

    fn write(&self, out: &mut Vec<u8>) {
        match self {
            Paint::Colour(c) => out.extend_from_slice(&c.flatten()),
            Paint::Gradient(g) => out.extend_from_slice(&g.encode()),
        }
    }

    fn visit(&self, f: &mut dyn FnMut(&VsfType)) {
        match self {
            Paint::Colour(c) => f(c),
            Paint::Gradient(g) => g.visit(f),
        }
    }
}

/// A stroke (`rok`): a line of `width` frame units painted along an outline.
///
/// Required: width ([`Num`]), paint (a colour or `rog`). Attributes: `j` join, `c` cap, `m` miter limit ([`Num`], a multiple of the width).
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    pub width: Num,
    pub paint: Paint,
    pub join: Option<Join>,
    pub cap: Option<Cap>,
    pub miter: Option<Num>,
}

/// A gradient (`rog`).
///
/// Required: kind (`u3` [`GradientKind`]), geometry ([`Nums`], shape by kind), stop offsets ([`Nums`] `[n]`, positions along the gradient from 0 to 1). Attributes: `a` angle (turns: a conic gradient's start, a radial one's rotation), `c` a stop colour, repeated once per offset in order.
#[derive(Debug, Clone, PartialEq)]
pub struct Gradient {
    pub kind: GradientKind,
    pub geometry: Nums,
    pub offsets: Nums,
    pub colours: Vec<VsfType>,
    pub angle: Option<Num>,
}

// ==================== OBJECTS ====================

/// A box (`rob`): sharp, rounded, squircle-cornered, a pill, or a whole superellipse.
///
/// Required: geometry ([`Nums`] `[2, 2]`: centre, then half-width and half-height).
/// Attributes: `a` rotation (turns), `c` corner radii, `n` corner exponent, `f` fill ([`Paint`]), `k` stroke (`rok`).
///
/// Corner radii (`c`) are `[1]` (every corner, both axes), `[2]` (x radius and y radius, every corner), `[4]` (one per corner, both axes) or `[4, 2]` (one per corner per axis). Corners run (−x,−y), (+x,−y), (+x,+y), (−x,+y): in a y-down frame top-left, top-right, bottom-right, bottom-left. No radii is a sharp box. A radius never exceeds the half-size on its axis.
///
/// Each corner is a superellipse quadrant, fluor's curve: with radii (rx, ry) and exponents (nx, ny), ((rx − u)/rx)^nx + ((ry − v)/ry)^ny = 1, u and v measured inward from the box's edges. The exponent (`n`) is one [`Num`] for both axes or `[2]` for x and y; absent it is 2, an elliptical quarter. Radii equal to the half-size on both axes make the whole box one superellipse. `n` without `c` is refused: a sharp corner has no curve to shape.
#[derive(Debug, Clone, PartialEq)]
pub struct Rect {
    pub geometry: Nums,
    pub rotation: Option<Num>,
    pub corners: Option<Nums>,
    pub exponent: Option<PerAxis>,
    pub fill: Option<Paint>,
    pub stroke: Option<Stroke>,
}

/// An ellipse (`roe`): circle, ellipse, pie sector, ring or ring segment.
///
/// Required: geometry ([`Nums`] `[2, 2]`: centre, radii; or `[3, 2]`: centre, outer radii, inner radii). A circle repeats its radius.
/// Attributes: `a` rotation (turns), `s` start (turns), `w` sweep (turns, signed), `f` fill, `k` stroke.
///
/// Angles are parametric: the point at angle t is centre + R(rotation)·(rx cos 2πt, ry sin 2πt), on the inner ellipse the same with its radii. No sweep is the whole ellipse (or the whole ring). A sweep is a closed region: a pie sector thru the centre, or with inner radii a ring segment. An open arc is a path's [`Verb::Arc`]. A start without a sweep is refused.
#[derive(Debug, Clone, PartialEq)]
pub struct Ellipse {
    pub geometry: Nums,
    pub rotation: Option<Num>,
    pub start: Option<Num>,
    pub sweep: Option<Num>,
    pub fill: Option<Paint>,
    pub stroke: Option<Stroke>,
}

/// A path (`rop`): subpaths of lines, Bézier curves and elliptical arcs.
///
/// Required: points ([`Nums`] `[n, 2]`), verbs (a 3-bit `p` tensor `[m]` of [`Verb`] codes).
/// Attributes: `t` angles ([`Nums`] `[k]`, turns, three per arc), `r` fill rule, `f` fill, `k` stroke.
///
/// The verbs consume the points and angles in order, and must use every one. The first verb is `Move` or `Arc`, and so is the verb after a `Close`.
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub points: Nums,
    pub verbs: Vec<Verb>,
    pub angles: Option<Nums>,
    pub rule: Option<FillRule>,
    pub fill: Option<Paint>,
    pub stroke: Option<Stroke>,
}

/// A polyline (`rol`): straight segments thru two or more points; closed, a polygon.
///
/// Required: points ([`Nums`] `[n, 2]`, n ≥ 2). Attributes: `z` closed (`u0` true; written only when closed), `f` fill, `k` stroke.
#[derive(Debug, Clone, PartialEq)]
pub struct Polyline {
    pub points: Nums,
    pub closed: bool,
    pub fill: Option<Paint>,
    pub stroke: Option<Stroke>,
}

/// A NURBS curve (`ror`).
///
/// Required: control points ([`Nums`] `[n, 2]`), degree (unsigned [`Num`], ≥ 1), knots ([`Nums`] `[n + degree + 1]`). Attributes: `w` weights ([`Nums`] `[n]`; absent, every weight is 1 and the curve is a plain B-spline), `f` fill, `k` stroke.
#[derive(Debug, Clone, PartialEq)]
pub struct Nurbs {
    pub points: Nums,
    pub degree: Num,
    pub knots: Nums,
    pub weights: Option<Nums>,
    pub fill: Option<Paint>,
    pub stroke: Option<Stroke>,
}

/// A node (`ron`): a frame, and the objects drawn in it.
///
/// No required values. Attributes: `v` view ([`Nums`] `[2, 2]`: centre and half-size of the region of this frame the node means to show), `t` translate ([`Nums`] `[2]`, in the parent's units), `a` rotation (turns), `s` scale ([`PerAxis`]), `m` mask (one renderable object: the children draw only where it covers), `c` a child, repeated, drawn in order.
///
/// A point p in this node's frame is at translate + R(rotation)·diag(scale)·p in its parent's. Integer coordinates take their unit from here: children on an `i3` grid under a scale of 2⁻⁴ sit on sixteenths of the parent's unit.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub view: Option<Nums>,
    pub translate: Option<Nums>,
    pub rotation: Option<Num>,
    pub scale: Option<PerAxis>,
    pub mask: Option<Box<VsfType>>,
    pub children: Vec<VsfType>,
}

/// A grid of cells (`roa`), laid out by the consumer.
///
/// Required: columns, rows (unsigned [`Num`]s). Attributes: `c` a cell, repeated, row-major. A cell is any value: a drawable object, or one the consumer shows as a table cell (text, a number, a colour).
#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    pub cols: Num,
    pub rows: Num,
    pub children: Vec<VsfType>,
}

/// Text (`rot`).
///
/// Required: the text (`x` or `a`), anchor ([`Nums`] `[2]`), size ([`Num`], em height in frame units), paint.
/// Attributes: `h` font (`hb`, BLAKE3 of the font file's bytes), `e` leading (line height, a multiple of the size), `s` letter spacing (frame units), `w` weight (100–900), `i` tilt (turns), `x` wrap width (frame units; enables wrapping), `j` align ([`Align`]), `a` rotation (turns).
#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    pub text: Box<VsfType>,
    pub anchor: Nums,
    pub size: Num,
    pub paint: Paint,
    pub font: Option<Box<VsfType>>,
    pub leading: Option<Num>,
    pub spacing: Option<Num>,
    pub weight: Option<Num>,
    pub tilt: Option<Num>,
    pub wrap: Option<Num>,
    pub align: Option<Align>,
    pub rotation: Option<Num>,
}

/// A button (`rou`). Required: geometry ([`Nums`] `[2, 2]`: centre, half-size), label (`x` or `a`), variant (`u3` [`ButtonVariant`]), paint.
#[derive(Debug, Clone, PartialEq)]
pub struct Button {
    pub geometry: Nums,
    pub label: Box<VsfType>,
    pub variant: ButtonVariant,
    pub paint: Paint,
}

/// A text input (`roq`). Required: geometry ([`Nums`] `[2, 2]`), placeholder (`x` or `a`), paint.
#[derive(Debug, Clone, PartialEq)]
pub struct Input {
    pub geometry: Nums,
    pub placeholder: Box<VsfType>,
    pub paint: Paint,
}

/// An image (`roi`) the consumer holds by handle. Required: geometry ([`Nums`] `[2, 2]`), handle (unsigned [`Num`]). Attributes: `f` tint.
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    pub geometry: Nums,
    pub handle: Num,
    pub tint: Option<Paint>,
}

/// A pixel surface (`rof`) the consumer holds by handle. Required: geometry ([`Nums`] `[2, 2]`), handle (unsigned [`Num`]).
#[derive(Debug, Clone, PartialEq)]
pub struct Surface {
    pub geometry: Nums,
    pub handle: Num,
}

// ==================== HELPERS ====================

/// A value's type marker for an error message, from the version table (which names every variant).
fn value_name(v: &VsfType) -> String {
    format!("'{}'", v.own_since().by)
}

fn bad(msg: impl Into<String>) -> DecodeError {
    DecodeError::InvalidDataMsg(msg.into())
}

/// A text value: `x` or `a`.
fn is_text(v: &VsfType) -> bool {
    matches!(v, VsfType::x(_) | VsfType::a(_))
}

/// A renderable object that draws something (every `ro*` but the paint types `rog` and `rok`).
pub fn is_drawable(v: &VsfType) -> bool {
    matches!(
        v,
        VsfType::rob(_) | VsfType::roe(_) | VsfType::rop(_) | VsfType::rol(_) | VsfType::ror(_) | VsfType::ron(_) | VsfType::roa(_) | VsfType::rot(_) | VsfType::rou(_) | VsfType::roq(_) | VsfType::roi(_) | VsfType::rof(_)
    )
}

/// A point list: `[n, 2]` with at least `min` points.
fn check_points(n: &Nums, what: &str, min: usize) -> Result<usize, String> {
    match n.shape() {
        [count, 2] if *count >= min => Ok(*count),
        s => Err(format!("{what} is [n, 2] with n ≥ {min}, not {s:?}")),
    }
}

fn check_shape(n: &Nums, what: &str, allowed: &[&[usize]]) -> Result<(), String> {
    if allowed.iter().any(|a| n.shape() == *a) {
        Ok(())
    } else {
        Err(format!("{what} is one of {allowed:?}, not {:?}", n.shape()))
    }
}

fn check_text(v: &VsfType, what: &str) -> Result<(), String> {
    if is_text(v) {
        Ok(())
    } else {
        Err(format!("{what} is text ('x' or 'a'), not {}", value_name(v)))
    }
}

fn check_unsigned(n: &Num, what: &str) -> Result<(), String> {
    if n.as_u64().is_some() {
        Ok(())
    } else {
        Err(format!("{what} is an unsigned integer that fits 64 bits, not {}", value_name(n.value())))
    }
}

// ==================== ENCODING ====================

/// Builds one object's bytes: required values, then tagged attributes.
struct Body(Vec<u8>);

impl Body {
    fn new() -> Body {
        Body(Vec::new())
    }

    /// A nested renderable object writes itself directly, so a deep tree never passes thru `flatten`'s large frame on the way down.
    fn value(&mut self, v: &VsfType) -> &mut Body {
        let bytes = match v {
            VsfType::rob(x) => x.encode(),
            VsfType::roe(x) => x.encode(),
            VsfType::rop(x) => x.encode(),
            VsfType::rol(x) => x.encode(),
            VsfType::ror(x) => x.encode(),
            VsfType::ron(x) => x.encode(),
            VsfType::roa(x) => x.encode(),
            VsfType::rot(x) => x.encode(),
            VsfType::rou(x) => x.encode(),
            VsfType::roq(x) => x.encode(),
            VsfType::roi(x) => x.encode(),
            VsfType::rof(x) => x.encode(),
            VsfType::rog(x) => x.encode(),
            VsfType::rok(x) => x.encode(),
            other => other.flatten(),
        };
        self.0.extend_from_slice(&bytes);
        self
    }

    fn paint(&mut self, p: &Paint) -> &mut Body {
        p.write(&mut self.0);
        self
    }

    fn code(&mut self, c: u8) -> &mut Body {
        self.value(&VsfType::u3(c))
    }

    fn tag(&mut self, t: u8, v: Option<&VsfType>) -> &mut Body {
        if let Some(v) = v {
            self.0.push(t);
            self.value(v);
        }
        self
    }

    fn tag_paint(&mut self, t: u8, p: Option<&Paint>) -> &mut Body {
        if let Some(p) = p {
            self.0.push(t);
            p.write(&mut self.0);
        }
        self
    }

    fn tag_code(&mut self, t: u8, c: Option<u8>) -> &mut Body {
        self.tag(t, c.map(VsfType::u3).as_ref())
    }

    fn tag_stroke(&mut self, s: Option<&Stroke>) -> &mut Body {
        if let Some(s) = s {
            self.0.push(b'k');
            self.0.extend_from_slice(&s.encode());
        }
        self
    }

    fn children(&mut self, children: &[VsfType]) -> &mut Body {
        for c in children {
            self.0.push(b'c');
            self.value(c);
        }
        self
    }

    /// `r o letter`, the length in bits, the body.
    fn finish(&mut self, letter: u8) -> Vec<u8> {
        use crate::encoding::traits::EncodeNumber;
        let body = core::mem::take(&mut self.0);
        // Bytes → bits as `v` writes them. On a 32-bit target `len << 3` would wrap past 512 MiB; it stops with a message instead of writing a corrupt length.
        let bits = usize::try_from((body.len() as u64) << 3).expect("renderable object exceeds 512 MiB, the limit of a 32-bit length-in-bits");
        let mut out = vec![b'r', b'o', letter];
        out.extend_from_slice(&bits.encode_number());
        out.extend_from_slice(&body);
        out
    }
}

fn opt_num(n: &Option<Num>) -> Option<&VsfType> {
    n.as_ref().map(Num::value)
}

fn opt_nums(n: &Option<Nums>) -> Option<&VsfType> {
    n.as_ref().map(Nums::value)
}

impl Stroke {
    /// The `rok` bytes.
    pub fn encode(&self) -> Vec<u8> {
        Body::new()
            .value(self.width.value())
            .paint(&self.paint)
            .tag_code(b'j', self.join.map(Join::code))
            .tag_code(b'c', self.cap.map(Cap::code))
            .tag(b'm', opt_num(&self.miter))
            .finish(b'k')
    }

    fn visit(&self, f: &mut dyn FnMut(&VsfType)) {
        f(self.width.value());
        self.paint.visit(f);
        if let Some(m) = &self.miter {
            f(m.value());
        }
    }
}

impl Gradient {
    /// The `rog` bytes.
    pub fn encode(&self) -> Vec<u8> {
        let mut b = Body::new();
        b.code(self.kind.code()).value(self.geometry.value()).value(self.offsets.value()).tag(b'a', opt_num(&self.angle));
        for c in &self.colours {
            b.tag(b'c', Some(c));
        }
        b.finish(b'g')
    }

    /// The shape rules a reader enforces: geometry by kind, one colour per offset, every colour a colour.
    pub fn check(&self) -> Result<(), String> {
        let allowed: &[&[usize]] = match self.kind {
            GradientKind::Linear | GradientKind::Radial => &[&[2, 2]],
            GradientKind::Conic => &[&[2]],
        };
        check_shape(&self.geometry, "gradient geometry", allowed)?;
        if self.kind == GradientKind::Linear && self.angle.is_some() {
            return Err("a linear gradient has no angle: its direction is its two points".into());
        }
        let n = match self.offsets.shape() {
            [n] if *n >= 1 => *n,
            s => return Err(format!("gradient offsets are [n] with n ≥ 1, not {s:?}")),
        };
        if self.colours.len() != n {
            return Err(format!("gradient has {n} offsets and {} colours", self.colours.len()));
        }
        if let Some(c) = self.colours.iter().find(|c| !c.is_colour()) {
            return Err(format!("gradient stop is not a colour: {}", value_name(c)));
        }
        Ok(())
    }

    fn visit(&self, f: &mut dyn FnMut(&VsfType)) {
        f(self.geometry.value());
        f(self.offsets.value());
        for c in &self.colours {
            f(c);
        }
        if let Some(a) = &self.angle {
            f(a.value());
        }
    }
}

impl Rect {
    pub fn encode(&self) -> Vec<u8> {
        Body::new()
            .value(self.geometry.value())
            .tag(b'a', opt_num(&self.rotation))
            .tag(b'c', opt_nums(&self.corners))
            .tag(b'n', self.exponent.as_ref().map(PerAxis::value))
            .tag_paint(b'f', self.fill.as_ref())
            .tag_stroke(self.stroke.as_ref())
            .finish(b'b')
    }

    pub fn check(&self) -> Result<(), String> {
        check_shape(&self.geometry, "box geometry", &[&[2, 2]])?;
        if let Some(c) = &self.corners {
            check_shape(c, "corner radii", &[&[1], &[2], &[4], &[4, 2]])?;
        }
        if let Some(PerAxis::Each(n)) = &self.exponent {
            check_shape(n, "corner exponent", &[&[2]])?;
        }
        if self.exponent.is_some() && self.corners.is_none() {
            return Err("a corner exponent without corner radii".into());
        }
        Ok(())
    }
}

impl Ellipse {
    pub fn encode(&self) -> Vec<u8> {
        Body::new()
            .value(self.geometry.value())
            .tag(b'a', opt_num(&self.rotation))
            .tag(b's', opt_num(&self.start))
            .tag(b'w', opt_num(&self.sweep))
            .tag_paint(b'f', self.fill.as_ref())
            .tag_stroke(self.stroke.as_ref())
            .finish(b'e')
    }

    pub fn check(&self) -> Result<(), String> {
        check_shape(&self.geometry, "ellipse geometry", &[&[2, 2], &[3, 2]])?;
        if self.start.is_some() && self.sweep.is_none() {
            return Err("an ellipse start without a sweep".into());
        }
        Ok(())
    }

    /// True when the geometry carries inner radii: a ring, or with a sweep a ring segment.
    pub fn is_ring(&self) -> bool {
        self.geometry.shape() == [3, 2]
    }
}

impl Path {
    pub fn encode(&self) -> Vec<u8> {
        let codes: Vec<u8> = self.verbs.iter().map(|v| v.code()).collect();
        let verbs = VsfType::p(BitPackedTensor::pack(3, vec![codes.len()], &codes));
        Body::new()
            .value(self.points.value())
            .value(&verbs)
            .tag(b't', opt_nums(&self.angles))
            .tag_code(b'r', self.rule.map(FillRule::code))
            .tag_paint(b'f', self.fill.as_ref())
            .tag_stroke(self.stroke.as_ref())
            .finish(b'p')
    }

    pub fn check(&self) -> Result<(), String> {
        if self.verbs.is_empty() {
            return Err("a path has at least one verb".into());
        }
        let n = check_points(&self.points, "path points", 0)?;
        let k = match &self.angles {
            None => 0,
            Some(a) => match a.shape() {
                [k] => *k,
                s => return Err(format!("path angles are [k], not {s:?}")),
            },
        };
        let (mut points, mut angles, mut open) = (0usize, 0usize, false);
        for (idx, &v) in self.verbs.iter().enumerate() {
            match v {
                Verb::Move => open = true,
                Verb::Arc => open = true,
                Verb::Line | Verb::Quad | Verb::Cubic if !open => {
                    return Err(format!("path verb {idx} ({v:?}) has no current point: a subpath starts with Move or Arc"));
                }
                Verb::Close if !open => return Err(format!("path verb {idx} closes no subpath")),
                Verb::Close => open = false,
                _ => {}
            }
            points += v.points();
            angles += v.angles();
        }
        if points != n {
            return Err(format!("path verbs take {points} points; the list has {n}"));
        }
        if angles != k {
            return Err(format!("path verbs take {angles} angles; the list has {k}"));
        }
        Ok(())
    }
}

impl Polyline {
    pub fn encode(&self) -> Vec<u8> {
        Body::new()
            .value(self.points.value())
            .tag(b'z', self.closed.then_some(&VsfType::u0(true)))
            .tag_paint(b'f', self.fill.as_ref())
            .tag_stroke(self.stroke.as_ref())
            .finish(b'l')
    }

    pub fn check(&self) -> Result<(), String> {
        check_points(&self.points, "polyline points", 2).map(|_| ())
    }
}

impl Nurbs {
    pub fn encode(&self) -> Vec<u8> {
        Body::new()
            .value(self.points.value())
            .value(self.degree.value())
            .value(self.knots.value())
            .tag(b'w', opt_nums(&self.weights))
            .tag_paint(b'f', self.fill.as_ref())
            .tag_stroke(self.stroke.as_ref())
            .finish(b'r')
    }

    pub fn check(&self) -> Result<(), String> {
        let n = check_points(&self.points, "NURBS control points", 2)?;
        check_unsigned(&self.degree, "NURBS degree")?;
        // as_u64 is Some: check_unsigned just passed.
        let degree = self.degree.as_u64().unwrap();
        if degree == 0 || degree >= n as u64 {
            return Err(format!("NURBS degree {degree} with {n} control points: 1 ≤ degree < points"));
        }
        // degree < n, and n is half a tensor's element count, so n + degree + 1 ≤ the element count, which fits a usize.
        let want = n + degree as usize + 1;
        check_shape(&self.knots, "NURBS knots", &[&[want]])?;
        if let Some(w) = &self.weights {
            check_shape(w, "NURBS weights", &[&[n]])?;
        }
        Ok(())
    }
}

impl Node {
    pub fn encode(&self) -> Vec<u8> {
        Body::new()
            .tag(b'v', opt_nums(&self.view))
            .tag(b't', opt_nums(&self.translate))
            .tag(b'a', opt_num(&self.rotation))
            .tag(b's', self.scale.as_ref().map(PerAxis::value))
            .tag(b'm', self.mask.as_deref())
            .children(&self.children)
            .finish(b'n')
    }

    pub fn check(&self) -> Result<(), String> {
        if let Some(v) = &self.view {
            check_shape(v, "node view", &[&[2, 2]])?;
        }
        if let Some(t) = &self.translate {
            check_shape(t, "node translate", &[&[2]])?;
        }
        if let Some(PerAxis::Each(s)) = &self.scale {
            check_shape(s, "node scale", &[&[2]])?;
        }
        if let Some(m) = &self.mask {
            if !is_drawable(m) {
                return Err(format!("node mask is not a drawable object: {}", value_name(m)));
            }
        }
        if let Some(c) = self.children.iter().find(|c| !is_drawable(c)) {
            return Err(format!("node child is not a drawable object: {}", value_name(c)));
        }
        Ok(())
    }

    /// An empty node: identity frame, no children.
    pub fn new() -> Node {
        Node { view: None, translate: None, rotation: None, scale: None, mask: None, children: Vec::new() }
    }
}

impl Default for Node {
    fn default() -> Self {
        Node::new()
    }
}

impl Grid {
    pub fn encode(&self) -> Vec<u8> {
        Body::new().value(self.cols.value()).value(self.rows.value()).children(&self.children).finish(b'a')
    }

    pub fn check(&self) -> Result<(), String> {
        check_unsigned(&self.cols, "grid columns")?;
        check_unsigned(&self.rows, "grid rows")
    }
}

impl Text {
    pub fn encode(&self) -> Vec<u8> {
        Body::new()
            .value(&self.text)
            .value(self.anchor.value())
            .value(self.size.value())
            .paint(&self.paint)
            .tag(b'h', self.font.as_deref())
            .tag(b'e', opt_num(&self.leading))
            .tag(b's', opt_num(&self.spacing))
            .tag(b'w', opt_num(&self.weight))
            .tag(b'i', opt_num(&self.tilt))
            .tag(b'x', opt_num(&self.wrap))
            .tag_code(b'j', self.align.map(Align::code))
            .tag(b'a', opt_num(&self.rotation))
            .finish(b't')
    }

    pub fn check(&self) -> Result<(), String> {
        check_text(&self.text, "text")?;
        check_shape(&self.anchor, "text anchor", &[&[2]])?;
        match self.font.as_deref() {
            None => Ok(()),
            Some(VsfType::hb(h)) if h.len() == 32 => Ok(()),
            Some(f) => Err(format!("text font is a 32-byte 'hb', not {}", value_name(f))),
        }
    }

    /// Text with no style attributes.
    pub fn new(text: VsfType, anchor: Nums, size: Num, paint: Paint) -> Text {
        Text { text: Box::new(text), anchor, size, paint, font: None, leading: None, spacing: None, weight: None, tilt: None, wrap: None, align: None, rotation: None }
    }
}

impl Button {
    pub fn encode(&self) -> Vec<u8> {
        Body::new().value(self.geometry.value()).value(&self.label).code(self.variant.code()).paint(&self.paint).finish(b'u')
    }

    pub fn check(&self) -> Result<(), String> {
        check_shape(&self.geometry, "button geometry", &[&[2, 2]])?;
        check_text(&self.label, "button label")
    }
}

impl Input {
    pub fn encode(&self) -> Vec<u8> {
        Body::new().value(self.geometry.value()).value(&self.placeholder).paint(&self.paint).finish(b'q')
    }

    pub fn check(&self) -> Result<(), String> {
        check_shape(&self.geometry, "text input geometry", &[&[2, 2]])?;
        check_text(&self.placeholder, "text input placeholder")
    }
}

impl Image {
    pub fn encode(&self) -> Vec<u8> {
        Body::new().value(self.geometry.value()).value(self.handle.value()).tag_paint(b'f', self.tint.as_ref()).finish(b'i')
    }

    pub fn check(&self) -> Result<(), String> {
        check_shape(&self.geometry, "image geometry", &[&[2, 2]])?;
        check_unsigned(&self.handle, "image handle")
    }
}

impl Surface {
    pub fn encode(&self) -> Vec<u8> {
        Body::new().value(self.geometry.value()).value(self.handle.value()).finish(b'f')
    }

    pub fn check(&self) -> Result<(), String> {
        check_shape(&self.geometry, "surface geometry", &[&[2, 2]])?;
        check_unsigned(&self.handle, "surface handle")
    }
}

/// Every value nested inside a renderable object, in wire order, for [`VsfType::since`]. Paint and strokes are walked into; they are not values of their own here.
pub(crate) fn visit(v: &VsfType, f: &mut dyn FnMut(&VsfType)) {
    let paint = |p: &Option<Paint>, f: &mut dyn FnMut(&VsfType)| {
        if let Some(p) = p {
            p.visit(f)
        }
    };
    let stroke = |s: &Option<Stroke>, f: &mut dyn FnMut(&VsfType)| {
        if let Some(s) = s {
            s.visit(f)
        }
    };
    let num = |n: &Option<Num>, f: &mut dyn FnMut(&VsfType)| {
        if let Some(n) = n {
            f(n.value())
        }
    };
    let nums = |n: &Option<Nums>, f: &mut dyn FnMut(&VsfType)| {
        if let Some(n) = n {
            f(n.value())
        }
    };
    match v {
        VsfType::rob(r) => {
            f(r.geometry.value());
            num(&r.rotation, f);
            nums(&r.corners, f);
            if let Some(e) = &r.exponent {
                f(e.value());
            }
            paint(&r.fill, f);
            stroke(&r.stroke, f);
        }
        VsfType::roe(e) => {
            f(e.geometry.value());
            num(&e.rotation, f);
            num(&e.start, f);
            num(&e.sweep, f);
            paint(&e.fill, f);
            stroke(&e.stroke, f);
        }
        VsfType::rop(p) => {
            f(p.points.value());
            nums(&p.angles, f);
            paint(&p.fill, f);
            stroke(&p.stroke, f);
        }
        VsfType::rol(l) => {
            f(l.points.value());
            paint(&l.fill, f);
            stroke(&l.stroke, f);
        }
        VsfType::ror(r) => {
            f(r.points.value());
            f(r.degree.value());
            f(r.knots.value());
            nums(&r.weights, f);
            paint(&r.fill, f);
            stroke(&r.stroke, f);
        }
        VsfType::ron(n) => {
            nums(&n.view, f);
            nums(&n.translate, f);
            num(&n.rotation, f);
            if let Some(s) = &n.scale {
                f(s.value());
            }
            if let Some(m) = &n.mask {
                f(m);
            }
            n.children.iter().for_each(|c| f(c));
        }
        VsfType::roa(g) => {
            f(g.cols.value());
            f(g.rows.value());
            g.children.iter().for_each(|c| f(c));
        }
        VsfType::rot(t) => {
            f(&t.text);
            f(t.anchor.value());
            f(t.size.value());
            t.paint.visit(f);
            if let Some(h) = &t.font {
                f(h);
            }
            for n in [&t.leading, &t.spacing, &t.weight, &t.tilt, &t.wrap, &t.rotation] {
                num(n, f);
            }
        }
        VsfType::rou(b) => {
            f(b.geometry.value());
            f(&b.label);
            b.paint.visit(f);
        }
        VsfType::roq(i) => {
            f(i.geometry.value());
            f(&i.placeholder);
            i.paint.visit(f);
        }
        VsfType::roi(i) => {
            f(i.geometry.value());
            f(i.handle.value());
            paint(&i.tint, f);
        }
        VsfType::rof(s) => {
            f(s.geometry.value());
            f(s.handle.value());
        }
        VsfType::rog(g) => g.visit(f),
        VsfType::rok(s) => s.visit(f),
        _ => {}
    }
}

// ==================== DECODING ====================
//
// Nesting recurses thru read_at → read_node / read_grid → Reader::need → Reader::value → read_at, so those frames are kept small: attributes are read in a plain loop, a container's fields are built on the heap, and every error message is formatted in a cold function off that path. A debug build once took more than 16 KB a level here, enough for 64 hostile levels to overflow a 1 MB stack.

/// Reads one object's body: required values, then tagged attributes until the body ends.
struct Reader<'a> {
    data: &'a [u8],
    p: usize,
    depth: u32,
}

#[cold]
#[inline(never)]
fn ends_before(what: &str) -> DecodeError {
    DecodeError::UnexpectedEofMsg(format!("object ends before its {what}"))
}

#[cold]
#[inline(never)]
fn wrong(what: &str, expected: &str, got: &VsfType) -> DecodeError {
    bad(format!("{what} is {expected}, not {}", value_name(got)))
}

#[cold]
#[inline(never)]
fn no_attribute(kind: &str, tag: u8) -> DecodeError {
    bad(format!("{kind} has no attribute '{}'", tag.escape_ascii()))
}

#[cold]
#[inline(never)]
fn unknown_code(what: &str, code: u8) -> DecodeError {
    bad(format!("unknown {what} {code}"))
}

#[cold]
#[inline(never)]
fn too_deep() -> DecodeError {
    bad(format!("renderable objects nested deeper than {MAX_DEPTH}"))
}

impl<'a> Reader<'a> {
    fn more(&self) -> bool {
        self.p < self.data.len()
    }

    /// The next attribute's tag, or `None` at the end of the body.
    /// WHY: the attributes run until the body's bytes end, so reaching the end is the loop's exit, not an error; the body's length came off the wire and was checked against the input in `header`.
    fn tag(&mut self) -> Option<u8> {
        let t = *self.data.get(self.p)?;
        self.p += 1;
        Some(t)
    }

    /// One value. A renderable object is read here, one level deeper; anything else goes to the top-level parser, which never reaches a renderable object from inside another type.
    fn value(&mut self) -> Result<VsfType, DecodeError> {
        if self.data[self.p..].starts_with(b"ro") {
            self.p += 2;
            read_at(self.data, &mut self.p, self.depth + 1)
        } else {
            crate::decoding::parse::parse(self.data, &mut self.p)
        }
    }

    /// WHY: a required value may be missing from a peer's body; without this check `value` would index past the body's end.
    fn need(&mut self, what: &str) -> Result<VsfType, DecodeError> {
        if !self.more() {
            return Err(ends_before(what));
        }
        self.value()
    }

    fn num(&mut self, what: &str) -> Result<Num, DecodeError> {
        let v = self.need(what)?;
        if real_scalar(&v) {
            Ok(Num(v))
        } else {
            Err(wrong(what, "a sized real number", &v))
        }
    }

    fn nums(&mut self, what: &str) -> Result<Nums, DecodeError> {
        let v = self.need(what)?;
        if real_tensor(&v) {
            Ok(Nums(v))
        } else {
            Err(wrong(what, "a real-number tensor", &v))
        }
    }

    fn per_axis(&mut self, what: &str) -> Result<PerAxis, DecodeError> {
        let v = self.need(what)?;
        if real_scalar(&v) {
            Ok(PerAxis::Both(Num(v)))
        } else if real_tensor(&v) {
            Ok(PerAxis::Each(Nums(v)))
        } else {
            Err(wrong(what, "a number or [2] numbers", &v))
        }
    }

    fn code(&mut self, what: &str) -> Result<u8, DecodeError> {
        match self.need(what)? {
            VsfType::u3(c) => Ok(c),
            other => Err(wrong(what, "a 'u3' code", &other)),
        }
    }

    fn paint(&mut self, what: &str) -> Result<Paint, DecodeError> {
        match self.need(what)? {
            VsfType::rog(g) => Ok(Paint::Gradient(g)),
            v if v.is_colour() => Ok(Paint::Colour(Box::new(v))),
            other => Err(wrong(what, "a colour or a 'rog' gradient", &other)),
        }
    }

    fn stroke(&mut self) -> Result<Stroke, DecodeError> {
        match self.need("stroke")? {
            VsfType::rok(s) => Ok(*s),
            other => Err(wrong("stroke", "a 'rok'", &other)),
        }
    }

    fn text(&mut self, what: &str) -> Result<Box<VsfType>, DecodeError> {
        Ok(Box::new(self.need(what)?))
    }
}

/// Sets an attribute once; a second time is refused.
fn once<T>(slot: &mut Option<T>, value: T, kind: &str, tag: u8) -> Result<(), DecodeError> {
    if slot.is_some() {
        return Err(repeats(kind, tag));
    }
    *slot = Some(value);
    Ok(())
}

#[cold]
#[inline(never)]
fn repeats(kind: &str, tag: u8) -> DecodeError {
    bad(format!("{kind} repeats attribute '{}'", tag.escape_ascii()))
}

fn checked(res: Result<(), String>) -> Result<(), DecodeError> {
    res.map_err(bad)
}

/// Read one renderable object, the pointer just past `r o`. The top-level parser calls this at depth 0.
pub(crate) fn read(data: &[u8], pointer: &mut usize) -> Result<VsfType, DecodeError> {
    read_at(data, pointer, 0)
}

/// The type letter and the body's bounds, the pointer moved past the whole object.
#[inline(never)]
fn header(data: &[u8], pointer: &mut usize) -> Result<(u8, usize, usize), DecodeError> {
    if *pointer >= data.len() {
        return Err(DecodeError::UnexpectedEofMsg("renderable object has no type letter".into()));
    }
    let letter = data[*pointer];
    *pointer += 1;
    let bits = crate::decoding::helpers::decode_usize(data, pointer)?;
    if bits & 7 != 0 {
        return Err(bad(format!("ro{} body is {bits} bits, not whole bytes", letter.escape_ascii())));
    }
    let len = bits >> 3;
    if crate::decoding::helpers::short(data, *pointer, len) {
        return Err(DecodeError::UnexpectedEofMsg(format!("ro{} body claims {len} bytes, more than are left", letter.escape_ascii())));
    }
    let start = *pointer;
    *pointer += len;
    Ok((letter, start, len))
}

fn read_at(data: &[u8], pointer: &mut usize, depth: u32) -> Result<VsfType, DecodeError> {
    if depth >= MAX_DEPTH {
        return Err(too_deep());
    }
    let (letter, start, len) = header(data, pointer)?;
    let mut r = Reader { data: &data[start..start + len], p: 0, depth };
    match letter {
        b'b' => read_rect(&mut r),
        b'e' => read_ellipse(&mut r),
        b'p' => read_path(&mut r),
        b'l' => read_polyline(&mut r),
        b'r' => read_nurbs(&mut r),
        b'n' => read_node(&mut r),
        b'a' => read_grid(&mut r),
        b't' => read_text(&mut r),
        b'u' => read_button(&mut r),
        b'q' => read_input(&mut r),
        b'i' => read_image(&mut r),
        b'f' => read_surface(&mut r),
        b'g' => read_gradient(&mut r),
        b'k' => read_stroke(&mut r),
        other => Err(unknown_object(other)),
    }
}

#[cold]
#[inline(never)]
fn unknown_object(letter: u8) -> DecodeError {
    bad(format!("unknown renderable object 'ro{}'", letter.escape_ascii()))
}

#[inline(never)]
fn read_rect(r: &mut Reader<'_>) -> Result<VsfType, DecodeError> {
    let mut v = Rect { geometry: r.nums("box geometry")?, rotation: None, corners: None, exponent: None, fill: None, stroke: None };
    while let Some(t) = r.tag() {
        match t {
            b'a' => once(&mut v.rotation, r.num("box rotation")?, "box", t)?,
            b'c' => once(&mut v.corners, r.nums("corner radii")?, "box", t)?,
            b'n' => once(&mut v.exponent, r.per_axis("corner exponent")?, "box", t)?,
            b'f' => once(&mut v.fill, r.paint("box fill")?, "box", t)?,
            b'k' => once(&mut v.stroke, r.stroke()?, "box", t)?,
            _ => return Err(no_attribute("box", t)),
        }
    }
    checked(v.check())?;
    Ok(VsfType::rob(Box::new(v)))
}

#[inline(never)]
fn read_ellipse(r: &mut Reader<'_>) -> Result<VsfType, DecodeError> {
    let mut v = Ellipse { geometry: r.nums("ellipse geometry")?, rotation: None, start: None, sweep: None, fill: None, stroke: None };
    while let Some(t) = r.tag() {
        match t {
            b'a' => once(&mut v.rotation, r.num("ellipse rotation")?, "ellipse", t)?,
            b's' => once(&mut v.start, r.num("ellipse start")?, "ellipse", t)?,
            b'w' => once(&mut v.sweep, r.num("ellipse sweep")?, "ellipse", t)?,
            b'f' => once(&mut v.fill, r.paint("ellipse fill")?, "ellipse", t)?,
            b'k' => once(&mut v.stroke, r.stroke()?, "ellipse", t)?,
            _ => return Err(no_attribute("ellipse", t)),
        }
    }
    checked(v.check())?;
    Ok(VsfType::roe(Box::new(v)))
}

#[inline(never)]
fn read_path(r: &mut Reader<'_>) -> Result<VsfType, DecodeError> {
    let points = r.nums("path points")?;
    let verbs = match r.need("path verbs")? {
        VsfType::p(t) if t.bit_depth == 3 && t.shape.len() == 1 => t.unpack_u8().into_iter().enumerate().map(|(idx, c)| Verb::from_code(c).ok_or_else(|| bad(format!("path verb {idx} has unknown code {c}")))).collect::<Result<Vec<Verb>, _>>()?,
        other => return Err(wrong("path verbs", "a 1-D 3-bit 'p' tensor", &other)),
    };
    let mut v = Path { points, verbs, angles: None, rule: None, fill: None, stroke: None };
    while let Some(t) = r.tag() {
        match t {
            b't' => once(&mut v.angles, r.nums("path angles")?, "path", t)?,
            b'r' => {
                let c = r.code("fill rule")?;
                once(&mut v.rule, FillRule::from_code(c).ok_or_else(|| unknown_code("fill rule", c))?, "path", t)?
            }
            b'f' => once(&mut v.fill, r.paint("path fill")?, "path", t)?,
            b'k' => once(&mut v.stroke, r.stroke()?, "path", t)?,
            _ => return Err(no_attribute("path", t)),
        }
    }
    checked(v.check())?;
    Ok(VsfType::rop(Box::new(v)))
}

#[inline(never)]
fn read_polyline(r: &mut Reader<'_>) -> Result<VsfType, DecodeError> {
    let mut v = Polyline { points: r.nums("polyline points")?, closed: false, fill: None, stroke: None };
    let mut closed = None;
    while let Some(t) = r.tag() {
        match t {
            b'z' => match r.need("polyline closed")? {
                VsfType::u0(true) => once(&mut closed, (), "polyline", t)?,
                other => return Err(wrong("polyline closed", "'u0' true (absent when open)", &other)),
            },
            b'f' => once(&mut v.fill, r.paint("polyline fill")?, "polyline", t)?,
            b'k' => once(&mut v.stroke, r.stroke()?, "polyline", t)?,
            _ => return Err(no_attribute("polyline", t)),
        }
    }
    v.closed = closed.is_some();
    checked(v.check())?;
    Ok(VsfType::rol(Box::new(v)))
}

#[inline(never)]
fn read_nurbs(r: &mut Reader<'_>) -> Result<VsfType, DecodeError> {
    let points = r.nums("NURBS control points")?;
    let degree = r.num("NURBS degree")?;
    let knots = r.nums("NURBS knots")?;
    let mut v = Nurbs { points, degree, knots, weights: None, fill: None, stroke: None };
    while let Some(t) = r.tag() {
        match t {
            b'w' => once(&mut v.weights, r.nums("NURBS weights")?, "NURBS", t)?,
            b'f' => once(&mut v.fill, r.paint("NURBS fill")?, "NURBS", t)?,
            b'k' => once(&mut v.stroke, r.stroke()?, "NURBS", t)?,
            _ => return Err(no_attribute("NURBS", t)),
        }
    }
    checked(v.check())?;
    Ok(VsfType::ror(Box::new(v)))
}

/// On the nesting path: the node is built on the heap, so a level's frame holds a pointer, not a 560-byte node.
#[inline(never)]
fn read_node(r: &mut Reader<'_>) -> Result<VsfType, DecodeError> {
    let mut v = Box::new(Node::new());
    while let Some(t) = r.tag() {
        match t {
            b'v' => once(&mut v.view, r.nums("node view")?, "node", t)?,
            b't' => once(&mut v.translate, r.nums("node translate")?, "node", t)?,
            b'a' => once(&mut v.rotation, r.num("node rotation")?, "node", t)?,
            b's' => once(&mut v.scale, r.per_axis("node scale")?, "node", t)?,
            b'm' => {
                let mask = Box::new(r.need("node mask")?);
                once(&mut v.mask, mask, "node", t)?
            }
            b'c' => {
                let child = r.need("node child")?;
                v.children.push(child)
            }
            _ => return Err(no_attribute("node", t)),
        }
    }
    checked(v.check())?;
    Ok(VsfType::ron(v))
}

/// On the nesting path, like [`read_node`].
#[inline(never)]
fn read_grid(r: &mut Reader<'_>) -> Result<VsfType, DecodeError> {
    let cols = r.num("grid columns")?;
    let rows = r.num("grid rows")?;
    let mut v = Box::new(Grid { cols, rows, children: Vec::new() });
    while let Some(t) = r.tag() {
        match t {
            b'c' => {
                let child = r.need("grid cell")?;
                v.children.push(child)
            }
            _ => return Err(no_attribute("grid", t)),
        }
    }
    checked(v.check())?;
    Ok(VsfType::roa(v))
}

#[inline(never)]
fn read_text(r: &mut Reader<'_>) -> Result<VsfType, DecodeError> {
    let text = r.text("text")?;
    let anchor = r.nums("text anchor")?;
    let size = r.num("text size")?;
    let paint = r.paint("text paint")?;
    let mut v = Text::new(*text, anchor, size, paint);
    while let Some(t) = r.tag() {
        match t {
            b'h' => {
                let font = Box::new(r.need("text font")?);
                once(&mut v.font, font, "text", t)?
            }
            b'e' => once(&mut v.leading, r.num("text leading")?, "text", t)?,
            b's' => once(&mut v.spacing, r.num("text spacing")?, "text", t)?,
            b'w' => once(&mut v.weight, r.num("text weight")?, "text", t)?,
            b'i' => once(&mut v.tilt, r.num("text tilt")?, "text", t)?,
            b'x' => once(&mut v.wrap, r.num("text wrap")?, "text", t)?,
            b'j' => {
                let c = r.code("text align")?;
                once(&mut v.align, Align::from_code(c).ok_or_else(|| unknown_code("text align", c))?, "text", t)?
            }
            b'a' => once(&mut v.rotation, r.num("text rotation")?, "text", t)?,
            _ => return Err(no_attribute("text", t)),
        }
    }
    checked(v.check())?;
    Ok(VsfType::rot(Box::new(v)))
}

#[inline(never)]
fn read_button(r: &mut Reader<'_>) -> Result<VsfType, DecodeError> {
    let geometry = r.nums("button geometry")?;
    let label = r.text("button label")?;
    let c = r.code("button variant")?;
    let variant = ButtonVariant::from_code(c).ok_or_else(|| unknown_code("button variant", c))?;
    let paint = r.paint("button paint")?;
    if let Some(t) = r.tag() {
        return Err(no_attribute("button", t));
    }
    let v = Button { geometry, label, variant, paint };
    checked(v.check())?;
    Ok(VsfType::rou(Box::new(v)))
}

#[inline(never)]
fn read_input(r: &mut Reader<'_>) -> Result<VsfType, DecodeError> {
    let geometry = r.nums("text input geometry")?;
    let placeholder = r.text("text input placeholder")?;
    let paint = r.paint("text input paint")?;
    if let Some(t) = r.tag() {
        return Err(no_attribute("text input", t));
    }
    let v = Input { geometry, placeholder, paint };
    checked(v.check())?;
    Ok(VsfType::roq(Box::new(v)))
}

#[inline(never)]
fn read_image(r: &mut Reader<'_>) -> Result<VsfType, DecodeError> {
    let mut v = Image { geometry: r.nums("image geometry")?, handle: r.num("image handle")?, tint: None };
    while let Some(t) = r.tag() {
        match t {
            b'f' => once(&mut v.tint, r.paint("image tint")?, "image", t)?,
            _ => return Err(no_attribute("image", t)),
        }
    }
    checked(v.check())?;
    Ok(VsfType::roi(Box::new(v)))
}

#[inline(never)]
fn read_surface(r: &mut Reader<'_>) -> Result<VsfType, DecodeError> {
    let v = Surface { geometry: r.nums("surface geometry")?, handle: r.num("surface handle")? };
    if let Some(t) = r.tag() {
        return Err(no_attribute("surface", t));
    }
    checked(v.check())?;
    Ok(VsfType::rof(Box::new(v)))
}

#[inline(never)]
fn read_gradient(r: &mut Reader<'_>) -> Result<VsfType, DecodeError> {
    let c = r.code("gradient kind")?;
    let kind = GradientKind::from_code(c).ok_or_else(|| unknown_code("gradient kind", c))?;
    let geometry = r.nums("gradient geometry")?;
    let offsets = r.nums("gradient offsets")?;
    let mut v = Gradient { kind, geometry, offsets, colours: Vec::new(), angle: None };
    while let Some(t) = r.tag() {
        match t {
            b'a' => once(&mut v.angle, r.num("gradient angle")?, "gradient", t)?,
            b'c' => {
                let colour = r.need("gradient colour")?;
                v.colours.push(colour)
            }
            _ => return Err(no_attribute("gradient", t)),
        }
    }
    checked(v.check())?;
    Ok(VsfType::rog(Box::new(v)))
}

#[inline(never)]
fn read_stroke(r: &mut Reader<'_>) -> Result<VsfType, DecodeError> {
    let width = r.num("stroke width")?;
    let paint = r.paint("stroke paint")?;
    let mut v = Stroke { width, paint, join: None, cap: None, miter: None };
    while let Some(t) = r.tag() {
        match t {
            b'j' => {
                let c = r.code("stroke join")?;
                once(&mut v.join, Join::from_code(c).ok_or_else(|| unknown_code("stroke join", c))?, "stroke", t)?
            }
            b'c' => {
                let c = r.code("stroke cap")?;
                once(&mut v.cap, Cap::from_code(c).ok_or_else(|| unknown_code("stroke cap", c))?, "stroke", t)?
            }
            b'm' => once(&mut v.miter, r.num("stroke miter limit")?, "stroke", t)?,
            _ => return Err(no_attribute("stroke", t)),
        }
    }
    Ok(VsfType::rok(Box::new(v)))
}
