//! [`MobGenerationPlugin`]: config-free install plus a per-mode config.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use bevy::prelude::*;
use layer_stack::{ActiveGenerationMode, GenerationMode};
use terrain_layer_model::TerrainModel;

use crate::mob::MobModel;

/// Systems that arm mob keep regions and sync selection models.
///
/// Scheme writes order `.before` LOD generate / present produce through this set.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MobGenerationSystems;

/// A model with its own mob generation stack.
pub trait MobGeneration: MobModel {
	type Config: Clone + Send + Sync + 'static;

	fn install_generation(app: &mut App);

	fn apply_generation(world: &mut World, config: &Self::Config);

	fn clear_generation(world: &mut World);
}

/// A mode's mob writes for model `B`.
pub trait MobScheme<B: MobGeneration>: GenerationMode {
	fn install(app: &mut App, config: &B::Config);
}

/// Shared install for `B`, added once.
pub struct MobGenerationCore<B>(PhantomData<fn() -> B>);

impl<B> Default for MobGenerationCore<B> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<B: MobGeneration> Plugin for MobGenerationCore<B> {
	fn build(&self, app: &mut App) {
		B::install_generation(app);
		app.configure_sets(Update, MobGenerationSystems);
	}
}

/// Per-mode config the scheme systems read, keyed by the mob model.
#[derive(Resource, Clone)]
pub struct MobModeConfig<Mode: GenerationMode, B: MobGeneration> {
	pub config: B::Config,
	_marker: PhantomData<fn() -> (Mode, B)>,
}

impl<Mode: GenerationMode, B: MobGeneration> MobModeConfig<Mode, B> {
	pub fn new(config: B::Config) -> Self {
		Self { config, _marker: PhantomData }
	}
}

/// Generation for model `B` in `Mode`.
pub struct MobGenerationPlugin<Mode, B: MobGeneration> {
	pub config: B::Config,
	_marker: PhantomData<fn() -> (Mode, B)>,
}

impl<Mode, B> MobGenerationPlugin<Mode, B>
where
	Mode: MobScheme<B>,
	B: MobGeneration,
{
	pub fn new(config: B::Config) -> Self {
		Self { config, _marker: PhantomData }
	}
}

impl<Mode, B> Default for MobGenerationPlugin<Mode, B>
where
	Mode: MobScheme<B>,
	B: MobGeneration,
	B::Config: Default,
{
	fn default() -> Self {
		Self::new(B::Config::default())
	}
}

impl<Mode, B> Plugin for MobGenerationPlugin<Mode, B>
where
	Mode: MobScheme<B>,
	B: MobGeneration,
{
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<MobGenerationCore<B>>() {
			app.add_plugins(MobGenerationCore::<B>::default());
		}
		app.insert_resource(MobModeConfig::<Mode, B>::new(self.config.clone()));
		app.add_systems(OnEnter(ActiveGenerationMode::of::<Mode>()), apply_mobs::<Mode, B>);
		app.add_systems(OnExit(ActiveGenerationMode::of::<Mode>()), clear_mobs::<B>);
		Mode::install(app, &self.config);
	}

	fn finish(&self, app: &mut App) {
		B::Ground::require_generation(app);
	}
}

fn apply_mobs<Mode, B>(world: &mut World)
where
	Mode: GenerationMode,
	B: MobGeneration,
{
	let config = world.resource::<MobModeConfig<Mode, B>>().config.clone();
	B::apply_generation(world, &config);
}

fn clear_mobs<B: MobGeneration>(world: &mut World) {
	B::clear_generation(world);
}
