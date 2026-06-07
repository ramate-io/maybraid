//! Grid coordinate math and canonical [`Cell`] construction ([RFC-142 §3.1]).

use bevy_math::bounding::Aabb3d;
use bevy_math::{DVec3, IVec3, Vec3A};

use crate::cell::Cell;
use crate::error::SpatialIndexError;

/// Multi-resolution grid level `d`; cell size is `base_scale * 2^d` per axis.
pub type Level = u32;

/// RFC base cell scale `d₀`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BaseScale(DVec3);

impl BaseScale {
	pub fn new(scale: DVec3) -> Result<Self, SpatialIndexError> {
		if !scale.x.is_finite()
			|| !scale.y.is_finite()
			|| !scale.z.is_finite()
			|| scale.x <= 0.0
			|| scale.y <= 0.0
			|| scale.z <= 0.0
		{
			return Err(SpatialIndexError::InvalidBaseScale);
		}
		Ok(Self(scale))
	}

	pub fn as_dvec3(self) -> DVec3 {
		self.0
	}

	pub fn cell_size_at(self, level: Level) -> DVec3 {
		let factor = 2_f64.powi(level as i32);
		self.0 * factor
	}

	/// Canonical world-space bounds for grid slot `coord` at `level`.
	pub fn cell_at(self, level: Level, coord: IVec3) -> Cell {
		let size = self.cell_size_at(level);
		let min = DVec3::new(
			coord.x as f64 * size.x,
			coord.y as f64 * size.y,
			coord.z as f64 * size.z,
		);
		let max = min + size;
		Cell::from_min_max(
			Vec3A::new(min.x as f32, min.y as f32, min.z as f32),
			Vec3A::new(max.x as f32, max.y as f32, max.z as f32),
		)
	}

	/// All canonical grid cells intersecting `bounds` at `level`.
	pub fn enumerate_cells(self, bounds: &Aabb3d, level: Level) -> Vec<Cell> {
		let cell_size = self.cell_size_at(level);
		let (min_coord, max_coord) = Self::grid_coord_range(bounds, cell_size);
		let mut cells = Vec::new();
		for x in min_coord.x..=max_coord.x {
			for y in min_coord.y..=max_coord.y {
				for z in min_coord.z..=max_coord.z {
					cells.push(self.cell_at(level, IVec3::new(x, y, z)));
				}
			}
		}
		cells
	}

	/// Smallest level whose cells can bound `bounds`.
	pub fn insertion_level(self, bounds: &Aabb3d) -> Level {
		let extent = DVec3::new(
			(bounds.max.x - bounds.min.x) as f64,
			(bounds.max.y - bounds.min.y) as f64,
			(bounds.max.z - bounds.min.z) as f64,
		);
		let cells_x = (extent.x / self.0.x).ceil().max(1.0) as u32;
		let cells_y = (extent.y / self.0.y).ceil().max(1.0) as u32;
		let cells_z = (extent.z / self.0.z).ceil().max(1.0) as u32;
		let max_cells = cells_x.max(cells_y).max(cells_z).max(1);
		Self::ceil_log2_u32(max_cells)
	}

	/// Suggested query levels for objects that might occupy `bounds`.
	pub fn levels_for_bounds(self, bounds: &Aabb3d) -> impl Iterator<Item = Level> {
		Self::levels_through(self.insertion_level(bounds))
	}

	/// Iterate levels `0..=max` inclusive.
	pub fn levels_through(max: Level) -> impl Iterator<Item = Level> {
		0..=max
	}

	/// RFC `ceil(log2(n))` for unsigned extent in base-cell units.
	pub fn ceil_log2_u32(n: u32) -> Level {
		debug_assert!(n > 0);
		if n <= 1 { 0 } else { u32::BITS - (n - 1).leading_zeros() }
	}

	fn grid_coord(world: DVec3, cell_size: DVec3) -> IVec3 {
		IVec3::new(
			(world.x / cell_size.x).floor() as i32,
			(world.y / cell_size.y).floor() as i32,
			(world.z / cell_size.z).floor() as i32,
		)
	}

	fn grid_coord_range(bounds: &Aabb3d, cell_size: DVec3) -> (IVec3, IVec3) {
		let min = DVec3::new(bounds.min.x as f64, bounds.min.y as f64, bounds.min.z as f64);
		let max = DVec3::new(bounds.max.x as f64, bounds.max.y as f64, bounds.max.z as f64);
		(Self::grid_coord(min, cell_size), Self::grid_coord(max, cell_size))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use bevy_math::Vec3;

	fn aabb(min: [f32; 3], max: [f32; 3]) -> Aabb3d {
		Aabb3d::from_min_max(Vec3::from_array(min), Vec3::from_array(max))
	}

	#[test]
	fn ceil_log2_matches_rfc_table() -> Result<()> {
		assert_eq!(BaseScale::ceil_log2_u32(1), 0);
		assert_eq!(BaseScale::ceil_log2_u32(2), 1);
		assert_eq!(BaseScale::ceil_log2_u32(3), 2);
		assert_eq!(BaseScale::ceil_log2_u32(4), 2);
		assert_eq!(BaseScale::ceil_log2_u32(5), 3);
		Ok(())
	}

	#[test]
	fn unit_cube_is_level_zero() -> Result<()> {
		let base = BaseScale::new(DVec3::ONE)?;
		let bounds = aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
		assert_eq!(base.insertion_level(&bounds), 0);
		Ok(())
	}

	#[test]
	fn double_extent_is_level_one() -> Result<()> {
		let base = BaseScale::new(DVec3::ONE)?;
		let bounds = aabb([0.0, 0.0, 0.0], [2.0, 1.0, 1.0]);
		assert_eq!(base.insertion_level(&bounds), 1);
		Ok(())
	}

	#[test]
	fn level_zero_cell_covers_unit_cube() -> Result<()> {
		let base = BaseScale::new(DVec3::ONE)?;
		let cell = base.cell_at(0, IVec3::ZERO);
		assert_eq!(cell.as_region().min, Vec3::ZERO.into());
		assert_eq!(cell.as_region().max, Vec3::ONE.into());
		Ok(())
	}

	#[test]
	fn enumerate_spans_two_by_two_footprint() -> Result<()> {
		let base = BaseScale::new(DVec3::ONE)?;
		let bounds = aabb([0.0, 0.0, 0.0], [1.5, 0.5, 1.5]);
		let cells = base.enumerate_cells(&bounds, 0);
		assert_eq!(cells.len(), 4);
		Ok(())
	}
}
