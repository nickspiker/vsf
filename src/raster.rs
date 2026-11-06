//! A reference rasterizer for renderable objects (`ro*`): a scene in, linear VSF RGB out, so any reader (opsin's preview, a test, a device without its own renderer) can see what a file draws.
//!
//! Every shape is flattened to straight edges within an eighth of a pixel of its true curve, then filled with exact horizontal coverage on sixteen sub-scanlines per pixel, under the nonzero or even-odd rule. Strokes are built as outline pieces (a quad per segment, joins and caps) filled together under nonzero, so their overlaps union. Paint is composited source-over, premultiplied, in linear VSF RGB; gradients interpolate in linear light. The outermost frame's +y runs down the image.
//!
//! Drawn: boxes (squircle corners included), ellipses and their sectors and rings, paths, polylines, NURBS, nodes (frames and masks), solid and gradient paint, strokes. Not drawn, and named in [`Rendered::skipped`]: text, buttons, text inputs, images, surfaces and grids, which need the consumer's fonts, widgets or pixels.

use crate::prelude::*;
use crate::types::renderable::{Cap, Ellipse, FillRule, Gradient, GradientKind, Join, Node, Nums, Nurbs, Paint, Path, PerAxis, Rect, Stroke, Verb};
#[cfg(test)]
use crate::types::renderable::Polyline;
use crate::types::VsfType;
use libm::{acos, atan2, ceil, cos, floor, pow, sin, sqrt};

/// Sub-scanlines per pixel row: vertical coverage resolves to 1/16; horizontal coverage is exact.
const SUBSCANLINES: usize = 1 << 4;

/// How far a flattened curve may stray from the true one, in output pixels.
const TOLERANCE_PX: f64 = 1. / (1u32 << 3) as f64;

/// Most segments one curve is cut into, whatever its size.
const MAX_SEGMENTS: usize = 1 << 12;

/// A miter longer than this many half-widths becomes a bevel, when the stroke names no limit (SVG's default).
const DEFAULT_MITER: f64 = 4.;

const TAU: f64 = core::f64::consts::TAU;

/// An affine map: (x, y) → (a·x + c·y + e, b·x + d·y + f).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affine {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl Affine {
    pub const IDENTITY: Affine = Affine { a: 1., b: 0., c: 0., d: 1., e: 0., f: 0. };

    pub fn apply(&self, p: [f64; 2]) -> [f64; 2] {
        [self.a * p[0] + self.c * p[1] + self.e, self.b * p[0] + self.d * p[1] + self.f]
    }

    /// `self` after `inner`: a point goes thru `inner` first.
    pub fn then(&self, inner: &Affine) -> Affine {
        Affine {
            a: self.a * inner.a + self.c * inner.b,
            b: self.b * inner.a + self.d * inner.b,
            c: self.a * inner.c + self.c * inner.d,
            d: self.b * inner.c + self.d * inner.d,
            e: self.a * inner.e + self.c * inner.f + self.e,
            f: self.b * inner.e + self.d * inner.f + self.f,
        }
    }

    /// The inverse, or `None` for a map that collapses the plane (a zero scale).
    pub fn inverse(&self) -> Option<Affine> {
        let det = self.a * self.d - self.b * self.c;
        if det == 0. {
            return None;
        }
        let (a, b, c, d) = (self.d / det, -self.b / det, -self.c / det, self.a / det);
        Some(Affine { a, b, c, d, e: -(a * self.e + c * self.f), f: -(b * self.e + d * self.f) })
    }

    /// How many output units one local unit spans, as the geometric mean of the two axes: what the flattening tolerance is divided by.
    fn scale(&self) -> f64 {
        sqrt((self.a * self.d - self.b * self.c).abs())
    }

    /// A node's frame in its parent's: translate · rotate · scale.
    fn of_node(node: &Node) -> Affine {
        let [tx, ty] = node.translate.as_ref().map_or([0., 0.], |t| pair(t));
        let turns = node.rotation.as_ref().map_or(0., |r| r.to_f64());
        let [sx, sy] = node.scale.as_ref().map_or([1., 1.], PerAxis::to_f64);
        let (s, c) = (sin(turns * TAU), cos(turns * TAU));
        Affine { a: c * sx, b: s * sx, c: -s * sy, d: c * sy, e: tx, f: ty }
    }

    /// The map that fits a frame region (centre, half-size) into a `width` × `height` image, centred, aspect kept.
    pub fn fit(centre: [f64; 2], half: [f64; 2], width: usize, height: usize) -> Affine {
        // The smaller of the two axis scales is the one at which the whole region fits: the definition of fitting, not a guard.
    let s = (width as f64 / (2. * half[0])).min(height as f64 / (2. * half[1]));
        Affine { a: s, b: 0., c: 0., d: s, e: width as f64 / 2. - s * centre[0], f: height as f64 / 2. - s * centre[1] }
    }
}

/// The first two numbers of a `[2]` tensor (shape enforced when the object was read or checked).
fn pair(n: &Nums) -> [f64; 2] {
    let v = n.to_f64();
    [v[0], v[1]]
}

