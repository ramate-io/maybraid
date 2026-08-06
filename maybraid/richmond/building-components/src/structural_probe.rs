//! Building-level structural LOD probe (meters outside XZ footprint).
//!
//! Distinct from mesh-resolution probes ([`crate::panels::PanelLodProbe`], …):
//! this selects which *layers* of authored IR a composite building emits
//! (e.g. internal walls on High only).

use bevy::prelude::{Component, Query, Res, Transform, With};
use bevy_math::bounding::Aabb2d;
use bevy_math::{Vec2, Vec3};
use lod::gen::{LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;
use lod::lod_scene_host::LodSceneHost;

/// High while the viewer is at most this many meters outside the XZ perimeter.
pub const STRUCTURAL_HIGH_OUTSIDE_METERS: f32 = 20.0;

/// Viewer distance band for whole-building structural thinning.
///
/// Footprints are axis-aligned XZ rectangles (`Aabb2d` with \(y\) = world \(z\)).
/// Distance is the planar distance outside the nearest footprint (0 inside).
#[derive(Debug, Clone, PartialEq, Component)]
pub struct BuildingStructuralLodProbe {
	pub footprints: Vec<Aabb2d>,
	pub high_outside_meters: f32,
}

impl Default for BuildingStructuralLodProbe {
	fn default() -> Self {
		Self {
			footprints: Vec::new(),
			high_outside_meters: STRUCTURAL_HIGH_OUTSIDE_METERS,
		}
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
		Self::new([Aabb2d {
			min: Vec2::new(min.x, min.z),
			max: Vec2::new(max.x, max.z),
		}])
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

	/// Meters outside the nearest footprint in XZ (0 when inside any).
	pub fn distance_outside(&self, viewer: &Transform) -> f32 {
		distance_outside_footprints(viewer.translation, &self.footprints)
	}

	pub fn level_for(&self, viewer: &Transform) -> LodSceneLevel {
		if self.distance_outside(viewer) <= self.high_outside_meters {
			LodSceneLevel::High
		} else {
			LodSceneLevel::Medium
		}
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

/// Fine-phase: update structural building host levels from the LOD viewer.
pub fn update_building_structural_host_levels(
	lod_state: Res<lod::LodViewerState>,
	mut hosts: Query<(&BuildingStructuralLodProbe, &mut LodSceneLevel), With<LodSceneHost>>,
) {
	if lod_state.entity == bevy::prelude::Entity::PLACEHOLDER {
		return;
	}
	let viewer = lod_state.current;
	for (probe, mut level) in &mut hosts {
		let next = probe.level_for(&viewer);
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
}
