//! WorldCell: Morton-coded Dymaxion coordinate — the hierarchical sibling of
//! [`WorldCoord`](super::world_coord::WorldCoord).
//!
//! Layout (MSB first): `[4 bits diamond 0–9][60 bits Morton(u, v)]`. The icosahedron's 20 faces are paired into 10 rhombic diamonds; face-local (u, v) is quantized to 2^30 steps per axis (~6.6 mm of ground) and interleaved from the MSBs down, u in the higher bit of each pair.
//!
//! The two families trade differently:
//! - **WorldCoord** (dense divmod): maximum precision per bit (~5.2 mm at
//!   64 bits), but widths are independent re-encodes — nothing nests.
//! - **WorldCell** (Morton): ~26% coarser (~6.6 mm at 64 bits), but every width nests by TRUNCATION: dropping 2k low bits yields the enclosing cell 2^k coarser, a tile address is a bit prefix, containment is a shift-and-compare, and the 128-bit extension's top 64 bits ARE the
//!   64-bit value.
//!
//! Wire: `wm` + size digit `3`–`7` + 2^(N-3) bytes, truncation-based. Coarse widths carry the top bits; decode re-centers (sets the next bit of each axis) so a degraded point reads as its cell's center rather than biased to the corner.
//!
//! Orientation, datum and vertex tables are shared verbatim with `WorldCoord` and mahere-coord (pole-vertex, Greenwich-aligned, geodetic lat/lon treated as spherical).

use crate::prelude::*;

#[cfg(not(feature = "std"))]
use num_traits::Float;

use super::world_coord::{lat_lon_to_xyz, xyz_to_lat_lon, VERTICES};

/// The 10 diamonds as vertex indices `[a0, q, r, b0]`: lower triangle (a0, q, r) and upper triangle (b0, q, r) share edge q–r. UV basis: origin a0, u along a0→q, v along a0→r; b0 at (1, 1). Identical to mahere-coord's table.
const DIAMONDS: [[usize; 4]; 10] = [
    [0, 1, 2, 6],
    [0, 2, 3, 7],
    [0, 3, 4, 8],
    [0, 4, 5, 9],
    [0, 1, 5, 10],
    [11, 6, 7, 2],
    [11, 7, 8, 3],
    [11, 8, 9, 4],
    [11, 9, 10, 5],
    [11, 6, 10, 1],
];

/// Maximum quadtree depth: one UV step per coordinate unit.
pub const MAX_DEPTH: u8 = 30;
const UV_BITS: u32 = 30;
const UV_STEPS: u64 = 1 << UV_BITS;
const MORTON_BITS: u32 = 60;

/// A Morton-coded Dymaxion coordinate at ~6.6 mm resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldCell(u64);

impl WorldCell {
    pub fn from_raw(raw: u64) -> Self {
        WorldCell(raw)
    }

    pub fn raw(&self) -> u64 {
        self.0
    }

    /// Encode WGS84 geodetic latitude/longitude (degrees), treated as spherical (same datum rule as `WorldCoord`).
    pub fn from_lat_lon(lat: f64, lon: f64) -> Self {
        let (x, y, z) = lat_lon_to_xyz(lat, lon);
        Self::from_xyz(x, y, z)
    }

    /// Decode to WGS84 geodetic (latitude, longitude) in degrees — the center of the finest cell.
    pub fn to_lat_lon(&self) -> (f64, f64) {
        let (x, y, z) = self.to_xyz();
        xyz_to_lat_lon(x, y, z)
    }

    /// Encode a direction from the sphere's center (need not be unit).
    pub fn from_xyz(x: f64, y: f64, z: f64) -> Self {
        let (diamond, u, v) = xyz_to_diamond_uv(x, y, z);
        let iu = quantize(u, UV_STEPS);
        let iv = quantize(v, UV_STEPS);
        WorldCell(((diamond as u64) << MORTON_BITS) | (spread(iu) << 1) | spread(iv))
    }

    /// Decode to a unit vector (the center of the finest cell).
    pub fn to_xyz(&self) -> (f64, f64, f64) {
        let (iu, iv) = self.uv();
        let u = (iu as f64 + 0.5) / UV_STEPS as f64;
        let v = (iv as f64 + 0.5) / UV_STEPS as f64;
        diamond_uv_to_xyz(self.diamond(), u, v)
    }

    /// Diamond ID, 0–9.
    pub fn diamond(&self) -> u8 {
        (self.0 >> MORTON_BITS) as u8
    }

