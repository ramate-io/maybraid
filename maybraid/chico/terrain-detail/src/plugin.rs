//! Generate / present / cull glue. Playgrounds share the plugins; only the
//! grow sample and stream driver differ.

use std::marker::PhantomData;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lod::presentation::RegionPresenter;
use lod::{
	LodGeneratePlugin, LodGenerateRegionPlugin, LodGenerateSystems, LodPresentCullPlugin,
	LodPresentPlugin, LodPresentRegionPlugin, LodPresentSystems, LodViewer,
};
use material_ref::StandardMaterialRefPlugin;
use scene_ref::SceneRefPlugin;

use crate::{
	TerrainDetailGenerateBullseye, TerrainDetailIndex, TerrainDetailLodChan,
	TerrainDetailPresentBullseye, TerrainDetailPresenterState, TerrainOutcropping,
};

/// Independent generate / present / cull plugins. `Pr` grows with the host sample.
pub fn register_terrain_detail_lod<Pr>(app: &mut App)
where
	Pr: SystemParam + 'static,
	for<'w, 's> Pr::Item<'w, 's>: RegionPresenter<TerrainOutcropping, TerrainDetailIndex>,
{
	if !app.is_plugin_added::<SceneRefPlugin>() {
		app.add_plugins(SceneRefPlugin);
	}
	if !app.is_plugin_added::<StandardMaterialRefPlugin>() {
		app.add_plugins(StandardMaterialRefPlugin);
	}
	app.init_resource::<TerrainDetailIndex>()
		.init_resource::<TerrainDetailPresenterState>()
		.init_resource::<TerrainDetailGenerateBullseye>()
		.init_resource::<TerrainDetailPresentBullseye>()
		.add_plugins(LodGenerateRegionPlugin::<
			TerrainDetailGenerateBullseye,
			With<LodViewer>,
			TerrainDetailLodChan,
		>::default())
		.add_plugins(LodGeneratePlugin::<
			TerrainOutcropping,
			TerrainDetailIndex,
			TerrainDetailLodChan,
			With<LodViewer>,
		>::default())
		.add_plugins(LodPresentRegionPlugin::<
			TerrainDetailPresentBullseye,
			With<LodViewer>,
			TerrainDetailLodChan,
		>::default())
		.add_plugins(LodPresentPlugin::<
			TerrainOutcropping,
			TerrainDetailIndex,
			Pr,
			TerrainDetailLodChan,
			With<LodViewer>,
		>::default())
		.add_plugins(LodPresentCullPlugin::<
			TerrainOutcropping,
			TerrainDetailIndex,
			Pr,
			TerrainDetailLodChan,
		>::default())
		.configure_sets(Update, LodPresentSystems::Produce.after(LodGenerateSystems::Drain));
}

/// Scene / material fulfill plus optional presenter type.
pub struct TerrainDetailPlugin<Pr> {
	_marker: PhantomData<fn() -> Pr>,
}

impl<Pr> Default for TerrainDetailPlugin<Pr> {
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<Pr> Plugin for TerrainDetailPlugin<Pr>
where
	Pr: SystemParam + 'static,
	for<'w, 's> Pr::Item<'w, 's>: RegionPresenter<TerrainOutcropping, TerrainDetailIndex>,
{
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<MaterialPlugin<StandardMaterial>>() {
			app.add_plugins(MaterialPlugin::<StandardMaterial>::default());
		}
		register_terrain_detail_lod::<Pr>(app);
	}
}
