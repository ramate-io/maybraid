//! Bevy plugins: vegetation view stack plus forest generate / present / cull.

use bevy::prelude::*;
use chico_vegetation_components::VegetationProceduralPlugin;
use chico_vegetation_shaders::{
	init_chico_material_caches, ChicoMaterialRefPlugin, ChicoVegetationShadersPlugin,
};
use scene_ref::SceneRefPlugin;

use crate::view::VegetationLodRefreshPlugin;

/// Shaders, kit caches, and Avian LOD refresh for forest / grove hosts.
pub struct VegetationViewPlugin;

impl Plugin for VegetationViewPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<SceneRefPlugin>() {
			app.add_plugins(SceneRefPlugin);
		}
		if !app.is_plugin_added::<VegetationProceduralPlugin>() {
			app.add_plugins(VegetationProceduralPlugin);
		}
		if !app.is_plugin_added::<VegetationLodRefreshPlugin>() {
			app.add_plugins(VegetationLodRefreshPlugin);
		}
		if !app.is_plugin_added::<ChicoVegetationShadersPlugin>() {
			app.add_plugins(ChicoVegetationShadersPlugin);
		}
		if !app.is_plugin_added::<ChicoMaterialRefPlugin>() {
			app.add_plugins(ChicoMaterialRefPlugin);
		}
		init_chico_material_caches(app);
		if !app.is_plugin_added::<MaterialPlugin<StandardMaterial>>() {
			app.add_plugins(MaterialPlugin::<StandardMaterial>::default());
		}
	}
}

/// Register shaders, kit caches, and vegetation LOD refresh if missing.
pub fn register_vegetation_view(app: &mut App) {
	if !app.is_plugin_added::<VegetationViewPlugin>() {
		app.add_plugins(VegetationViewPlugin);
	}
}
