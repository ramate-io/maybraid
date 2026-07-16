//! Terrain cell size and origin tiling helpers.

use bevy::math::bounding::Aabb3d;
use bevy::math::{IVec2, UVec2, Vec3};
use bevy::prelude::*;

/// Naturescapes cascade `min_size`.
pub const NATURESCAPES_MIN_SIZE: f32 = 20.0;

/// Naturescapes `grid_multiple_2` (chunk size = [`NATURESCAPES_MIN_SIZE`] × 2^this).
pub const NATURESCAPES_GRID_MULTIPLE_2: u8 = 3;

/// Naturescapes `grid_radius` X/Z (inclusive range is `[-r, r]` → `2r + 1` cells).
pub const NATURESCAPES_GRID_RADIUS_XZ: i32 = 12;

/// Default edge length of a procedural terrain origin cell (world units).
///
/// Matches naturescapes grid chunk size: `min_size * 2^grid_multiple_2` = 20 × 8.
pub const TERRAIN_CELL_SIZE: f32 =
	NATURESCAPES_MIN_SIZE * (1_u32 << NATURESCAPES_GRID_MULTIPLE_2) as f32;

/// Default cell count along +X / +Z (`2 * grid_radius + 1`).
pub const TERRAIN_CELL_EXTENTS_XZ: u32 = (2 * NATURESCAPES_GRID_RADIUS_XZ + 1) as u32;

/// Default cell-grid origin so the request region is centered like naturescapes at the world origin.
pub const TERRAIN_CELL_ORIGIN: IVec2 =
	IVec2::new(-NATURESCAPES_GRID_RADIUS_XZ, -NATURESCAPES_GRID_RADIUS_XZ);

/// Default vertical half-extent so cells cover naturescapes-scale heightfields
/// (`height_scale=500`, bedrock at `-4 * height_scale`).
pub const TERRAIN_CELL_VERTICAL_HALF_EXTENT: f32 = 2000.0;

/// Layout for tiling terrain origin cells in the XZ plane.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct TerrainCellLayout {
	/// Edge length of each origin cell in world units.
	pub cell_size: f32,
	/// Half-extent along Y for cell bounds / SDF sampling volumes.
	pub vertical_half_extent: f32,
	/// Cell-grid coordinates of the min corner (XZ).
	pub origin: IVec2,
	/// Number of cells along +X and +Z from [`Self::origin`].
	pub extents: UVec2,
}

impl Default for TerrainCellLayout {
	fn default() -> Self {
		Self {
			cell_size: TERRAIN_CELL_SIZE,
			vertical_half_extent: TERRAIN_CELL_VERTICAL_HALF_EXTENT,
			origin: TERRAIN_CELL_ORIGIN,
			extents: UVec2::new(TERRAIN_CELL_EXTENTS_XZ, TERRAIN_CELL_EXTENTS_XZ),
		}
	}
}

impl TerrainCellLayout {
	pub fn request_region(&self) -> Aabb3d {
		let size = self.cell_size.max(1e-3);
		let vy = self.vertical_half_extent.max(size);
		let min = Vec3::new(
			self.origin.x as f32 * size,
			-vy,
			self.origin.y as f32 * size,
		);
		let max = Vec3::new(
			(self.origin.x + self.extents.x as i32) as f32 * size,
			vy,
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
pub fn cell_bounds(ix: i32, iz: i32, cell_size: f32, vertical_half_extent: f32) -> Aabb3d {
	let size = cell_size.max(1e-3);
	let vy = vertical_half_extent.max(size);
	let min = Vec3::new(ix as f32 * size, -vy, iz as f32 * size);
	let max = Vec3::new((ix + 1) as f32 * size, vy, (iz + 1) as f32 * size);
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
