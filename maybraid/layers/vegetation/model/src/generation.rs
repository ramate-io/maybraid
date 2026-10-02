//! [`VegetationGenerationPlugin`]: forest, grove, and bump-out selection.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use bevy::prelude::*;
use terrain_layer_model::{ActiveGenerationMode, GenerationMode};

use crate::config::VegetationLayerConfig;
use crate::stream::{configure_stream_systems, register_bump_out_generate, register_forest_generate};

/// Systems that arm forest and bump-out keep regions.
///
/// Callers that edit the layer config order `.before` this set.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VegetationGenerationSystems;

/// Shared forest / bump-out generate registration, added once.
pub struct VegetationGenerationCore {
	config: VegetationLayerConfig,
}

impl Plugin for VegetationGenerationCore {
	fn build(&self, app: &mut App) {
		app.insert_resource(self.config.clone())
			.insert_resource(InstalledVegetationBudgets::from(&self.config));
		register_forest_generate(app, self.config.forest_budget);
		register_bump_out_generate(
			app,
			self.config.bump_out_budget,
			self.config.medium_bump_out_budget,
		);
		configure_stream_systems(app);
	}
}

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
struct InstalledVegetationBudgets {
	forest_budget: u32,
	bump_out_budget: u32,
	medium_bump_out_budget: u32,
}

impl From<&VegetationLayerConfig> for InstalledVegetationBudgets {
	fn from(config: &VegetationLayerConfig) -> Self {
		Self {
			forest_budget: config.forest_budget,
			bump_out_budget: config.bump_out_budget,
			medium_bump_out_budget: config.medium_bump_out_budget,
		}
	}
}

/// Per-mode forest spec written into [`VegetationLayerConfig`] on enter.
#[derive(Resource, Clone)]
pub struct VegetationModeConfig<Mode: GenerationMode> {
	pub config: VegetationLayerConfig,
	_mode: PhantomData<fn() -> Mode>,
}

impl<Mode: GenerationMode> VegetationModeConfig<Mode> {
	pub fn new(config: VegetationLayerConfig) -> Self {
		Self { config, _mode: PhantomData }
	}
}

/// Writes this mode's forest spec when it differs from the live config.
fn apply_vegetation_mode<Mode: GenerationMode>(
	mode: Res<VegetationModeConfig<Mode>>,
	mut layer: ResMut<VegetationLayerConfig>,
) {
	if layer.forest != mode.config.forest {
		layer.forest = mode.config.forest;
	}
}

/// Forest / grove / bump-out selection for `Mode`. No grow, no hosts, no terrain.
///
/// Budgets are shared. The forest spec is per mode.
pub struct VegetationGenerationPlugin<Mode: GenerationMode> {
	pub config: VegetationLayerConfig,
	_mode: PhantomData<fn() -> Mode>,
}

impl<Mode: GenerationMode> VegetationGenerationPlugin<Mode> {
	pub fn new(config: VegetationLayerConfig) -> Self {
		Self { config, _mode: PhantomData }
	}
}

impl<Mode: GenerationMode> Default for VegetationGenerationPlugin<Mode> {
	fn default() -> Self {
		Self::new(VegetationLayerConfig::default())
	}
}

impl<Mode: GenerationMode> Plugin for VegetationGenerationPlugin<Mode> {
	fn build(&self, app: &mut App) {
		let Some(state) = app.world().get_resource::<State<ActiveGenerationMode>>() else {
			panic!(
				"VegetationGenerationPlugin<{}> requires GenerationModePlugin first",
				Mode::name()
			);
		};
		if state.get().is::<Mode>() && !app.is_plugin_added::<VegetationGenerationCore>() {
			app.add_plugins(VegetationGenerationCore { config: self.config.clone() });
		}
		app.insert_resource(VegetationModeConfig::<Mode>::new(self.config.clone()));
		app.add_systems(
			OnEnter(ActiveGenerationMode::of::<Mode>()),
			apply_vegetation_mode::<Mode>,
		);
	}

