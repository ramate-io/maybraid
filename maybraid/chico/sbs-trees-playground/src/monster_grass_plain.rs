//! Tiled monster-grass plain for LOD refresh testing.

use bevy::prelude::*;
use chico_groves::{GroveExtent, MonsterGrassParams, DEFAULT_GROVE_EXTENT_XZ};
use chico_vegetation_components::{spawn_lod_scene_host, vegetation_bounds, VegetationComponents};

/// Grove-tile radius from center (`[-radius, radius]` on each axis).
pub const PLAIN_GROVE_RADIUS: i32 = 10;

/// Plains tile span on XZ (half the default grove preview extent).
pub const PLAIN_GROVE_EXTENT_XZ: f32 = DEFAULT_GROVE_EXTENT_XZ * 1.0;

/// Spawn a centered `(2 × [`PLAIN_GROVE_RADIUS`] + 1)²` tile of default monster-grass groves.
pub fn spawn_monster_grass_plain(commands: &mut Commands, transform: Transform) -> Vec<Entity> {
	let tile = PLAIN_GROVE_EXTENT_XZ;
	let axis = 2 * PLAIN_GROVE_RADIUS + 1;
	let mut entities = Vec::with_capacity((axis * axis) as usize);

	for ix in -PLAIN_GROVE_RADIUS..=PLAIN_GROVE_RADIUS {
		for iz in -PLAIN_GROVE_RADIUS..=PLAIN_GROVE_RADIUS {
			let min = Vec3::new(ix as f32 * tile, 0.0, iz as f32 * tile);
			let max = min + Vec3::new(tile, 1.0, tile);
			let mut params = MonsterGrassParams::default().with_extent(GroveExtent::new(min, max));
			params.merge_collections = 0;
			let grove = params.build();
			let bounds = grove
				.structural_lod()
				.map(|p| p.footprint_aabb())
				.unwrap_or_else(|| vegetation_bounds(&grove));
			entities.extend(spawn_lod_scene_host(commands, &grove, transform, bounds));
		}
	}

	entities
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn plain_is_radius_ten_centered() {
		assert_eq!(PLAIN_GROVE_RADIUS, 10);
		assert_eq!((-PLAIN_GROVE_RADIUS..=PLAIN_GROVE_RADIUS).count(), 21);
		assert!((PLAIN_GROVE_EXTENT_XZ - DEFAULT_GROVE_EXTENT_XZ * 1.0).abs() < 1e-5);
	}
}
