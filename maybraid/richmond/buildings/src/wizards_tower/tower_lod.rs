//! Shared Wizard's Tower LOD banding from max AABB extent → [`LodSceneLevel`].

use bevy::prelude::Transform;
use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use lod::lod_ref::LodRef;

/// High (exterior + internals) when distance ≤ this × max AABB extent.
pub const HIGH_EXTENT_MULTIPLIER: f32 = 5.0;
/// Medium (exterior walls only) out to this × max extent; beyond → Low cylinder.
pub const MEDIUM_EXTENT_MULTIPLIER: f32 = 500.0;

/// Deprecated aliases (same values as the extent multipliers).
pub const HIGH_RADIUS_MULTIPLIER: f32 = HIGH_EXTENT_MULTIPLIER;
pub const MEDIUM_RADIUS_MULTIPLIER: f32 = MEDIUM_EXTENT_MULTIPLIER;
/// Historical name; cylinder swap is gated by [`MEDIUM_EXTENT_MULTIPLIER`].
pub const LOW_RADIUS_MULTIPLIER: f32 = MEDIUM_EXTENT_MULTIPLIER;

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

	/// Characteristic size for LOD factors: max AABB axis.
	fn max_extent(&self) -> f32 {
		let aabb = self.lod_aabb();
		let extent = aabb.max - aabb.min;
		extent.x.max(extent.y).max(extent.z).max(1e-4)
	}

	fn tower_height(&self) -> f32 {
		let aabb = self.lod_aabb();
		(aabb.max.y - aabb.min.y).max(1e-4)
	}

	fn level_for(&self, viewer: &Transform) -> LodSceneLevel {
		let center = self.footprint_center_xz();
		let extent = self.max_extent();
		let p = viewer.translation;
		let dist = p.distance(center);
		let factor = dist / extent;
		if factor <= HIGH_EXTENT_MULTIPLIER {
			LodSceneLevel::High
		} else if factor <= MEDIUM_EXTENT_MULTIPLIER {
			LodSceneLevel::Medium
		} else {
			// Low + UltraLow both use the cylinder silhouette.
			LodSceneLevel::Low
		}
	}

	fn level_for_lod_ref(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.level_for(lod_ref.current_transform)
	}
}
