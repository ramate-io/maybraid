//! Authored grove bucket ([RFC-183 3.4.2.2]).

use super::constraints::PlacementConstraints;

/// One authored bucket: weight, placement constraints, and typed item payload.
#[derive(Debug, Clone, PartialEq)]
pub struct Bucket<V> {
	pub weight: f32,
	pub placement_constraints: PlacementConstraints,
	pub item: V,
}
