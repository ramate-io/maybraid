//! Optional field counters. Off unless `MAYBRAID_GRASS_DIAGNOSTICS` is set.

use bevy::prelude::*;

use crate::select::CompactedLists;

/// Last-frame CPU-side submission stats (GPU counters are not read back).
#[derive(Resource, Clone, Debug, Default)]
pub struct GrassFieldStats {
	pub blades_resident: u32,
	pub patches_resident: u32,
	pub considered: u32,
	pub submitted: u32,
	pub overflow: u32,
	pub triangles: u32,
	pub instances_near: u32,
	pub instances_mid: u32,
	pub instances_card: u32,
	pub generation: u64,
}

impl GrassFieldStats {
	pub fn from_lists(
		blades_resident: u32,
		patches_resident: u32,
		generation: u64,
		lists: &CompactedLists,
	) -> Self {
		Self {
			blades_resident,
			patches_resident,
			considered: lists.considered,
			submitted: lists.submitted,
			overflow: lists.overflow.iter().copied().sum(),
			triangles: lists.triangle_count(),
			instances_near: lists.instance_count(0),
			instances_mid: lists.instance_count(1),
			instances_card: lists.instance_count(2),
			generation,
		}
	}
}
