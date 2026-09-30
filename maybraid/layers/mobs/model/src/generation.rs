//! [`MobGenerationPlugin`]: which mob groups exist where, over urbanized ground `G`.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use bevy::prelude::*;
use lod::gen::LodGenerateBudget;
use lod::{
	LodGeneratePlugin, LodGenerateRegionPlugin, LodGenerateSystems, LodPresentRegionPlugin,
	LodPresentSystems, LodViewer,
};
use terrain_layer_model::RequireLayer;
use urbanization_layer_model::UrbanModel;
use vegetation_layer_model::VegetationGenerationPlugin;

use crate::config::MobLayerConfig;
use crate::index::{MobCell, MobIndex};
use crate::stream::{
	stream_mobs, sync_mob_models, sync_mob_plant_hosts, MobGenerateBullseye, MobLodChan,
	MobPresentBullseye, MobStreamSuspended,
};

/// Systems that arm mob keep regions and sync selection models.
///
/// Training copies [`MobStreamSuspended`] `.before` this set.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MobGenerationSystems;

/// Mob cells, group kinds, and spawn anchors. Reads urbanization through
/// [`UrbanModel`] and forest layering from vegetation generation.
pub struct MobGenerationPlugin<G> {
	pub config: MobLayerConfig,
	_marker: PhantomData<fn() -> G>,
}

impl<G> MobGenerationPlugin<G> {
	pub fn new(config: MobLayerConfig) -> Self {
		Self { config, _marker: PhantomData }
	}
}

impl<G> Default for MobGenerationPlugin<G> {
	fn default() -> Self {
		Self::new(MobLayerConfig::default())
	}
}

impl<G: UrbanModel> Plugin for MobGenerationPlugin<G> {
	fn build(&self, app: &mut App) {
		app.insert_resource(self.config.clone())
			.init_resource::<MobIndex>()
			.init_resource::<MobGenerateBullseye>()
			.init_resource::<MobPresentBullseye>()
			.init_resource::<MobStreamSuspended>()
			.insert_resource(LodGenerateBudget::<MobLodChan>::new(self.config.generate_budget))
			.add_plugins(LodGenerateRegionPlugin::<
				MobGenerateBullseye,
				With<LodViewer>,
				MobLodChan,
			>::default())
			.add_plugins(LodGeneratePlugin::<
				MobCell,
				MobIndex,
				MobLodChan,
				With<LodViewer>,
			>::default())
			.add_plugins(LodPresentRegionPlugin::<
				MobPresentBullseye,
				With<LodViewer>,
				MobLodChan,
			>::default())
			.configure_sets(Update, LodPresentSystems::Produce.after(LodGenerateSystems::Drain))
			.add_systems(
				Update,
				(sync_mob_models::<G>, sync_mob_plant_hosts::<G>)
					.chain()
					.in_set(MobGenerationSystems)
					.after(LodGenerateSystems::Produce)
					.before(LodGenerateSystems::Drain),
			)
			.add_systems(
				Update,
				stream_mobs.in_set(MobGenerationSystems).before(LodGenerateSystems::Produce),
			);
	}

	fn finish(&self, app: &mut App) {
		G::require_generation(app);
		app.require_layer::<VegetationGenerationPlugin, Self>();
	}
}
