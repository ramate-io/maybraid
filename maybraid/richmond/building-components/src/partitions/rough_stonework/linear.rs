//! Full-height linear rough stonework partition (normalized \(X \in [-1, 1]\)).

use crate::assets::partitions::rough_stonework::LINEAR;
/// Linear wall segment for radial subdividers and straight partitions.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneworkLinear;


crate::impl_glb_lod_scene!(RoughStoneworkLinear, LINEAR);
