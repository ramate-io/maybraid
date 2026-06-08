//! Grove LOD footprint and overspill policy ([RFC-170 §3.1.3], [RFC-183 §3.4.2.3]).

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
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroveExtent {
	min: Vec3,
	max: Vec3,
}

impl GroveExtent {
	/// Union of all vegetation cell regions in one grove instance.
	pub fn from_cells(cells: &[Cell]) -> Option<Self> {
		let first = cells.first()?;
		let mut min = Vec3::from(first.as_region().min);
		let mut max = Vec3::from(first.as_region().max);
		for cell in cells.iter().skip(1) {
			let region = cell.as_region();
			let region_min = Vec3::from(region.min);
			let region_max = Vec3::from(region.max);
			min = min.min(region_min);
			max = max.max(region_max);
		}
		Some(Self { min, max })
	}

	pub fn min(&self) -> Vec3 {
		self.min
	}

	pub fn max(&self) -> Vec3 {
		self.max
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
	use bevy_math::bounding::Aabb3d;

	fn cell_at(x: f32, z: f32, extent: f32) -> Cell {
		let origin = Vec3::new(x, 0.0, z);
		Cell(Aabb3d::from_min_max(origin, origin + Vec3::new(extent, 1.0, extent)))
	}

	#[test]
	fn from_cells_unions_regions() -> Result<()> {
		let cells = vec![cell_at(0.0, 0.0, 4.0), cell_at(4.0, 0.0, 4.0)];
		let extent =
			GroveExtent::from_cells(&cells).ok_or_else(|| anyhow::anyhow!("missing extent"))?;
		assert_eq!(extent.min(), Vec3::new(0.0, 0.0, 0.0));
		assert_eq!(extent.max(), Vec3::new(8.0, 1.0, 4.0));
		Ok(())
	}

	#[test]
	fn discard_rejects_outside_extent() -> Result<()> {
		let extent = GroveExtent::from_cells(&[cell_at(0.0, 0.0, 4.0)])
			.ok_or_else(|| anyhow::anyhow!("missing extent"))?;
		let inside = Vec3::new(2.0, 0.0, 2.0);
		let outside = Vec3::new(6.0, 0.0, 2.0);
		assert_eq!(extent.resolve_xz(inside, GroveOverspillPolicy::Discard), Some(inside));
		assert_eq!(extent.resolve_xz(outside, GroveOverspillPolicy::Discard), None);
		Ok(())
	}

	#[test]
	fn reflect_folds_overspill_into_extent() -> Result<()> {
		let extent = GroveExtent::from_cells(&[cell_at(0.0, 0.0, 4.0)])
			.ok_or_else(|| anyhow::anyhow!("missing extent"))?;
		let reflected = extent
			.resolve_xz(Vec3::new(6.0, 0.0, -1.0), GroveOverspillPolicy::Reflect)
			.ok_or_else(|| anyhow::anyhow!("expected reflected position"))?;
		assert!((reflected.x - 2.0).abs() < 1e-5);
		assert!((reflected.z - 1.0).abs() < 1e-5);
		Ok(())
	}
}