/// Premultiplied linear VSF RGB and alpha, one per pixel, row-major.
#[derive(Debug, Clone)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<[f32; 4]>,
}

impl Image {
    pub fn new(width: usize, height: usize) -> Image {
        Image { width, height, rgba: vec![[0.; 4]; width * height] }
    }

    /// Composite over an opaque background (linear VSF RGB) and return planar `[3, h, w]` u16, white at 65535: what opsin ingests.
    pub fn planar_u16_over(&self, background: [f32; 3]) -> Vec<u16> {
        let n = self.width * self.height;
        let mut out = vec![0u16; 3 * n];
        for (i, p) in self.rgba.iter().enumerate() {
            for ch in 0..3 {
                // A float-to-int cast saturates: light past white holds at 65535 and nothing goes below 0, so no clamp is needed.
                out[ch * n + i] = ((p[ch] + background[ch] * (1. - p[3])) * 65535.) as u16;
            }
        }
        out
    }
}

/// The picture, and the kinds of object it met but does not draw.
#[derive(Debug, Clone)]
pub struct Rendered {
    pub image: Image,
    pub skipped: Vec<&'static str>,
}

/// Draw `scene` into a `width` × `height` image, `view` mapping the scene's outermost frame to pixels.
pub fn render(scene: &VsfType, width: usize, height: usize, view: Affine) -> Rendered {
    let mut out = Rendered { image: Image::new(width, height), skipped: Vec::new() };
    draw(scene, &view, &mut out.image, &mut out.skipped);
    out
}

/// Draw `scene` fitted to its own `view` (a root node's), or to `fallback` (centre, half-size) when it has none.
pub fn render_fitted(scene: &VsfType, width: usize, height: usize, fallback: ([f64; 2], [f64; 2])) -> Rendered {
    let (c, h) = match scene {
        VsfType::ron(n) => n.view.as_ref().map_or(fallback, |v| {
            let g = v.to_f64();
            ([g[0], g[1]], [g[2], g[3]])
        }),
        _ => fallback,
    };
    render(scene, width, height, Affine::fit(c, h, width, height))
}

fn draw(v: &VsfType, to_px: &Affine, img: &mut Image, skipped: &mut Vec<&'static str>) {
    match v {
        VsfType::ron(node) => draw_node(node, to_px, img, skipped),
        VsfType::rob(r) => shape(&rect_contours(r, to_px), FillRule::NonZero, &r.fill, &r.stroke, true, to_px, img),
        VsfType::roe(e) => shape(&ellipse_contours(e, to_px), FillRule::NonZero, &e.fill, &e.stroke, true, to_px, img),
        VsfType::rop(p) => {
            let (contours, closed) = path_contours(p, to_px);
            stroke_and_fill_open(&contours, &closed, p.rule.unwrap_or(FillRule::NonZero), &p.fill, &p.stroke, to_px, img);
        }
        VsfType::rol(l) => {
            let pts = points(&l.points);
            stroke_and_fill_open(&[pts], &[l.closed], FillRule::NonZero, &l.fill, &l.stroke, to_px, img);
        }
        VsfType::ror(r) => {
            let pts = nurbs_points(r, to_px);
            stroke_and_fill_open(&[pts], &[false], FillRule::NonZero, &r.fill, &r.stroke, to_px, img);
        }
        VsfType::rot(_) => skipped.push("rot text"),
        VsfType::rou(_) => skipped.push("rou button"),
        VsfType::roq(_) => skipped.push("roq text input"),
        VsfType::roi(_) => skipped.push("roi image"),
        VsfType::rof(_) => skipped.push("rof surface"),
        VsfType::roa(_) => skipped.push("roa grid"),
        _ => skipped.push("a value that is not a drawable object"),
    }
}

fn draw_node(node: &Node, parent: &Affine, img: &mut Image, skipped: &mut Vec<&'static str>) {
    let to_px = parent.then(&Affine::of_node(node));
    match &node.mask {
        None => node.children.iter().for_each(|c| draw(c, &to_px, img, skipped)),
        Some(mask) => {
            // The children draw into their own layer, which lands on the parent only where the mask covers.
            let mut layer = Image::new(img.width, img.height);
            node.children.iter().for_each(|c| draw(c, &to_px, &mut layer, skipped));
            let mut cover = Image::new(img.width, img.height);
            draw(mask, &to_px, &mut cover, skipped);
            for ((dst, src), m) in img.rgba.iter_mut().zip(&layer.rgba).zip(&cover.rgba) {
                let k = m[3];
                for ch in 0..4 {
                    dst[ch] = src[ch] * k + dst[ch] * (1. - src[3] * k);
                }
            }
        }
    }
}

/// A closed shape: fill its contours, then stroke each contour closed.
fn shape(contours: &[Vec<[f64; 2]>], rule: FillRule, fill: &Option<Paint>, stroke: &Option<Stroke>, closed: bool, to_px: &Affine, img: &mut Image) {
    let flags = vec![closed; contours.len()];
    stroke_and_fill_open(contours, &flags, rule, fill, stroke, to_px, img);
}

