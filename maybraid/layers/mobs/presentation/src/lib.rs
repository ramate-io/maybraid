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
use maybraid_mobs::{MobLodRefreshMode, MobScene, MobSceneSystems};
use mob_groups::MobGroupsPlugin;
use mob_layer_model::{MobCell, MobGenerationCore, MobIndex, MobLodChan};
use terrain_layer_model::{subscribe_mode, GenerationMode, RequireLayer};
use urbanization_layer_model::UrbanModel;

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
/// Bound on [`UrbanModel`] only because mob generation needs urbanization today.
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

impl<G: UrbanModel> Plugin for MobPresentationCore<G> {
	fn build(&self, app: &mut App) {
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
		present::install_mob_cell_teardown::<G>(app);
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

impl<Mode: GenerationMode, G: UrbanModel> Plugin for MobPresentationPlugin<Mode, G> {
	fn build(&self, app: &mut App) {
		subscribe_mode::<(G, MobPresent), Mode>(app);
		if !app.is_plugin_added::<MobPresentationCore<G>>() {
			app.add_plugins(MobPresentationCore::<G>::default());
		}
	}

	fn finish(&self, app: &mut App) {
		app.require_layer::<MobGenerationCore<G>, MobPresentationCore<G>>();
	}
}

#[cfg(test)]
mod tests;
