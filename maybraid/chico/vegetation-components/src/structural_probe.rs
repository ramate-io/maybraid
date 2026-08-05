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
#[derive(Debug, Clone, Copy, Component, Default)]
pub struct VegetationStructuralLodProbe {
	pub center: Vec3,
	/// Characteristic footprint / canopy radius used as the distance unit.
	pub tree_radius: f32,
	pub high_factor: f32,
	pub medium_factor: f32,
	pub low_factor: f32,
}

impl VegetationStructuralLodProbe {
	pub fn new(center: Vec3, tree_radius: f32) -> Self {
		Self {
			center,
			tree_radius: tree_radius.max(1e-4),
			high_factor: STRUCTURAL_HIGH_FACTOR,
			medium_factor: STRUCTURAL_MEDIUM_FACTOR,
			low_factor: STRUCTURAL_LOW_FACTOR,
		}
	}

	pub fn with_factors(mut self, high: f32, medium: f32, low: f32) -> Self {
		self.high_factor = high;
		self.medium_factor = medium;
		self.low_factor = low;
		self
	}

	fn factor_for(self, viewer: &Transform) -> f32 {
		viewer.translation.distance(self.center) / self.tree_radius.max(1e-4)
	}

	pub fn level_for(self, viewer: &Transform) -> LodSceneLevel {
		DistanceLodBand::from_factors(
			self.factor_for(viewer),
			self.high_factor,
			self.medium_factor,
			self.low_factor,
		)
		.to_lod_scene_level()
	}

	pub fn status_for_lod_ref(self, lod_ref: &LodRef) -> LodSceneStatus {
		let prev = DistanceLodBand::from_factors(
			lod_ref.previous_transform.translation.distance(self.center)
				/ self.tree_radius.max(1e-4),
			self.high_factor,
			self.medium_factor,
			self.low_factor,
		);
		let curr = DistanceLodBand::from_factors(
			lod_ref.current_transform.translation.distance(self.center)
				/ self.tree_radius.max(1e-4),
			self.high_factor,
			self.medium_factor,
			self.low_factor,
		);
		curr.status_vs(prev)
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
