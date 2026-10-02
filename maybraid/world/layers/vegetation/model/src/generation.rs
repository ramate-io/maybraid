//! [`VegetationGenerationPlugin`]: forest, grove, and bump-out selection.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use bevy::prelude::*;
use chico::{BumpOutLodChan, ForestLodChan, MediumBumpOutLodChan};
use layer_stack::{ActiveGenerationMode, GenerationMode};
use lod::gen::LodGenerateBudget;

use crate::config::VegetationLayerConfig;
use crate::stream::{
	clear_vegetation_stream, install_vegetation_stream, register_bump_out_generate,
	register_forest_generate, VegetationStreamKey,
};

/// Systems that arm forest and bump-out keep regions.
///
/// Callers that edit the layer config order `.before` this set.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VegetationGenerationSystems;

/// Shared forest / bump-out generate registration, added once.
pub struct VegetationGenerationCore;

impl Plugin for VegetationGenerationCore {
	fn build(&self, app: &mut App) {
		app.init_resource::<VegetationStreamKey>();
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

/// Forest / grove / bump-out selection for `Mode`. No grow, no hosts.
/// `M` is the ground whose [`terrain_streaming`](terrain_layer_model::terrain_streaming) gates the stream.
pub struct VegetationGenerationPlugin<Mode: GenerationMode, M> {
	pub config: VegetationLayerConfig,
	_marker: PhantomData<fn() -> (Mode, M)>,
}

impl<Mode: GenerationMode, M: Send + Sync + 'static> VegetationGenerationPlugin<Mode, M> {
	pub fn new(config: VegetationLayerConfig) -> Self {
		Self { config, _marker: PhantomData }
	}
}

impl<Mode: GenerationMode, M: Send + Sync + 'static> Default for VegetationGenerationPlugin<Mode, M> {
	fn default() -> Self {
		Self::new(VegetationLayerConfig::default())
	}
}

impl<Mode: GenerationMode, M: Send + Sync + 'static> Plugin for VegetationGenerationPlugin<Mode, M> {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<VegetationGenerationCore>() {
			app.add_plugins(VegetationGenerationCore);
		}
		app.insert_resource(VegetationModeConfig::<Mode>::new(self.config.clone()));
		app.add_systems(
			OnEnter(ActiveGenerationMode::of::<Mode>()),
			apply_vegetation_mode::<Mode>,
		);
		app.add_systems(
			OnExit(ActiveGenerationMode::of::<Mode>()),
			clear_vegetation_mode,
		);
		install_vegetation_stream::<Mode, M>(app);
	}
}

fn clear_vegetation_mode(
	key: Option<ResMut<VegetationStreamKey>>,
	forest: Option<crate::stream::ForestStreamLod>,
	bump_outs: Option<crate::stream::BumpOutStreamLod>,
) {
	clear_vegetation_stream(key, forest, bump_outs);
}

#[cfg(test)]
mod tests {
	use bevy::prelude::{App, AssetPlugin, MinimalPlugins, NextState};
	use bevy::state::app::StatesPlugin;
	use chico::ForestLodChan;
	use layer_stack::{ActiveGenerationMode, GenerationMode, GenerationModePlugin};
	use lod::gen::LodGenerateBudget;
	use terrain_layer_model::TerrainStreaming;

	use super::{VegetationGenerationCore, VegetationGenerationPlugin, VegetationModeConfig};
	use crate::config::VegetationLayerConfig;

	struct Alpha;

	impl GenerationMode for Alpha {}

	struct Beta;

	impl GenerationMode for Beta {}

	struct Ground;

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
			VegetationGenerationPlugin::<Alpha, Ground>::new(alpha),
			VegetationGenerationPlugin::<Beta, Ground>::new(beta),
		));
		app.insert_resource(TerrainStreaming::<Ground>::new(false));
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
			VegetationGenerationPlugin::<Beta, Ground>::new(VegetationLayerConfig::grove()),
			VegetationGenerationPlugin::<Alpha, Ground>::new(VegetationLayerConfig::world_defaults()),
			GenerationModePlugin::<Alpha>::initial(),
			GenerationModePlugin::<Beta>::default(),
		));
		generation_first.insert_resource(TerrainStreaming::<Ground>::new(false));
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
			VegetationGenerationPlugin::<Beta, Ground>::new(VegetationLayerConfig::grove()),
			VegetationGenerationPlugin::<Alpha, Ground>::new(VegetationLayerConfig::world_defaults()),
		));
		beta_first.insert_resource(TerrainStreaming::<Ground>::new(false));
		beta_first.finish();
		beta_first.update();
		anyhow::ensure!(
			forest_radius::<Alpha>(&beta_first) == Some(1),
			"beta plugin first still started from alpha"
		);
		Ok(())
	}

	fn origin_forest_is_selected(app: &App) -> bool {
		use chico::{ChicoForest, ForestExtent, ForestIndex};
		use lod::gen::SpatialIndex;

		let index = app.world().resource::<ForestIndex>();
		SpatialIndex::<ChicoForest>::get(index, ForestExtent::from_cell_index(0, 0).id()).is_some()
	}

	fn plant_origin_forest(app: &mut App) {
		use chico::{ForestExtent, ForestIndex};

		app.world_mut()
			.resource_mut::<ForestIndex>()
			.ensure_forest_selected(ForestExtent::from_cell_index(0, 0));
	}

	#[test]
	fn hopping_modes_writes_each_forest_spec() -> anyhow::Result<()> {
		use crate::stream::VegetationStreamKey;

		let mut app = vegetation_app(
			VegetationLayerConfig::world_defaults(),
			VegetationLayerConfig::grove(),
		);
		app.update();
		anyhow::ensure!(forest_radius::<Alpha>(&app) == Some(1), "initial mode is radius 1");
		plant_origin_forest(&mut app);
		anyhow::ensure!(origin_forest_is_selected(&app), "planted cell is in the index");

		hop(&mut app, ActiveGenerationMode::of::<Beta>())?;
		anyhow::ensure!(forest_radius::<Beta>(&app) == Some(0), "grove mode is radius 0");
		anyhow::ensure!(!origin_forest_is_selected(&app), "leaving alpha clears the index");
		anyhow::ensure!(
			app.world().resource::<VegetationStreamKey>().0.is_none(),
			"leaving alpha clears the stream key"
		);

		plant_origin_forest(&mut app);
		anyhow::ensure!(origin_forest_is_selected(&app), "beta planted its own cell");

		hop(&mut app, ActiveGenerationMode::of::<Alpha>())?;
		anyhow::ensure!(forest_radius::<Alpha>(&app) == Some(1), "return restores radius 1");
		anyhow::ensure!(
			!origin_forest_is_selected(&app),
			"return hop clears the index so alpha cannot keep beta's groves"
		);
		anyhow::ensure!(
			app.world().resource::<VegetationStreamKey>().0.is_none(),
			"return hop clears the stream key"
		);
		Ok(())
	}
}
