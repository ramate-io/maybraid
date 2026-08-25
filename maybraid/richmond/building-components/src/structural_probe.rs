//! Building-level structural LOD probe (meters outside XZ footprint).
//!
//! Distinct from mesh-resolution probes ([`crate::panels::PanelLodProbe`], …):
//! this selects which *layers* of authored IR a composite building emits
//! (e.g. internal walls on High only).
//!
//! Footprints are authored in the host's **local** XZ. Fine-phase updates map the
//! viewer into that local frame via [`GlobalTransform`] so gallery offsets work.

use bevy::prelude::{Component, GlobalTransform, Query, Transform, With};
use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use lod::gen::{LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;
use lod::lod_scene_host::LodSceneHost;

/// High while the viewer is at most this many meters outside the XZ perimeter.
pub const STRUCTURAL_HIGH_OUTSIDE_METERS: f32 = 80.0;

/// Viewer distance band for whole-building structural thinning.
///
/// Footprints are axis-aligned XZ rectangles (`Aabb2d` with \(y\) = world \(z\))
/// in the **host local** frame. Distance is planar meters outside the nearest
/// footprint (0 inside).
#[derive(Debug, Clone, PartialEq, Component)]
pub struct BuildingStructuralLodProbe {
	pub footprints: Vec<Aabb2d>,
	pub high_outside_meters: f32,
}

impl Default for BuildingStructuralLodProbe {
	fn default() -> Self {
		Self { footprints: Vec::new(), high_outside_meters: STRUCTURAL_HIGH_OUTSIDE_METERS }
	}
}

impl BuildingStructuralLodProbe {
	pub fn new(footprints: impl IntoIterator<Item = Aabb2d>) -> Self {
		Self {
			footprints: footprints.into_iter().collect(),
			high_outside_meters: STRUCTURAL_HIGH_OUTSIDE_METERS,
		}
	}

	pub fn from_aabb3d_xz(min: Vec3, max: Vec3) -> Self {
		Self::new([Aabb2d { min: Vec2::new(min.x, min.z), max: Vec2::new(max.x, max.z) }])
	}

	pub fn with_high_outside_meters(mut self, meters: f32) -> Self {
		self.high_outside_meters = meters.max(0.0);
		self
	}

	/// Append another probe's footprints (keep the tighter high cutoff).
	pub fn merge(mut self, other: Self) -> Self {
		self.footprints.extend(other.footprints);
		self.high_outside_meters = self.high_outside_meters.min(other.high_outside_meters);
		self
	}

	/// Meters outside the nearest footprint in local XZ (0 when inside any).
	pub fn distance_outside_local(&self, viewer_local: Vec3) -> f32 {
		distance_outside_footprints(viewer_local, &self.footprints)
	}

	/// [`distance_outside_local`] treating `viewer.translation` as already local.
	pub fn distance_outside(&self, viewer: &Transform) -> f32 {
		self.distance_outside_local(viewer.translation)
	}

	pub fn level_for_local(&self, viewer_local: Vec3) -> LodSceneLevel {
		if self.distance_outside_local(viewer_local) <= self.high_outside_meters {
			LodSceneLevel::High
		} else {
			LodSceneLevel::Medium
		}
	}

	/// Level when `viewer` is in the same space as the footprints (usually local).
	pub fn level_for(&self, viewer: &Transform) -> LodSceneLevel {
		self.level_for_local(viewer.translation)
	}

	/// Level for a world-space viewer against local footprints on `host_global`.
	pub fn level_for_world(
		&self,
		viewer_world: Vec3,
		host_global: &GlobalTransform,
	) -> LodSceneLevel {
		let viewer_local = host_global.affine().inverse().transform_point3(viewer_world);
		self.level_for_local(viewer_local)
	}

	pub fn status_for_lod_ref(&self, lod_ref: &LodRef) -> LodSceneStatus {
		let prev = self.level_for(lod_ref.previous_transform);
		let curr = self.level_for(lod_ref.current_transform);
		if prev == curr {
			LodSceneStatus::Unchanged
		} else {
			LodSceneStatus::Changed(curr)
		}
	}

	/// Coarse local AABB covering all XZ footprints (unit height when empty).
	pub fn footprint_aabb(&self) -> Aabb3d {
		if self.footprints.is_empty() {
			return Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		}
		let mut min_x = f32::INFINITY;
		let mut max_x = f32::NEG_INFINITY;
		let mut min_z = f32::INFINITY;
		let mut max_z = f32::NEG_INFINITY;
		for rect in &self.footprints {
			min_x = min_x.min(rect.min.x);
			max_x = max_x.max(rect.max.x);
			min_z = min_z.min(rect.min.y);
			max_z = max_z.max(rect.max.y);
		}
		Aabb3d::from_min_max(Vec3::new(min_x, 0.0, min_z), Vec3::new(max_x, 1.0, max_z))
	}
}

/// Planar distance outside an XZ footprint (`Aabb2d.y` = world \(z\)).
pub fn distance_outside_aabb2d_xz(p: Vec3, rect: &Aabb2d) -> f32 {
	let dx = if p.x < rect.min.x {
		rect.min.x - p.x
	} else if p.x > rect.max.x {
		p.x - rect.max.x
	} else {
		0.0
	};
	let dz = if p.z < rect.min.y {
		rect.min.y - p.z
	} else if p.z > rect.max.y {
		p.z - rect.max.y
	} else {
		0.0
	};
	if dx <= 0.0 && dz <= 0.0 {
		0.0
	} else {
		(dx * dx + dz * dz).sqrt()
	}
}

/// Min distance outside a set of footprints (`∞` when empty).
pub fn distance_outside_footprints(p: Vec3, footprints: &[Aabb2d]) -> f32 {
	if footprints.is_empty() {
		return f32::INFINITY;
	}
	footprints
		.iter()
		.map(|r| distance_outside_aabb2d_xz(p, r))
		.fold(f32::INFINITY, f32::min)
}

/// Update structural building host levels from the [`lod::LodViewer`] pose.
///
/// Viewer is world-space; footprints are host-local — convert via [`GlobalTransform`].
pub fn update_building_structural_host_levels(
	viewer: Query<&lod::LodNodePose, With<lod::LodViewer>>,
	mut hosts: Query<
		(&BuildingStructuralLodProbe, &GlobalTransform, &mut LodSceneLevel),
		With<LodSceneHost>,
	>,
) {
	let Ok(pose) = viewer.single() else {
		return;
	};
	let viewer_world = pose.current.translation;
	for (probe, global, mut level) in &mut hosts {
		let next = probe.level_for_world(viewer_world, global);
		if *level != next {
			*level = next;
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn inside_footprint_is_high() {
		let probe = BuildingStructuralLodProbe::from_aabb3d_xz(
			Vec3::new(-5.0, 0.0, -5.0),
			Vec3::new(5.0, 3.0, 5.0),
		);
		let viewer = Transform::from_xyz(0.0, 1.5, 0.0);
		assert_eq!(probe.distance_outside(&viewer), 0.0);
		assert_eq!(probe.level_for(&viewer), LodSceneLevel::High);
	}

	#[test]
	fn switches_to_medium_past_high_outside_meters() {
		let probe = BuildingStructuralLodProbe::from_aabb3d_xz(
			Vec3::new(-5.0, 0.0, -5.0),
			Vec3::new(5.0, 3.0, 5.0),
		)
		.with_high_outside_meters(20.0);
		let near = Transform::from_xyz(5.0 + 19.0, 1.5, 0.0);
		let far = Transform::from_xyz(5.0 + 21.0, 1.5, 0.0);
		assert_eq!(probe.level_for(&near), LodSceneLevel::High);
		assert_eq!(probe.level_for(&far), LodSceneLevel::Medium);
	}

	#[test]
	fn merge_uses_nearest_footprint() {
		let a = BuildingStructuralLodProbe::from_aabb3d_xz(
			Vec3::new(-10.0, 0.0, -2.0),
			Vec3::new(-6.0, 3.0, 2.0),
		);
		let b = BuildingStructuralLodProbe::from_aabb3d_xz(
			Vec3::new(6.0, 0.0, -2.0),
			Vec3::new(10.0, 3.0, 2.0),
		);
		let probe = a.merge(b);
		let near_b = Transform::from_xyz(10.0 + 5.0, 1.5, 0.0);
		assert!((probe.distance_outside(&near_b) - 5.0).abs() < 1e-4);
	}

	#[test]
	fn world_viewer_respects_host_translation() {
		let probe = BuildingStructuralLodProbe::from_aabb3d_xz(
			Vec3::new(-5.0, 0.0, -5.0),
			Vec3::new(5.0, 3.0, 5.0),
		)
		.with_high_outside_meters(20.0);
		// Building placed 200 m away; local footprint still [-5,5].
		let host = GlobalTransform::from_translation(Vec3::new(200.0, 0.0, 0.0));
		// World point just outside the *placed* building → High.
		let near_world = Vec3::new(200.0 + 5.0 + 10.0, 1.5, 0.0);
		assert_eq!(probe.level_for_world(near_world, &host), LodSceneLevel::High);
		// Same offset from origin (no host transform) would look far from local footprint.
		assert_eq!(
			probe.level_for(&Transform::from_translation(near_world)),
			LodSceneLevel::Medium
		);
	}
}
