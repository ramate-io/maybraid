//! Linear rough stonework subsegment (normalized \(X \in [-1, 0.8]\)).

/// Partial-length linear wall used beside openings.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneworkLinearSubsegment;

crate::impl_empty_lod_scene!(RoughStoneworkLinearSubsegment);