    /// Face-local (u, v), each 0..2^30.
    pub fn uv(&self) -> (u64, u64) {
        (compact(self.0 >> 1), compact(self.0))
    }

    /// The enclosing cell at `depth` (0..=30), corner-anchored: the low bits are zeroed. Truncation IS spatial coarsening.
    pub fn truncate(&self, depth: u8) -> WorldCell {
        WorldCell(self.0 & prefix_mask(depth))
    }

    /// The enclosing cell's center at full resolution: truncate, then set the next bit of each axis — the re-centering rule for degraded points (uniform ±half-cell error instead of corner bias).
    pub fn center_at(&self, depth: u8) -> WorldCell {
        if depth >= MAX_DEPTH {
            return *self;
        }
        WorldCell(
            (self.0 & prefix_mask(depth)) | (0b11 << (MORTON_BITS - 2 * (depth as u32 + 1))),
        )
    }

    /// True if both coordinates lie in the same cell at `depth`: one mask and compare.
    pub fn same_cell(&self, other: &WorldCell, depth: u8) -> bool {
        (self.0 ^ other.0) & prefix_mask(depth) == 0
    }

    /// Wire bytes: `wm` tag is added by the flattener; this returns the size digit's payload. Width `'6'` is the full value; `'3'`/`'4'`/ `'5'` truncate to the top 8/16/32 bits (cells of ~1,760 km / ~110 km / ~430 m); `'7'` zero-extends to 128 bits (low bits reserved for [`encode128`]-precision inputs).
    pub fn wire_bytes(&self, size_byte: u8) -> Option<Vec<u8>> {
        let raw = self.0;
        Some(match size_byte {
            b'3' => vec![(raw >> 56) as u8],
            b'4' => ((raw >> 48) as u16).to_be_bytes().to_vec(),
            b'5' => ((raw >> 32) as u32).to_be_bytes().to_vec(),
            b'6' => raw.to_be_bytes().to_vec(),
            b'7' => (((raw as u128) << 64).to_be_bytes()).to_vec(),
            _ => return None,
        })
    }

    /// Decode wire bytes at a width. Coarse widths re-center: the result is the transmitted cell's center at full resolution.
    pub fn from_wire(size_byte: u8, bytes: &[u8]) -> Option<Self> {
        let (raw, depth) = match size_byte {
            b'3' if !bytes.is_empty() => ((bytes[0] as u64) << 56, 2u8),
            b'4' if bytes.len() >= 2 => {
                ((u16::from_be_bytes([bytes[0], bytes[1]]) as u64) << 48, 6)
            }
            b'5' if bytes.len() >= 4 => (
                (u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as u64) << 32,
                14,
            ),
            b'6' if bytes.len() >= 8 => (
                u64::from_be_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                    bytes[7],
                ]),
                MAX_DEPTH,
            ),
            b'7' if bytes.len() >= 16 => (
                u64::from_be_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                    bytes[7],
                ]),
                MAX_DEPTH,
            ), // low 64 bits beyond u64 resolution; ignored today
            _ => return None,
        };
        let cell = WorldCell(raw);
        Some(if depth < MAX_DEPTH { cell.center_at(depth) } else { cell })
    }
}

/// 128-bit encode: `[4 bits diamond][124 bits Morton]`, 62 bits per axis.
/// The top 64 bits are EXACTLY the [`WorldCell`] encoding (power-of-two quantization commutes with truncation). Note f64 inputs carry ~52 significant bits, so depths beyond ~52 quantize rounding noise; the layout is exact, the geometry is input-limited.
pub fn encode128(lat: f64, lon: f64) -> u128 {
    let (x, y, z) = lat_lon_to_xyz(lat, lon);
    let (diamond, u, v) = xyz_to_diamond_uv(x, y, z);
    let iu = quantize(u, 1 << 62);
    let iv = quantize(v, 1 << 62);
    ((diamond as u128) << 124) | (spread128(iu) << 1) | spread128(iv)
}

/// Inverse of [`encode128`]: the finest-cell center as (lat, lon) degrees.
pub fn decode128(code: u128) -> (f64, f64) {
    let diamond = (code >> 124) as u8;
    let iu = compact128(code >> 1);
    let iv = compact128(code);
    let u = (iu as f64 + 0.5) / (1u64 << 62) as f64;
    let v = (iv as f64 + 0.5) / (1u64 << 62) as f64;
    let (x, y, z) = diamond_uv_to_xyz(diamond, u, v);
    xyz_to_lat_lon(x, y, z)
}

