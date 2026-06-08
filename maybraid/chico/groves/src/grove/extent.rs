//! Grove LOD footprint and overspill policy ([RFC-170 §3.1.3], [RFC-183 §3.4.2.3]).

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use gimme_gen::Cell;
/// How candidate placements that exceed the grove LOD footprint are handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GroveOverspillPolicy {
	/// Reject candidates outside the grove extent, matching [RFC-170 §3.1.3].
	#[default]
	Discard,
	/// Fold overspill back into the grove footprint on XZ before constraint checks.
	Reflect,
}

/// Axis-aligned grove LOD unit in world space (first-order cell \(C\) in [RFC-170 §3.1.3]).
///
/// Vegetation cells may overspill their own bounds; ownership and culling derive from this
/// footprint, not from per-instance placement cells.
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

	/// Uniform XZ sampling cells inside this grove extent.
	pub fn subdivide_xz(&self, cells_per_axis: u32) -> Vec<Cell> {
		let count = cells_per_axis.max(1);
		let span = self.max - self.min;
		let cell_x = span.x / count as f32;
		let cell_z = span.z / count as f32;
		let mut cells = Vec::with_capacity((count * count) as usize);
		for x in 0..count {
			for z in 0..count {
				let min = Vec3::new(
					self.min.x + x as f32 * cell_x,
					self.min.y,
					self.min.z + z as f32 * cell_z,
				);
				let max = Vec3::new(min.x + cell_x, self.max.y, min.z + cell_z);
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

	/// Apply overspill policy to a candidate placement point.
	pub fn resolve_xz(&self, position: Vec3, policy: GroveOverspillPolicy) -> Option<Vec3> {
		match policy {
			GroveOverspillPolicy::Discard => {
				if self.contains_xz(position) {
					Some(position)
				} else {
					None
				}
			}
			GroveOverspillPolicy::Reflect => Some(reflect_xz(position, self.min, self.max)),
		}
	}
}

fn reflect_xz(mut position: Vec3, min: Vec3, max: Vec3) -> Vec3 {
	position.x = reflect_scalar(position.x, min.x, max.x);
	position.z = reflect_scalar(position.z, min.z, max.z);
	position
}

fn reflect_scalar(value: f32, min: f32, max: f32) -> f32 {
	let span = max - min;
	if span <= f32::EPSILON {
		return value.clamp(min, max);
	}
	let mut v = value;
	for _ in 0..8 {
		if v >= min && v <= max {
			return v;
		}
		if v < min {
			v = min + (min - v);
		} else {
			v = max - (v - max);
		}
	}
	v.clamp(min, max)
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
	fn discard_rejects_outside_extent() -> Result<()> {
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(4.0, 1.0, 4.0));
		let inside = Vec3::new(2.0, 0.0, 2.0);
		let outside = Vec3::new(6.0, 0.0, 2.0);
		assert_eq!(extent.resolve_xz(inside, GroveOverspillPolicy::Discard), Some(inside));
		assert_eq!(extent.resolve_xz(outside, GroveOverspillPolicy::Discard), None);
		Ok(())
	}

	#[test]
	fn reflect_folds_overspill_into_extent() -> Result<()> {
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(4.0, 1.0, 4.0));
		let reflected = extent
			.resolve_xz(Vec3::new(6.0, 0.0, -1.0), GroveOverspillPolicy::Reflect)
			.ok_or_else(|| anyhow::anyhow!("expected reflected position"))?;
		assert!((reflected.x - 2.0).abs() < 1e-5);
		assert!((reflected.z - 1.0).abs() < 1e-5);
		Ok(())
	}
}