/// Fill (every contour taken as closed) and stroke (each closed or open as flagged), in local units.
fn stroke_and_fill_open(contours: &[Vec<[f64; 2]>], closed: &[bool], rule: FillRule, fill: &Option<Paint>, stroke: &Option<Stroke>, to_px: &Affine, img: &mut Image) {
    if let Some(p) = fill {
        let px: Vec<Vec<[f64; 2]>> = contours.iter().map(|c| c.iter().map(|p| to_px.apply(*p)).collect()).collect();
        paint(img, &px, rule, p, to_px);
    }
    if let Some(s) = stroke {
        let tol = TOLERANCE_PX / to_px.scale();
        let mut pieces = Vec::new();
        for (c, &closed) in contours.iter().zip(closed) {
            stroke_pieces(c, closed, s, tol, &mut pieces);
        }
        let px: Vec<Vec<[f64; 2]>> = pieces.iter().map(|c| c.iter().map(|p| to_px.apply(*p)).collect()).collect();
        paint(img, &px, FillRule::NonZero, &s.paint, to_px);
    }
}

// ==================== GEOMETRY → CONTOURS (local units) ====================

/// Segments for an arc of `turns` on a curve whose largest radius is `r` local units, keeping the chord within the tolerance.
fn arc_segments(r: f64, turns: f64, tol: f64) -> usize {
    if r <= tol {
        return 1;
    }
    // Chord sagitta r·(1 − cos(θ/2)) ≤ tol gives the widest step θ.
    let step = 2. * acos(1. - tol / r);
    // WHY: a zero sweep still draws its end points (one segment), and the radius and sweep come from the file, so a hostile size would otherwise ask for billions of points; past MAX_SEGMENTS the curve is coarser than the tolerance, never unbounded.
    (ceil(turns.abs() * TAU / step) as usize).clamp(1, MAX_SEGMENTS)
}

/// Points of an ellipse arc, parametric angle from `t0` turning `sweep`, both ends included.
fn ellipse_arc(c: [f64; 2], radii: [f64; 2], rot: f64, t0: f64, sweep: f64, tol: f64) -> Vec<[f64; 2]> {
    let n = arc_segments(radii[0].abs().max(radii[1].abs()), sweep, tol);
    let (rs, rc) = (sin(rot * TAU), cos(rot * TAU));
    // A polygon inscribed in a curve is short of its area by about ⅔ × sagitta × length, so every shape would shrink. Pushing the vertices out by √(Δθ / sin Δθ) makes each wedge's area equal the curve's sector: an n-gon's area is ½ n r² sin Δθ, the sector's ½ n r² Δθ.
    let step = (sweep * TAU / n as f64).abs();
    let grow = if step > 0. { sqrt(step / sin(step)) } else { 1. };
    (0..=n)
        .map(|k| {
            let t = (t0 + sweep * k as f64 / n as f64) * TAU;
            let (x, y) = (radii[0] * grow * cos(t), radii[1] * grow * sin(t));
            [c[0] + x * rc - y * rs, c[1] + x * rs + y * rc]
        })
        .collect()
}

fn ellipse_contours(e: &Ellipse, to_px: &Affine) -> Vec<Vec<[f64; 2]>> {
    let tol = TOLERANCE_PX / to_px.scale();
    let g = e.geometry.to_f64();
    let (c, outer) = ([g[0], g[1]], [g[2], g[3]]);
    let inner = e.is_ring().then(|| [g[4], g[5]]);
    let rot = e.rotation.as_ref().map_or(0., |r| r.to_f64());
    match (&e.sweep, inner) {
        // Whole ellipse, or whole ring: the inner contour runs the other way, so nonzero leaves it empty.
        (None, None) => vec![ellipse_arc(c, outer, rot, 0., 1., tol)],
        (None, Some(i)) => vec![ellipse_arc(c, outer, rot, 0., 1., tol), ellipse_arc(c, i, rot, 0., -1., tol)],
        (Some(sw), inner) => {
            let start = e.start.as_ref().map_or(0., |s| s.to_f64());
            let sweep = sw.to_f64();
            let mut contour = ellipse_arc(c, outer, rot, start, sweep, tol);
            match inner {
                Some(i) => contour.extend(ellipse_arc(c, i, rot, start + sweep, -sweep, tol)),
                None => contour.push(c),
            }
            vec![contour]
        }
    }
}

