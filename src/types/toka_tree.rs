//! Toka Tree (Loom) layout node types for VSF `vt` wrapped encoding
//!
//! These types represent hierarchical UI layouts that can be serialized to VSF
//! and rendered by the Toka VM. All coordinates use parent-relative positioning
//! (0.0 = parent edge, 1.0 = opposite edge).

#[cfg(feature = "spirix")]
use spirix::{CircleF4E4, ScalarF4E4};

/// Box container primitive
#[derive(Debug, Clone, PartialEq)]
pub struct TokaBox {
    pub pos: CircleF4E4,
    pub size: CircleF4E4,
    pub colour: [ScalarF4E4; 4], // RGBA
}

#[cfg(feature = "spirix")]
impl TokaBox {
    pub fn encode_inner(&self) -> Vec<u8> {
        let mut bytes = vec![b'b'];
        bytes.extend_from_slice(&self.pos.real.to_be_bytes());
        bytes.extend_from_slice(&self.pos.imaginary.to_be_bytes());
        bytes.extend_from_slice(&self.pos.exponent.to_be_bytes());
        bytes.extend_from_slice(&self.size.real.to_be_bytes());
        bytes.extend_from_slice(&self.size.imaginary.to_be_bytes());
        bytes.extend_from_slice(&self.size.exponent.to_be_bytes());
        // RGBA: 4 ScalarF4E4 values (16 bytes total)
        for channel in &self.colour {
            bytes.extend_from_slice(&channel.fraction.to_be_bytes());
            bytes.extend_from_slice(&channel.exponent.to_be_bytes());
        }
        bytes
    }

    pub fn to_vsf_type(&self) -> super::VsfType {
        let inner_data = self.encode_inner();
        super::VsfType::v(b't', inner_data)
    }
}

/// Node container (logical only, no visual)
#[derive(Debug, Clone, PartialEq)]
pub struct TokaNodeContainer {
    pub pos: CircleF4E4,
    pub size: CircleF4E4,
    pub children: Vec<TokaNode>,
}

#[cfg(feature = "spirix")]
impl TokaNodeContainer {
    pub fn encode_inner(&self) -> Vec<u8> {
        let mut bytes = vec![b'n'];
        bytes.extend_from_slice(&self.pos.real.to_be_bytes());
        bytes.extend_from_slice(&self.pos.imaginary.to_be_bytes());
        bytes.extend_from_slice(&self.pos.exponent.to_be_bytes());
        bytes.extend_from_slice(&self.size.real.to_be_bytes());
        bytes.extend_from_slice(&self.size.imaginary.to_be_bytes());
        bytes.extend_from_slice(&self.size.exponent.to_be_bytes());
        bytes.push(self.children.len() as u8);
        // Encode each child as a complete vt capsule (nested VSF fields)
        for child in &self.children {
            let child_vt = child.to_vsf_type(); // Wrap in v(b't', inner_bytes)
            bytes.extend_from_slice(&child_vt.flatten()); // Flatten to complete vt capsule
        }
        bytes
    }

    pub fn to_vsf_type(&self) -> super::VsfType {
        let inner_data = self.encode_inner();
        super::VsfType::v(b't', inner_data)
    }
}

/// Circle shape primitive
#[derive(Debug, Clone, PartialEq)]
pub struct TokaCircle {
    pub pos: CircleF4E4,
    pub span: ScalarF4E4,
    pub colour: [ScalarF4E4; 4], // RGBA
}

#[cfg(feature = "spirix")]
impl TokaCircle {
    pub fn encode_inner(&self) -> Vec<u8> {
        let mut bytes = vec![b'c'];
        bytes.extend_from_slice(&self.pos.real.to_be_bytes());
        bytes.extend_from_slice(&self.pos.imaginary.to_be_bytes());
        bytes.extend_from_slice(&self.pos.exponent.to_be_bytes());
        bytes.extend_from_slice(&self.span.fraction.to_be_bytes());
        bytes.extend_from_slice(&self.span.exponent.to_be_bytes());
        // RGBA: 4 ScalarF4E4 values (16 bytes total)
        for channel in &self.colour {
            bytes.extend_from_slice(&channel.fraction.to_be_bytes());
            bytes.extend_from_slice(&channel.exponent.to_be_bytes());
        }
        bytes
    }

