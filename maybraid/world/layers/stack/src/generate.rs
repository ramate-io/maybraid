//! [`Generate<Mode, L>`]: shared install plus a per-mode config.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use bevy::prelude::*;

use crate::layer::{register_layer_label, Layer, LayerGenerationCore, LayerModeConfig, Scheme};
use crate::{ActiveGenerationMode, GenerationMode};

/// Generation for layer `L` in `Mode`.
pub struct Generate<Mode, L: Layer> {
	pub config: L::Config,
	_mode: PhantomData<fn() -> Mode>,
}

impl<Mode, L: Layer> Generate<Mode, L> {
	pub fn new(config: L::Config) -> Self {
		Self { config, _mode: PhantomData }
	}
}

impl<Mode, L: Layer> Default for Generate<Mode, L>
where
	L::Config: Default,
{
	fn default() -> Self {
		Self::new(L::Config::default())
	}
}

impl<Mode, L> Plugin for Generate<Mode, L>
where
	Mode: Scheme<L>,
	L: Layer,
{
	fn build(&self, app: &mut App) {
		register_layer_label::<L>(app);
		if !app.is_plugin_added::<LayerGenerationCore<L>>() {
			L::install_generation(app);
			app.add_plugins(LayerGenerationCore::<L>::default());
		}
		app.insert_resource(LayerModeConfig::<Mode, L>::new(self.config.clone()));
		app.add_systems(OnEnter(ActiveGenerationMode::of::<Mode>()), apply_layer::<Mode, L>);
		app.add_systems(OnExit(ActiveGenerationMode::of::<Mode>()), clear_layer::<L>);
		Mode::install(app, &self.config);
	}

	fn finish(&self, app: &mut App) {
		L::require_lower(app);
	}
}

fn apply_layer<Mode, L>(world: &mut World)
where
	Mode: GenerationMode,
	L: Layer,
{
	let config = world.resource::<LayerModeConfig<Mode, L>>().config.clone();
	L::apply_generation(world, &config);
}

fn clear_layer<L: Layer>(world: &mut World) {
	L::clear_generation(world);
}
