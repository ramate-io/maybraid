//! Shared Wizard's Tower LOD banding (near / far) from footprint radius.

use bevy::prelude::Transform;
use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::gen::LodSceneStatus;
use lod::lod_ref::LodRef;
use richmond_building_components::partitions::WallNode;

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

	/// Near/Far flip, else per-storey representative external wall mesh LOD.
	///
	/// Does not walk wall children or internal floors/stairs.
	fn storey_scene_lod_status(&self, storey_height: f32, lod_ref: &LodRef) -> LodSceneStatus {
		let prev = self.band_for(lod_ref.previous_transform);
		let curr = self.band_for(lod_ref.current_transform);
		if prev != curr {
			return LodSceneStatus::Changed;
		}
		let aabb = self.lod_aabb();
		let center = Vec3::from((aabb.min + aabb.max) * 0.5);
		let radius = self.footprint_radius().max(1e-4);
		let extent = Vec3::new(radius, storey_height.max(1e-4), radius);
		WallNode::representative_lod_status(center, extent, lod_ref)
	}
}
