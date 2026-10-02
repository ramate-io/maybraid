//! [`MobGenerationPlugin`]: which mob groups exist where, over urbanized ground `G`.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use bevy::prelude::*;
use lod::gen::LodGenerateBudget;
use lod::{
	LodGeneratePlugin, LodGenerateRegionPlugin, LodGenerateSystems, LodPresentRegionPlugin,
	LodPresentSystems, LodViewer,
};
use layer_stack::{ActiveGenerationMode, GenerationMode, RequireLayer};
use procedural_common::NoiseParams;
use urbanization_cells::UrbanizationKind;
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
pub trait MobScheme<G: UrbanModel<Selection = NoiseParams, Kind = UrbanizationKind>>: GenerationMode {
	fn install(app: &mut App, config: &MobLayerConfig);
}

/// Shared install for mobs on `G`, added once.
pub struct MobGenerationCore<G>(PhantomData<fn() -> G>);

impl<G> Default for MobGenerationCore<G> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<G: UrbanModel<Selection = NoiseParams, Kind = UrbanizationKind>> Plugin for MobGenerationCore<G> {
	fn build(&self, app: &mut App) {
		app.init_resource::<MobIndex>()
			.init_resource::<MobGenerateBullseye>()
			.init_resource::<MobPresentBullseye>()
			.init_resource::<LodGenerateBudget<MobLodChan>>()
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

/// Per-mode mob budget applied on enter.
#[derive(Resource, Clone)]
pub struct MobModeConfig<Mode: GenerationMode> {
	pub config: MobLayerConfig,
	_mode: PhantomData<fn() -> Mode>,
}

impl<Mode: GenerationMode> MobModeConfig<Mode> {
	pub fn new(config: MobLayerConfig) -> Self {
		Self { config, _mode: PhantomData }
	}
}

fn apply_mob_mode<Mode: GenerationMode>(
	mode: Res<MobModeConfig<Mode>>,
	mut budget: ResMut<LodGenerateBudget<MobLodChan>>,
) {
	*budget = LodGenerateBudget::new(mode.config.generate_budget);
}

/// Generation for ground `G` in `Mode`.
pub struct MobGenerationPlugin<Mode, G> {
	pub config: MobLayerConfig,
	_marker: PhantomData<fn() -> (Mode, G)>,
}

impl<Mode, G> MobGenerationPlugin<Mode, G> {
	pub fn new(config: MobLayerConfig) -> Self {
		Self { config, _marker: PhantomData }
	}
}

impl<Mode, G> Default for MobGenerationPlugin<Mode, G> {
	fn default() -> Self {
		Self::new(MobLayerConfig::default())
	}
}

impl<Mode, G> Plugin for MobGenerationPlugin<Mode, G>
where
	Mode: MobScheme<G>,
	G: UrbanModel<Selection = NoiseParams, Kind = UrbanizationKind>,
{
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<MobGenerationCore<G>>() {
			app.add_plugins(MobGenerationCore::<G>::default());
		}
		app.insert_resource(MobModeConfig::<Mode>::new(self.config.clone()));
		app.add_systems(
			OnEnter(ActiveGenerationMode::of::<Mode>()),
			apply_mob_mode::<Mode>,
		);
		app.add_systems(OnExit(ActiveGenerationMode::of::<Mode>()), clear_mob_index);
		Mode::install(app, &self.config);
	}

	fn finish(&self, app: &mut App) {
		G::require_generation(app);
		app.require_layer::<VegetationGenerationCore, Self>();
	}
}

fn clear_mob_index(mut index: ResMut<MobIndex>) {
	index.clear();
}
