//! Optional field counters. Off unless `MAYBRAID_TREE_DIAGNOSTICS` is set.

use bevy::prelude::*;

use crate::select::CompactedTreeLists;

/// Last-frame resident totals across owners and CPU-side submission stats for
/// the first camera per owner (GPU counters are not read back).
#[derive(Resource, Clone, Debug, Default)]
pub struct TreeFieldStats {
	pub plants_resident: u32,
	pub branches_resident: u32,
	pub leaves_resident: u32,
	pub considered_wood: u32,
	pub considered_foliage: u32,
	pub submitted_wood: u32,
	pub submitted_foliage: u32,
	pub overflow: u32,
	pub triangles: u32,
	/// Highest resident owner generation; distinct owners may have equal generations.
	pub generation: u64,
}

impl TreeFieldStats {
	pub fn from_lists(
		plants_resident: u32,
		branches_resident: u32,
		leaves_resident: u32,
		generation: u64,
		lists: &CompactedTreeLists,
	) -> Self {
		Self {
			plants_resident,
			branches_resident,
			leaves_resident,
			considered_wood: lists.considered_wood,
			considered_foliage: lists.considered_foliage,
			submitted_wood: lists.submitted_wood,
			submitted_foliage: lists.submitted_foliage,
			overflow: lists
				.wood_overflow
				.iter()
				.chain(lists.foliage_overflow.iter())
				.copied()
				.sum(),
			triangles: lists.triangle_count(),
			generation,
		}
	}
}
