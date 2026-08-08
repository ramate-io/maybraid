//! Tree-level structural LOD probe (distance / tree-radius bands).

use bevy::prelude::{Component, Query, Res, Transform, With};
use bevy_math::Vec3;
use lod::gen::{LodSceneLevel, LodSceneStatus};
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
}

/// Fine-phase: update structural vegetation host levels from the LOD viewer.
pub fn update_vegetation_structural_host_levels(
	lod_state: Res<lod::LodViewerState>,
	mut hosts: Query<(&VegetationStructuralLodProbe, &mut LodSceneLevel), With<LodSceneHost>>,
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