	fn finish(&self, app: &mut App) {
		let Some(installed) = app.world().get_resource::<InstalledVegetationBudgets>() else {
			panic!("the initial generation mode never registered VegetationGenerationPlugin");
		};
		let budgets = InstalledVegetationBudgets::from(&self.config);
		if *installed != budgets {
			panic!(
				"VegetationGenerationPlugin shared config disagrees: {installed:?} vs {budgets:?}"
			);
		}
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::{App, AssetPlugin, MinimalPlugins, NextState};
	use bevy::state::app::StatesPlugin;
	use durham_terrain_models::TerrainStreamingEnabled;
	use terrain_layer_model::{ActiveGenerationMode, GenerationMode, GenerationModePlugin};

	use super::{VegetationGenerationCore, VegetationGenerationPlugin};
	use crate::config::VegetationLayerConfig;

	struct Alpha;

	impl GenerationMode for Alpha {}

	struct Beta;

	impl GenerationMode for Beta {}

	fn forest_radius(app: &App) -> Option<u32> {
		app.world()
			.get_resource::<VegetationLayerConfig>()
			.and_then(|config| config.forest)
			.map(|spec| spec.stream_radius)
	}

	fn hop(app: &mut App, mode: ActiveGenerationMode) -> anyhow::Result<()> {
		app.world_mut()
			.resource_mut::<NextState<ActiveGenerationMode>>()
			.set(mode);
		app.update();
		Ok(())
	}

	fn vegetation_app() -> App {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			AssetPlugin::default(),
			StatesPlugin,
			GenerationModePlugin::<Alpha>::initial(),
			GenerationModePlugin::<Beta>::default(),
			VegetationGenerationPlugin::<Alpha>::new(VegetationLayerConfig::world_defaults()),
			VegetationGenerationPlugin::<Beta>::new(VegetationLayerConfig::grove()),
		));
		app.insert_resource(TerrainStreamingEnabled(false));
		app.finish();
		app
	}

	#[test]
	fn two_modes_install_the_shared_part_once() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			AssetPlugin::default(),
			StatesPlugin,
			GenerationModePlugin::<Alpha>::initial(),
			GenerationModePlugin::<Beta>::default(),
			VegetationGenerationPlugin::<Alpha>::new(VegetationLayerConfig::world_defaults()),
			VegetationGenerationPlugin::<Beta>::new(VegetationLayerConfig::grove()),
		));
		app.finish();
		anyhow::ensure!(
			app.is_plugin_added::<VegetationGenerationCore>(),
			"shared generate registration is installed"
		);
		Ok(())
	}

	#[test]
	fn a_disagreeing_budget_fails() -> anyhow::Result<()> {
		let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
			let mut app = App::new();
			app.add_plugins((
				MinimalPlugins,
				AssetPlugin::default(),
				StatesPlugin,
				GenerationModePlugin::<Alpha>::initial(),
				GenerationModePlugin::<Beta>::default(),
				VegetationGenerationPlugin::<Alpha>::new(VegetationLayerConfig::world_defaults()),
				VegetationGenerationPlugin::<Beta>::new(VegetationLayerConfig {
					forest_budget: 32,
					..VegetationLayerConfig::grove()
				}),
			));
			app.finish();
		}));
		anyhow::ensure!(failed.is_err(), "disagreeing budgets must fail loudly");
		Ok(())
	}

	#[test]
	fn startup_leaves_the_initial_spec_untouched() -> anyhow::Result<()> {
		let mut app = vegetation_app();
		let before = app
			.world()
			.get_resource::<VegetationLayerConfig>()
			.cloned()
			.ok_or_else(|| anyhow::anyhow!("layer config"))?;
		app.update();
		let after = app
			.world()
			.get_resource::<VegetationLayerConfig>()
			.cloned()
			.ok_or_else(|| anyhow::anyhow!("layer config after startup"))?;
		anyhow::ensure!(after == before, "startup OnEnter does not rewrite the spec");
		anyhow::ensure!(
			after.forest.map(|spec| spec.stream_radius) == Some(1),
			"initial mode keeps stream radius 1"
		);
		Ok(())
	}

	#[test]
	fn hopping_modes_writes_each_forest_spec() -> anyhow::Result<()> {
		let mut app = vegetation_app();
		app.update();
		anyhow::ensure!(forest_radius(&app) == Some(1), "initial mode is radius 1");

		hop(&mut app, ActiveGenerationMode::of::<Beta>())?;
		anyhow::ensure!(forest_radius(&app) == Some(0), "grove mode is radius 0");

		hop(&mut app, ActiveGenerationMode::of::<Alpha>())?;
		anyhow::ensure!(forest_radius(&app) == Some(1), "return restores radius 1");
		Ok(())
	}
}