/// A box's outline: four corners in increasing angle, each a superellipse quadrant ((r − u)/r)^n, a single point when its radius is zero.
fn rect_contours(r: &Rect, to_px: &Affine) -> Vec<Vec<[f64; 2]>> {
    let tol = TOLERANCE_PX / to_px.scale();
    let g = r.geometry.to_f64();
    let (c, h) = ([g[0], g[1]], [g[2], g[3]]);
    // Radii per corner, in the wire's corner order: (−x,−y), (+x,−y), (+x,+y), (−x,+y).
    let radii: [[f64; 2]; 4] = match &r.corners {
        None => [[0.; 2]; 4],
        Some(n) => {
            let v = n.to_f64();
            match n.shape() {
                [1] => [[v[0]; 2]; 4],
                [2] => [[v[0], v[1]]; 4],
                [4] => [[v[0]; 2], [v[1]; 2], [v[2]; 2], [v[3]; 2]],
                _ => [[v[0], v[1]], [v[2], v[3]], [v[4], v[5]], [v[6], v[7]]],
            }
        }
    };
    let [nx, ny] = r.exponent.as_ref().map_or([2., 2.], PerAxis::to_f64);
    let rot = r.rotation.as_ref().map_or(0., |a| a.to_f64());
    let (rs, rc) = (sin(rot * TAU), cos(rot * TAU));
    // Corners by quadrant angle: (+,+) from 0, (−,+) from ¼, (−,−) from ½, (+,−) from ¾; each maps to its wire index.
    let quadrants: [([f64; 2], usize); 4] = [([1., 1.], 2), ([-1., 1.], 3), ([-1., -1.], 0), ([1., -1.], 1)];
    let mut pts = Vec::new();
    for (q, &(sign, idx)) in quadrants.iter().enumerate() {
        let [rx, ry] = radii[idx];
        let corner = [sign[0] * h[0], sign[1] * h[1]];
        if rx <= 0. || ry <= 0. {
            pts.push(corner);
            continue;
        }
        let centre = [corner[0] - sign[0] * rx, corner[1] - sign[1] * ry];
        // A high exponent bends sharply near the quadrant's ends; sample in proportion to the radius in pixels, more than a circle would need.
        let n = arc_segments(rx.max(ry), 1. / 4., tol).max(1 << 3) * 2;
        for k in 0..=n {
            let phi = (q as f64 + k as f64 / n as f64) * TAU / 4.;
            let (cp, sp) = (cos(phi), sin(phi));
            let x = rx * cp.signum() * pow(cp.abs(), 2. / nx);
            let y = ry * sp.signum() * pow(sp.abs(), 2. / ny);
            pts.push([centre[0] + x, centre[1] + y]);
        }
    }
    vec![pts.into_iter().map(|p| [c[0] + p[0] * rc - p[1] * rs, c[1] + p[0] * rs + p[1] * rc]).collect()]
}

fn points(n: &Nums) -> Vec<[f64; 2]> {
    n.to_f64().chunks(2).map(|p| [p[0], p[1]]).collect()
}

/// Segments for a Bézier: enough that the control polygon, cut that many times, stays within the tolerance.
fn bezier_segments(ctrl: &[[f64; 2]], tol: f64) -> usize {
    let len: f64 = ctrl
        .windows(2)
        .map(|w| {
            let (dx, dy) = (w[1][0] - w[0][0], w[1][1] - w[0][1]);
            sqrt(dx * dx + dy * dy)
        })
        .sum();
    // WHY: as in `arc_segments`, at least one segment, and no more than MAX_SEGMENTS however large the file says the curve is.
    (ceil(sqrt(len / tol)) as usize).clamp(1, MAX_SEGMENTS)
}

/// A path's subpaths, each with whether it was closed.
fn path_contours(p: &Path, to_px: &Affine) -> (Vec<Vec<[f64; 2]>>, Vec<bool>) {
    let tol = TOLERANCE_PX / to_px.scale();
    let pts = points(&p.points);
    let angles = p.angles.as_ref().map_or(Vec::new(), |a| a.to_f64());
    let (mut pi, mut ai) = (0usize, 0usize);
    let (mut contours, mut closed) = (Vec::new(), Vec::new());
    let mut cur: Vec<[f64; 2]> = Vec::new();
    let finish = |cur: &mut Vec<[f64; 2]>, contours: &mut Vec<Vec<[f64; 2]>>, closed: &mut Vec<bool>, was_closed: bool| {
        if cur.len() > 1 {
            contours.push(core::mem::take(cur));
            closed.push(was_closed);
        }
        cur.clear();
    };
    // Path::check proved the verbs take exactly the points and angles there are, and that every Line, Quad and Cubic has a current point.
    for &v in &p.verbs {
        match v {
            Verb::Move => {
                finish(&mut cur, &mut contours, &mut closed, false);
                cur.push(pts[pi]);
            }
            Verb::Line => cur.push(pts[pi]),
            Verb::Quad | Verb::Cubic => {
                let k = v.points();
                let mut ctrl = vec![*cur.last().unwrap()];
                ctrl.extend_from_slice(&pts[pi..pi + k]);
                let n = bezier_segments(&ctrl, tol);
                for s in 1..=n {
                    cur.push(bezier(&ctrl, s as f64 / n as f64));
                }
            }
            Verb::Arc => {
                let (c, r) = (pts[pi], pts[pi + 1]);
                let (rot, start, sweep) = (angles[ai], angles[ai + 1], angles[ai + 2]);
                cur.extend(ellipse_arc(c, r, rot, start, sweep, tol));
            }
            Verb::Close => finish(&mut cur, &mut contours, &mut closed, true),
        }
        pi += v.points();
        ai += v.angles();
    }
    finish(&mut cur, &mut contours, &mut closed, false);
    (contours, closed)
}

