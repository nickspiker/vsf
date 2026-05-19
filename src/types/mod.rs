//! VSF Type System
//!
//! This module contains all type definitions for VSF v2:
//! - VsfType: Main enum with all supported types
//! - EtType: Eagle Time numeric representations
//! - EagleTime: Eagle Time abstraction
//! - Tensor: Contiguous tensor types
//! - StridedTensor: Strided tensor types
//! - WorldCoord: Dymaxion geographic coordinates

pub mod eagle_time;
pub mod network;
pub mod tensor;
pub mod vsf_type;
pub mod world_coord;

#[cfg(feature = "spirix")]
pub mod toka_tree;

// Re-export main types
pub use eagle_time::{datetime_to_eagle_time, EagleTime, EtType, OSCILLATIONS_PER_SECOND};
#[cfg(feature = "std")]
pub use eagle_time::{eagle_time_nanos, eagle_time_oscillations};
pub use tensor::{BitPackedTensor, LayoutOrder, StridedTensor, Tensor, Vector};
pub use network::{NaScheme, WaAddress};
pub use vsf_type::VsfType;
pub use world_coord::WorldCoord;

// Re-export ro* supporting types
#[cfg(feature = "spirix")]
pub use toka_tree::{
    ButtonVariant, Fill, GradientStop, GradientVariant, Node, NodeKind, PathCommand, SplineType,
    Stroke, StrokeCap, StrokeJoin, TextStyle, TokaBox, TokaButton, TokaCircle, TokaContainer,
    TokaImage, TokaLine, TokaPath, TokaSurface, TokaText, Transform,
};