    pub fn to_vsf_type(&self) -> super::VsfType {
        let inner_data = self.encode_inner();
        super::VsfType::v(b't', inner_data)
    }
}

/// Line stroke primitive
#[derive(Debug, Clone, PartialEq)]
pub struct TokaLine {
    pub start: CircleF4E4,
    pub end: CircleF4E4,
    pub width: ScalarF4E4,
    pub colour: [ScalarF4E4; 4], // RGBA
}

#[cfg(feature = "spirix")]
impl TokaLine {
    pub fn encode_inner(&self) -> Vec<u8> {
        let mut bytes = vec![b'l'];
        bytes.extend_from_slice(&self.start.real.to_be_bytes());
        bytes.extend_from_slice(&self.start.imaginary.to_be_bytes());
        bytes.extend_from_slice(&self.start.exponent.to_be_bytes());
        bytes.extend_from_slice(&self.end.real.to_be_bytes());
        bytes.extend_from_slice(&self.end.imaginary.to_be_bytes());
        bytes.extend_from_slice(&self.end.exponent.to_be_bytes());
        bytes.extend_from_slice(&self.width.fraction.to_be_bytes());
        bytes.extend_from_slice(&self.width.exponent.to_be_bytes());
        // RGBA: 4 ScalarF4E4 values (16 bytes total)
        for channel in &self.colour {
            bytes.extend_from_slice(&channel.fraction.to_be_bytes());
            bytes.extend_from_slice(&channel.exponent.to_be_bytes());
        }
        bytes
    }

    pub fn to_vsf_type(&self) -> super::VsfType {
        let inner_data = self.encode_inner();
        super::VsfType::v(b't', inner_data)
    }
}

/// Text label primitive
#[derive(Debug, Clone, PartialEq)]
pub struct TokaText {
    pub pos: CircleF4E4,
    pub size: CircleF4E4,
    pub content: String,
    pub colour: [ScalarF4E4; 4], // RGBA
}

#[cfg(feature = "spirix")]
impl TokaText {
    pub fn encode_inner(&self) -> Vec<u8> {
        let mut bytes = vec![b'x'];
        bytes.extend_from_slice(&self.pos.real.to_be_bytes());
        bytes.extend_from_slice(&self.pos.imaginary.to_be_bytes());
        bytes.extend_from_slice(&self.pos.exponent.to_be_bytes());
        bytes.extend_from_slice(&self.size.real.to_be_bytes());
        bytes.extend_from_slice(&self.size.imaginary.to_be_bytes());
        bytes.extend_from_slice(&self.size.exponent.to_be_bytes());
        // Encode content as VSF string type
        let content_vsf = super::VsfType::x(self.content.clone());
        bytes.extend_from_slice(&content_vsf.flatten());
        // RGBA: 4 ScalarF4E4 values (16 bytes total)
        for channel in &self.colour {
            bytes.extend_from_slice(&channel.fraction.to_be_bytes());
            bytes.extend_from_slice(&channel.exponent.to_be_bytes());
        }
        bytes
    }

    pub fn to_vsf_type(&self) -> super::VsfType {
        let inner_data = self.encode_inner();
        super::VsfType::v(b't', inner_data)
    }
}

/// Button UI element primitive
#[derive(Debug, Clone, PartialEq)]
pub struct TokaButton {
    pub pos: CircleF4E4,
    pub size: CircleF4E4,
    pub label: String,
    pub variant: ButtonVariant,
    pub colour: [ScalarF4E4; 4], // RGBA
}

#[cfg(feature = "spirix")]
impl TokaButton {
    pub fn encode_inner(&self) -> Vec<u8> {
        let mut bytes = vec![b'u'];
        bytes.extend_from_slice(&self.pos.real.to_be_bytes());
        bytes.extend_from_slice(&self.pos.imaginary.to_be_bytes());
        bytes.extend_from_slice(&self.pos.exponent.to_be_bytes());
        bytes.extend_from_slice(&self.size.real.to_be_bytes());
        bytes.extend_from_slice(&self.size.imaginary.to_be_bytes());
        bytes.extend_from_slice(&self.size.exponent.to_be_bytes());
        // Encode label as VSF string type
        let label_vsf = super::VsfType::x(self.label.clone());
        bytes.extend_from_slice(&label_vsf.flatten());
        bytes.push(self.variant as u8);
        // RGBA: 4 ScalarF4E4 values (16 bytes total)
        for channel in &self.colour {
            bytes.extend_from_slice(&channel.fraction.to_be_bytes());
            bytes.extend_from_slice(&channel.exponent.to_be_bytes());
        }
        bytes
    }

