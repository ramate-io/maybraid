//! [`FurnishingGenerationPlugin`]: config-free install plus a per-mode config.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use bevy::prelude::*;
use layer_stack::{ActiveGenerationMode, GenerationMode};
use terrain_layer_model::TerrainModel;

use crate::furnishing::FurnishingModel;

/// Systems that keep the furniture neighborhood and bin occupied cells.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FurnishingGenerationSystems;

/// A model with its own furnishing generation stack.
pub trait FurnishingGeneration: FurnishingModel {
	type Config: Clone + Send + Sync + 'static;

	fn install_generation(app: &mut App);

	fn apply_generation(world: &mut World, config: &Self::Config);

	fn clear_generation(world: &mut World);
}

/// A mode's furnishing writes for model `F`.
pub trait FurnishingScheme<F: FurnishingGeneration>: GenerationMode {
	fn install(app: &mut App, config: &F::Config);
}

/// Shared install for `F`, added once.
pub struct FurnishingGenerationCore<F>(PhantomData<fn() -> F>);

impl<F> Default for FurnishingGenerationCore<F> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<F: FurnishingGeneration> Plugin for FurnishingGenerationCore<F> {
	fn build(&self, app: &mut App) {
		F::install_generation(app);
		app.configure_sets(Update, FurnishingGenerationSystems);
	}
}

/// Per-mode config the scheme systems read, keyed by the furnishing model.
#[derive(Resource, Clone)]
pub struct FurnishingModeConfig<Mode: GenerationMode, F: FurnishingGeneration> {
	pub config: F::Config,
	_marker: PhantomData<fn() -> (Mode, F)>,
}

impl<Mode: GenerationMode, F: FurnishingGeneration> FurnishingModeConfig<Mode, F> {
	pub fn new(config: F::Config) -> Self {
		Self { config, _marker: PhantomData }
	}
}

/// Generation for model `F` in `Mode`.
pub struct FurnishingGenerationPlugin<Mode, F: FurnishingGeneration> {
	pub config: F::Config,
	_marker: PhantomData<fn() -> (Mode, F)>,
}

impl<Mode, F> FurnishingGenerationPlugin<Mode, F>
where
	Mode: FurnishingScheme<F>,
	F: FurnishingGeneration,
{
	pub fn new(config: F::Config) -> Self {
		Self { config, _marker: PhantomData }
	}
}

impl<Mode, F> Default for FurnishingGenerationPlugin<Mode, F>
where
	Mode: FurnishingScheme<F>,
	F: FurnishingGeneration,
	F::Config: Default,
{
	fn default() -> Self {
		Self::new(F::Config::default())
	}
}

impl<Mode, F> Plugin for FurnishingGenerationPlugin<Mode, F>
where
	Mode: FurnishingScheme<F>,
	F: FurnishingGeneration,
{
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<FurnishingGenerationCore<F>>() {
			app.add_plugins(FurnishingGenerationCore::<F>::default());
		}
		app.insert_resource(FurnishingModeConfig::<Mode, F>::new(self.config.clone()));
		app.add_systems(OnEnter(ActiveGenerationMode::of::<Mode>()), apply_furnishing::<Mode, F>);
		app.add_systems(OnExit(ActiveGenerationMode::of::<Mode>()), clear_furnishing::<F>);
		Mode::install(app, &self.config);
	}

	fn finish(&self, app: &mut App) {
		F::Ground::require_generation(app);
	}
}

fn apply_furnishing<Mode, F>(world: &mut World)
where
	Mode: GenerationMode,
	F: FurnishingGeneration,
{
	let config = world.resource::<FurnishingModeConfig<Mode, F>>().config.clone();
	F::apply_generation(world, &config);
}

fn clear_furnishing<F: FurnishingGeneration>(world: &mut World) {
	F::clear_generation(world);
}
