//! When each part of the format took the form this build writes — the source of a file's `y`.
//!
//! `y` in a header is the oldest reader that reads that file correctly. A writer works it out from what the file actually holds: the highest [`VsfType::since`] across every value it writes, and [`HEADER_SINCE`] for the header and section framing around them. A file that uses nothing newer than v9 says `y9` however new its writer is, so a v9 reader opens it; a file holding one v11 type says `y11`, so a v10 reader refuses it by name instead of misreading it.
//!
//! A version here is when a type's **current** bytes and meaning arrived, not when the type first existed. Tracking starts at v9: everything that existed by then is marked 9, because earlier eras changed bytes (the spirix payload break at v9, the v7 `x` codebook and `l`→`a` remaps) and a pre-v9 reader cannot read today's header at all.
//!
//! What raises a version:
//! - **New bytes** — a new type, or a changed layout of an existing one. VSF values are not all length-prefixed, so an old reader cannot skip a type it does not know; it would misparse everything after it. `tests/since.rs` pins every type's bytes, and refuses a change unless that type's version here rises with it.
//! - **New meaning in the same bytes** — the dymaxion re-orientation, where a `w` parses identically but names another place on Earth. No byte test can see this; raise the version by hand when a meaning changes.
//!
//! What does not: a new encoding byte inside a `v` (length-prefixed, so an old reader carries it as an opaque blob and knows it cannot decode it), and new section or field names (self-delimiting, so an old reader skips them). A reader that skips those reads less, but reads nothing wrong.
//!
//! Adding a variant to [`VsfType`] without a line in the table below is a compile error: the match is exhaustive.

use super::VsfType;
use crate::file_format::{VsfField, VsfSection};

/// The header, table of contents, section framing, generation header and trailer as this build writes them.
pub const HEADER_SINCE: usize = 9;

/// The oldest reader version a value needs, and the type that sets it (the value's own, or a nested one's).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Since {
    pub version: usize,
    /// Type marker as the variant names it: `"s53"`, `"t_f5"`, `"header"`.
    pub by: &'static str,
}

impl Since {
    /// The header and framing alone — the floor of a file that holds no values.
    pub const HEADER: Since = Since { version: HEADER_SINCE, by: "header" };

    /// The later of two; on a tie, `self` (so a container reports itself rather than an equally old child).
    pub fn max(self, other: Since) -> Since {
        if other.version > self.version {
            other
        } else {
            self
        }
    }
}

