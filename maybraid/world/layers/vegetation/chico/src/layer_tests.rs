use bevy::ecs::system::SystemParamItem;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::{App, AssetPlugin, MinimalPlugins, NextState};
use bevy::state::app::StatesPlugin;
use layer_stack::{ActiveGenerationMode, Generate, GenerationMode, GenerationModePlugin, LayerGenerationCore, LayerModeConfig, Scheme};
use lod::gen::LodGenerateBudget;
use lod::presentation::LodPresentBudget;
use lod::lod_ref::LodRef;
use terrain_layer_model::{HeightField, TerrainCell, TerrainModel, TerrainStreaming};
use vegetation_layer_model::Vegetation;

use crate::config::ChicoConfig;
use crate::generation::{BumpOutLodChan, ForestLodChan, MediumBumpOutLodChan};
use crate::ground::ChicoGround;
use crate::layer_stream::VegetationStreamKey;
use crate::model::Chico;

struct Alpha;

impl GenerationMode for Alpha {}

struct Beta;

impl GenerationMode for Beta {}

struct Ground;

struct GroundCell;

impl TerrainCell for GroundCell {
	type Mesh = ();
	fn bounds(&self) -> Aabb3d {
		Aabb3d::from_min_max(bevy::math::Vec3::ZERO, bevy::math::Vec3::ONE)
	}
	fn mesh_builder(&self) {}
	fn chunk_pose(&self) -> bevy::prelude::Transform {
		bevy::prelude::Transform::IDENTITY
	}
	fn seeds_collision(&self) -> bool {
		false
	}
	fn res_2(&self) -> u8 {
		0
	}
}

#[derive(Clone)]
struct GroundField;

impl HeightField for GroundField {
	fn height_at(&self, _xz: bevy::math::Vec2) -> Option<f32> {
		None
	}
	fn fallback_height_at(&self, _xz: bevy::math::Vec2) -> f32 {
		0.0
	}
}

impl TerrainModel for Ground {
	type Base = Self;
	type Cell = GroundCell;
	type Read = ();
	type Snapshot = GroundField;
	type Prepare = ();

	fn prepare(
		_prepare: &mut SystemParamItem<'_, '_, Self::Prepare>,
		_bounds: Aabb3d,
		_lod_ref: &LodRef,
	) {
	}

	fn height_at(
		_read: &SystemParamItem<'_, '_, Self::Read>,
		_xz: bevy::math::Vec2,
	) -> Option<f32> {
		None
	}

	fn fallback_height_at(
		_read: &SystemParamItem<'_, '_, Self::Read>,
		_xz: bevy::math::Vec2,
	) -> f32 {
		0.0
	}

	fn overlay_cell<'a>(
		_read: &'a SystemParamItem<'_, '_, Self::Read>,
		_bounds: Aabb3d,
		_target_size: f32,
		_overlay_size_tolerance: Option<f32>,
	) -> Option<&'a dyn TerrainCell<Mesh = ()>> {
		None
	}

	fn snapshot(_read: &SystemParamItem<'_, '_, Self::Read>, _region: Aabb3d) -> GroundField {
		GroundField
	}

	fn require_generation(_app: &App) {}
}

impl ChicoGround for Ground {
	fn fine_overlay_size() -> f32 {
		40.0
	}

	fn cascade_chunk(_bounds: Aabb3d, _res_2: u8) -> (bevy::math::Vec3, bevy::math::Vec3) {
		(bevy::math::Vec3::ZERO, bevy::math::Vec3::ONE)
	}
}

impl Scheme<Vegetation<Chico<Ground>>> for Alpha {
	fn install(app: &mut App, _config: &ChicoConfig) {
		crate::install_vegetation_stream::<Alpha, Chico<Ground>>(app);
	}
}

impl Scheme<Vegetation<Chico<Ground>>> for Beta {
	fn install(app: &mut App, _config: &ChicoConfig) {
		crate::install_vegetation_stream::<Beta, Chico<Ground>>(app);
	}
}

fn forest_radius<Mode: GenerationMode>(app: &App) -> Option<u32> {
	app.world()
		.get_resource::<LayerModeConfig<Mode, Vegetation<Chico<Ground>>>>()
		.and_then(|config| config.config.forest)
		.map(|spec| spec.stream_radius)
}

fn hop(app: &mut App, mode: ActiveGenerationMode) -> anyhow::Result<()> {
	app.world_mut().resource_mut::<NextState<ActiveGenerationMode>>().set(mode);
	app.update();
	Ok(())
}

