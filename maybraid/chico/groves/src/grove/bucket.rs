//! Authored grove bucket ([RFC-183 3.4.2.2]).

use super::{constraints::PlacementConstraints, palette::PaletteMix};

/// One authored bucket: weight, placement constraints, palette, and typed item payload.
#[derive(Debug, Clone, PartialEq)]
pub struct Bucket<V> {
	pub weight: f32,
	pub placement_constraints: PlacementConstraints,
	pub palette_mix: PaletteMix,
	pub item: V,
}
