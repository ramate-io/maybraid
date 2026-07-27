//! Full-height linear rough stonework partition (normalized \(X \in [-1, 1]\)).

/// Linear wall segment for radial subdividers and straight partitions.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneworkLinear;

crate::impl_empty_lod_scene!(RoughStoneworkLinear);
