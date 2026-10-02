//! [`UrbanizationGenerationPlugin`]: urbanization cells, pads, and padded cells over `M`.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::LodGenerateBudget;
use lod::LodPresentSystems;
use richmond::{register_richmond_plugin, DevelopmentConfig, DevelopmentEntryStore};
use terrain_layer_model::{
	install_terrain_contract_forward, terrain_streaming, TerrainContractForward,
	TerrainLayerSystems, TerrainModel,
};
use layer_stack::{ActiveGenerationMode, GenerationMode};
use urbanization_cells::UrbanizationLodChan;

use crate::config::UrbanizationLayerConfig;
use crate::model::Urbanization;
use crate::stream::{
	clear_urbanization_stream, generate_urbanization_padded_terrain, UrbanizationStreamKey,
	UrbanizationStreamLod,
};

/// Systems that write urbanization / development / padded-cell storage.
///
/// Presentation orders `.after(UrbanizationGenerationSystems)` so the split
/// chain matches today's single `.chain()`.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UrbanizationGenerationSystems;

/// Scheme store writes finish here. Shared padding runs after this set.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UrbanizationStoreSystems;

/// A mode's urbanization writes for model `M`.
pub trait UrbanizationScheme<M: TerrainModel>: GenerationMode {
	fn install(app: &mut App, config: &UrbanizationLayerConfig);
}

/// Shared install for `Urbanization<M>`, added once.
pub struct UrbanizationGenerationCore<M>(PhantomData<fn() -> M>);

impl<M> Default for UrbanizationGenerationCore<M> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<M> Plugin for UrbanizationGenerationCore<M>
where
	M: TerrainModel + Send + Sync + 'static,
	Urbanization<M>: TerrainModel,
{
	fn build(&self, app: &mut App) {
		register_richmond_plugin(app);
		app.init_resource::<DevelopmentConfig>()
			.init_resource::<LodGenerateBudget<UrbanizationLodChan>>()
			.init_resource::<UrbanizationLayerRegion>()
			.init_resource::<UrbanizationStreamKey>();
		install_terrain_contract_forward::<M, Urbanization<M>>(
			app,
			TerrainContractForward::Outer,
		);
		app.configure_sets(
				Update,
				(
					UrbanizationGenerationSystems.after(TerrainContractForward::Inner),
					UrbanizationStoreSystems
						.in_set(UrbanizationGenerationSystems)
						.run_if(terrain_streaming::<M>),
				),
			)
			.add_systems(
				Update,
				generate_urbanization_padded_terrain::<M>
					.in_set(UrbanizationGenerationSystems)
					.after(UrbanizationStoreSystems)
					.run_if(terrain_streaming::<M>)
					.before(LodPresentSystems::Produce)
					.before(TerrainLayerSystems::<M>::QueueColliders),
			);
	}
}

/// Per-mode config the scheme systems read.
#[derive(Resource, Clone)]
pub struct UrbanizationModeConfig<Mode: GenerationMode> {
	pub config: UrbanizationLayerConfig,
	_mode: PhantomData<fn() -> Mode>,
}

impl<Mode: GenerationMode> UrbanizationModeConfig<Mode> {
	pub fn new(config: UrbanizationLayerConfig) -> Self {
		Self { config, _mode: PhantomData }
	}
}

/// Region schemes write and padding / hosts read.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct UrbanizationLayerRegion {
	pub region: Option<Aabb3d>,
}

/// Generation for model `M` in `Mode`.
pub struct UrbanizationGenerationPlugin<Mode, M>
where
	Mode: UrbanizationScheme<M>,
	M: TerrainModel,
{
	pub config: UrbanizationLayerConfig,
	_marker: PhantomData<fn() -> (Mode, M)>,
}

impl<Mode, M> UrbanizationGenerationPlugin<Mode, M>
where
	Mode: UrbanizationScheme<M>,
	M: TerrainModel,
{
	pub fn new(config: UrbanizationLayerConfig) -> Self {
		Self { config, _marker: PhantomData }
	}
}

impl<Mode, M> Default for UrbanizationGenerationPlugin<Mode, M>
where
	Mode: UrbanizationScheme<M>,
	M: TerrainModel,
{
	fn default() -> Self {
		Self::new(UrbanizationLayerConfig::default())
	}
}

impl<Mode, M> Plugin for UrbanizationGenerationPlugin<Mode, M>
where
	Mode: UrbanizationScheme<M>,
	M: TerrainModel,
	Urbanization<M>: TerrainModel,
{
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<UrbanizationGenerationCore<M>>() {
			app.add_plugins(UrbanizationGenerationCore::<M>::default());
		}
		app.insert_resource(UrbanizationModeConfig::<Mode>::new(self.config.clone()));
		app.add_systems(
			OnEnter(ActiveGenerationMode::of::<Mode>()),
			apply_urbanization_mode::<Mode>,
		);
		app.add_systems(
			OnExit(ActiveGenerationMode::of::<Mode>()),
			clear_urbanization_mode,
		);
		Mode::install(app, &self.config);
	}

	fn finish(&self, app: &mut App) {
		M::require_generation(app);
	}
}

pub(crate) fn apply_urbanization_mode<Mode: GenerationMode>(
	mode: Res<UrbanizationModeConfig<Mode>>,
	mut development: ResMut<DevelopmentConfig>,
	mut budget: ResMut<LodGenerateBudget<UrbanizationLodChan>>,
) {
	*development = mode.config.development_config();
	*budget = LodGenerateBudget::new(mode.config.generate_budget);
}

pub(crate) fn clear_urbanization_mode(
	mut store: ResMut<DevelopmentEntryStore>,
	mut layer: ResMut<UrbanizationLayerRegion>,
	key: Option<ResMut<UrbanizationStreamKey>>,
	lod: Option<UrbanizationStreamLod>,
) {
	store.clear();
	layer.region = None;
	clear_urbanization_stream(None, key, lod);
}
