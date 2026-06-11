//! Per-cell placement geometry ([RFC-183 3.4.2.3]).

use super::terrain::TerrainSample;
use bevy_math::{bounding::BoundingVolume, Vec3};
use gimme_gen::Cell;
/// World-space horizontal shift from a parent cell center ([RFC-183 §3.4.1.4]).
///
/// Signed **metres** from the cell center on X and Z (not fractions of cell extent).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellXzOffset {
	pub x: f32,
	pub z: f32,
}

impl CellXzOffset {
	pub const fn new(x: f32, z: f32) -> Self {
		Self { x, z }
	}

	pub const ZERO: Self = Self::new(0.0, 0.0);

	/// Parent cell center used for placement ownership.
	pub fn cell_center(cell: &Cell) -> Vec3 {
		cell.as_region().center().into()
	}

	/// Candidate world point for this offset in `cell`.
	pub fn place_in(self, cell: &Cell, elevation: &impl TerrainSample) -> Vec3 {
		let center = Self::cell_center(cell);
		Vec3::new(center.x + self.x, elevation.elevation_at(center), center.z + self.z)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use bevy_math::bounding::Aabb3d;
	use gimme_gen::Cell;

	#[test]
	fn offset_shifts_from_cell_center() -> Result<()> {
		let cell = Cell(Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(10.0, 1.0, 10.0)));
		let p = CellXzOffset::new(1.0, -2.0).place_in(&cell, &0.0);
		assert!((p.x - 6.0).abs() < 1e-5);
		assert!((p.z - 3.0).abs() < 1e-5);
		Ok(())
	}
}
