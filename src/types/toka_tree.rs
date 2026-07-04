//! Scene Graph Supporting Types for VSF ro* Renderable Objects
//!
//! These types support the ro* VSF type family (rob, roc, roe, etc.) for building hierarchical scene graphs with transforms, fills, and strokes.

#[cfg(feature = "spirix")]
use spirix::{CircleF4E4, ScalarF4E4};

use super::VsfType;

/// Fill style for shapes
#[derive(Debug, Clone, PartialEq)]
pub enum Fill {
    Solid(Box<VsfType>),    // Any colour type (rcr, ra, rw, etc.)
    Gradient(Box<VsfType>), // rog type
}

/// Stroke style for shape outlines
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    pub width: ScalarF4E4,
    pub colour: Box<VsfType>,
    pub join: StrokeJoin,
    pub cap: StrokeCap,
}

/// Stroke join style
#[derive(Debug, Clone, PartialEq)]
pub enum StrokeJoin {
    Miter,
    Round,
    Bevel,
}

/// Stroke cap style
#[derive(Debug, Clone, PartialEq)]
pub enum StrokeCap {
    Butt,
    Round,
    Square,
}

/// Gradient variant with positioning parameters
#[derive(Debug, Clone, PartialEq)]
pub enum GradientVariant {
    Linear {
        start: CircleF4E4,
        end: CircleF4E4,
    },
    Radial {
        center: CircleF4E4,
        radius: ScalarF4E4,
    },
    Conic {
        center: CircleF4E4,
        angle: ScalarF4E4,
    },
}

/// Gradient colour stop
#[derive(Debug, Clone, PartialEq)]
pub struct GradientStop {
    pub offset: ScalarF4E4,      // 0.0 to 1.0
    pub colour: [ScalarF4E4; 4], // RGBA
}

/// Vector path command
#[derive(Debug, Clone, PartialEq)]
pub enum PathCommand {
    MoveTo(CircleF4E4),
    LineTo(CircleF4E4),
    QuadraticTo(CircleF4E4, CircleF4E4),         // control, end
    CubicTo(CircleF4E4, CircleF4E4, CircleF4E4), // control1, control2, end
    Close,
}

/// Spline interpolation type
#[derive(Debug, Clone, PartialEq)]
pub enum SplineType {
    Bezier,
    Cubic,
    CatmullRom,
}

/// Button visual variant
#[derive(Debug, Clone, PartialEq)]
pub enum ButtonVariant {
    Filled,
    Outlined,
    Text,
}

/// Transform for row (group) nodes
#[derive(Debug, Clone, PartialEq)]
pub struct Transform {
    pub translate: Option<CircleF4E4>,
    pub rotate: Option<ScalarF4E4>, // Radians
    pub scale: Option<CircleF4E4>,
    pub origin: Option<CircleF4E4>, // Transform origin
}

impl Transform {
    /// Create an identity transform (all fields None)
    pub fn identity() -> Self {
        Transform {
            translate: None,
            rotate: None,
            scale: None,
            origin: None,
        }
    }

    /// Check if this transform is identity
    pub fn is_identity(&self) -> bool {
        self.translate.is_none()
            && self.rotate.is_none()
            && self.scale.is_none()
            && self.origin.is_none()
    }
}

/// Text styling options for rot (text rendering).
///
/// Binary encoding: tagged fields, loop until 0x00 terminator. Tags (all lowercase ASCII; no caps to avoid collision with 2^N length-prefix scheme): 0x00      — end of style b'l'      — align left  (no value bytes; absence = center default) b'r'      — align right (no value bytes) b'f' + 32 — font BLAKE3 hash b'e' + s44 — leading (line height multiplier) b'k' + s44 — kerning (letter spacing in RU) b'w' + s44 — weight (100–900) b'i' + s44 — tilt (italic angle in degrees) b'x' + s44 — wrap width (box width in RU; enables word wrap) s44 = i16 BE fraction + i16 BE exponent (4 bytes total)
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    pub font: Option<[u8; 32]>, // BLAKE3 hash of font bytes  (tag b'f' + 32 bytes)
    pub leading: Option<ScalarF4E4>, // Line height multiplier     (tag b'e' + s44)
    pub kerning: Option<ScalarF4E4>, // Letter spacing in RU       (tag b'k' + s44)
    pub weight: Option<ScalarF4E4>, // Font weight 100–900        (tag b'w' + s44)
    pub tilt: Option<ScalarF4E4>, // Italic angle in degrees    (tag b'i' + s44)
    pub wrap: Option<ScalarF4E4>, // Wrap box width in RU       (tag b'x' + s44)
    pub align: Option<u8>,      // 0=center(default), 1=left(b'l'), 2=right(b'r')
}

