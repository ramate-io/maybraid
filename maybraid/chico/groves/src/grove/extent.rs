//! Grove LOD footprint ([RFC-170 §3.1.3], [RFC-183 §3.4.2.3]).

use bevy_math::bounding::Aabb3d;
use bevy_math::{Vec2, Vec3};
use gimme_gen::Cell;

/// Default square grove preview / isolation-render footprint in metres on X and Z.
pub const DEFAULT_GROVE_EXTENT_XZ: f32 = 100.0;

/// Axis-aligned grove LOD unit in world space (first-order cell \(C\) in [RFC-170 §3.1.3]).
///
/// Vegetation cells may overspill their own bounds; ownership and culling derive from this
/// footprint, not from per-instance placement cells. Candidates outside the footprint are
/// discarded.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GroveExtent {
	min: Vec3,
	max: Vec3,
}

impl GroveExtent {
	pub fn new(min: Vec3, max: Vec3) -> Self {
		Self { min: min.min(max), max: min.max(max) }
	}

	pub fn min(&self) -> Vec3 {
		self.min
	}

	pub fn max(&self) -> Vec3 {
		self.max
	}

	/// Square-ish sampling cells with the requested world-space XZ span.
	///
	/// The grove extent owns the area; `cell_extent_xz` only determines how many internal
	/// sampling cells are needed. Edge cells are clipped to the grove extent.
	pub fn subdivide_xz(&self, cell_extent_xz: Vec2) -> Vec<Cell> {
		let span = self.max - self.min;
		let target = cell_extent_xz.max(Vec2::splat(0.1));
		let x_count = (span.x / target.x).ceil().max(1.0) as u32;
		let z_count = (span.z / target.y).ceil().max(1.0) as u32;
		let mut cells = Vec::with_capacity((x_count * z_count) as usize);
		for x in 0..x_count {
			for z in 0..z_count {
				let min = Vec3::new(
					self.min.x + x as f32 * target.x,
					self.min.y,
					self.min.z + z as f32 * target.y,
				);
				let max = Vec3::new(
					(min.x + target.x).min(self.max.x),
					self.max.y,
					(min.z + target.y).min(self.max.z),
				);
				cells.push(Cell(Aabb3d::from_min_max(min, max)));
			}
		}
		cells
	}

	/// Whether `position` lies inside the grove footprint on XZ (Y is ignored).
	pub fn contains_xz(&self, position: Vec3) -> bool {
		position.x >= self.min.x
			&& position.x <= self.max.x
			&& position.z >= self.min.z
			&& position.z <= self.max.z
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn new_orders_bounds() -> Result<()> {
		let extent = GroveExtent::new(Vec3::new(8.0, 1.0, 4.0), Vec3::ZERO);
		assert_eq!(extent.min(), Vec3::new(0.0, 0.0, 0.0));
		assert_eq!(extent.max(), Vec3::new(8.0, 1.0, 4.0));
		Ok(())
	}

	#[test]
	fn subdivide_xz_uses_independent_cell_axes() -> Result<()> {
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(10.0, 1.0, 9.0));
		let cells = extent.subdivide_xz(Vec2::new(4.0, 3.0));
		assert_eq!(cells.len(), 9);
		let last = cells
			.last()
			.ok_or_else(|| anyhow::anyhow!("expected at least one subdivision cell"))?;
		assert_eq!(Vec3::from(last.as_region().min), Vec3::new(8.0, 0.0, 6.0));
		assert_eq!(Vec3::from(last.as_region().max), Vec3::new(10.0, 1.0, 9.0));
		Ok(())
	}

	#[test]
	fn contains_xz_ignores_y() -> Result<()> {
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(4.0, 1.0, 4.0));
		assert!(extent.contains_xz(Vec3::new(2.0, 99.0, 2.0)));
		assert!(!extent.contains_xz(Vec3::new(6.0, 0.0, 2.0)));
		Ok(())
	}
}
