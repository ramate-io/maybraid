//! Pending player placement: destination, support kind, and collider readiness.
//!
//! Generation produces the destination. This module only decides when physics
//! may release the capsule. Elapsed time is not readiness.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

/// Outdoor presented ground vs authored interior floor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpawnSupport {
	Terrain,
	Floor,
}

impl SpawnSupport {
	pub const fn stands_on_floor(self) -> bool {
		matches!(self, Self::Floor)
	}
}

/// Why gravity is still held. Agent diagnostics print this; it is not a timer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlacementPendingReason {
	PhysicsDisabled,
	SurfaceUnpublished,
	ColliderMissing,
	ElevationUnsupported,
}

impl PlacementPendingReason {
	pub const fn as_str(self) -> &'static str {
		match self {
			Self::PhysicsDisabled => "physics-disabled",
			Self::SurfaceUnpublished => "surface-unpublished",
			Self::ColliderMissing => "collider-missing",
			Self::ElevationUnsupported => "elevation-unsupported",
		}
	}
}

/// Destination-specific hold until support exists at the intended elevation.
///
/// Distinct from [`super::player::OffTerrainAnchor`], which is a void-recovery
/// target for interiors and must not by itself enable gravity.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PendingPlacement {
	pub support: SpawnSupport,
	pub surface: Vec3,
}

impl PendingPlacement {
	pub fn terrain(surface: Vec3) -> Self {
		Self { support: SpawnSupport::Terrain, surface }
	}

	pub fn floor(surface: Vec3) -> Self {
		Self { support: SpawnSupport::Floor, surface }
	}

	pub fn stands_on_floor(self) -> bool {
		self.support.stands_on_floor()
	}

	/// Probe around the walk-on so a collider at the same XZ but a different
	/// storey does not count as ready.
	pub fn support_probe(self) -> Aabb3d {
		Aabb3d::from_min_max(
			self.surface + Vec3::new(-0.4, -0.25, -0.4),
			self.surface + Vec3::new(0.4, 0.45, 0.4),
		)
	}

	pub fn overlaps_volume(self, volume: Aabb3d) -> bool {
		aabb_intersects(self.support_probe(), volume)
	}
}

/// Test / presentation AABB that can support a pending capsule.
#[derive(Component, Clone, Copy, Debug)]
pub struct PlacementSupportVolume(pub Aabb3d);

impl PlacementSupportVolume {
	pub fn around(surface: Vec3, half_xz: f32, half_y: f32) -> Self {
		Self(Aabb3d::from_min_max(
			surface - Vec3::new(half_xz, half_y, half_xz),
			surface + Vec3::new(half_xz, half_y, half_xz),
		))
	}

	pub fn supports(self, pending: PendingPlacement) -> bool {
		pending.overlaps_volume(self.0)
	}
}

/// Live presented outdoor height under the awaiting player, if published.
///
/// Discovery writes padded height. Playgrounds leave this unset so snap uses
/// Durham composed height. Cleared once placement completes — not sampled
/// every frame after release.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct PresentedOutdoorHeight(pub Option<f32>);

pub fn aabb_intersects(a: Aabb3d, b: Aabb3d) -> bool {
	a.min.x <= b.max.x
		&& a.max.x >= b.min.x
		&& a.min.y <= b.max.y
		&& a.max.y >= b.min.y
		&& a.min.z <= b.max.z
		&& a.max.z >= b.min.z
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn floor_probe_rejects_a_column_at_the_wrong_storey() {
		let pending = PendingPlacement::floor(Vec3::new(10.0, 12.0, -4.0));
		let ground = PlacementSupportVolume::around(Vec3::new(10.0, 0.0, -4.0), 8.0, 1.0);
		let storey = PlacementSupportVolume::around(Vec3::new(10.0, 12.0, -4.0), 8.0, 0.2);
		assert!(!ground.supports(pending));
		assert!(storey.supports(pending));
	}

	#[test]
	fn terrain_probe_rejects_xz_coverage_without_elevation() {
		let pending = PendingPlacement::terrain(Vec3::new(0.0, 40.0, 0.0));
		let under = PlacementSupportVolume::around(Vec3::new(0.0, 2.0, 0.0), 20.0, 1.0);
		assert!(!under.supports(pending));
		assert!(
			PlacementSupportVolume::around(Vec3::new(0.0, 40.0, 0.0), 20.0, 1.0).supports(pending)
		);
	}
}
