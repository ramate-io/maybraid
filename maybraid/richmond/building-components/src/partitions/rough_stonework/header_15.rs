//! Header-height 15° rough stonework arc for curved door frames.

use crate::assets::partitions::rough_stonework::HEADER_15;
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneworkHeader15;


crate::impl_glb_lod_scene!(RoughStoneworkHeader15, HEADER_15);
