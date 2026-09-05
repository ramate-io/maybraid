//! Bevy plugins: vegetation view stack plus forest generate / present / cull.

use std::marker::PhantomData;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use chico_vegetation_components::VegetationProceduralPlugin;
use chico_vegetation_shaders::{
	init_chico_material_caches, ChicoMaterialRefPlugin, ChicoVegetationShadersPlugin,
};
use lod::{
	LodGenerateBudget, LodGeneratePlugin, LodGenerateRegionPlugin, LodGenerateSystems,
	LodPresentCullPlugin, LodPresentPlugin, LodPresentRegionPlugin, LodPresentSystems, LodViewer,
};
use scene_ref::SceneRefPlugin;

use crate::generation::{ForestGenerateBullseye, ForestLodChan, ForestPresentBullseye};
use crate::grove::ChicoGrove;
use crate::index::ForestIndex;
use crate::present::ForestPresenterState;
use crate::stream::{drive_forest_stream, ForestStream};
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

/// Generate, present, and cull [`ChicoGrove`]. `Pr` is [`ForestPresenter`]`<S>` for a world source `S`.
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
	for<'w, 's> Pr::Item<'w, 's>: lod::presentation::RegionPresenter<ChicoGrove, ForestIndex>,
{
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<VegetationViewPlugin>() {
			app.add_plugins(VegetationViewPlugin);
		}
		app.init_resource::<ForestIndex>()
			.init_resource::<ForestPresenterState>()
			.init_resource::<ForestStream>()
			.insert_resource(LodGenerateBudget { ids_per_frame: 16 })
			.add_plugins(LodGenerateRegionPlugin::<
				ForestGenerateBullseye,
				With<LodViewer>,
				ForestLodChan,
			>::default())
			.add_plugins(LodGeneratePlugin::<
				ChicoGrove,
				ForestIndex,
				ForestLodChan,
				With<LodViewer>,
			>::default())
			.add_plugins(LodPresentRegionPlugin::<
				ForestPresentBullseye,
				With<LodViewer>,
				ForestLodChan,
			>::default())
			.add_plugins(LodPresentPlugin::<
				ChicoGrove,
				ForestIndex,
				Pr,
				ForestLodChan,
				With<LodViewer>,
			>::default())
			.add_plugins(LodPresentCullPlugin::<ChicoGrove, ForestIndex, Pr, ForestLodChan>::default())
			.configure_sets(Update, LodPresentSystems::Produce.after(LodGenerateSystems::Drain))
			.add_systems(
				Update,
				drive_forest_stream
					.before(LodGenerateSystems::Produce)
					.before(LodPresentSystems::Produce),
			);
	}
}