// ==================== GEOMETRY ====================

/// Project a direction onto its containing diamond: (diamond, u, v) with u, v in [0, 1] — lower triangle u + v <= 1, upper above.
fn xyz_to_diamond_uv(x: f64, y: f64, z: f64) -> (u8, f64, f64) {
    let mag = (x * x + y * y + z * z).sqrt();
    let p = [x / mag, y / mag, z / mag];

    // Closest face among the 20 diamond halves (spherical Voronoi of face
    // centroids = radial face projection for a regular icosahedron).
    let mut best = (0usize, false);
    let mut best_dot = f64::NEG_INFINITY;
    for (di, [a0, q, r, b0]) in DIAMONDS.iter().enumerate() {
        for (upper, f0) in [(false, *a0), (true, *b0)] {
            let (va, vb, vc) = (VERTICES[f0], VERTICES[*q], VERTICES[*r]);
            let cx = va[0] + vb[0] + vc[0];
            let cy = va[1] + vb[1] + vc[1];
            let cz = va[2] + vb[2] + vc[2];
            let dot = p[0] * cx + p[1] * cy + p[2] * cz;
            if dot > best_dot {
                best_dot = dot;
                best = (di, upper);
            }
        }
    }
    let (di, upper) = best;
    let [a0, q, r, b0] = DIAMONDS[di];
    let f0 = if upper { b0 } else { a0 };
    let (s, t) = gnomonic_barycentric(p, VERTICES[f0], VERTICES[q], VERTICES[r]);
    // Lower: UV = (s, t). Upper (b0, q, r): u = 1 - t, v = 1 - s keeps the parametrization continuous across the shared diagonal.
    let (u, v) = if upper { (1.0 - t, 1.0 - s) } else { (s, t) };
    (di as u8, u, v)
}

/// Reconstruct a unit vector from diamond UV.
fn diamond_uv_to_xyz(diamond: u8, u: f64, v: f64) -> (f64, f64, f64) {
    let [a0, q, r, b0] = DIAMONDS[diamond as usize];
    let (w0, wq, wr, origin) = if u + v <= 1.0 {
        (1.0 - u - v, u, v, VERTICES[a0])
    } else {
        (u + v - 1.0, 1.0 - v, 1.0 - u, VERTICES[b0])
    };
    let (vq, vr) = (VERTICES[q], VERTICES[r]);
    let x = w0 * origin[0] + wq * vq[0] + wr * vr[0];
    let y = w0 * origin[1] + wq * vq[1] + wr * vr[1];
    let z = w0 * origin[2] + wq * vq[2] + wr * vr[2];
    let mag = (x * x + y * y + z * z).sqrt();
    (x / mag, y / mag, z / mag)
}

