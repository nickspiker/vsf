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
pub use eagle_time::{
    datetime_to_eagle_time, from_tai_ns, from_unix_ns, legacy_to_lock, lock_eagle_now, tai_minus_utc, to_tai_ns, to_unix_ns, EagleTime, EtType, GpsTai, PtpTai, TaiSource, EAGLE_EPOCH_TAI_SECS,
    EAGLE_EPOCH_UNIX_SECS, LEAP_TABLE, LOCK_MINUS_LEGACY_SECS, OSCILLATIONS_PER_SECOND,
};
#[cfg(feature = "std")]
pub use eagle_time::{set_leap_table, NtpTai};
#[cfg(feature = "std")]
pub use eagle_time::{eagle_time_nanos, eagle_time_oscillations};
pub use network::{NaScheme, WaAddress};
pub use tensor::{BitPackedTensor, LayoutOrder, StridedTensor, Tensor, Vector};
pub use vsf_type::VsfType;
pub use world_coord::WorldCoord;

// Re-export ro* supporting types
#[cfg(feature = "spirix")]
pub use toka_tree::{
    ButtonVariant, Fill, GradientStop, GradientVariant, Node, NodeKind, PathCommand, SplineType,
    Stroke, StrokeCap, StrokeJoin, TextStyle, TokaBox, TokaButton, TokaCircle, TokaContainer,
    TokaImage, TokaLine, TokaPath, TokaSurface, TokaText, Transform,
};
