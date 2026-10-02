//! [`VegetationPresentationPlugin`]: grow groves and bump-outs on model `G`.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use bevy::prelude::*;
use vegetation_bumpout::BumpOutPlugin;
use chico::{
	register_vegetation_view, BumpOutLodChan, CanopyBumpOut, ChicoGrove, ForestIndex, ForestLodChan,
	MediumBumpOutLodChan, MediumCanopyBumpOut,
};
use durham::terrain_streaming_enabled;
use lod::{
	LodGenerateSystems, LodPresentCullPlugin, LodPresentPlugin, LodPresentSystems, LodViewer,
};
use terrain_layer_model::TerrainModel;
use layer_stack::{subscribe_mode, GenerationMode, RequireLayer};
use vegetation_layer_model::{VegetationGenerationCore, VegetationGenerationSystems};

mod material;
mod present;

pub use material::{VegetationOnTerrainMaterialLib, VegetationOnTerrainMaterialRefPlugin};
pub use present::{
	bump_out_from_cell, bump_out_noise, retire_vegetation_presenters, GroundCanopyBumpOutPresenter,
	GroundForestPresenter, GroundGroveSample, GroundMediumCanopyBumpOutPresenter,
};

use present::{CanopyBumpOutPresenterState, MediumCanopyBumpOutPresenterState};

/// Marker for vegetation-presenter subscriptions on ground `G`.
pub struct VegetationPresent;

/// Presents generated vegetation on ground `G` while `Mode` is subscribed.
/// Pads reach groves through `G`'s height (`Urbanization<…>` already composes
/// them), not a special case.
pub struct VegetationPresentationPlugin<Mode, G>(PhantomData<fn() -> (Mode, G)>);

impl<Mode, G> Default for VegetationPresentationPlugin<Mode, G> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

pub struct VegetationPresentationCore<G>(PhantomData<fn() -> G>);

impl<G> Default for VegetationPresentationCore<G> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<G: TerrainModel> Plugin for VegetationPresentationCore<G>
where
	G::Snapshot: Clone + Send + Sync,
	G::Cell: terrain_layer_model::TerrainCell<Mesh = durham::TerrainMeshBuilder>,
{
	fn build(&self, app: &mut App) {
		register_vegetation_view(app);
		if !app.is_plugin_added::<VegetationOnTerrainMaterialRefPlugin>() {
			app.add_plugins(VegetationOnTerrainMaterialRefPlugin);
		}
		if !app.is_plugin_added::<BumpOutPlugin>() {
			app.add_plugins(BumpOutPlugin);
		}
		app.init_resource::<chico::ForestPresenterState>()
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
				retire_vegetation_presenters::<G>
					.after(VegetationGenerationSystems)
					.before(LodPresentSystems::Produce)
					.run_if(terrain_streaming_enabled),
			);
	}
}

impl<Mode: GenerationMode, G: TerrainModel> Plugin for VegetationPresentationPlugin<Mode, G>
where
	G::Snapshot: Clone + Send + Sync,
	G::Cell: terrain_layer_model::TerrainCell<Mesh = durham::TerrainMeshBuilder>,
{
	fn build(&self, app: &mut App) {
		subscribe_mode::<(G, VegetationPresent), Mode>(app);
		if !app.is_plugin_added::<VegetationPresentationCore<G>>() {
			app.add_plugins(VegetationPresentationCore::<G>::default());
		}
	}

	fn finish(&self, app: &mut App) {
		G::require_generation(app);
		app.require_layer::<VegetationGenerationCore, VegetationPresentationCore<G>>();
	}
}

#[cfg(test)]
mod tests;
