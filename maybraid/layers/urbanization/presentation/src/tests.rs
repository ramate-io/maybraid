use std::collections::HashSet;

use bevy::app::{App, Plugin};
use bevy::ecs::system::SystemState;
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::prelude::{AssetPlugin, MinimalPlugins, NextState, Visibility, World};
use bevy::state::app::StatesPlugin;
use durham_terrain_models::{
	Durham, DurhamTerrainConfig, PresentedTerrainScene, TerrainColliderMeshSource,
	TerrainSuperseded, TerrainTrimeshCollider,
};
use lod::gen::Id;
use richmond_development_models::PresentedPaddedTerrainScene;
use terrain_layer_model::{
	subscribe_mode, ActiveGenerationMode, BaseTerrainGenerationPlugin, BaseTerrainScheme,
	GenerationMode, GenerationModePlugin, ModeSubscribers, ModeSubscription, OnTerrain,
};
use terrain_layer_presentation::TerrainPresentationPlugin;
use urbanization_layer_model::{
	Urbanization, UrbanizationGenerationPlugin, UrbanizationLayerConfig,
};

use crate::{
	sync_raw_terrain_replacements, PaddedCells, UrbanizationHosts, UrbanizationPaddedTerrainState,
	UrbanizationPresentationPlugin,
};

#[test]
fn raw_cell_hands_its_floor_to_a_cooked_padded_replacement() -> anyhow::Result<()> {
	use bevy::ecs::system::RunSystemOnce;
	let id = Id::from_cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE));
	let mut world = World::new();
	world.insert_resource(UrbanizationPaddedTerrainState {
		wanted: HashSet::from([id]),
		replaced: HashSet::new(),
	});
	let raw = world.spawn((PresentedTerrainScene(id), Visibility::Inherited)).id();
	let padded = world.spawn((PresentedPaddedTerrainScene(id), TerrainColliderMeshSource)).id();
	let run = |world: &mut World| {
		world
			.run_system_once(sync_raw_terrain_replacements)
			.map_err(|error| anyhow::anyhow!("{error:?}"))
	};

	run(&mut world)?;
	assert!(world.get::<TerrainSuperseded>(raw).is_none(), "uncooked pads cannot bear weight");
	assert_eq!(world.get::<Visibility>(raw), Some(&Visibility::Inherited));

	world.entity_mut(padded).insert(TerrainTrimeshCollider);
	run(&mut world)?;
	assert!(world.get::<TerrainSuperseded>(raw).is_some(), "raw floor stays under pads");
	assert_eq!(world.get::<Visibility>(raw), Some(&Visibility::Hidden));

	world.resource_mut::<UrbanizationPaddedTerrainState>().wanted.clear();
	run(&mut world)?;
	assert!(world.get::<TerrainSuperseded>(raw).is_none(), "raw cell collides again");
	assert_eq!(world.get::<Visibility>(raw), Some(&Visibility::Inherited));
	Ok(())
}

#[test]
fn raw_cells_another_owner_superseded_stay_superseded() -> anyhow::Result<()> {
	use bevy::ecs::system::RunSystemOnce;
	let id = Id::from_cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE));
	let mut world = World::new();
	world.init_resource::<UrbanizationPaddedTerrainState>();
	let raw = world
		.spawn((PresentedTerrainScene(id), Visibility::Hidden, TerrainSuperseded))
		.id();
	world
		.run_system_once(sync_raw_terrain_replacements)
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	assert!(world.get::<TerrainSuperseded>(raw).is_some());
	assert_eq!(world.get::<Visibility>(raw), Some(&Visibility::Hidden));
	Ok(())
}

#[test]
fn padded_viewer_quant_is_stable_inside_cell() -> anyhow::Result<()> {
	use crate::padded::quantize_viewer_xz_for_test;
	assert_eq!(quantize_viewer_xz_for_test(Vec3::new(0.1, 12.0, 7.9)), (0, 0));
	assert_eq!(quantize_viewer_xz_for_test(Vec3::new(8.0, 0.0, -0.1)), (1, -1));
	Ok(())
}

struct TestMode;

impl GenerationMode for TestMode {}

impl BaseTerrainScheme<Durham> for TestMode {
	fn install(_app: &mut App, _config: &DurhamTerrainConfig) {}
}

impl urbanization_layer_model::UrbanizationScheme<OnTerrain<Durham>> for TestMode {
	fn install(_app: &mut App, _config: &UrbanizationLayerConfig) {}
}

#[test]
fn urbanization_layers_finish_with_base_generation() {
	// Adding Durham / Richmond plugins to a bare `App` needs a render world
	// (shaders, `MaterialPlugin`). Assemblers (world, playground binary) are
	// that app. `finish` without those plugins is the should-panic sibling;
	// this locks the stack types the assemblers add.
	let _stack = (
		GenerationModePlugin::<TestMode>::initial(),
		BaseTerrainGenerationPlugin::<TestMode, Durham>::new(DurhamTerrainConfig::fine_patch(2)),
		UrbanizationGenerationPlugin::<TestMode, OnTerrain<Durham>>::new(
			UrbanizationLayerConfig::default(),
		),
		TerrainPresentationPlugin::<TestMode, Urbanization<OnTerrain<Durham>>, PaddedCells>::default(),
		UrbanizationPresentationPlugin::<TestMode, Urbanization<OnTerrain<Durham>>>::default(),
	);
}

#[test]
#[should_panic(expected = "requires terrain_layer_model::generation::BaseTerrainGenerationCore")]
fn urbanization_layers_without_base_generation_name_the_missing_plugin() {
	// `build` installs Richmond / furniture plugins that need a full Bevy app.
	// `finish` is the requirement check the assemblers actually run.
	UrbanizationGenerationPlugin::<TestMode, OnTerrain<Durham>>::default().finish(&mut App::new());
}

