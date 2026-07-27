//! 90° angular rough stonework partition for circular outer walls.

/// Quarter-ring wall sweep through \(-Z\) from \(X = -1\) to \(X = 0\).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStonework90;

crate::impl_empty_lod_scene!(RoughStonework90);