    pub fn to_vsf_type(&self) -> super::VsfType {
        let inner_data = self.encode_inner();
        super::VsfType::v(b't', inner_data)
    }
}

/// Button visual variants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ButtonVariant {
    Filled = 0,
    Outlined = 1,
    Text = 2,
}

/// Vector path primitive
#[derive(Debug, Clone, PartialEq)]
pub struct TokaPath {
    pub colour: [ScalarF4E4; 4], // RGBA
    pub width: ScalarF4E4,
    pub commands: Vec<PathCommand>,
}

#[cfg(feature = "spirix")]
impl TokaPath {
    pub fn encode_inner(&self) -> Vec<u8> {
        let mut bytes = vec![b'p'];
        // RGBA: 4 ScalarF4E4 values (16 bytes total)
        for channel in &self.colour {
            bytes.extend_from_slice(&channel.fraction.to_be_bytes());
            bytes.extend_from_slice(&channel.exponent.to_be_bytes());
        }
        bytes.extend_from_slice(&self.width.fraction.to_be_bytes());
        bytes.extend_from_slice(&self.width.exponent.to_be_bytes());
        bytes.push(self.commands.len() as u8);
        for cmd in &self.commands {
            bytes.extend_from_slice(&cmd.encode());
        }
        bytes
    }

    pub fn to_vsf_type(&self) -> super::VsfType {
        let inner_data = self.encode_inner();
        super::VsfType::v(b't', inner_data)
    }
}

/// Vector path command
#[derive(Debug, Clone, PartialEq)]
pub enum PathCommand {
    MoveTo(CircleF4E4),
    LineTo(CircleF4E4),
    QuadraticTo { ctrl: CircleF4E4, end: CircleF4E4 },
    CubicTo {
        ctrl1: CircleF4E4,
        ctrl2: CircleF4E4,
        end: CircleF4E4,
    },
    Close,
}

#[cfg(feature = "spirix")]
impl PathCommand {
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        match self {
            PathCommand::MoveTo(pos) => {
                bytes.push(b'M');
                bytes.extend_from_slice(&pos.real.to_be_bytes());
                bytes.extend_from_slice(&pos.imaginary.to_be_bytes());
                bytes.extend_from_slice(&pos.exponent.to_be_bytes());
            }
            PathCommand::LineTo(pos) => {
                bytes.push(b'L');
                bytes.extend_from_slice(&pos.real.to_be_bytes());
                bytes.extend_from_slice(&pos.imaginary.to_be_bytes());
                bytes.extend_from_slice(&pos.exponent.to_be_bytes());
            }
            PathCommand::QuadraticTo { ctrl, end } => {
                bytes.push(b'Q');
                bytes.extend_from_slice(&ctrl.real.to_be_bytes());
                bytes.extend_from_slice(&ctrl.imaginary.to_be_bytes());
                bytes.extend_from_slice(&ctrl.exponent.to_be_bytes());
                bytes.extend_from_slice(&end.real.to_be_bytes());
                bytes.extend_from_slice(&end.imaginary.to_be_bytes());
                bytes.extend_from_slice(&end.exponent.to_be_bytes());
            }
            PathCommand::CubicTo { ctrl1, ctrl2, end } => {
                bytes.push(b'C');
                bytes.extend_from_slice(&ctrl1.real.to_be_bytes());
                bytes.extend_from_slice(&ctrl1.imaginary.to_be_bytes());
                bytes.extend_from_slice(&ctrl1.exponent.to_be_bytes());
                bytes.extend_from_slice(&ctrl2.real.to_be_bytes());
                bytes.extend_from_slice(&ctrl2.imaginary.to_be_bytes());
                bytes.extend_from_slice(&ctrl2.exponent.to_be_bytes());
                bytes.extend_from_slice(&end.real.to_be_bytes());
                bytes.extend_from_slice(&end.imaginary.to_be_bytes());
                bytes.extend_from_slice(&end.exponent.to_be_bytes());
            }
            PathCommand::Close => {
                bytes.push(b'Z');
            }
        }
        bytes
    }
}