/// Gnomonic projection of `p` onto the triangle's plane, as edge-basis barycentric (s, t): fp = A + s·(B−A) + t·(C−A).
fn gnomonic_barycentric(p: [f64; 3], a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> (f64, f64) {
    let v0 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v1 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let n = [
        v0[1] * v1[2] - v0[2] * v1[1],
        v0[2] * v1[0] - v0[0] * v1[2],
        v0[0] * v1[1] - v0[1] * v1[0],
    ];
    let scale = (n[0] * a[0] + n[1] * a[1] + n[2] * a[2])
        / (n[0] * p[0] + n[1] * p[1] + n[2] * p[2]);
    let fp = [p[0] * scale, p[1] * scale, p[2] * scale];
    let v2 = [fp[0] - a[0], fp[1] - a[1], fp[2] - a[2]];
    let d00 = v0[0] * v0[0] + v0[1] * v0[1] + v0[2] * v0[2];
    let d01 = v0[0] * v1[0] + v0[1] * v1[1] + v0[2] * v1[2];
    let d11 = v1[0] * v1[0] + v1[1] * v1[1] + v1[2] * v1[2];
    let d20 = v2[0] * v0[0] + v2[1] * v0[1] + v2[2] * v0[2];
    let d21 = v2[0] * v1[0] + v2[1] * v1[1] + v2[2] * v1[2];
    let denom = d00 * d11 - d01 * d01;
    (
        (d11 * d20 - d01 * d21) / denom,
        (d00 * d21 - d01 * d20) / denom,
    )
}

// ==================== BIT MANIPULATION ====================

fn prefix_mask(depth: u8) -> u64 {
    if depth as u32 >= MAX_DEPTH as u32 {
        !0
    } else {
        !0 << (MORTON_BITS - 2 * depth as u32)
    }
}

// WHY/PROOF: u or v is exactly 1 on a diamond's far edge, giving steps, one past the last cell (and rounding can land a hair outside 0 to 1) — without this the index carries into the bit above the axis and spread drops it, wrapping the point to the opposite edge.
fn quantize(x: f64, steps: u64) -> u64 {
    ((x * steps as f64) as i64).clamp(0, steps as i64 - 1) as u64
}

/// Spread the low 30 bits of `x` into the even bit positions.
fn spread(x: u64) -> u64 {
    let mut x = x & 0x3FFF_FFFF;
    x = (x | (x << 16)) & 0x0000_FFFF_0000_FFFF;
    x = (x | (x << 8)) & 0x00FF_00FF_00FF_00FF;
    x = (x | (x << 4)) & 0x0F0F_0F0F_0F0F_0F0F;
    x = (x | (x << 2)) & 0x3333_3333_3333_3333;
    x = (x | (x << 1)) & 0x5555_5555_5555_5555;
    x
}

/// Inverse of [`spread`].
fn compact(x: u64) -> u64 {
    let mut x = x & 0x5555_5555_5555_5555;
    x = (x | (x >> 1)) & 0x3333_3333_3333_3333;
    x = (x | (x >> 2)) & 0x0F0F_0F0F_0F0F_0F0F;
    x = (x | (x >> 4)) & 0x00FF_00FF_00FF_00FF;
    x = (x | (x >> 8)) & 0x0000_FFFF_0000_FFFF;
    x = (x | (x >> 16)) & 0x0000_0000_FFFF_FFFF;
    x & 0x3FFF_FFFF
}

/// Spread up to 62 bits into the even positions of a u128, by spreading each 31-bit half through the standard 32-bit spreader (positional, so the halves combine by shift: bit 62 is even, continuing the pattern).
fn spread128(x: u64) -> u128 {
    let low = spread32(x & 0x7FFF_FFFF);
    let high = spread32((x >> 31) & 0x7FFF_FFFF);
    ((high as u128) << 62) | low as u128
}

/// Standard 32-bit Morton spread (admits up to 32 input bits).
fn spread32(x: u64) -> u64 {
    let mut x = x & 0xFFFF_FFFF;
    x = (x | (x << 16)) & 0x0000_FFFF_0000_FFFF;
    x = (x | (x << 8)) & 0x00FF_00FF_00FF_00FF;
    x = (x | (x << 4)) & 0x0F0F_0F0F_0F0F_0F0F;
    x = (x | (x << 2)) & 0x3333_3333_3333_3333;
    x = (x | (x << 1)) & 0x5555_5555_5555_5555;
    x
}

fn compact128(x: u128) -> u64 {
    let low = compact32((x & 0x3FFF_FFFF_FFFF_FFFF) as u64);
    let high = compact32(((x >> 62) & 0x3FFF_FFFF_FFFF_FFFF) as u64);
    (high << 31) | low
}

/// Standard 32-bit Morton compact (inverse of [`spread32`]).
fn compact32(x: u64) -> u64 {
    let mut x = x & 0x5555_5555_5555_5555;
    x = (x | (x >> 1)) & 0x3333_3333_3333_3333;
    x = (x | (x >> 2)) & 0x0F0F_0F0F_0F0F_0F0F;
    x = (x | (x >> 4)) & 0x00FF_00FF_00FF_00FF;
    x = (x | (x >> 8)) & 0x0000_FFFF_0000_FFFF;
    x = (x | (x >> 16)) & 0x0000_0000_FFFF_FFFF;
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    const EARTH_RADIUS_M: f64 = 6_371_000.0;

    fn ground_error_m(lat: f64, lon: f64, cell: WorldCell) -> f64 {
        let (px, py, pz) = lat_lon_to_xyz(lat, lon);
        let (qx, qy, qz) = cell.to_xyz();
        let (dx, dy, dz) = (px - qx, py - qy, pz - qz);
        (dx * dx + dy * dy + dz * dz).sqrt() * EARTH_RADIUS_M
    }

    #[test]
    fn global_grid_round_trips_under_two_centimeters() {
        let mut worst: f64 = 0.0;
        let mut lat = -89.5;
        while lat <= 89.5 {
            let mut lon = -180.0;
            while lon < 180.0 {
                let c = WorldCell::from_lat_lon(lat, lon);
                worst = worst.max(ground_error_m(lat, lon, c));
                lon += 3.7;
            }
            lat += 2.9;
        }
        assert!(worst < 0.02, "worst round-trip error {worst} m");
    }

    #[test]
    fn truncation_is_containment() {
        let a = WorldCell::from_lat_lon(47.6062, -122.3321);
        let b = WorldCell::from_lat_lon(47.6063, -122.3322); // ~13 m away
        // Same coarse cells, distinct fine cells, nested all the way down.
        assert!(a.same_cell(&b, 8));
        assert!(!a.same_cell(&b, 30));
        for depth in 0..30 {
            assert_eq!(a.truncate(depth).raw() & prefix_mask(depth + 1) & prefix_mask(depth), a.truncate(depth).raw());
            assert!(a.truncate(depth).raw() <= a.truncate(depth + 1).raw());
            assert!(a.same_cell(&a.center_at(depth), depth));
        }
    }

    #[test]
    fn morton_spread_round_trips_including_128() {
        for x in [0u64, 1, 0x3FFF_FFFF, 0x2AAA_AAAA, 123_456_789] {
            assert_eq!(compact(spread(x)), x);
        }
        for x in [0u64, 1, (1 << 62) - 1, 0x2AAA_AAAA_AAAA_AAAA & ((1 << 62) - 1), 987_654_321_987] {
            assert_eq!(compact128(spread128(x)), x, "x = {x:#x}");
        }
    }

    /// The 128-bit encoding's top 64 bits are exactly the 64-bit encoding (power-of-two quantization commutes with truncation).
    #[test]
    fn u128_top_bits_are_the_u64_encoding() {
        for (lat, lon) in [
            (47.6062, -122.3321),
            (-41.3, 174.8),
            (0.0, 0.0),
            (89.9, 10.0),
            (-33.0, 151.0),
            (26.565051, 0.0),
        ] {
            let c64 = WorldCell::from_lat_lon(lat, lon);
            let c128 = encode128(lat, lon);
            assert_eq!((c128 >> 64) as u64, c64.raw(), "at ({lat}, {lon})");
            // and decode128 lands within the 64-bit cell
            let (dlat, dlon) = decode128(c128);
            let back = WorldCell::from_lat_lon(dlat, dlon);
            assert!(c64.same_cell(&back, 29), "decode128 strayed at ({lat}, {lon})");
        }
    }

    #[test]
    fn wire_widths_truncate_and_recenter() {
        let c = WorldCell::from_lat_lon(47.6062, -122.3321);
        // (size, depth carried, allowed error ~ cell diagonal with stretch)
        for (size, tol_m) in [
            (b'3', 2_600_000.0), // depth 2: ~1,760 km cells
            (b'4', 170_000.0),   // depth 6: ~110 km cells
            (b'5', 700.0),       // depth 14: ~430 m cells
        ] {
            let wire = c.wire_bytes(size).unwrap();
            let back = WorldCell::from_wire(size, &wire).unwrap();
            let (lat, lon) = c.to_lat_lon();
            let err = ground_error_m(lat, lon, back);
            assert!(err < tol_m, "wm{}: {} m", size as char, err);
        }
        let wire = c.wire_bytes(b'6').unwrap();
        assert_eq!(WorldCell::from_wire(b'6', &wire).unwrap().raw(), c.raw());
        let wire = c.wire_bytes(b'7').unwrap();
        assert_eq!(wire.len(), 16);
        assert_eq!(WorldCell::from_wire(b'7', &wire).unwrap().raw(), c.raw());
    }

    /// WorldCell and WorldCoord agree on the ground: converting through the sphere lands within the sum of their quantization radii.
    #[test]
    fn agrees_with_dense_worldcoord() {
        use super::super::world_coord::WorldCoord;
        for (lat, lon) in [(47.6062, -122.3321), (-41.3, 174.8), (26.565051, 0.0)] {
            let cell = WorldCell::from_lat_lon(lat, lon);
            let coord = WorldCoord::from_lat_lon(lat, lon);
            let (ax, ay, az) = cell.to_xyz();
            let (bx, by, bz) = coord.to_xyz();
            let d = ((ax - bx).powi(2) + (ay - by).powi(2) + (az - bz).powi(2)).sqrt()
                * EARTH_RADIUS_M;
            assert!(d < 0.02, "families disagree by {d} m at ({lat}, {lon})");
        }
    }
}
