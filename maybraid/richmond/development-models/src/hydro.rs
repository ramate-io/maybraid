//! Skip development cells that overlap Marazion hydro primitives.

use durham_terrain_models::{origin_cell_ids_for_layout, TerrainCellLayout, TerrainEntryStore};
use lod::gen::OriginalId;
use marazion_watersheds::{WaterFill, WaterSurface};
use procedural_common::Bounds2;

/// True when `bounds` overlaps any hydro primitive support in `fills`.
pub fn hydro_overlaps_xz(fills: &[WaterFill], bounds: Bounds2) -> bool {
	for fill in fills {
		match &fill.surface {
			WaterSurface::Hydro { complex } => {
				for node in &complex.hydrology {
					if node.correction_intersects(bounds) {
						return true;
					}
				}
			}
			WaterSurface::Flat { region, .. } => {
				let c = bounds.center();
				if region.sdf(c) <= 0.0 {
					return true;
				}
				let corners = [
					bounds.min,
					bevy::math::Vec2::new(bounds.max.x, bounds.min.y),
					bounds.max,
					bevy::math::Vec2::new(bounds.min.x, bounds.max.y),
				];
				if corners.iter().any(|p| region.sdf(*p) <= 0.0) {
					return true;
				}
			}
		}
	}
	false
}

/// True when any stored terrain cell overlapping `cell` has hydro that intersects `bounds`.
pub fn terrain_hydro_overlaps(
	store: &TerrainEntryStore,
	layout: &TerrainCellLayout,
	cell: bevy::math::bounding::Aabb3d,
	bounds: Bounds2,
) -> bool {
	for OriginalId(id) in origin_cell_ids_for_layout(layout, cell) {
		let Some(terrain) = store.terrain(id) else {
			continue;
		};
		if hydro_overlaps_xz(&terrain.marazion_fills, bounds) {
			return true;
		}
	}
	false
}

/// Height sample from composed post-Marazion terrain, if the covering cell is stored.
pub fn composed_height_at(
	store: &TerrainEntryStore,
	layout: &TerrainCellLayout,
	x: f32,
	z: f32,
) -> Option<f32> {
	store.composed_height_at(layout, x, z)
}

#[cfg(test)]
mod tests {
	use super::*;
	use jersey_terrain_stamps::{CircleRegion, Region2D};

	#[test]
	fn flat_fill_overlaps_center() {
		let fills = vec![WaterFill {
			surface: WaterSurface::Flat {
				level: 10.0,
				region: Region2D::Circle(CircleRegion {
					center: bevy::math::Vec2::ZERO,
					radius: 20.0,
				}),
			},
		}];
		let hit = Bounds2::from_xz(-5.0, -5.0, 5.0, 5.0);
		let miss = Bounds2::from_xz(80.0, 80.0, 90.0, 90.0);
		assert!(hydro_overlaps_xz(&fills, hit));
		assert!(!hydro_overlaps_xz(&fills, miss));
	}
}
