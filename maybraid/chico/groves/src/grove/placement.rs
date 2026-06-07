//! Candidate position selection ([RFC-183 3.4.2.3]).

use bevy_math::{bounding::BoundingVolume, Vec2, Vec3};
use gimme_gen::Cell;

/// Parent cell origin used for placement ownership.
pub fn cell_origin(cell: &Cell) -> Vec3 {
	cell.as_region().center().into()
}

/// Deterministic candidate point from cell origin plus authored offset.
pub fn candidate_position(cell: &Cell, offset: Vec2) -> Vec3 {
	let origin = cell_origin(cell);
	Vec3::new(origin.x + offset.x, origin.y, origin.z + offset.y)
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
		let p = candidate_position(&cell, Vec2::new(1.0, -2.0));
		assert!((p.x - 6.0).abs() < 1e-5);
		assert!((p.z - 3.0).abs() < 1e-5);
		Ok(())
	}
}
