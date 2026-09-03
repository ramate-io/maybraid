//! Idempotent plugin for Richmond development models.

use bevy::prelude::*;
use lod::LodRefreshSystems;
use lod_lazy_refs::LodLazyRefsPlugin;
use richmond_building_components::{
	apply_parent_confines, FurnitureWireframePlugin, LabelWireframePlugin,
};
use richmond_building_shaders::{RichmondBuildingShadersPlugin, RichmondUrbanMaterialRefPlugin};
use scene_ref::SceneRefPlugin;

use crate::buildings_lod::register_developments_buildings_lod_plugin;
use crate::config::DevelopmentConfig;
use crate::index::DevelopmentEntryStore;
use crate::presentation::PaddedTerrainPresenterState;

/// Registers SceneRef, urban MaterialRef, placeholder wireframes, and Les Halles LOD.
#[derive(Default)]
pub struct RichmondDevelopmentModelsPlugin;

/// Idempotent registration of [`RichmondDevelopmentModelsPlugin`].
pub fn register_richmond_development_models_plugin(app: &mut App) {
	if app.is_plugin_added::<RichmondDevelopmentModelsPlugin>() {
		return;
	}
	app.add_plugins(RichmondDevelopmentModelsPlugin);
}

impl Plugin for RichmondDevelopmentModelsPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<SceneRefPlugin>() {
			app.add_plugins(SceneRefPlugin);
		}
		if !app.is_plugin_added::<LodLazyRefsPlugin>() {
			app.add_plugins(LodLazyRefsPlugin);
		}
		if !app.is_plugin_added::<RichmondBuildingShadersPlugin>() {
			app.add_plugins(RichmondBuildingShadersPlugin);
		}
		if !app.is_plugin_added::<RichmondUrbanMaterialRefPlugin>() {
			app.add_plugins(RichmondUrbanMaterialRefPlugin);
		}
		if !app.is_plugin_added::<FurnitureWireframePlugin>() {
			app.add_plugins(FurnitureWireframePlugin);
		}
		if !app.is_plugin_added::<LabelWireframePlugin>() {
			app.add_plugins(LabelWireframePlugin);
		}
		register_developments_buildings_lod_plugin(app);

		app.init_resource::<DevelopmentEntryStore>()
			.init_resource::<DevelopmentConfig>()
			.init_resource::<PaddedTerrainPresenterState>()
			.add_systems(Update, apply_parent_confines.after(LodRefreshSystems::Cull));
	}
}