struct OtherMode;

impl GenerationMode for OtherMode {}

type Urbanized = Urbanization<OnTerrain<Durham>>;

fn subscribed_app() -> App {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		StatesPlugin,
		GenerationModePlugin::<TestMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
	));
	subscribe_mode::<(Urbanized, UrbanizationHosts), TestMode>(&mut app);
	subscribe_mode::<(Urbanized, PaddedCells), TestMode>(&mut app);
	app
}

#[test]
fn losing_subscription_is_inactive_and_returning_is_active() -> anyhow::Result<()> {
	let mut app = subscribed_app();
	app.update();
	{
		let mut state = SystemState::<ModeSubscription<(Urbanized, UrbanizationHosts)>>::new(
			app.world_mut(),
		);
		anyhow::ensure!(
			state.get(app.world()).map_err(|error| anyhow::anyhow!("{error:?}"))?.active(),
			"hosts follow the subscribed mode"
		);
	}
	{
		let mut state = SystemState::<ModeSubscription<(Urbanized, PaddedCells)>>::new(
			app.world_mut(),
		);
		anyhow::ensure!(
			state.get(app.world()).map_err(|error| anyhow::anyhow!("{error:?}"))?.active(),
			"padded follows the subscribed mode"
		);
	}

	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<OtherMode>());
	app.update();
	{
		let mut state = SystemState::<ModeSubscription<(Urbanized, UrbanizationHosts)>>::new(
			app.world_mut(),
		);
		anyhow::ensure!(
			!state.get(app.world()).map_err(|error| anyhow::anyhow!("{error:?}"))?.active(),
			"unsubscribed mode retires hosts"
		);
	}
	{
		let mut state = SystemState::<ModeSubscription<(Urbanized, PaddedCells)>>::new(
			app.world_mut(),
		);
		anyhow::ensure!(
			!state.get(app.world()).map_err(|error| anyhow::anyhow!("{error:?}"))?.active(),
			"unsubscribed mode culls padded"
		);
	}

	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<TestMode>());
	app.update();
	{
		let mut state = SystemState::<ModeSubscription<(Urbanized, UrbanizationHosts)>>::new(
			app.world_mut(),
		);
		anyhow::ensure!(
			state.get(app.world()).map_err(|error| anyhow::anyhow!("{error:?}"))?.active(),
			"return presents hosts again"
		);
	}
	Ok(())
}

#[test]
fn two_modes_install_the_core_once() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AssetPlugin::default(), StatesPlugin));
	app.add_plugins((
		GenerationModePlugin::<TestMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
		UrbanizationPresentationPlugin::<TestMode, Urbanized>::default(),
		UrbanizationPresentationPlugin::<OtherMode, Urbanized>::default(),
	));
	anyhow::ensure!(
		app.is_plugin_added::<crate::UrbanizationPresentationCore<Urbanized>>(),
		"core is installed"
	);
	let hosts = app.world().resource::<ModeSubscribers<(Urbanized, UrbanizationHosts)>>();
	anyhow::ensure!(hosts.contains::<TestMode>());
	anyhow::ensure!(hosts.contains::<OtherMode>());
	Ok(())
}

#[test]
fn hosts_walk_a_stored_development_with_no_hopscotch() -> anyhow::Result<()> {
	use durham_terrain_models::{
		fine_patch_cell_layout, BaseTerrainNoise, TerrainConfig, TerrainEntryStore,
		WorldBaseTerrain,
	};
	use richmond_development_models::{DevelopmentCell, DevelopmentConfig, DevelopmentEntryStore};
	use richmond_urbanization::UrbanizationIndex;
	use urbanization_layer_model::{
		urbanization_host_region, UrbanModel, UrbanizationLayerRegion,
	};

	let mut world = World::new();
	world.insert_resource(UrbanizationLayerRegion::default());
	let layout = fine_patch_cell_layout(2, bevy::math::IVec2::ZERO);
	world.insert_resource(layout.clone());
	world.insert_resource(TerrainEntryStore::default());
	world.insert_resource(WorldBaseTerrain(BaseTerrainNoise::from_config(
		&TerrainConfig::new(42),
	)));
	world.insert_resource(UrbanizationIndex::default());
	world.insert_resource(DevelopmentEntryStore::default());

	let cell = Aabb3d::from_min_max(Vec3::new(-20.0, 0.0, -20.0), Vec3::new(20.0, 1.0, 20.0));
	let config = DevelopmentConfig::from_world_seed(42);
	let filled = DevelopmentCell::with_les_halles(cell, 12.0, &config);
	let built = filled
		.built(config.seed as i32)
		.ok_or_else(|| anyhow::anyhow!("les halles built"))?;
	let id = Id::from_cell(filled.cell);
	{
		let mut store = world.resource_mut::<DevelopmentEntryStore>();
		store.insert_cell(id, filled);
		store.insert_built(id, built, cell);
	}

	let region = urbanization_host_region(&layout, None)
		.ok_or_else(|| anyhow::anyhow!("fine-patch host region"))?;
	let mut state = SystemState::<terrain_layer_model::TerrainView<Urbanized>>::new(&mut world);
	let view = state.get(&world).map_err(|error| anyhow::anyhow!("{error:?}"))?;
	let ids: Vec<_> = Urbanized::built_overlapping(&view.read, region)
		.into_iter()
		.map(|(id, _, _)| id)
		.collect();
	anyhow::ensure!(
		ids.contains(&id),
		"hosts walk a stored development with no hopscotch selection"
	);
	anyhow::ensure!(
		Urbanized::urbanization_leaves(&view.read, region).is_empty(),
		"no hopscotch cells are selected"
	);
	Ok(())
}