/// de Casteljau.
fn bezier(ctrl: &[[f64; 2]], t: f64) -> [f64; 2] {
    let mut p = ctrl.to_vec();
    while p.len() > 1 {
        p = p.windows(2).map(|w| [w[0][0] + (w[1][0] - w[0][0]) * t, w[0][1] + (w[1][1] - w[0][1]) * t]).collect();
    }
    p[0]
}

/// A NURBS curve sampled by de Boor in homogeneous coordinates over its whole domain.
fn nurbs_points(r: &Nurbs, to_px: &Affine) -> Vec<[f64; 2]> {
    let tol = TOLERANCE_PX / to_px.scale();
    let ctrl = points(&r.points);
    let n = ctrl.len();
    let w = r.weights.as_ref().map_or(vec![1.; n], |w| w.to_f64());
    let knots = r.knots.to_f64();
    // Nurbs::check proved 1 ≤ degree < points and knots = points + degree + 1.
    let p = r.degree.as_u64().unwrap() as usize;
    let (t0, t1) = (knots[p], knots[n]);
    let samples = bezier_segments(&ctrl, tol) * p;
    (0..=samples)
        .map(|s| {
            let t = t0 + (t1 - t0) * s as f64 / samples as f64;
            // The span k with knots[k] ≤ t < knots[k+1], held to [p, n − 1] so the curve's last point uses the last span.
            let mut k = p;
            while k + 1 < n && knots[k + 1] <= t {
                k += 1;
            }
            let mut d: Vec<[f64; 3]> = (0..=p).map(|j| {
                let i = j + k - p;
                [ctrl[i][0] * w[i], ctrl[i][1] * w[i], w[i]]
            }).collect();
            for level in 1..=p {
                for j in (level..=p).rev() {
                    let i = j + k - p;
                    let span = knots[i + p + 1 - level] - knots[i];
                    let a = if span == 0. { 0. } else { (t - knots[i]) / span };
                    for c in 0..3 {
                        d[j][c] = (1. - a) * d[j - 1][c] + a * d[j][c];
                    }
                }
            }
            [d[p][0] / d[p][2], d[p][1] / d[p][2]]
        })
        .collect()
}

// ==================== STROKES ====================

fn circle(c: [f64; 2], r: f64, tol: f64) -> Vec<[f64; 2]> {
    let mut v = ellipse_arc(c, [r, r], 0., 0., 1., tol);
    v.pop();
    v
}

/// Twice the signed area, for turning every stroke piece the same way so nonzero unions them.
fn area2(c: &[[f64; 2]]) -> f64 {
    (0..c.len()).map(|i| {
        let (p, q) = (c[i], c[(i + 1) % c.len()]);
        p[0] * q[1] - q[0] * p[1]
    }).sum()
}

fn push_piece(pieces: &mut Vec<Vec<[f64; 2]>>, mut piece: Vec<[f64; 2]>) {
    if area2(&piece) < 0. {
        piece.reverse();
    }
    pieces.push(piece);
}

/// The outline pieces of one stroked contour, in local units.
fn stroke_pieces(c: &[[f64; 2]], closed: bool, s: &Stroke, tol: f64, pieces: &mut Vec<Vec<[f64; 2]>>) {
    let hw = s.width.to_f64() / 2.;
    // Consecutive duplicates have no direction; drop them.
    let mut pts: Vec<[f64; 2]> = Vec::with_capacity(c.len());
    for &p in c {
        if pts.last() != Some(&p) {
            pts.push(p);
        }
    }
    if closed && pts.len() > 1 && pts.first() == pts.last() {
        pts.pop();
    }
    if pts.len() < 2 || hw <= 0. {
        if let (Some(&p), Some(Cap::Round)) = (pts.first(), s.cap) {
            push_piece(pieces, circle(p, hw, tol));
        }
        return;
    }
    let seg_count = if closed { pts.len() } else { pts.len() - 1 };
    let normal = |a: [f64; 2], b: [f64; 2]| {
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let len = sqrt(dx * dx + dy * dy);
        [-dy / len * hw, dx / len * hw]
    };
    for i in 0..seg_count {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        let n = normal(a, b);
        push_piece(pieces, vec![[a[0] + n[0], a[1] + n[1]], [b[0] + n[0], b[1] + n[1]], [b[0] - n[0], b[1] - n[1]], [a[0] - n[0], a[1] - n[1]]]);
    }
    // Joins at every vertex between two segments.
    let joins: Vec<usize> = if closed { (0..pts.len()).collect() } else { (1..pts.len() - 1).collect() };
    let join = s.join.unwrap_or(Join::Miter);
    let limit = s.miter.as_ref().map_or(DEFAULT_MITER, |m| m.to_f64());
    for i in joins {
        let v = pts[i];
        let prev = pts[(i + pts.len() - 1) % pts.len()];
        let next = pts[(i + 1) % pts.len()];
        let (n0, n1) = (normal(prev, v), normal(v, next));
        match join {
            Join::Round => push_piece(pieces, circle(v, hw, tol)),
            Join::Bevel | Join::Miter => {
                for side in [1., -1.] {
                    let a = [v[0] + side * n0[0], v[1] + side * n0[1]];
                    let b = [v[0] + side * n1[0], v[1] + side * n1[1]];
                    // The miter tip, where the two offset edges meet: along the bisector of the normals, hw / cos(half the turn) from the vertex.
                    let bis = [n0[0] + n1[0], n0[1] + n1[1]];
                    let bl2 = bis[0] * bis[0] + bis[1] * bis[1];
                    let miter_ok = join == Join::Miter && bl2 > 0. && {
                        let reach = 2. * hw * hw / bl2;
                        reach * sqrt(bl2) / hw <= limit
                    };
                    if miter_ok {
                        let k = 2. * hw * hw / bl2 * side;
                        push_piece(pieces, vec![v, a, [v[0] + bis[0] * k, v[1] + bis[1] * k], b]);
                    } else {
                        push_piece(pieces, vec![v, a, b]);
                    }
                }
            }
        }
    }
    if !closed {
        for (end, inward) in [(pts[0], pts[1]), (pts[pts.len() - 1], pts[pts.len() - 2])] {
            match s.cap.unwrap_or(Cap::Butt) {
                Cap::Butt => {}
                Cap::Round => push_piece(pieces, circle(end, hw, tol)),
                Cap::Square => {
                    let n = normal(inward, end);
                    let out = [n[1], -n[0]];
                    push_piece(pieces, vec![[end[0] + n[0], end[1] + n[1]], [end[0] + n[0] + out[0], end[1] + n[1] + out[1]], [end[0] - n[0] + out[0], end[1] - n[1] + out[1]], [end[0] - n[0], end[1] - n[1]]]);
                }
            }
        }
    }
}

