//! Terrain cell size and origin tiling helpers.

use bevy::math::bounding::Aabb3d;
use bevy::math::{IVec2, UVec2, Vec3};
use bevy::prelude::*;

/// Default edge length of a procedural terrain origin cell (world units).
pub const TERRAIN_CELL_SIZE: f32 = 32.0;

/// Layout for tiling terrain origin cells in the XZ plane.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct TerrainCellLayout {
	/// Edge length of each origin cell in world units.
	pub cell_size: f32,
	/// Cell-grid coordinates of the min corner (XZ).
	pub origin: IVec2,
	/// Number of cells along +X and +Z from [`Self::origin`].
	pub extents: UVec2,
}

impl Default for TerrainCellLayout {
	fn default() -> Self {
		Self {
			cell_size: TERRAIN_CELL_SIZE,
			origin: IVec2::new(0, 0),
			extents: UVec2::new(2, 2),
		}
	}
}

impl TerrainCellLayout {
	pub fn request_region(&self) -> Aabb3d {
		let size = self.cell_size.max(1e-3);
		let min = Vec3::new(
			self.origin.x as f32 * size,
			-size,
			self.origin.y as f32 * size,
		);
		let max = Vec3::new(
			(self.origin.x + self.extents.x as i32) as f32 * size,
			size,
			(self.origin.y + self.extents.y as i32) as f32 * size,
		);
		Aabb3d::from_min_max(min, max)
	}

	/// World-space center of the request region on XZ (Y = 0).
	pub fn region_center_xz(&self) -> Vec3 {
		let region = self.request_region();
		let min = Vec3::from(region.min);
		let max = Vec3::from(region.max);
		Vec3::new((min.x + max.x) * 0.5, 0.0, (min.z + max.z) * 0.5)
	}
}

/// Types that expose the active terrain cell layout for generation.
pub trait HasTerrainCellLayout {
	fn cell_layout(&self) -> &TerrainCellLayout;
}

/// Build an origin-cell AABB from integer cell coordinates on the XZ plane.
pub fn cell_bounds(ix: i32, iz: i32, cell_size: f32) -> Aabb3d {
	let size = cell_size.max(1e-3);
	let min = Vec3::new(ix as f32 * size, -size, iz as f32 * size);
	let max = Vec3::new((ix + 1) as f32 * size, size, (iz + 1) as f32 * size);
	Aabb3d::from_min_max(min, max)
}

/// Integer cell coordinates covering a region on XZ (Y ignored for tiling).
pub fn cell_coords_for_region(
	region: Aabb3d,
	cell_size: f32,
) -> impl Iterator<Item = (i32, i32)> {
	let size = cell_size.max(1e-3);
	let min_x = (region.min.x / size).floor() as i32;
	let max_x = (region.max.x / size).ceil() as i32 - 1;
	let min_z = (region.min.z / size).floor() as i32;
	let max_z = (region.max.z / size).ceil() as i32 - 1;
	(min_x..=max_x).flat_map(move |ix| (min_z..=max_z).map(move |iz| (ix, iz)))
}
