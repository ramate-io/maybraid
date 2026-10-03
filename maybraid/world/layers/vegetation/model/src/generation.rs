//! [`VegetationGenerationPlugin`]: config-free install plus a per-mode config.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use bevy::prelude::*;
use layer_stack::{ActiveGenerationMode, GenerationMode};
use terrain_layer_model::TerrainModel;

use crate::vegetation::VegetationModel;

/// Systems that arm forest and bump-out keep regions.
///
/// Callers that edit the layer config order `.before` this set.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VegetationGenerationSystems;

/// A model with its own vegetation generation stack.
pub trait VegetationGeneration: VegetationModel {
	type Config: Clone + Send + Sync + 'static;

	fn install_generation(app: &mut App);

	fn apply_generation(world: &mut World, config: &Self::Config);

	fn clear_generation(world: &mut World);
}

/// A mode's vegetation writes for model `V`.
pub trait VegetationScheme<V: VegetationGeneration>: GenerationMode {
	fn install(app: &mut App, config: &V::Config);
}

/// Shared install for `V`, added once.
pub struct VegetationGenerationCore<V>(PhantomData<fn() -> V>);

impl<V> Default for VegetationGenerationCore<V> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<V: VegetationGeneration> Plugin for VegetationGenerationCore<V> {
	fn build(&self, app: &mut App) {
		V::install_generation(app);
		app.configure_sets(Update, VegetationGenerationSystems);
	}
}

/// Per-mode config the scheme systems read, keyed by the vegetation model.
#[derive(Resource, Clone)]
pub struct VegetationModeConfig<Mode: GenerationMode, V: VegetationGeneration> {
	pub config: V::Config,
	_marker: PhantomData<fn() -> (Mode, V)>,
}

impl<Mode: GenerationMode, V: VegetationGeneration> VegetationModeConfig<Mode, V> {
	pub fn new(config: V::Config) -> Self {
		Self { config, _marker: PhantomData }
	}
}

/// Generation for model `V` in `Mode`.
pub struct VegetationGenerationPlugin<Mode, V: VegetationGeneration> {
	pub config: V::Config,
	_marker: PhantomData<fn() -> (Mode, V)>,
}

impl<Mode, V> VegetationGenerationPlugin<Mode, V>
where
	Mode: VegetationScheme<V>,
	V: VegetationGeneration,
{
	pub fn new(config: V::Config) -> Self {
		Self { config, _marker: PhantomData }
	}
}

impl<Mode, V> Default for VegetationGenerationPlugin<Mode, V>
where
	Mode: VegetationScheme<V>,
	V: VegetationGeneration,
	V::Config: Default,
{
	fn default() -> Self {
		Self::new(V::Config::default())
	}
}

impl<Mode, V> Plugin for VegetationGenerationPlugin<Mode, V>
where
	Mode: VegetationScheme<V>,
	V: VegetationGeneration,
{
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<VegetationGenerationCore<V>>() {
			app.add_plugins(VegetationGenerationCore::<V>::default());
		}
		app.insert_resource(VegetationModeConfig::<Mode, V>::new(self.config.clone()));
		app.add_systems(OnEnter(ActiveGenerationMode::of::<Mode>()), apply_vegetation::<Mode, V>);
		app.add_systems(OnExit(ActiveGenerationMode::of::<Mode>()), clear_vegetation::<V>);
		Mode::install(app, &self.config);
	}

	fn finish(&self, app: &mut App) {
		V::Ground::require_generation(app);
	}
}

fn apply_vegetation<Mode, V>(world: &mut World)
where
	Mode: GenerationMode,
	V: VegetationGeneration,
{
	let config = world.resource::<VegetationModeConfig<Mode, V>>().config.clone();
	V::apply_generation(world, &config);
}

fn clear_vegetation<V: VegetationGeneration>(world: &mut World) {
	V::clear_generation(world);
}