// ==================== COVERAGE ====================

/// Coverage of a set of contours (pixel coordinates) over the pixels they touch: `cov` is `w × h` from (x0, y0).
struct Mask {
    x0: usize,
    y0: usize,
    w: usize,
    h: usize,
    cov: Vec<f32>,
}

fn coverage(contours: &[Vec<[f64; 2]>], rule: FillRule, width: usize, height: usize) -> Option<Mask> {
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for p in contours.iter().flatten() {
        for k in 0..2 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    // WHY: shapes may lie partly or wholly off the image; the mask covers only pixels that exist. The float-to-usize casts saturate negatives at 0.
    let x0 = floor(lo[0]).max(0.) as usize;
    let y0 = floor(lo[1]).max(0.) as usize;
    let x1 = (ceil(hi[0]) as usize).min(width);
    let y1 = (ceil(hi[1]) as usize).min(height);
    if x0 >= x1 || y0 >= y1 {
        return None;
    }
    let (w, h) = (x1 - x0, y1 - y0);
    let rows = h * SUBSCANLINES;
    let mut crossings: Vec<Vec<(f64, i32)>> = vec![Vec::new(); rows];
    for c in contours {
        for i in 0..c.len() {
            let (p, q) = (c[i], c[(i + 1) % c.len()]);
            if p[1] == q[1] {
                continue;
            }
            let (dir, top, bot) = if q[1] > p[1] { (1, p, q) } else { (-1, q, p) };
            // Sub-scanline j samples y = y0 + (j + ½)/S; this edge crosses those with top ≤ y < bot.
            let s = SUBSCANLINES as f64;
            // WHY: the mask spans only rows on the image; an edge reaching above or below it crosses none of the missing rows.
            let first = ceil((top[1] - y0 as f64) * s - 0.5).max(0.);
            let last = ceil((bot[1] - y0 as f64) * s - 0.5).min(rows as f64);
            let mut j = first as usize;
            while (j as f64) < last {
                let y = y0 as f64 + (j as f64 + 0.5) / s;
                let x = top[0] + (y - top[1]) * (bot[0] - top[0]) / (bot[1] - top[1]);
                crossings[j].push((x - x0 as f64, dir));
                j += 1;
            }
        }
    }
    let mut cov = vec![0f32; w * h];
    let mut diff = vec![0f32; w + 1];
    let weight = 1. / SUBSCANLINES as f32;
    for row in 0..h {
        diff.iter_mut().for_each(|d| *d = 0.);
        let out = &mut cov[row * w..(row + 1) * w];
        for sub in 0..SUBSCANLINES {
            let xs = &mut crossings[row * SUBSCANLINES + sub];
            xs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(core::cmp::Ordering::Equal));
            let mut winding = 0;
            for k in 0..xs.len() {
                let inside_before = match rule {
                    FillRule::NonZero => winding != 0,
                    FillRule::EvenOdd => winding & 1 != 0,
                };
                winding += xs[k].1;
                if inside_before && k > 0 {
                    span(out, &mut diff, xs[k - 1].0, xs[k].0, weight);
                }
            }
        }
        let mut run = 0f32;
        for (o, d) in out.iter_mut().zip(&diff) {
            run += d;
            *o += run;
        }
    }
    Some(Mask { x0, y0, w, h, cov })
}

