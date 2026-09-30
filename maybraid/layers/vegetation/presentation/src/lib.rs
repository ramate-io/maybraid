//! [`VegetationPresentationPlugin`]: grow groves and bump-outs on model `G`.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use bevy::prelude::*;
use chico_bumpout::ChicoBumpOutPlugin;
use chico_forests::{
	register_vegetation_view, BumpOutLodChan, CanopyBumpOut, ChicoGrove, ForestIndex, ForestLodChan,
	MediumBumpOutLodChan, MediumCanopyBumpOut,
};
use durham_terrain_models::terrain_streaming_enabled;
use lod::{
	LodGenerateSystems, LodPresentCullPlugin, LodPresentPlugin, LodPresentSystems, LodViewer,
};
use terrain_layer_model::{RequireLayer, TerrainModel};
use vegetation_layer_model::{VegetationGenerationPlugin, VegetationGenerationSystems};

mod material;
mod present;

pub use material::{VegetationOnTerrainMaterialLib, VegetationOnTerrainMaterialRefPlugin};
pub use present::{
	bump_out_from_cell, bump_out_noise, retire_vegetation_presenters, GroundCanopyBumpOutPresenter,
	GroundForestPresenter, GroundGroveSample, GroundMediumCanopyBumpOutPresenter,
};

use present::{CanopyBumpOutPresenterState, MediumCanopyBumpOutPresenterState};

/// Presents generated vegetation on ground `G`. Pads reach groves through
/// `G`'s height (`Urbanization<…>` already composes them), not a special case.
pub struct VegetationPresentationPlugin<G>(PhantomData<fn() -> G>);

impl<G> Default for VegetationPresentationPlugin<G> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<G: TerrainModel> Plugin for VegetationPresentationPlugin<G>
where
	G::Snapshot: Clone + Send + Sync,
	G::Cell: terrain_layer_model::TerrainCell<Mesh = durham_terrain_models::TerrainMeshBuilder>,
{
	fn build(&self, app: &mut App) {
		register_vegetation_view(app);
		if !app.is_plugin_added::<VegetationOnTerrainMaterialRefPlugin>() {
			app.add_plugins(VegetationOnTerrainMaterialRefPlugin);
		}
		if !app.is_plugin_added::<ChicoBumpOutPlugin>() {
			app.add_plugins(ChicoBumpOutPlugin);
		}
		app.init_resource::<chico_forests::ForestPresenterState>()
			.init_resource::<CanopyBumpOutPresenterState>()
			.init_resource::<MediumCanopyBumpOutPresenterState>()
			.add_plugins(LodPresentPlugin::<
				ChicoGrove,
				ForestIndex,
				GroundForestPresenter<G>,
				ForestLodChan,
				With<LodViewer>,
			>::default())
			.add_plugins(LodPresentCullPlugin::<
				ChicoGrove,
				ForestIndex,
				GroundForestPresenter<G>,
				ForestLodChan,
			>::default())
			.add_plugins(LodPresentPlugin::<
				CanopyBumpOut,
				ForestIndex,
				GroundCanopyBumpOutPresenter<G>,
				BumpOutLodChan,
				With<LodViewer>,
			>::default())
			.add_plugins(LodPresentCullPlugin::<
				CanopyBumpOut,
				ForestIndex,
				GroundCanopyBumpOutPresenter<G>,
				BumpOutLodChan,
			>::default())
			.add_plugins(LodPresentPlugin::<
				MediumCanopyBumpOut,
				ForestIndex,
				GroundMediumCanopyBumpOutPresenter<G>,
				MediumBumpOutLodChan,
				With<LodViewer>,
			>::default())
			.add_plugins(LodPresentCullPlugin::<
				MediumCanopyBumpOut,
				ForestIndex,
				GroundMediumCanopyBumpOutPresenter<G>,
				MediumBumpOutLodChan,
			>::default())
			.configure_sets(Update, LodPresentSystems::Produce.after(LodGenerateSystems::Drain))
			.add_systems(
				Update,
				retire_vegetation_presenters
					.after(VegetationGenerationSystems)
					.before(LodPresentSystems::Produce)
					.run_if(terrain_streaming_enabled),
			);
	}

	fn finish(&self, app: &mut App) {
		G::require_generation(app);
		app.require_layer::<VegetationGenerationPlugin, Self>();
	}
}

#[cfg(test)]
mod tests;
