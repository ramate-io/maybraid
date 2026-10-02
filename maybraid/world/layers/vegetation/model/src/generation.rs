//! [`VegetationGenerationPlugin`]: forest, grove, and bump-out selection.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use bevy::prelude::*;
use chico::{BumpOutLodChan, ForestLodChan, MediumBumpOutLodChan};
use layer_stack::{ActiveGenerationMode, GenerationMode};
use lod::gen::LodGenerateBudget;

use crate::config::VegetationLayerConfig;
use crate::stream::{install_vegetation_stream, register_bump_out_generate, register_forest_generate};

/// Systems that arm forest and bump-out keep regions.
///
/// Callers that edit the layer config order `.before` this set.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VegetationGenerationSystems;

/// Shared forest / bump-out generate registration, added once.
pub struct VegetationGenerationCore;

impl Plugin for VegetationGenerationCore {
	fn build(&self, app: &mut App) {
		register_forest_generate(app);
		register_bump_out_generate(app);
	}
}

/// Per-mode forest spec and budgets the stream systems read.
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

fn apply_vegetation_mode<Mode: GenerationMode>(
	mode: Res<VegetationModeConfig<Mode>>,
	mut forest: ResMut<LodGenerateBudget<ForestLodChan>>,
	mut bump_out: ResMut<LodGenerateBudget<BumpOutLodChan>>,
	mut medium: ResMut<LodGenerateBudget<MediumBumpOutLodChan>>,
) {
	*forest = LodGenerateBudget::new(mode.config.forest_budget);
	*bump_out = LodGenerateBudget::new(mode.config.bump_out_budget);
	*medium = LodGenerateBudget::new(mode.config.medium_bump_out_budget);
}

/// Forest / grove / bump-out selection for `Mode`. No grow, no hosts, no terrain.
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
		if !app.is_plugin_added::<VegetationGenerationCore>() {
			app.add_plugins(VegetationGenerationCore);
		}
		app.insert_resource(VegetationModeConfig::<Mode>::new(self.config.clone()));
		app.add_systems(
			OnEnter(ActiveGenerationMode::of::<Mode>()),
			apply_vegetation_mode::<Mode>,
		);
		install_vegetation_stream::<Mode>(app);
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::{App, AssetPlugin, MinimalPlugins, NextState};
	use bevy::state::app::StatesPlugin;
	use chico::ForestLodChan;
	use durham::TerrainStreamingEnabled;
	use layer_stack::{ActiveGenerationMode, GenerationMode, GenerationModePlugin};
	use lod::gen::LodGenerateBudget;

	use super::{VegetationGenerationCore, VegetationGenerationPlugin, VegetationModeConfig};
	use crate::config::VegetationLayerConfig;

	struct Alpha;

	impl GenerationMode for Alpha {}

	struct Beta;

	impl GenerationMode for Beta {}

	fn forest_radius<Mode: GenerationMode>(app: &App) -> Option<u32> {
		app.world()
			.get_resource::<VegetationModeConfig<Mode>>()
			.and_then(|config| config.config.forest)
			.map(|spec| spec.stream_radius)
	}

	fn hop(app: &mut App, mode: ActiveGenerationMode) -> anyhow::Result<()> {
		app.world_mut()
			.resource_mut::<NextState<ActiveGenerationMode>>()
			.set(mode);
		app.update();
		Ok(())
	}

	fn vegetation_app(alpha: VegetationLayerConfig, beta: VegetationLayerConfig) -> App {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			AssetPlugin::default(),
			StatesPlugin,
			GenerationModePlugin::<Alpha>::initial(),
			GenerationModePlugin::<Beta>::default(),
			VegetationGenerationPlugin::<Alpha>::new(alpha),
			VegetationGenerationPlugin::<Beta>::new(beta),
		));
		app.insert_resource(TerrainStreamingEnabled(false));
		app.finish();
		app
	}

	#[test]
	fn different_budgets_build_and_apply_on_enter() -> anyhow::Result<()> {
		let mut beta = VegetationLayerConfig::grove();
		beta.forest_budget = 32;
		beta.bump_out_budget = 8;
		beta.medium_bump_out_budget = 4;
		let mut app = vegetation_app(VegetationLayerConfig::world_defaults(), beta);
		app.update();
		anyhow::ensure!(
			app.world().resource::<LodGenerateBudget<ForestLodChan>>().ids_per_frame == 16,
			"initial forest budget"
		);

		hop(&mut app, ActiveGenerationMode::of::<Beta>())?;
		anyhow::ensure!(
			app.world().resource::<LodGenerateBudget<ForestLodChan>>().ids_per_frame == 32,
			"beta forest budget"
		);
		anyhow::ensure!(
			app.world()
				.resource::<LodGenerateBudget<chico::BumpOutLodChan>>()
				.ids_per_frame
				== 8,
			"beta bump-out budget"
		);

		hop(&mut app, ActiveGenerationMode::of::<Alpha>())?;
		anyhow::ensure!(
			app.world().resource::<LodGenerateBudget<ForestLodChan>>().ids_per_frame == 16,
			"return restores forest budget"
		);
		Ok(())
	}

	#[test]
	fn plugin_order_does_not_matter() -> anyhow::Result<()> {
		let mut generation_first = App::new();
		generation_first.add_plugins((
			MinimalPlugins,
			AssetPlugin::default(),
			StatesPlugin,
			VegetationGenerationPlugin::<Beta>::new(VegetationLayerConfig::grove()),
			VegetationGenerationPlugin::<Alpha>::new(VegetationLayerConfig::world_defaults()),
			GenerationModePlugin::<Alpha>::initial(),
			GenerationModePlugin::<Beta>::default(),
		));
		generation_first.insert_resource(TerrainStreamingEnabled(false));
		generation_first.finish();
		generation_first.update();
		anyhow::ensure!(
			generation_first.is_plugin_added::<VegetationGenerationCore>(),
			"core is installed"
		);
		anyhow::ensure!(
			forest_radius::<Alpha>(&generation_first) == Some(1),
			"alpha keeps radius 1"
		);

		let mut beta_first = App::new();
		beta_first.add_plugins((
			MinimalPlugins,
			AssetPlugin::default(),
			StatesPlugin,
			GenerationModePlugin::<Alpha>::initial(),
			GenerationModePlugin::<Beta>::default(),
			VegetationGenerationPlugin::<Beta>::new(VegetationLayerConfig::grove()),
			VegetationGenerationPlugin::<Alpha>::new(VegetationLayerConfig::world_defaults()),
		));
		beta_first.insert_resource(TerrainStreamingEnabled(false));
		beta_first.finish();
		beta_first.update();
		anyhow::ensure!(
			forest_radius::<Alpha>(&beta_first) == Some(1),
			"beta plugin first still started from alpha"
		);
		Ok(())
	}

	#[test]
	fn hopping_modes_writes_each_forest_spec() -> anyhow::Result<()> {
		let mut app = vegetation_app(
			VegetationLayerConfig::world_defaults(),
			VegetationLayerConfig::grove(),
		);
		app.update();
		anyhow::ensure!(forest_radius::<Alpha>(&app) == Some(1), "initial mode is radius 1");

		hop(&mut app, ActiveGenerationMode::of::<Beta>())?;
		anyhow::ensure!(forest_radius::<Beta>(&app) == Some(0), "grove mode is radius 0");

		hop(&mut app, ActiveGenerationMode::of::<Alpha>())?;
		anyhow::ensure!(forest_radius::<Alpha>(&app) == Some(1), "return restores radius 1");
		Ok(())
	}
}
