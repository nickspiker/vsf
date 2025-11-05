//! VSF Type System
//!
//! This module contains all type definitions for VSF v2:
//! - VsfType: Main enum with all supported types
//! - EtType: Eagle Time numeric representations
//! - EagleTime: Eagle Time abstraction
//! - Tensor: Contiguous tensor types
//! - StridedTensor: Strided tensor types
//! - WorldCoord: Dymaxion geographic coordinates

pub mod eagle_time_type;
pub mod tensor;
pub mod vsf_type;
pub mod world_coord;

// Re-export main types
pub use eagle_time_type::{datetime_to_eagle_time, EagleTime, EtType};
pub use tensor::{BitPackedTensor, LayoutOrder, StridedTensor, Tensor};
pub use vsf_type::VsfType;
pub use world_coord::WorldCoord;
