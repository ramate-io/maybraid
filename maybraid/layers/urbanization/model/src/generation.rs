//! [`UrbanizationGenerationPlugin`]: urbanization cells, pads, and padded cells over `M`.

use std::any::type_name;
use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use durham_terrain_models::{terrain_streaming_enabled, TerrainColliderSystems};
use lod::LodPresentSystems;
use richmond_development_models::{
	register_richmond_development_models_plugin, DevelopmentEntryStore,
};
use terrain_layer_model::{ActiveGenerationMode, GenerationMode, TerrainModel};

use crate::config::{UrbanizationLayerConfig, UrbanizationSharedConfig};
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
pub struct UrbanizationGenerationCore<M> {
	pub shared: UrbanizationSharedConfig,
	_marker: PhantomData<fn() -> M>,
}

impl<M> Plugin for UrbanizationGenerationCore<M>
where
	M: TerrainModel + Send + Sync + 'static,
	Urbanization<M>: TerrainModel,
{
	fn build(&self, app: &mut App) {
		register_richmond_development_models_plugin(app);
		app.insert_resource(InstalledUrbanizationShared::<M>(
			self.shared.clone(),
			PhantomData,
		))
		.insert_resource(self.shared.development.clone())
		.init_resource::<UrbanizationLayerRegion>()
		.init_resource::<UrbanizationStreamKey>()
		.configure_sets(
			Update,
			UrbanizationStoreSystems
				.in_set(UrbanizationGenerationSystems)
				.run_if(terrain_streaming_enabled),
		)
		.add_systems(
			Update,
			generate_urbanization_padded_terrain
				.in_set(UrbanizationGenerationSystems)
				.after(UrbanizationStoreSystems)
				.run_if(terrain_streaming_enabled)
				.before(LodPresentSystems::Produce)
				.before(TerrainColliderSystems::QueueMeshes),
		);
	}
}

#[derive(Resource)]
struct InstalledUrbanizationShared<M: Send + Sync + 'static>(
	UrbanizationSharedConfig,
	PhantomData<fn() -> M>,
);

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
		let Some(state) = app.world().get_resource::<State<ActiveGenerationMode>>() else {
			panic!(
				"UrbanizationGenerationPlugin<{}, {}> requires GenerationModePlugin first",
				Mode::name(),
				type_name::<M>()
			);
		};
		if state.get().is::<Mode>() && !app.is_plugin_added::<UrbanizationGenerationCore<M>>() {
			app.add_plugins(UrbanizationGenerationCore::<M> {
				shared: self.config.shared_config(),
				_marker: PhantomData,
			});
		}
		app.insert_resource(UrbanizationModeConfig::<Mode>::new(self.config.clone()));
		app.add_systems(
			OnExit(ActiveGenerationMode::of::<Mode>()),
			clear_urbanization_mode,
		);
		Mode::install(app, &self.config);
	}

	fn finish(&self, app: &mut App) {
		M::require_generation(app);
		let Some(installed) = app.world().get_resource::<InstalledUrbanizationShared<M>>() else {
			panic!(
				"the initial generation mode never registered UrbanizationGenerationPlugin for {}",
				type_name::<M>()
			);
		};
		let shared = self.config.shared_config();
		if installed.0 != shared {
			panic!(
				"UrbanizationGenerationPlugin shared config disagrees for {}: {:?} vs {shared:?}",
				type_name::<M>(),
				installed.0
			);
		}
	}
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
