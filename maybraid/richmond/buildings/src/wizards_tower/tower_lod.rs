//! Shared Wizard's Tower LOD banding (near / far) from footprint radius.

use bevy::prelude::Transform;
use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;

/// Viewer distance band relative to the tower footprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TowerLodBand {
	/// Within [`NEAR_RADIUS_MULTIPLIER`] × footprint radius (XZ).
	Near,
	/// Outside the near band — external silhouette only.
	Far,
}

/// Near LOD when XZ distance ≤ this × footprint radius.
pub const NEAR_RADIUS_MULTIPLIER: f32 = 3.0;

/// Footprint-derived LOD helpers for tower storeys / column / root.
pub(crate) trait TowerLodFootprint {
	fn lod_aabb(&self) -> &Aabb3d;

	fn footprint_center_xz(&self) -> Vec3 {
		let aabb = self.lod_aabb();
		let c = (aabb.min + aabb.max) * 0.5;
		Vec3::new(c.x, aabb.min.y, c.z)
	}

	fn footprint_radius(&self) -> f32 {
		let aabb = self.lod_aabb();
		let extent = aabb.max - aabb.min;
		0.5 * extent.x.min(extent.z)
	}

	fn band_for(&self, viewer: &Transform) -> TowerLodBand {
		let center = self.footprint_center_xz();
		let radius = self.footprint_radius().max(1e-4);
		let p = viewer.translation;
		let dx = p.x - center.x;
		let dz = p.z - center.z;
		let dist_xz = (dx * dx + dz * dz).sqrt();
		if dist_xz <= NEAR_RADIUS_MULTIPLIER * radius {
			TowerLodBand::Near
		} else {
			TowerLodBand::Far
		}
	}

	fn is_near(&self, viewer: &Transform) -> bool {
		matches!(self.band_for(viewer), TowerLodBand::Near)
	}
}