impl TextStyle {
    /// Create default text style (all None = centered, no wrap, default weight/leading)
    pub fn default() -> Self {
        TextStyle {
            font: None,
            leading: None,
            kerning: None,
            weight: None,
            tilt: None,
            wrap: None,
            align: None,
        }
    }

    /// Check if this style is default (all None)
    pub fn is_default(&self) -> bool {
        self.font.is_none()
            && self.leading.is_none()
            && self.kerning.is_none()
            && self.weight.is_none()
            && self.tilt.is_none()
            && self.wrap.is_none()
            && self.align.is_none()
    }
}

// DEPRECATED - Old vt hack types (will be removed) These types are kept temporarily for backward compatibility but should not be used in new code. Use the ro* VsfType variants instead (rob, roc, roe, etc.)

/// Box container primitive (DEPRECATED - use VsfType::rob)
#[derive(Debug, Clone, PartialEq)]
pub struct TokaBox {
    pub pos: CircleF4E4,
    pub size: CircleF4E4,
    pub colour: [ScalarF4E4; 4], // RGBA
}

/// Group container (DEPRECATED - use VsfType::row or VsfType::ron)
#[derive(Debug, Clone, PartialEq)]
pub struct TokaContainer {
    pub children: Vec<Node>,
}

/// Circle shape primitive (DEPRECATED - use VsfType::roc)
#[derive(Debug, Clone, PartialEq)]
pub struct TokaCircle {
    pub pos: CircleF4E4,
    pub span: ScalarF4E4,
    pub colour: [ScalarF4E4; 4], // RGBA
}

/// Line stroke primitive (DEPRECATED - use VsfType::rol)
#[derive(Debug, Clone, PartialEq)]
pub struct TokaLine {
    pub start: CircleF4E4,
    pub end: CircleF4E4,
    pub width: ScalarF4E4,
    pub colour: [ScalarF4E4; 4], // RGBA
}

/// Text label primitive (DEPRECATED - use VsfType::rot)
#[derive(Debug, Clone, PartialEq)]
pub struct TokaText {
    pub pos: CircleF4E4,
    pub size: CircleF4E4,
    pub content: String,
    pub colour: [ScalarF4E4; 4], // RGBA
}

/// Button UI element primitive (DEPRECATED - use VsfType::rou)
#[derive(Debug, Clone, PartialEq)]
pub struct TokaButton {
    pub pos: CircleF4E4,
    pub size: CircleF4E4,
    pub label: String,
    pub variant: ButtonVariant,
    pub colour: [ScalarF4E4; 4], // RGBA
}

/// Vector path primitive (DEPRECATED - use VsfType::rop)
#[derive(Debug, Clone, PartialEq)]
pub struct TokaPath {
    pub colour: [ScalarF4E4; 4], // RGBA
    pub width: ScalarF4E4,
    pub commands: Vec<PathCommand>,
}

/// Image (raster) primitive (DEPRECATED - use VsfType::roi)
#[derive(Debug, Clone, PartialEq)]
pub struct TokaImage {
    pub pos: CircleF4E4,
    pub size: CircleF4E4,
    pub handle: u64,
    pub tint: [ScalarF4E4; 4], // RGBA tint
}

/// Surface (raw pixel buffer) primitive (DEPRECATED - use VsfType::rof)
#[derive(Debug, Clone, PartialEq)]
pub struct TokaSurface {
    pub pos: CircleF4E4,
    pub size: CircleF4E4,
    pub handle: u64,
}

/// Unified enum for all Node kinds (DEPRECATED - use VsfType ro* variants)
#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    Box(TokaBox),
    Circle(TokaCircle),
    Line(TokaLine),
    Text(TokaText),
    Button(TokaButton),
    Path(TokaPath),
    Image(TokaImage),
    Surface(TokaSurface),
    Container(TokaContainer),
}

/// Hierarchical UI node (DEPRECATED - use VsfType::row with Transform)
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub kind: NodeKind,
    pub transform: Transform,
    pub children: Vec<Node>,
}

impl Node {
    /// Create a new node with the given kind
    pub fn new(kind: NodeKind) -> Self {
        Node {
            kind,
            transform: Transform::identity(),
            children: Vec::new(),
        }
    }

    /// Set the transform for this node
    pub fn set_transform(mut self, transform: Transform) -> Self {
        self.transform = transform;
        self
    }

    /// Add a child node
    pub fn add_child(mut self, child: Node) -> Self {
        self.children.push(child);
        self
    }
}