/// Image (raster) primitive
#[derive(Debug, Clone, PartialEq)]
pub struct TokaImage {
    pub pos: CircleF4E4,
    pub size: CircleF4E4,
    pub handle: u64,
    pub tint: [ScalarF4E4; 4], // RGBA tint
}

#[cfg(feature = "spirix")]
impl TokaImage {
    pub fn encode_inner(&self) -> Vec<u8> {
        let mut bytes = vec![b'i'];
        bytes.extend_from_slice(&self.pos.real.to_be_bytes());
        bytes.extend_from_slice(&self.pos.imaginary.to_be_bytes());
        bytes.extend_from_slice(&self.pos.exponent.to_be_bytes());
        bytes.extend_from_slice(&self.size.real.to_be_bytes());
        bytes.extend_from_slice(&self.size.imaginary.to_be_bytes());
        bytes.extend_from_slice(&self.size.exponent.to_be_bytes());
        bytes.extend_from_slice(&self.handle.to_be_bytes());
        // RGBA tint: 4 ScalarF4E4 values (16 bytes total)
        for channel in &self.tint {
            bytes.extend_from_slice(&channel.fraction.to_be_bytes());
            bytes.extend_from_slice(&channel.exponent.to_be_bytes());
        }
        bytes
    }

    pub fn to_vsf_type(&self) -> super::VsfType {
        let inner_data = self.encode_inner();
        super::VsfType::v(b't', inner_data)
    }
}

/// Surface (raw pixel buffer) primitive
#[derive(Debug, Clone, PartialEq)]
pub struct TokaSurface {
    pub pos: CircleF4E4,
    pub size: CircleF4E4,
    pub handle: u64,
}

#[cfg(feature = "spirix")]
impl TokaSurface {
    pub fn encode_inner(&self) -> Vec<u8> {
        let mut bytes = vec![b's'];
        bytes.extend_from_slice(&self.pos.real.to_be_bytes());
        bytes.extend_from_slice(&self.pos.imaginary.to_be_bytes());
        bytes.extend_from_slice(&self.pos.exponent.to_be_bytes());
        bytes.extend_from_slice(&self.size.real.to_be_bytes());
        bytes.extend_from_slice(&self.size.imaginary.to_be_bytes());
        bytes.extend_from_slice(&self.size.exponent.to_be_bytes());
        bytes.extend_from_slice(&self.handle.to_be_bytes());
        bytes
    }

    pub fn to_vsf_type(&self) -> super::VsfType {
        let inner_data = self.encode_inner();
        super::VsfType::v(b't', inner_data)
    }
}

/// Unified enum for all Toka Tree node types
#[derive(Debug, Clone, PartialEq)]
pub enum TokaNode {
    Box(TokaBox),
    Node(TokaNodeContainer),
    Circle(TokaCircle),
    Line(TokaLine),
    Text(TokaText),
    Button(TokaButton),
    Path(TokaPath),
    Image(TokaImage),
    Surface(TokaSurface),
}

#[cfg(feature = "spirix")]
impl TokaNode {
    pub fn encode_inner(&self) -> Vec<u8> {
        match self {
            TokaNode::Box(b) => b.encode_inner(),
            TokaNode::Node(n) => n.encode_inner(),
            TokaNode::Circle(c) => c.encode_inner(),
            TokaNode::Line(l) => l.encode_inner(),
            TokaNode::Text(t) => t.encode_inner(),
            TokaNode::Button(b) => b.encode_inner(),
            TokaNode::Path(p) => p.encode_inner(),
            TokaNode::Image(i) => i.encode_inner(),
            TokaNode::Surface(s) => s.encode_inner(),
        }
    }

    pub fn to_vsf_type(&self) -> super::VsfType {
        let inner_data = self.encode_inner();
        super::VsfType::v(b't', inner_data)
    }
}

// Display implementations for human-readable output in vsfinfo
#[cfg(feature = "spirix")]
impl std::fmt::Display for TokaBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Box {{ pos: {}, size: {}, colour: [{}, {}, {}, {}] }}",
            self.pos, self.size,
            self.colour[0], self.colour[1], self.colour[2], self.colour[3]
        )
    }
}

#[cfg(feature = "spirix")]
impl std::fmt::Display for TokaCircle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Circle {{ pos: {}, span: {}, colour: [{}, {}, {}, {}] }}",
            self.pos, self.span,
            self.colour[0], self.colour[1], self.colour[2], self.colour[3]
        )
    }
}