fn vegetation_app(alpha: ChicoConfig, beta: ChicoConfig) -> App {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		StatesPlugin,
		GenerationModePlugin::<Alpha>::initial(),
		GenerationModePlugin::<Beta>::default(),
		Generate::<Alpha, Vegetation<Chico<Ground>>>::new(alpha),
		Generate::<Beta, Vegetation<Chico<Ground>>>::new(beta),
	));
	app.insert_resource(TerrainStreaming::<Ground>::new(false));
	app.finish();
	app
}

#[test]
fn different_budgets_build_and_apply_on_enter() -> anyhow::Result<()> {
	let mut beta = ChicoConfig::grove();
	beta.forest_budget = 32;
	beta.bump_out_budget = 8;
	beta.medium_bump_out_budget = 4;
	let mut app = vegetation_app(ChicoConfig::world_defaults(), beta);
	app.update();
	anyhow::ensure!(
		app.world().resource::<LodGenerateBudget<ForestLodChan>>().ids_per_frame == 16,
		"initial forest budget"
	);
	anyhow::ensure!(
		app.world().resource::<LodPresentBudget<ForestLodChan>>().ids_per_frame == 16,
		"initial forest present budget"
	);

	hop(&mut app, ActiveGenerationMode::of::<Beta>())?;
	anyhow::ensure!(
		app.world().resource::<LodGenerateBudget<ForestLodChan>>().ids_per_frame == 32,
		"beta forest budget"
	);
	anyhow::ensure!(
		app.world().resource::<LodPresentBudget<ForestLodChan>>().ids_per_frame == 32,
		"beta forest present budget"
	);
	anyhow::ensure!(
		app.world().resource::<LodGenerateBudget<BumpOutLodChan>>().ids_per_frame == 8,
		"beta bump-out budget"
	);
	anyhow::ensure!(
		app.world().resource::<LodPresentBudget<BumpOutLodChan>>().ids_per_frame == 8,
		"beta bump-out present budget"
	);
	anyhow::ensure!(
		app.world().resource::<LodPresentBudget<MediumBumpOutLodChan>>().ids_per_frame == 4,
		"beta medium bump-out present budget"
	);

	hop(&mut app, ActiveGenerationMode::of::<Alpha>())?;
	anyhow::ensure!(
		app.world().resource::<LodGenerateBudget<ForestLodChan>>().ids_per_frame == 16,
		"return restores forest budget"
	);
	anyhow::ensure!(
		app.world().resource::<LodPresentBudget<ForestLodChan>>().ids_per_frame == 16,
		"return restores forest present budget"
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
		Generate::<Beta, Vegetation<Chico<Ground>>>::new(ChicoConfig::grove()),
		Generate::<Alpha, Vegetation<Chico<Ground>>>::new(ChicoConfig::world_defaults()),
		GenerationModePlugin::<Alpha>::initial(),
		GenerationModePlugin::<Beta>::default(),
	));
	generation_first.insert_resource(TerrainStreaming::<Ground>::new(false));
	generation_first.finish();
	generation_first.update();
	anyhow::ensure!(
		generation_first.is_plugin_added::<LayerGenerationCore<Vegetation<Chico<Ground>>>>(),
		"core is installed"
	);
	anyhow::ensure!(forest_radius::<Alpha>(&generation_first) == Some(1), "alpha keeps radius 1");

	let mut beta_first = App::new();
	beta_first.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		StatesPlugin,
		GenerationModePlugin::<Alpha>::initial(),
		GenerationModePlugin::<Beta>::default(),
		Generate::<Beta, Vegetation<Chico<Ground>>>::new(ChicoConfig::grove()),
		Generate::<Alpha, Vegetation<Chico<Ground>>>::new(ChicoConfig::world_defaults()),
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
	use lod::gen::SpatialIndex;

	use crate::{ChicoForest, ForestExtent, ForestIndex};

	let index = app.world().resource::<ForestIndex>();
	SpatialIndex::<ChicoForest>::get(index, ForestExtent::from_cell_index(0, 0).id()).is_some()
}

fn plant_origin_forest(app: &mut App) {
	use crate::{ForestExtent, ForestIndex};

	app.world_mut()
		.resource_mut::<ForestIndex>()
		.ensure_forest_selected(ForestExtent::from_cell_index(0, 0));
}

#[test]
fn hopping_modes_writes_each_forest_spec() -> anyhow::Result<()> {
	let mut app = vegetation_app(ChicoConfig::world_defaults(), ChicoConfig::grove());
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
