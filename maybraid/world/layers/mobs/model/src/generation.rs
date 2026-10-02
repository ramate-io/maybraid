//! [`MobGenerationPlugin`]: which mob groups exist where, over urbanized ground `G`.

use std::any::type_name;
use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use bevy::prelude::*;
use lod::gen::LodGenerateBudget;
use lod::{
	LodGeneratePlugin, LodGenerateRegionPlugin, LodGenerateSystems, LodPresentRegionPlugin,
	LodPresentSystems, LodViewer,
};
use layer_stack::{ActiveGenerationMode, GenerationMode, RequireLayer};
use urbanization_layer_model::UrbanModel;
use vegetation_layer_model::VegetationGenerationCore;

use crate::config::MobLayerConfig;
use crate::index::{MobCell, MobIndex};
use crate::stream::{
	stream_mob_present, sync_mob_models, sync_mob_plant_hosts, MobGenerateBullseye, MobLodChan,
	MobPresentBullseye,
};

/// Systems that arm mob keep regions and sync selection models.
///
/// Scheme writes order `.before` LOD generate / present produce through this set.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MobGenerationSystems;

/// A mode's mob writes for ground `G`.
pub trait MobScheme<G: UrbanModel>: GenerationMode {
	fn install(app: &mut App, config: &MobLayerConfig);
}

/// Shared install for mobs on `G`, added once.
pub struct MobGenerationCore<G> {
	budget: u32,
	_marker: PhantomData<fn() -> G>,
}

impl<G: UrbanModel> Plugin for MobGenerationCore<G> {
	fn build(&self, app: &mut App) {
		app.init_resource::<MobIndex>()
			.init_resource::<MobGenerateBullseye>()
			.init_resource::<MobPresentBullseye>()
			.insert_resource(InstalledMobBudget(self.budget))
			.insert_resource(LodGenerateBudget::<MobLodChan>::new(self.budget))
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
				stream_mob_present
					.in_set(MobGenerationSystems)
					.before(LodGenerateSystems::Produce)
					.before(LodPresentSystems::Produce),
			);
	}
}

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
struct InstalledMobBudget(u32);

/// Generation for ground `G` in `Mode`.
pub struct MobGenerationPlugin<Mode, G>
where
	Mode: MobScheme<G>,
	G: UrbanModel,
{
	pub config: MobLayerConfig,
	_marker: PhantomData<fn() -> (Mode, G)>,
}

impl<Mode, G> MobGenerationPlugin<Mode, G>
where
	Mode: MobScheme<G>,
	G: UrbanModel,
{
	pub fn new(config: MobLayerConfig) -> Self {
		Self { config, _marker: PhantomData }
	}
}

impl<Mode, G> Default for MobGenerationPlugin<Mode, G>
where
	Mode: MobScheme<G>,
	G: UrbanModel,
{
	fn default() -> Self {
		Self::new(MobLayerConfig::default())
	}
}

impl<Mode, G> Plugin for MobGenerationPlugin<Mode, G>
where
	Mode: MobScheme<G>,
	G: UrbanModel,
{
	fn build(&self, app: &mut App) {
		let Some(state) = app.world().get_resource::<State<ActiveGenerationMode>>() else {
			panic!(
				"MobGenerationPlugin<{}, {}> requires GenerationModePlugin first",
				Mode::name(),
				type_name::<G>()
			);
		};
		if state.get().is::<Mode>() && !app.is_plugin_added::<MobGenerationCore<G>>() {
			app.add_plugins(MobGenerationCore::<G> {
				budget: self.config.generate_budget,
				_marker: PhantomData,
			});
		}
		app.add_systems(OnExit(ActiveGenerationMode::of::<Mode>()), clear_mob_index);
		Mode::install(app, &self.config);
	}

	fn finish(&self, app: &mut App) {
		G::require_generation(app);
		app.require_layer::<VegetationGenerationCore, Self>();
		let Some(installed) = app.world().get_resource::<InstalledMobBudget>() else {
			panic!(
				"the initial generation mode never registered MobGenerationPlugin for {}",
				type_name::<G>()
			);
		};
		if installed.0 != self.config.generate_budget {
			panic!(
				"MobGenerationPlugin shared config disagrees for {}: {} vs {}",
				type_name::<G>(),
				installed.0,
				self.config.generate_budget
			);
		}
	}
}

fn clear_mob_index(mut index: ResMut<MobIndex>) {
	index.clear();
}