/// Add `weight` × the covered fraction of each pixel in [xa, xb) to one row: partial pixels at the ends directly, whole ones thru the difference array.
fn span(out: &mut [f32], diff: &mut [f32], xa: f64, xb: f64, weight: f32) {
    let w = out.len() as f64;
    // WHY: a span runs off either side of the mask when its shape does; the pixels past the edge do not exist.
    let (xa, xb) = (xa.max(0.), xb.min(w));
    if xa >= xb {
        return;
    }
    let (ia, ib) = (floor(xa) as usize, floor(xb) as usize);
    if ia == ib {
        out[ia] += (xb - xa) as f32 * weight;
        return;
    }
    out[ia] += (ia as f64 + 1. - xa) as f32 * weight;
    diff[ia + 1] += weight;
    diff[ib] -= weight;
    // ib can equal the row's width when the span reaches its right edge: no partial pixel there.
    if ib < out.len() {
        out[ib] += (xb - ib as f64) as f32 * weight;
    }
}

// ==================== PAINT ====================

/// A colour value as straight linear VSF RGB and alpha.
fn colour_of(v: &VsfType) -> Option<[f32; 4]> {
    if let Some(c) = v.to_rgba_linear_f32() {
        return Some([c.r, c.g, c.b, c.a]);
    }
    v.to_rgb_linear_f32().map(|c| [c.r, c.g, c.b, 1.])
}

fn premultiply(c: [f32; 4]) -> [f32; 4] {
    [c[0] * c[3], c[1] * c[3], c[2] * c[3], c[3]]
}

/// Fill contours (pixel coordinates) with paint, source-over. `to_px` maps the shape's frame to pixels, for a gradient's geometry.
fn paint(img: &mut Image, contours: &[Vec<[f64; 2]>], rule: FillRule, p: &Paint, to_px: &Affine) {
    let Some(m) = coverage(contours, rule, img.width, img.height) else { return };
    let solid = match p {
        Paint::Colour(c) => match colour_of(c) {
            Some(c) => Some(premultiply(c)),
            None => return,
        },
        Paint::Gradient(_) => None,
    };
    let gradient = match p {
        Paint::Gradient(g) => to_px.inverse().map(|inv| (g.as_ref(), inv, stops(g))),
        Paint::Colour(_) => None,
    };
    for row in 0..m.h {
        for col in 0..m.w {
            let c = m.cov[row * m.w + col];
            if c <= 0. {
                continue;
            }
            let (x, y) = (m.x0 + col, m.y0 + row);
            let src = match (&solid, &gradient) {
                (Some(s), _) => *s,
                (None, Some((g, inv, st))) => premultiply(gradient_at(g, st, inv.apply([x as f64 + 0.5, y as f64 + 0.5]))),
                (None, None) => return,
            };
            let dst = &mut img.rgba[y * img.width + x];
            for ch in 0..4 {
                dst[ch] = src[ch] * c + dst[ch] * (1. - src[3] * c);
            }
        }
    }
}

fn stops(g: &Gradient) -> Vec<(f64, [f32; 4])> {
    g.offsets.to_f64().into_iter().zip(&g.colours).filter_map(|(o, c)| colour_of(c).map(|c| (o, c))).collect()
}

