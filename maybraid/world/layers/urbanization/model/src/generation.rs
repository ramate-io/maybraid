//! [`UrbanizationGenerationPlugin`]: config-free install plus a per-mode config.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use layer_stack::{ActiveGenerationMode, GenerationMode};
use terrain_layer_model::{terrain_streaming, TerrainModel};

use crate::urban::UrbanizationModel;

/// Systems that write urbanization storage.
///
/// Presentation orders `.after(UrbanizationGenerationSystems)` so the split
/// chain matches today's single `.chain()`.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UrbanizationGenerationSystems;

/// Scheme store writes finish here. Shared padding runs after this set.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UrbanizationStoreSystems;

/// A model with its own urbanization generation stack.
pub trait UrbanizationGeneration: UrbanizationModel {
	type Config: Clone + Send + Sync + 'static;

	fn install_generation(app: &mut App);

	fn apply_generation(world: &mut World, config: &Self::Config);

	fn clear_generation(world: &mut World);
}

/// A mode's urbanization writes for model `U`.
pub trait UrbanizationScheme<U: UrbanizationGeneration>: GenerationMode {
	fn install(app: &mut App, config: &U::Config);
}

/// Shared install for `U`, added once.
pub struct UrbanizationGenerationCore<U>(PhantomData<fn() -> U>);

impl<U> Default for UrbanizationGenerationCore<U> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<U: UrbanizationGeneration> Plugin for UrbanizationGenerationCore<U> {
	fn build(&self, app: &mut App) {
		U::install_generation(app);
		app.configure_sets(
			Update,
			(
				UrbanizationGenerationSystems,
				UrbanizationStoreSystems
					.in_set(UrbanizationGenerationSystems)
					.run_if(terrain_streaming::<U::Ground>),
			),
		);
	}
}

/// Per-mode config the scheme systems read, keyed by the urbanization model.
#[derive(Resource, Clone)]
pub struct UrbanizationModeConfig<Mode: GenerationMode, U: UrbanizationGeneration> {
	pub config: U::Config,
	_marker: PhantomData<fn() -> (Mode, U)>,
}

impl<Mode: GenerationMode, U: UrbanizationGeneration> UrbanizationModeConfig<Mode, U> {
	pub fn new(config: U::Config) -> Self {
		Self { config, _marker: PhantomData }
	}
}

/// Region schemes write and padding / hosts read.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct UrbanizationLayerRegion {
	pub region: Option<Aabb3d>,
}

/// Generation for model `U` in `Mode`.
pub struct UrbanizationGenerationPlugin<Mode, U: UrbanizationGeneration> {
	pub config: U::Config,
	_marker: PhantomData<fn() -> (Mode, U)>,
}

impl<Mode, U> UrbanizationGenerationPlugin<Mode, U>
where
	Mode: UrbanizationScheme<U>,
	U: UrbanizationGeneration,
{
	pub fn new(config: U::Config) -> Self {
		Self { config, _marker: PhantomData }
	}
}

impl<Mode, U> Default for UrbanizationGenerationPlugin<Mode, U>
where
	Mode: UrbanizationScheme<U>,
	U: UrbanizationGeneration,
	U::Config: Default,
{
	fn default() -> Self {
		Self::new(U::Config::default())
	}
}

impl<Mode, U> Plugin for UrbanizationGenerationPlugin<Mode, U>
where
	Mode: UrbanizationScheme<U>,
	U: UrbanizationGeneration,
{
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<UrbanizationGenerationCore<U>>() {
			app.add_plugins(UrbanizationGenerationCore::<U>::default());
		}
		app.insert_resource(UrbanizationModeConfig::<Mode, U>::new(self.config.clone()));
		app.add_systems(OnEnter(ActiveGenerationMode::of::<Mode>()), apply_urbanization::<Mode, U>);
		app.add_systems(OnExit(ActiveGenerationMode::of::<Mode>()), clear_urbanization::<U>);
		Mode::install(app, &self.config);
	}

	fn finish(&self, app: &mut App) {
		U::Ground::require_generation(app);
	}
}

fn apply_urbanization<Mode, U>(world: &mut World)
where
	Mode: GenerationMode,
	U: UrbanizationGeneration,
{
	let config = world.resource::<UrbanizationModeConfig<Mode, U>>().config.clone();
	U::apply_generation(world, &config);
}

fn clear_urbanization<U: UrbanizationGeneration>(world: &mut World) {
	U::clear_generation(world);
}
