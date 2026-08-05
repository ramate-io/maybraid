//! Distance / extent probe for stick LOD hosts.

use bevy::prelude::{Component, Query, Res, Transform, With};
use bevy_math::Vec3;
use lod::gen::{LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;
use lod::lod_scene_host::LodSceneHost;

use crate::lod_band::{
	characteristic_extent_abs, placement_center, DistanceLodBand,
};
use crate::placed::Placement;

pub const STICK_HIGH_FACTOR: f32 = 5.0;
pub const STICK_MEDIUM_FACTOR: f32 = 20.0;
pub const STICK_LOW_FACTOR: f32 = 500.0;

#[derive(Debug, Clone, Copy, Component, Default)]
pub struct StickLodProbe {
	pub center: Vec3,
	pub extent: f32,
}

impl StickLodProbe {
	pub fn from_placement(placement: &Placement) -> Self {
		Self {
			center: placement_center(placement),
			extent: characteristic_extent_abs(placement),
		}
	}

	pub fn level_for(self, viewer: &Transform) -> LodSceneLevel {
		let distance = viewer.translation.distance(self.center);
		let factor = distance / self.extent.max(1e-4);
		DistanceLodBand::from_factors(factor, STICK_HIGH_FACTOR, STICK_MEDIUM_FACTOR, STICK_LOW_FACTOR)
			.to_lod_scene_level()
	}

	pub fn status_for_lod_ref(self, lod_ref: &LodRef) -> LodSceneStatus {
		let prev = DistanceLodBand::from_factors(
			lod_ref.previous_transform.translation.distance(self.center) / self.extent.max(1e-4),
			STICK_HIGH_FACTOR,
			STICK_MEDIUM_FACTOR,
			STICK_LOW_FACTOR,
		);
		let curr = DistanceLodBand::from_factors(
			lod_ref.current_transform.translation.distance(self.center) / self.extent.max(1e-4),
			STICK_HIGH_FACTOR,
			STICK_MEDIUM_FACTOR,
			STICK_LOW_FACTOR,
		);
		curr.status_vs(prev)
	}
}

/// Fine-phase: update stick host levels from viewer transform.
pub fn update_stick_host_levels(
	lod_state: Res<lod::LodViewerState>,
	mut hosts: Query<(&StickLodProbe, &mut LodSceneLevel), With<LodSceneHost>>,
) {
	let viewer = lod_state.current;
	for (probe, mut level) in &mut hosts {
		let next = probe.level_for(&viewer);
		if *level != next {
			*level = next;
		}
	}
}
