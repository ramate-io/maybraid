//! Even cubic lattice: max-extent AABB of a Chebyshev ring around the driver.

use bevy::math::bounding::Aabb3d;
use bevy::math::{IVec3, Vec3};
use bevy::prelude::*;

use crate::lod_ref::LodRef;

use super::produce::{LodRefreshRegions, LodRefreshRegionsStatus};

/// Square-cell lattice ringing out in 3D around a [`LodRef`].
///
/// On cell change, emits the axis-aligned max extent of all cells with Chebyshev
/// distance `≤ ring_radius` (including the center cell).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct InnerOuterLattice {
	/// Edge length of each cubic cell (world units).
	pub cell_size: f32,
	/// Inclusive Chebyshev radius in cells (0 = center only).
	pub ring_radius: u32,
}

impl Default for InnerOuterLattice {
	fn default() -> Self {
		Self {
			cell_size: 100.0,
			ring_radius: 10,
		}
	}
}

impl InnerOuterLattice {
	pub fn new(cell_size: f32, ring_radius: u32) -> Self {
		Self {
			cell_size,
			ring_radius,
		}
	}

	fn cell_index(&self, point: Vec3) -> IVec3 {
		let s = self.cell_size;
		IVec3::new(
			(point.x / s).floor() as i32,
			(point.y / s).floor() as i32,
			(point.z / s).floor() as i32,
		)
	}

	/// Max AABB covering the Chebyshev ring around `center` (inclusive).
	fn ring_aabb(&self, center: IVec3) -> Aabb3d {
		let s = self.cell_size;
		let r = self.ring_radius as i32;
		let min_idx = center - IVec3::splat(r);
		let max_idx = center + IVec3::splat(r);
		let min = min_idx.as_vec3() * s;
		let max = (max_idx + IVec3::ONE).as_vec3() * s;
		Aabb3d::from_min_max(min, max)
	}
}

impl LodRefreshRegions for InnerOuterLattice {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		let current = self.cell_index(lod_ref.current_transform.translation);
		let previous = self.cell_index(lod_ref.previous_transform.translation);
		if current == previous {
			return LodRefreshRegionsStatus::Unchanged;
		}
		LodRefreshRegionsStatus::Changed(self.ring_aabb(current))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::Transform;

	fn lod_ref_at<'a>(
		prev: &'a Transform,
		curr: &'a Transform,
		bounds: &'a Aabb3d,
	) -> LodRef<'a> {
		LodRef {
			entity: Entity::from_bits(1),
			previous_transform: prev,
			current_transform: curr,
			bounds,
		}
	}

	#[test]
	fn unchanged_when_cell_stable() {
		let lattice = InnerOuterLattice::new(100.0, 1);
		let prev = Transform::from_translation(Vec3::new(10.0, 10.0, 10.0));
		let curr = Transform::from_translation(Vec3::new(20.0, 20.0, 20.0));
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ZERO);
		let status = lattice.lod_refresh_regions(&lod_ref_at(&prev, &curr, &bounds));
		assert!(matches!(status, LodRefreshRegionsStatus::Unchanged));
	}

	#[test]
	fn changed_is_max_extent_of_ring() {
		let lattice = InnerOuterLattice::new(100.0, 1);
		let prev = Transform::from_translation(Vec3::new(-50.0, 0.0, 0.0));
		let curr = Transform::from_translation(Vec3::new(10.0, 10.0, 10.0));
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ZERO);
		let LodRefreshRegionsStatus::Changed(region) =
			lattice.lod_refresh_regions(&lod_ref_at(&prev, &curr, &bounds))
		else {
			panic!("expected Changed");
		};
		// center cell [0,100)^3, radius 1 → cells [-1,1]^3 → AABB [-100, 200]
		assert_eq!(region.min.x, -100.0);
		assert_eq!(region.max.x, 200.0);
		assert_eq!(region.min.y, -100.0);
		assert_eq!(region.max.y, 200.0);
	}
}
