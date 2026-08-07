//! Rough-stonework unit rectangle panel kit (LOD triad + flat UltraLow).

use bevy::scene::prelude::Scene;
use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;

use crate::assets::panels::rough_stonework::{RECTANGLE_HIGH, RECTANGLE_LOW, RECTANGLE_MID};
use crate::panels::lod::{
	leaf_panel_scene_ref_lod, PanelLodProbe, PANEL_ULTRA_LOW_RECTANGLE,
};

/// Unit rectangle \(X, Z \in [0, 1]\), \(Y \in [-0.2, 0.2]\).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStonePanelRectangle;

impl RoughStonePanelRectangle {
	/// LOD host for panel leaves (style triad + flat UltraLow).
	///
	/// Unit-kit preview: probe the subject AABB.
	pub fn scene_with_lod(lod_ref: &LodRef) -> impl Scene + 'static {
		leaf_panel_scene_ref_lod(
			RECTANGLE_HIGH.scene_ref(),
			RECTANGLE_MID.scene_ref(),
			RECTANGLE_LOW.scene_ref(),
			PANEL_ULTRA_LOW_RECTANGLE.scene_ref(),
			lod_ref,
			PanelLodProbe::from_aabb(lod_ref.bounds),
		)
	}
}

impl LodScene for RoughStonePanelRectangle {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		PanelLodProbe::from_aabb(lod_ref.bounds).level_for(lod_ref.current_transform)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		PanelLodProbe::from_aabb(lod_ref.bounds).status_for_lod_ref(lod_ref)
	}

	fn scene_lod_culls(&self, _lod_ref: &LodRef, current: LodSceneLevel) -> LodSceneCulls {
		crate::lod_band::warm_mesh_lod_culls(current)
	}

	fn scene_with_level(
		&self,
		lod_ref: &LodRef,
		_level: LodSceneLevel,
	) -> impl Scene + 'static {
		RoughStonePanelRectangle::scene_with_lod(lod_ref)
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		RoughStonePanelRectangle::scene_with_lod(lod_ref)
	}
}