/// One line per type: `marker => version`, the version in which its current wire form and meaning arrived. Spirix types are listed separately because they exist only with that feature.
macro_rules! since_table {
    (
        always { $($v:ident => $ver:literal),* $(,)? }
        spirix { $($sv:ident => $sver:literal),* $(,)? }
    ) => {
        impl VsfType {
            /// This value's own type: its version and marker, not counting nested values.
            pub fn own_since(&self) -> Since {
                match self {
                    $( VsfType::$v { .. } => Since { version: $ver, by: stringify!($v) }, )*
                    $( #[cfg(feature = "spirix")] VsfType::$sv { .. } => Since { version: $sver, by: stringify!($sv) }, )*
                }
            }
        }

        /// Every type marker in the table with its version — what `tests/since.rs` checks its samples against.
        pub const SINCE_TABLE: &[(&str, usize)] = &[
            $( (stringify!($v), $ver), )*
            $( #[cfg(feature = "spirix")] (stringify!($sv), $sver), )*
        ];
    };
}

since_table! {
    always {
        // Structure and header
        d => 9, o => 9, b => 9, l => 9, n => 9, z => 9, y => 9,
        // Network
        na => 9, nh => 9, ni => 9, nj => 9, nc => 9, nm => 9, np => 9, ns => 9, nu => 9, nn => 9,
        // Hashes
        hp => 9, hb => 9, hs => 9, hm => 9, hg => 9, hc => 9, hk => 9, hP => 9, hR => 9, hI => 9, hV => 9,
        // Signatures
        ge => 9, gm => 9, gp => 9, gd => 9, gs => 9, gf => 9, gH => 9, gr => 9,
        // Keys. kvr and kgr arrived after 0.10.2 shipped and are still marked 10 (Nick, 2026-10-28: not worth a version number for keys only a bootloader uses). 0.10.0–0.10.2 readers refuse a file holding one with an unknown-key error; they never misread it.
        ke => 9, kx => 9, kp => 9, kk => 9, kc => 9, ka => 9, km => 9, kf => 9, kl => 9, kn => 9, kh => 9, kd => 9, kb => 9,
        kvf => 9, kvs => 9, kvr => 10, kgr => 10, ksx => 9, ksp => 9, ksk => 9, ksf => 9, ksn => 9, ksl => 9, ksh => 9, ksm => 9,
        // MACs
        mh => 9, mp => 9, mb => 9, mc => 9,
        // Numbers
        u0 => 9, u => 9, u3 => 9, u4 => 9, u5 => 9, u6 => 9, u7 => 9,
        i => 9, i3 => 9, i4 => 9, i5 => 9, i6 => 9, i7 => 9,
        f5 => 9, f6 => 9, j5 => 9, j6 => 9,
        // Text, time, place
        x => 9, a => 9, e => 9, w => 9, wm => 9, wa => 9,
        // Wrapped data
        v => 9,
        // Colour
        r => 9,
        rcb => 9, rcc => 9, rcg => 9, rcj => 9, rck => 9, rcl => 9, rcn => 9, rco => 9, rcq => 9, rcr => 9, rcv => 9, rcw => 9, rcy => 9,
        re => 9, rx => 9, rz => 9, ri => 9, rp => 9, ru => 9, rs => 9, rf => 9, ra => 9, rt => 9, rh => 9, rm => 9,
        // Contiguous tensors (a 1-D tensor's compact form `tn` included)
        t_u0 => 9, t_u3 => 9, t_u4 => 9, t_u5 => 9, t_u6 => 9, t_u7 => 9,
        t_i3 => 9, t_i4 => 9, t_i5 => 9, t_i6 => 9, t_i7 => 9,
        t_f5 => 9, t_f6 => 9, t_j5 => 9, t_j6 => 9,
        // Strided tensors
        q_u0 => 9, q_u3 => 9, q_u4 => 9, q_u5 => 9, q_u6 => 9, q_u7 => 9,
        q_i3 => 9, q_i4 => 9, q_i5 => 9, q_i6 => 9, q_i7 => 9,
        q_f5 => 9, q_f6 => 9, q_j5 => 9, q_j6 => 9,
        // Bit-packed tensors, opcodes
        p => 9, op => 9,
        // Renderable objects: redesigned in v11 (any numeric type, frames from the containing node, angles in turns, length-prefixed bodies), and no longer behind the spirix feature
        rob => 11, roe => 11, rop => 11, rol => 11, ror => 11, ron => 11, roa => 11, rot => 11, rou => 11, roq => 11, roi => 11, rof => 11, rog => 11, rok => 11,
        // Human language
        wl => 11,
    }
    spirix {
        // Spirix colour
        rd => 9, rb => 9, rw => 9, rq => 9,
        // Scalars
        s33 => 9, s34 => 9, s35 => 9, s36 => 9, s37 => 9, s43 => 9, s44 => 9, s45 => 9, s46 => 9, s47 => 9,
        s53 => 9, s54 => 9, s55 => 9, s56 => 9, s57 => 9, s63 => 9, s64 => 9, s65 => 9, s66 => 9, s67 => 9,
        s73 => 9, s74 => 9, s75 => 9, s76 => 9, s77 => 9,
        // Circles
        c33 => 9, c34 => 9, c35 => 9, c36 => 9, c37 => 9, c43 => 9, c44 => 9, c45 => 9, c46 => 9, c47 => 9,
        c53 => 9, c54 => 9, c55 => 9, c56 => 9, c57 => 9, c63 => 9, c64 => 9, c65 => 9, c66 => 9, c67 => 9,
        c73 => 9, c74 => 9, c75 => 9, c76 => 9, c77 => 9,
        // Scalar tensors
        t_s33 => 9, t_s34 => 9, t_s35 => 9, t_s36 => 9, t_s37 => 9, t_s43 => 9, t_s44 => 9, t_s45 => 9, t_s46 => 9, t_s47 => 9,
        t_s53 => 9, t_s54 => 9, t_s55 => 9, t_s56 => 9, t_s57 => 9, t_s63 => 9, t_s64 => 9, t_s65 => 9, t_s66 => 9, t_s67 => 9,
        t_s73 => 9, t_s74 => 9, t_s75 => 9, t_s76 => 9, t_s77 => 9,
        // Circle tensors
        t_c33 => 9, t_c34 => 9, t_c35 => 9, t_c36 => 9, t_c37 => 9, t_c43 => 9, t_c44 => 9, t_c45 => 9, t_c46 => 9, t_c47 => 9,
        t_c53 => 9, t_c54 => 9, t_c55 => 9, t_c56 => 9, t_c57 => 9, t_c63 => 9, t_c64 => 9, t_c65 => 9, t_c66 => 9, t_c67 => 9,
        t_c73 => 9, t_c74 => 9, t_c75 => 9, t_c76 => 9, t_c77 => 9,
        // Strided scalar tensors
        q_s33 => 9, q_s34 => 9, q_s35 => 9, q_s36 => 9, q_s37 => 9, q_s43 => 9, q_s44 => 9, q_s45 => 9, q_s46 => 9, q_s47 => 9,
        q_s53 => 9, q_s54 => 9, q_s55 => 9, q_s56 => 9, q_s57 => 9, q_s63 => 9, q_s64 => 9, q_s65 => 9, q_s66 => 9, q_s67 => 9,
        q_s73 => 9, q_s74 => 9, q_s75 => 9, q_s76 => 9, q_s77 => 9,
        // Strided circle tensors: until v11 they wrote their element type as a scalar (`s`), so no reader could read one back
        q_c33 => 11, q_c34 => 11, q_c35 => 11, q_c36 => 11, q_c37 => 11, q_c43 => 11, q_c44 => 11, q_c45 => 11, q_c46 => 11, q_c47 => 11,
        q_c53 => 11, q_c54 => 11, q_c55 => 11, q_c56 => 11, q_c57 => 11, q_c63 => 11, q_c64 => 11, q_c65 => 11, q_c66 => 11, q_c67 => 11,
        q_c73 => 11, q_c74 => 11, q_c75 => 11, q_c76 => 11, q_c77 => 11,
    }
}

impl VsfType {
    /// The oldest reader this value needs: its own type's version, or a nested value's if that is later (the children, fills and strokes of a renderable object).
    pub fn since(&self) -> Since {
        let mut latest = self.own_since();
        super::renderable::visit(self, &mut |v| latest = latest.max(v.since()));
        latest
    }
}

impl VsfField {
    /// The latest type among this field's values; `None` for a flag with no values.
    pub fn since(&self) -> Option<Since> {
        self.values.iter().map(VsfType::since).reduce(Since::max)
    }
}

impl VsfSection {
    /// The oldest reader this section's body needs, header framing included. For a section that will be encrypted, pass this to the builder's `requires` — the builder sees only ciphertext.
    pub fn since(&self) -> Since {
        self.fields.iter().filter_map(VsfField::since).fold(Since::HEADER, Since::max)
    }
}