#[cfg(feature = "spirix")]
impl std::fmt::Display for TokaLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Line {{ start: {}, end: {}, width: {}, colour: [{}, {}, {}, {}] }}",
            self.start, self.end, self.width,
            self.colour[0], self.colour[1], self.colour[2], self.colour[3]
        )
    }
}

#[cfg(feature = "spirix")]
impl std::fmt::Display for TokaText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Text {{ pos: {}, size: {}, content: \"{}\", colour: [...] }}",
            self.pos, self.size, self.content
        )
    }
}

#[cfg(feature = "spirix")]
impl std::fmt::Display for TokaButton {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let variant_str = match self.variant {
            ButtonVariant::Filled => "Filled",
            ButtonVariant::Outlined => "Outlined",
            ButtonVariant::Text => "Text",
        };
        write!(
            f,
            "Button {{ pos: {}, size: {}, label: \"{}\", variant: {} }}",
            self.pos, self.size, self.label, variant_str
        )
    }
}

#[cfg(feature = "spirix")]
impl std::fmt::Display for TokaNodeContainer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Node {{ pos: {}, size: {}, children: {} }}",
            self.pos, self.size, self.children.len()
        )
    }
}

#[cfg(feature = "spirix")]
impl std::fmt::Display for TokaPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Path {{ width: {}, {} commands, colour: [...] }}",
            self.width, self.commands.len()
        )
    }
}

#[cfg(feature = "spirix")]
impl std::fmt::Display for TokaImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Image {{ pos: {}, size: {}, handle: {} }}",
            self.pos, self.size, self.handle
        )
    }
}

#[cfg(feature = "spirix")]
impl std::fmt::Display for TokaSurface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Surface {{ pos: {}, size: {}, handle: {} }}",
            self.pos, self.size, self.handle
        )
    }
}

#[cfg(feature = "spirix")]
impl std::fmt::Display for TokaNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokaNode::Box(b) => write!(f, "{}", b),
            TokaNode::Circle(c) => write!(f, "{}", c),
            TokaNode::Line(l) => write!(f, "{}", l),
            TokaNode::Text(t) => write!(f, "{}", t),
            TokaNode::Button(b) => write!(f, "{}", b),
            TokaNode::Node(n) => write!(f, "{}", n),
            TokaNode::Path(p) => write!(f, "{}", p),
            TokaNode::Image(i) => write!(f, "{}", i),
            TokaNode::Surface(s) => write!(f, "{}", s),
        }
    }
}

// ==================== NEW SUPPORTING TYPES FOR RO* ====================

/// Fill type for renderable objects - solid colour or gradient
#[derive(Debug, Clone)]
pub enum Fill {
    Solid(Box<super::VsfType>),    // Any colour type (rcr, ra, rw, etc.)
    Gradient(Box<super::VsfType>), // rog type
}

/// Stroke properties for renderable objects
#[derive(Debug, Clone)]
pub struct Stroke {
    pub width: ScalarF4E4,
    pub colour: Box<super::VsfType>,
    pub join: StrokeJoin,
    pub cap: StrokeCap,
}

/// Stroke line join style
#[derive(Debug, Clone, PartialEq)]
pub enum StrokeJoin {
    Miter,
    Round,
    Bevel,
}

/// Stroke line cap style
#[derive(Debug, Clone, PartialEq)]
pub enum StrokeCap {
    Butt,
    Round,
    Square,
}

/// Gradient variant - linear, radial, or conic
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

/// Spline curve type
#[derive(Debug, Clone, PartialEq)]
pub enum SplineType {
    Bezier,
    Cubic,
    CatmullRom,
}

/// Transform properties for group nodes
#[derive(Debug, Clone, PartialEq)]
pub struct Transform {
    pub translate: Option<CircleF4E4>,
    pub rotate: Option<ScalarF4E4>, // Radians
    pub scale: Option<CircleF4E4>,
    pub origin: Option<CircleF4E4>, // Transform origin point
}

impl Transform {
    pub fn identity() -> Self {
        Self {
            translate: None,
            rotate: None,
            scale: None,
            origin: None,
        }
    }

    pub fn is_identity(&self) -> bool {
        self.translate.is_none()
            && self.rotate.is_none()
            && self.scale.is_none()
            && self.origin.is_none()
    }
}
