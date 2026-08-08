//! Distance / extent probe for foliage LOD hosts.

use bevy::prelude::{Component, Query, Transform, With};
use bevy_math::Vec3;
use lod::gen::{LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;
use lod::lod_scene_host::LodSceneHost;

use crate::foliage::collection::{
	FrondCollection, FROND_COLLECTION_HIGH_FACTOR, FROND_COLLECTION_LOW_FACTOR,
	FROND_COLLECTION_MEDIUM_FACTOR,
};
use crate::lod_band::{characteristic_extent_abs, DistanceLodBand};
use crate::placed::Placement;

pub const FOLIAGE_HIGH_FACTOR: f32 = 5.0;
pub const FOLIAGE_MEDIUM_FACTOR: f32 = 20.0;
pub const FOLIAGE_LOW_FACTOR: f32 = 500.0;

/// Fine-phase probe for foliage / frond-collection hosts (center + characteristic extent).
///
/// Ball / single-kit foliage uses [`FOLIAGE_*`](FOLIAGE_HIGH_FACTOR) factors and collapses
/// UltraLow onto Low. Frond collections use [`FROND_COLLECTION_*`](FROND_COLLECTION_HIGH_FACTOR)
/// (defined in [`crate::foliage::collection`]) with a real UltraLow tier.
#[derive(Debug, Clone, Copy, Component)]
pub struct FoliageLodProbe {
	pub center: Vec3,
	pub extent: f32,
	pub high_factor: f32,
	pub medium_factor: f32,
	pub low_factor: f32,
	/// When true, band UltraLow maps to [`LodSceneLevel::UltraLow`] (frond collections).
	pub preserve_ultra_low: bool,
}

impl Default for FoliageLodProbe {
	fn default() -> Self {
		Self {
			center: Vec3::ZERO,
			extent: 1.0,
			high_factor: FOLIAGE_HIGH_FACTOR,
			medium_factor: FOLIAGE_MEDIUM_FACTOR,
			low_factor: FOLIAGE_LOW_FACTOR,
			preserve_ultra_low: false,
		}
	}
}

impl FoliageLodProbe {
	pub fn from_placement(placement: &Placement) -> Self {
		Self {
			center: placement.translation,
			extent: characteristic_extent_abs(placement),
			..Self::default()
		}
	}

	/// Probe for a frond collection: max AABB extent + collection band factors.
	///
	/// Band constants: [`FROND_COLLECTION_HIGH_FACTOR`], [`FROND_COLLECTION_MEDIUM_FACTOR`],
	/// [`FROND_COLLECTION_LOW_FACTOR`] in [`crate::foliage::collection`].
	pub fn for_frond_collection(collection: &FrondCollection) -> Self {
		let (center, extent) = collection.center_and_extent();
		Self {
			center,
			extent,
			high_factor: FROND_COLLECTION_HIGH_FACTOR,
			medium_factor: FROND_COLLECTION_MEDIUM_FACTOR,
			low_factor: FROND_COLLECTION_LOW_FACTOR,
			preserve_ultra_low: true,
		}
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
		let factor = viewer.translation.distance(self.center) / self.extent.max(1e-4);
		self.band_to_level(DistanceLodBand::from_factors(
			factor,
			self.high_factor,
			self.medium_factor,
			self.low_factor,
		))
	}

	pub fn status_for_lod_ref(self, lod_ref: &LodRef) -> LodSceneStatus {
		let prev = self.band_to_level(DistanceLodBand::from_factors(
			lod_ref.previous_transform.translation.distance(self.center) / self.extent.max(1e-4),
			self.high_factor,
			self.medium_factor,
			self.low_factor,
		));
		let curr = self.band_to_level(DistanceLodBand::from_factors(
			lod_ref.current_transform.translation.distance(self.center) / self.extent.max(1e-4),
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

pub fn update_foliage_host_levels(
	viewer: Query<&lod::LodNodePose, With<lod::LodViewer>>,
	mut hosts: Query<(&FoliageLodProbe, &mut LodSceneLevel), With<LodSceneHost>>,
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