/// The gradient's straight colour at a point in its frame. Past the first and last stops the end colours extend.
fn gradient_at(g: &Gradient, stops: &[(f64, [f32; 4])], p: [f64; 2]) -> [f32; 4] {
    let v = g.geometry.to_f64();
    let angle = g.angle.as_ref().map_or(0., |a| a.to_f64());
    let t = match g.kind {
        GradientKind::Linear => {
            let (d0, d1) = (v[2] - v[0], v[3] - v[1]);
            let l2 = d0 * d0 + d1 * d1;
            if l2 == 0. { 0. } else { ((p[0] - v[0]) * d0 + (p[1] - v[1]) * d1) / l2 }
        }
        GradientKind::Radial => {
            let (dx, dy) = (p[0] - v[0], p[1] - v[1]);
            let (s, c) = (sin(-angle * TAU), cos(-angle * TAU));
            let (x, y) = ((dx * c - dy * s) / v[2], (dx * s + dy * c) / v[3]);
            sqrt(x * x + y * y)
        }
        GradientKind::Conic => {
            let t = atan2(p[1] - v[1], p[0] - v[0]) / TAU - angle;
            t - floor(t)
        }
    };
    let Some(first) = stops.first() else { return [0.; 4] };
    if t <= first.0 {
        return first.1;
    }
    for w in stops.windows(2) {
        if t <= w[1].0 {
            let span = w[1].0 - w[0].0;
            let f = if span == 0. { 1. } else { ((t - w[0].0) / span) as f32 };
            return core::array::from_fn(|ch| w[0].1[ch] + (w[1].1[ch] - w[0].1[ch]) * f);
        }
    }
    stops[stops.len() - 1].1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::renderable::{Num, Nums, Paint};

    fn white() -> Option<Paint> {
        Some(Paint::colour(VsfType::rcw).unwrap())
    }

    fn total_alpha(img: &Image) -> f64 {
        img.rgba.iter().map(|p| p[3] as f64).sum()
    }

    /// A filled circle's coverage sums to its area within the flattening and sub-scanline error.
    #[test]
    fn circle_area() {
        let e = Ellipse { geometry: Nums::centre_half_f32([32., 32.], [20., 20.]), rotation: None, start: None, sweep: None, fill: white(), stroke: None };
        let r = render(&VsfType::roe(Box::new(e)), 64, 64, Affine::IDENTITY);
        let area = core::f64::consts::PI * 400.;
        assert!((total_alpha(&r.image) - area).abs() < area / 256., "{} vs {area}", total_alpha(&r.image));
    }

    /// A ring segment of a quarter turn covers a quarter of the annulus.
    #[test]
    fn ring_segment_area() {
        let e = Ellipse { geometry: Nums::f32(vec![3, 2], vec![64., 64., 60., 60., 30., 30.]).unwrap(), rotation: None, start: Some(Num::from(0.125f32)), sweep: Some(Num::from(0.25f32)), fill: white(), stroke: None };
        let r = render(&VsfType::roe(Box::new(e)), 128, 128, Affine::IDENTITY);
        let area = core::f64::consts::PI * (3600. - 900.) / 4.;
        assert!((total_alpha(&r.image) - area).abs() < area / 256., "{} vs {area}", total_alpha(&r.image));
    }

    /// A whole ring leaves its hole empty.
    #[test]
    fn ring_hole_is_empty() {
        let e = Ellipse { geometry: Nums::f32(vec![3, 2], vec![32., 32., 30., 30., 15., 15.]).unwrap(), rotation: None, start: None, sweep: None, fill: white(), stroke: None };
        let r = render(&VsfType::roe(Box::new(e)), 64, 64, Affine::IDENTITY);
        assert_eq!(r.image.rgba[32 * 64 + 32][3], 0.);
        assert!(r.image.rgba[32 * 64 + 32 + 22][3] > 0.99);
    }

    /// An axis-aligned box on pixel edges covers exactly its pixels.
    #[test]
    fn box_is_exact() {
        let b = Rect { geometry: Nums::centre_half_f32([8., 8.], [4., 2.]), rotation: None, corners: None, exponent: None, fill: white(), stroke: None };
        let r = render(&VsfType::rob(Box::new(b)), 16, 16, Affine::IDENTITY);
        assert_eq!(total_alpha(&r.image), 32.);
        assert_eq!(r.image.rgba[8 * 16 + 8][3], 1.);
        assert_eq!(r.image.rgba[0][3], 0.);
    }

    /// Even-odd leaves the overlap of two squares empty; nonzero fills it.
    #[test]
    fn fill_rules() {
        let pts = Nums::points_f32(&[[0., 0.], [8., 0.], [8., 8.], [0., 8.], [4., 4.], [12., 4.], [12., 12.], [4., 12.]]);
        let verbs = vec![Verb::Move, Verb::Line, Verb::Line, Verb::Line, Verb::Close, Verb::Move, Verb::Line, Verb::Line, Verb::Line, Verb::Close];
        for (rule, expect) in [(FillRule::NonZero, 112.), (FillRule::EvenOdd, 96.)] {
            let p = Path { points: pts.clone(), verbs: verbs.clone(), angles: None, rule: Some(rule), fill: white(), stroke: None };
            let r = render(&VsfType::rop(Box::new(p)), 16, 16, Affine::IDENTITY);
            assert_eq!(total_alpha(&r.image), expect, "{rule:?}");
        }
    }

    /// A node's frame moves its children: a scale of ½ and a translate put a 16-unit box in 8 pixels.
    #[test]
    fn node_frames() {
        let b = Rect { geometry: Nums::centre_half_f32([0., 0.], [8., 8.]), rotation: None, corners: None, exponent: None, fill: white(), stroke: None };
        let mut n = Node::new();
        n.translate = Some(Nums::f32(vec![2], vec![8., 8.]).unwrap());
        n.scale = Some(PerAxis::Both(Num::from(0.5f32)));
        n.children.push(VsfType::rob(Box::new(b)));
        let r = render(&VsfType::ron(Box::new(n)), 16, 16, Affine::IDENTITY);
        assert_eq!(total_alpha(&r.image), 64.);
    }

    /// A butt-capped stroke along a line covers width × length.
    #[test]
    fn stroke_area() {
        let s = Stroke { width: Num::from(2f32), paint: Paint::colour(VsfType::rcw).unwrap(), join: None, cap: None, miter: None };
        let l = Polyline { points: Nums::points_f32(&[[2., 8.], [14., 8.]]), closed: false, fill: None, stroke: Some(s) };
        let r = render(&VsfType::rol(Box::new(l)), 16, 16, Affine::IDENTITY);
        assert_eq!(total_alpha(&r.image), 24.);
    }
}
