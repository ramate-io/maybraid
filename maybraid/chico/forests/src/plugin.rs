//! Bevy plugins: vegetation view stack plus forest generate / present / cull.

use std::marker::PhantomData;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use chico_vegetation_components::VegetationProceduralPlugin;
use chico_vegetation_shaders::{
	init_chico_material_caches, ChicoMaterialRefPlugin, ChicoVegetationShadersPlugin,
};
use lod::presentation::RegionPresenter;
use scene_ref::SceneRefPlugin;

use crate::stream::register_forest_lod;
use crate::view::VegetationLodRefreshPlugin;
use crate::{ChicoGrove, ForestIndex};

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

/// Generate + present [`ChicoGrove`]. Does not drive keep regions; the host
/// still calls [`crate::ForestStreamLod::apply_spec`].
pub struct ForestPlugin<Pr> {
	_marker: PhantomData<fn() -> Pr>,
}

impl<Pr> Default for ForestPlugin<Pr> {
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<Pr> Plugin for ForestPlugin<Pr>
where
	Pr: SystemParam + 'static,
	for<'w, 's> Pr::Item<'w, 's>: RegionPresenter<ChicoGrove, ForestIndex>,
{
	fn build(&self, app: &mut App) {
		register_vegetation_view(app);
		register_forest_lod::<Pr>(app);
	}
}
