//! Tree-level structural LOD probe (distance / tree-radius bands).

use bevy::prelude::{Component, Query, Transform, Visibility, With};
use bevy::scene::prelude::{bsn, Scene};
use bevy_math::Vec3;
use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;
use lod::lod_scene_host::LodSceneHost;

use crate::lod_band::DistanceLodBand;

/// High when `distance / tree_radius ≤` this (Sope default: 3).
pub const STRUCTURAL_HIGH_FACTOR: f32 = 3.0;
/// Medium when at or below this multiple of tree radius (Sope default: 12).
pub const STRUCTURAL_MEDIUM_FACTOR: f32 = 12.0;
/// Low when at or below this multiple of tree radius (Sope default: 24).
pub const STRUCTURAL_LOW_FACTOR: f32 = 24.0;

/// Viewer distance band for whole-tree structural thinning.
///
/// Stored on structural hosts so [`crate::ComponentsOnly`] can band levels / culls.
/// Message-based refresh is registered on the parent [`crate::LodScene`] type, not this probe.
#[derive(Debug, Clone, Copy, Component)]
pub struct VegetationStructuralLodProbe {
	pub center: Vec3,
	/// Characteristic footprint / canopy radius used as the distance unit.
	pub tree_radius: f32,
	pub high_factor: f32,
	pub medium_factor: f32,
	pub low_factor: f32,
	/// When true, band UltraLow maps to [`LodSceneLevel::UltraLow`].
	pub preserve_ultra_low: bool,
}

impl Default for VegetationStructuralLodProbe {
	fn default() -> Self {
		Self {
			center: Vec3::ZERO,
			tree_radius: 1.0,
			high_factor: STRUCTURAL_HIGH_FACTOR,
			medium_factor: STRUCTURAL_MEDIUM_FACTOR,
			low_factor: STRUCTURAL_LOW_FACTOR,
			preserve_ultra_low: false,
		}
	}
}

impl VegetationStructuralLodProbe {
	pub fn new(center: Vec3, tree_radius: f32) -> Self {
		Self {
			center,
			tree_radius: tree_radius.max(1e-4),
			high_factor: STRUCTURAL_HIGH_FACTOR,
			medium_factor: STRUCTURAL_MEDIUM_FACTOR,
			low_factor: STRUCTURAL_LOW_FACTOR,
			preserve_ultra_low: false,
		}
	}

	pub fn with_factors(mut self, high: f32, medium: f32, low: f32) -> Self {
		self.high_factor = high;
		self.medium_factor = medium;
		self.low_factor = low;
		self
	}

	pub fn with_preserve_ultra_low(mut self, preserve: bool) -> Self {
		self.preserve_ultra_low = preserve;
		self
	}

	fn factor_for(self, viewer: &Transform) -> f32 {
		viewer.translation.distance(self.center) / self.tree_radius.max(1e-4)
	}

	fn band_to_level(self, band: DistanceLodBand) -> LodSceneLevel {
		match band {
			DistanceLodBand::High => LodSceneLevel::High,
			DistanceLodBand::Medium => LodSceneLevel::Medium,
			DistanceLodBand::Low => LodSceneLevel::Low,
			DistanceLodBand::UltraLow if self.preserve_ultra_low => LodSceneLevel::UltraLow,
			DistanceLodBand::UltraLow => LodSceneLevel::Low,
		}
	}

	pub fn level_for(self, viewer: &Transform) -> LodSceneLevel {
		self.band_to_level(DistanceLodBand::from_factors(
			self.factor_for(viewer),
			self.high_factor,
			self.medium_factor,
			self.low_factor,
		))
	}

	pub fn status_for_lod_ref(self, lod_ref: &LodRef) -> LodSceneStatus {
		let prev = self.band_to_level(DistanceLodBand::from_factors(
			lod_ref.previous_transform.translation.distance(self.center)
				/ self.tree_radius.max(1e-4),
			self.high_factor,
			self.medium_factor,
			self.low_factor,
		));
		let curr = self.band_to_level(DistanceLodBand::from_factors(
			lod_ref.current_transform.translation.distance(self.center)
				/ self.tree_radius.max(1e-4),
			self.high_factor,
			self.medium_factor,
			self.low_factor,
		));
		if prev == curr {
			LodSceneStatus::Unchanged
		} else {
			LodSceneStatus::Changed(curr)
		}
	}

	/// Local AABB covering the structural footprint (for colliders / indexing).
	pub fn footprint_aabb(self) -> bevy_math::bounding::Aabb3d {
		let r = self.tree_radius.max(1.0);
		let half = Vec3::new(r, r.max(2.0), r);
		bevy_math::bounding::Aabb3d::from_min_max(self.center - half, self.center + half)
	}
}

impl LodScene for VegetationStructuralLodProbe {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.level_for(lod_ref.current_transform)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		self.status_for_lod_ref(lod_ref)
	}

	fn scene_lod_culls(&self, _lod_ref: &LodRef, _current: LodSceneLevel) -> LodSceneCulls {
		// Warm H/M/L(/UL) roots — keep them; respawn is expensive.
		LodSceneCulls::None
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, _level: LodSceneLevel) -> impl Scene + 'static {
		// Structural hosts are presented warm; this is only a fulfill fallback.
		bsn! {
			Transform::default()
			Visibility::Inherited
		}
	}

	fn scene_bounds(&self) -> bevy_math::bounding::Aabb3d {
		self.footprint_aabb()
	}
}

/// Legacy every-frame structural level writer (prefer region → level messages).
pub fn update_vegetation_structural_host_levels(
	viewer: Query<&lod::LodNodePose, With<lod::LodViewer>>,
	mut hosts: Query<(&VegetationStructuralLodProbe, &mut LodSceneLevel), With<LodSceneHost>>,
) {
	let Ok(pose) = viewer.single() else {
		return;
	};
	let viewer = pose.current;
	for (probe, mut level) in &mut hosts {
		let next = probe.level_for(&viewer);
		if *level != next {
			*level = next;
		}
	}
}
