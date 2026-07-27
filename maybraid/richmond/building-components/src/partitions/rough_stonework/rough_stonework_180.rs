//! 180° angular rough stonework partition for circular outer walls.

/// Half-ring wall sweep through \(-Z\) from \(X = -1\) to \(X = 1\).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStonework180;

crate::impl_empty_lod_scene!(RoughStonework180);
