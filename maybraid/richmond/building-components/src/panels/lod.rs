//! Panel mesh-resolution LOD (distance / extent banding).
//!
//! Reuses roof High / Medium / Low distance factors (panels previously hosted under
//! [`crate::roofs::lod::RoofLodProbe`]). Unlike roofs and partitions, **UltraLow is a
//! distinct [`LodSceneLevel`]**: every style drops to the shared flat low-res kit.

use bevy::prelude::{Component, Query, Res, Transform, With};
use bevy::scene::prelude::Scene;
use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;
use lod::lod_scene_host::LodSceneHost;
use scene_ref::SceneRef;

use crate::assets::panels::flat;
use crate::assets::AssetPath;
use crate::empty_scene;
use crate::lod_band::{
	center_extent_from_aabb, characteristic_extent_abs, placement_center, warm_mesh_lod_culls,
	DistanceLodBand,
};
use crate::lod_host::warm_content_host_hslu;
use crate::placed::Placement;
use crate::roofs::lod::{ROOF_HIGH_FACTOR, ROOF_LOW_FACTOR, ROOF_MEDIUM_FACTOR};

/// Same thresholds as [`crate::roofs::lod`] (panels shared that probe historically).
pub const PANEL_HIGH_FACTOR: f32 = ROOF_HIGH_FACTOR;
pub const PANEL_MEDIUM_FACTOR: f32 = ROOF_MEDIUM_FACTOR;
pub const PANEL_LOW_FACTOR: f32 = ROOF_LOW_FACTOR;

/// Shared UltraLow rectangle kit for every [`crate::panels::PanelStyle`].
pub const PANEL_ULTRA_LOW_RECTANGLE: AssetPath = flat::RECTANGLE_LOW;
/// Shared UltraLow right-triangle kit for every [`crate::panels::PanelStyle`].
pub const PANEL_ULTRA_LOW_RIGHT_TRIANGLE: AssetPath = flat::RIGHT_TRIANGLE_LOW;

/// Viewer distance band for panel mesh resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelLodBand {
	UltraLow,
	Low,
	Medium,
	High,
}

impl PanelLodBand {
	pub fn from_distance_factor(factor: f32) -> Self {
		match DistanceLodBand::from_factors(
			factor,
			PANEL_HIGH_FACTOR,
			PANEL_MEDIUM_FACTOR,
			PANEL_LOW_FACTOR,
		) {
			DistanceLodBand::High => Self::High,
			DistanceLodBand::Medium => Self::Medium,
			DistanceLodBand::Low => Self::Low,
			DistanceLodBand::UltraLow => Self::UltraLow,
		}
	}

	/// UltraLow is a real host root (flat low-res), not collapsed onto Low.
	pub fn to_lod_scene_level(self) -> LodSceneLevel {
		match self {
			Self::High => LodSceneLevel::High,
			Self::Medium => LodSceneLevel::Medium,
			Self::Low => LodSceneLevel::Low,
			Self::UltraLow => LodSceneLevel::UltraLow,
		}
	}

	pub fn status_vs(self, prev: Self) -> LodSceneStatus {
		let prev_l = prev.to_lod_scene_level();
		let curr_l = self.to_lod_scene_level();
		if prev_l == curr_l {
			LodSceneStatus::Unchanged
		} else {
			LodSceneStatus::Changed(curr_l)
		}
	}
}

/// Fine-phase probe for panel mesh hosts (center + characteristic extent).
#[derive(Debug, Clone, Copy, Component, Default)]
pub struct PanelLodProbe {
	pub center: Vec3,
	pub extent: f32,
}

impl PanelLodProbe {
	pub fn from_placement(placement: &Placement) -> Self {
		Self { center: placement_center(placement), extent: characteristic_extent_abs(placement) }
	}

	pub fn from_aabb(aabb: &Aabb3d) -> Self {
		let (center, extent) = center_extent_from_aabb(aabb);
		Self { center, extent }
	}

	pub fn band_for(&self, viewer: &Transform) -> PanelLodBand {
		let factor = viewer.translation.distance(self.center) / self.extent.max(1e-4);
		PanelLodBand::from_distance_factor(factor)
	}

	pub fn level_for(&self, viewer: &Transform) -> LodSceneLevel {
		self.band_for(viewer).to_lod_scene_level()
	}

	pub fn status_for_lod_ref(&self, lod_ref: &LodRef) -> LodSceneStatus {
		self.band_for(lod_ref.current_transform)
			.status_vs(self.band_for(lod_ref.previous_transform))
	}

	pub fn culls_for_lod_ref(&self, lod_ref: &LodRef) -> LodSceneCulls {
		warm_mesh_lod_culls(self.level_for(lod_ref.current_transform))
	}
}

impl LodScene for PanelLodProbe {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.level_for(lod_ref.current_transform)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		self.status_for_lod_ref(lod_ref)
	}

	fn scene_with_level(
		&self,
		_lod_ref: &LodRef,
		_level: LodSceneLevel,
	) -> impl Scene + 'static {
		bevy::scene::SceneFunction(empty_scene)
	}
}

/// Warm High/Medium/Low/UltraLow panel host driven by an explicit probe.
///
/// Composite buildings should pass [`PanelLodProbe::from_placement`] for each kit so
/// panels band independently. Unit-kit previews may use [`PanelLodProbe::from_aabb`]
/// on the subject bounds.
pub fn leaf_panel_scene_ref_lod(
	high: SceneRef,
	mid: SceneRef,
	low: SceneRef,
	ultra_low: SceneRef,
	lod_ref: &LodRef,
	probe: PanelLodProbe,
) -> impl Scene + 'static {
	let level = probe.level_for(lod_ref.current_transform);
	warm_content_host_hslu(
		level,
		probe,
		high.scene(),
		mid.scene(),
		low.scene(),
		ultra_low.scene(),
	)
}

/// Fine-phase: update panel host levels from [`lod::LodViewerState`].
pub fn update_panel_host_levels(
	viewer: Res<lod::LodViewerState>,
	mut hosts: Query<(&PanelLodProbe, &mut LodSceneLevel), With<LodSceneHost>>,
) {
	if viewer.entity == bevy::prelude::Entity::PLACEHOLDER {
		return;
	}
	for (probe, mut level) in &mut hosts {
		let desired = probe.level_for(&viewer.current);
		if *level != desired {
			*level = desired;
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn ultra_low_is_distinct_from_low() -> anyhow::Result<()> {
		assert_eq!(
			PanelLodBand::from_distance_factor(PANEL_LOW_FACTOR).to_lod_scene_level(),
			LodSceneLevel::Low
		);
		assert_eq!(
			PanelLodBand::from_distance_factor(PANEL_LOW_FACTOR + 1.0).to_lod_scene_level(),
			LodSceneLevel::UltraLow
		);
		assert_eq!(PANEL_ULTRA_LOW_RECTANGLE, flat::RECTANGLE_LOW);
		assert_eq!(PANEL_ULTRA_LOW_RIGHT_TRIANGLE, flat::RIGHT_TRIANGLE_LOW);
		Ok(())
	}
}
