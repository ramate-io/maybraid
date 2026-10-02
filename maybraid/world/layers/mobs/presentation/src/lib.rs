//! [`MobPresentationPlugin`]: mob hosts grounded on `G`, surface fit, High LOD.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use lod::{
	update_lod_host_levels, LodGenerateSystems, LodPresentCullPlugin, LodPresentPlugin,
	LodPresentSystems, LodRefreshSystems, LodSceneRefreshRegionPlugin, LodViewer,
};
use lod_gimme::GimmeLodSceneRefreshPlugin;
use mob_scenes::{MobLodRefreshMode, MobScene, MobSceneSystems};
use barking::MobGroupsPlugin;
use mob_layer_model::{MobCell, MobGenerationCore, MobIndex, MobLodChan};
use layer_stack::{install_lod_present_gate, subscribe_mode, GenerationMode, RequireLayer};
use terrain_layer_model::TerrainModel;
use vegetation_layer_model::Vegetation;

mod present;

use present::{
	fit_mob_hosts_to_surface, pulse_mob_high_lod, MobHighLodChan, MobHighLodRegion, MobPresenter,
	MobPresenterState,
};

pub use present::PresentedMobCell;

/// Marker for mob-presenter subscriptions on ground `G`.
pub struct MobPresent;

/// Presents generated mob groups on ground `G` while `Mode` is subscribed.
///
/// Presentation itself reads nothing but `G`'s height.
pub struct MobPresentationPlugin<Mode, G>(PhantomData<fn() -> (Mode, G)>);

impl<Mode, G> Default for MobPresentationPlugin<Mode, G> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

pub struct MobPresentationCore<G>(PhantomData<fn() -> G>);

impl<G> Default for MobPresentationCore<G> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<G: TerrainModel> Plugin for MobPresentationCore<G> {
	fn build(&self, app: &mut App) {
		// Indexed must be visible when MobScenesPlugin builds. Guarding
		// MobGroupsPlugin would hide an assembler that already added groups
		// under FullScan.
		app.insert_resource(MobLodRefreshMode::Indexed);
		app.add_plugins(MobGroupsPlugin);
		app.init_resource::<MobPresenterState>()
			.add_plugins(LodPresentPlugin::<
				MobCell,
				MobIndex,
				MobPresenter<'_, '_, G>,
				MobLodChan,
				With<LodViewer>,
			>::default())
			.add_plugins(LodPresentCullPlugin::<
				MobCell,
				MobIndex,
				MobPresenter<'_, '_, G>,
				MobLodChan,
			>::default())
			.add_plugins(LodSceneRefreshRegionPlugin::<
				MobHighLodRegion,
				With<LodViewer>,
				MobHighLodChan,
			>::default())
			.add_plugins(GimmeLodSceneRefreshPlugin::<
				MobScene,
				MobHighLodChan,
				With<LodViewer>,
			>::default())
			.configure_sets(Update, LodPresentSystems::Produce.after(LodGenerateSystems::Drain));
		present::install_mob_cell_teardown(app);
		app.add_systems(
			Update,
			fit_mob_hosts_to_surface::<G>.in_set(MobSceneSystems::Surface),
		)
		.add_systems(
			Update,
			pulse_mob_high_lod
				.run_if(on_timer(present::MOB_HIGH_LOD_REFRESH_INTERVAL))
				.in_set(LodRefreshSystems::ProduceRegions),
		)
		.add_systems(
			Update,
			update_lod_host_levels::<MobScene, (), With<LodViewer>>
				.run_if(on_timer(present::MOB_HIGH_LOD_RECONCILE_INTERVAL))
				.in_set(LodRefreshSystems::UpdateLevels),
		);
	}
}

impl<Mode: GenerationMode, V> Plugin for MobPresentationPlugin<Mode, Vegetation<V>>
where
	Vegetation<V>: TerrainModel,
	MobGenerationCore<Vegetation<V>>: Plugin,
{
	fn build(&self, app: &mut App) {
		subscribe_mode::<(Vegetation<V>, MobPresent), Mode>(app);
		install_lod_present_gate::<(Vegetation<V>, MobPresent), MobLodChan>(app);
		if !app.is_plugin_added::<MobPresentationCore<Vegetation<V>>>() {
			app.add_plugins(MobPresentationCore::<Vegetation<V>>::default());
		}
	}

	fn finish(&self, app: &mut App) {
		app.require_layer::<MobGenerationCore<Vegetation<V>>, MobPresentationCore<Vegetation<V>>>();
	}
}

#[cfg(test)]
mod tests;
