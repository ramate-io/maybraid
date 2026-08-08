//! Tiled monster-grass plain for LOD ring-shift testing.

use bevy::prelude::*;
use chico_groves::{GroveExtent, MonsterGrassParams, DEFAULT_GROVE_EXTENT_XZ};
use chico_vegetation_components::{spawn_vegetation_components, vegetation_bounds};
use lod::InnerOuterLattice;

/// Number of default-extent groves along each horizontal axis (centered).
pub const PLAIN_GROVES_PER_AXIS: i32 = 3;

/// Vegetation LOD lattice used by `/show monster-grass-plains`.
pub fn plain_inner_outer_lattice() -> InnerOuterLattice {
	InnerOuterLattice {
		cell_size: DEFAULT_GROVE_EXTENT_XZ,
		ring_radius: 3,
	}
}

/// Spawn a centered [`PLAIN_GROVES_PER_AXIS`]² tile of default monster-grass groves.
pub fn spawn_monster_grass_plain(commands: &mut Commands, transform: Transform) -> Vec<Entity> {
	let tile = DEFAULT_GROVE_EXTENT_XZ;
	let half = PLAIN_GROVES_PER_AXIS / 2;
	let mut entities =
		Vec::with_capacity((PLAIN_GROVES_PER_AXIS * PLAIN_GROVES_PER_AXIS) as usize);

	for ix in -half..=half {
		for iz in -half..=half {
			let min = Vec3::new(ix as f32 * tile, 0.0, iz as f32 * tile);
			let max = min + Vec3::new(tile, 1.0, tile);
			let params = MonsterGrassParams::default().with_extent(GroveExtent::new(min, max));
			let grove = params.build();
			let bounds = vegetation_bounds(&grove);
			entities.extend(spawn_vegetation_components(
				commands,
				&grove,
				transform,
				bounds,
			));
		}
	}

	entities
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn plain_is_three_by_three_centered() {
		assert_eq!(PLAIN_GROVES_PER_AXIS, 3);
		let half = PLAIN_GROVES_PER_AXIS / 2;
		assert_eq!((-half..=half).count(), 3);
		let lattice = plain_inner_outer_lattice();
		assert_eq!(lattice.cell_size, DEFAULT_GROVE_EXTENT_XZ);
		assert_eq!(lattice.ring_radius, 3);
	}
}
