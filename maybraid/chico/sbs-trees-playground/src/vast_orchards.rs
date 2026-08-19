//! Tiled orchard plain for LOD / scale testing (flattened tree hosts, shared kit meshes).

use bevy::prelude::*;
use chico_groves::{GroveExtent, OrchardParams, DEFAULT_GROVE_EXTENT_XZ};
use chico_vegetation_components::{spawn_lod_scene_host, vegetation_bounds, VegetationComponents};

/// Grove-tile radius from center (`[-radius, radius]` on each axis).
pub const VAST_ORCHARD_RADIUS: i32 = 10;

/// Plains tile span on XZ (matches default grove preview extent).
pub const VAST_ORCHARD_EXTENT_XZ: f32 = DEFAULT_GROVE_EXTENT_XZ;

/// Spawn a centered `(2 × [`VAST_ORCHARD_RADIUS`] + 1)²` tile of default orchard groves.
pub fn spawn_vast_orchards(commands: &mut Commands, transform: Transform) -> Vec<Entity> {
	let tile = VAST_ORCHARD_EXTENT_XZ;
	let axis = 2 * VAST_ORCHARD_RADIUS + 1;
	let mut entities = Vec::with_capacity((axis * axis) as usize);

	for ix in -VAST_ORCHARD_RADIUS..=VAST_ORCHARD_RADIUS {
		for iz in -VAST_ORCHARD_RADIUS..=VAST_ORCHARD_RADIUS {
			let min = Vec3::new(ix as f32 * tile, 0.0, iz as f32 * tile);
			let max = min + Vec3::new(tile, 1.0, tile);
			let grove = OrchardParams::default().with_extent(GroveExtent::new(min, max)).build();
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
	fn vast_is_radius_ten_centered() {
		assert_eq!(VAST_ORCHARD_RADIUS, 10);
		assert_eq!((-VAST_ORCHARD_RADIUS..=VAST_ORCHARD_RADIUS).count(), 21);
		assert!((VAST_ORCHARD_EXTENT_XZ - DEFAULT_GROVE_EXTENT_XZ).abs() < 1e-5);
	}
}
