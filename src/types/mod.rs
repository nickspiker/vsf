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
pub mod tensor;
#[cfg(feature = "spirix")]
pub mod toka_tree;
#[cfg(all(test, feature = "spirix"))]
mod toka_tree_tests;
pub mod vsf_type;
pub mod world_coord;

// Re-export main types
pub use eagle_time::{
    datetime_to_eagle_time, eagle_time_nanos, eagle_time_oscillations, EagleTime, EtType,
};
pub use tensor::{BitPackedTensor, LayoutOrder, StridedTensor, Tensor, Vector};
#[cfg(feature = "spirix")]
pub use toka_tree::{
    ButtonVariant, Fill, GradientStop, GradientVariant, PathCommand, SplineType, Stroke,
    StrokeCap, StrokeJoin, TokaBox, TokaButton, TokaCircle, TokaImage, TokaLine, TokaNode,
    TokaNodeContainer, TokaPath, TokaSurface, TokaText, Transform,
};
pub use vsf_type::VsfType;
pub use world_coord::WorldCoord;
