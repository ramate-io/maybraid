//! Shared Wizard's Tower LOD banding from footprint radius → [`LodSceneLevel`].

use bevy::prelude::Transform;
use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use lod::lod_ref::LodRef;

/// High (exterior + internals) when XZ distance ≤ this × footprint radius.
pub const HIGH_RADIUS_MULTIPLIER: f32 = 3.0;
/// Medium (exterior walls only) out to this × radius.
pub const MEDIUM_RADIUS_MULTIPLIER: f32 = 8.0;
/// Beyond medium → Low cylinder silhouette.
pub const LOW_RADIUS_MULTIPLIER: f32 = 20.0;

/// Footprint-derived LOD helpers for the tower host.
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

	fn tower_height(&self) -> f32 {
		let aabb = self.lod_aabb();
		(aabb.max.y - aabb.min.y).max(1e-4)
	}

	fn level_for(&self, viewer: &Transform) -> LodSceneLevel {
		let center = self.footprint_center_xz();
		let radius = self.footprint_radius().max(1e-4);
		let p = viewer.translation;
		let dx = p.x - center.x;
		let dz = p.z - center.z;
		let dist_xz = (dx * dx + dz * dz).sqrt();
		let factor = dist_xz / radius;
		if factor <= HIGH_RADIUS_MULTIPLIER {
			LodSceneLevel::High
		} else if factor <= MEDIUM_RADIUS_MULTIPLIER {
			LodSceneLevel::Medium
		} else {
			// Low + UltraLow both use the cylinder silhouette.
			LodSceneLevel::Low
		}
	}

	fn level_for_lod_ref(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.level_for(lod_ref.current_transform)
	}

	/// Ball for [`richmond_building_components::ParentConfines::Internal`] on internals.
	///
	/// Scaled to envelop the tower's open interior (footprint × high band).
	fn internal_confine_ball(&self) -> (Vec3, f32) {
		let aabb = self.lod_aabb();
		let c = (aabb.min + aabb.max) * 0.5;
		let radius = self.footprint_radius() * HIGH_RADIUS_MULTIPLIER;
		(Vec3::from(c), radius.max(1e-4))
	}
}
