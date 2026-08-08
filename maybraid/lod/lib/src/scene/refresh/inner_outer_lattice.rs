//! Even cubic lattice: fine = center cell, coarse = Chebyshev ring around the driver.

use bevy::math::bounding::Aabb3d;
use bevy::math::{IVec3, Vec3};
use bevy::prelude::*;

use crate::lod_ref::LodRef;

use super::mark::LodSceneRefreshRegions;
use super::regions::{LodRefreshRegions, LodRefreshRegionsStatus};

/// Square-cell lattice ringing out in 3D around a [`LodRef`].
///
/// - **Fine:** the single cell containing the driver.
/// - **Coarse:** all other cells with Chebyshev distance `≤ ring_radius`.
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

	fn cell_aabb(&self, index: IVec3) -> Aabb3d {
		let s = self.cell_size;
		let min = index.as_vec3() * s;
		Aabb3d::from_min_max(min, min + Vec3::splat(s))
	}

	fn regions_at(&self, center: IVec3) -> LodSceneRefreshRegions {
		let r = self.ring_radius as i32;
		let mut fine = Vec::with_capacity(1);
		let extent = (2 * self.ring_radius as usize + 1).pow(3).saturating_sub(1);
		let mut coarse = Vec::with_capacity(extent);

		for dx in -r..=r {
			for dy in -r..=r {
				for dz in -r..=r {
					let index = center + IVec3::new(dx, dy, dz);
					let aabb = self.cell_aabb(index);
					if dx == 0 && dy == 0 && dz == 0 {
						fine.push(aabb);
					} else {
						coarse.push(aabb);
					}
				}
			}
		}

		LodSceneRefreshRegions { fine, coarse }
	}
}

impl LodRefreshRegions for InnerOuterLattice {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		let current = self.cell_index(lod_ref.current_transform.translation);
		let previous = self.cell_index(lod_ref.previous_transform.translation);
		if current == previous {
			return LodRefreshRegionsStatus::Unchanged;
		}
		LodRefreshRegionsStatus::Changed(self.regions_at(current))
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
	fn fine_is_center_coarse_is_ring() {
		let lattice = InnerOuterLattice::new(100.0, 1);
		let prev = Transform::from_translation(Vec3::new(-50.0, 0.0, 0.0));
		let curr = Transform::from_translation(Vec3::new(10.0, 10.0, 10.0));
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ZERO);
		let LodRefreshRegionsStatus::Changed(regions) =
			lattice.lod_refresh_regions(&lod_ref_at(&prev, &curr, &bounds))
		else {
			panic!("expected Changed");
		};
		assert_eq!(regions.fine.len(), 1);
		// 3^3 - 1 = 26 coarse cells for radius 1
		assert_eq!(regions.coarse.len(), 26);
		let fine = &regions.fine[0];
		assert_eq!(fine.min.x, 0.0);
		assert_eq!(fine.max.x, 100.0);
	}
}
