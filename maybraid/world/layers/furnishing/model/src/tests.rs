use bevy::ecs::system::SystemParamItem;
use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::{
	App, IntoScheduleConfigs, MinimalPlugins, NextState, Plugin, ResMut, Resource, Transform,
	Update, World,
};
use bevy::state::app::StatesPlugin;
use layer_stack::{
	ActiveGenerationMode, GenerationMode, GenerationModePlugin, GenerationModeSystems, RequireLayer,
};
use lod::lod_ref::LodRef;
use terrain_layer_model::{
	BaseTerrainGenerationCore, HeightField, TerrainCell, TerrainGeneration, TerrainModel,
};

use crate::{
	Furnishing, FurnishingGeneration, FurnishingGenerationCore, FurnishingGenerationPlugin,
	FurnishingModel, FurnishingScheme,
};

struct TestMode;
struct OtherMode;

impl GenerationMode for TestMode {}
impl GenerationMode for OtherMode {}

struct StubRaw;

impl TerrainCell for StubRaw {
	type Mesh = ();
	fn bounds(&self) -> Aabb3d {
		Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE)
	}
	fn mesh_builder(&self) {}
	fn chunk_pose(&self) -> Transform {
		Transform::IDENTITY
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
	fn height_at(&self, _xz: Vec2) -> Option<f32> {
		None
	}
	fn fallback_height_at(&self, _xz: Vec2) -> f32 {
		0.0
	}
}

struct StubGround;

impl TerrainModel for StubGround {
	type Base = Self;
	type Cell = StubRaw;
	type Read = ();
	type Snapshot = GroundField;
	type Prepare = ();

	fn prepare(
		_prepare: &mut SystemParamItem<'_, '_, Self::Prepare>,
		_bounds: Aabb3d,
		_lod_ref: &LodRef,
	) {
	}

	fn height_at(_read: &SystemParamItem<'_, '_, Self::Read>, _xz: Vec2) -> Option<f32> {
		None
	}

	fn fallback_height_at(_read: &SystemParamItem<'_, '_, Self::Read>, _xz: Vec2) -> f32 {
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

	fn require_generation(app: &App) {
		app.require_layer::<BaseTerrainGenerationCore<Self>, Self>();
	}
}

impl TerrainGeneration for StubGround {
	type Config = ();
	fn install_generation(_app: &mut App) {}
	fn apply_generation(_world: &mut World, _config: &()) {}
}

struct StubCell;

struct StubFurnishing;

impl FurnishingModel for StubFurnishing {
	type Ground = StubGround;
	type Cell = StubCell;

	fn require_generation(app: &App) {
		app.require_layer::<FurnishingGenerationCore<Self>, Furnishing<Self>>();
	}
}

impl FurnishingGeneration for StubFurnishing {
	type Config = ();
	fn install_generation(_app: &mut App) {}
	fn apply_generation(_world: &mut World, _config: &()) {}
	fn clear_generation(_world: &mut World) {}
}

#[derive(Resource, Default)]
struct ModeTicks {
	test: u32,
	other: u32,
}

fn tick_test(mut ticks: ResMut<ModeTicks>) {
	ticks.test += 1;
}

fn tick_other(mut ticks: ResMut<ModeTicks>) {
	ticks.other += 1;
}

impl FurnishingScheme<StubFurnishing> for TestMode {
	fn install(app: &mut App, _config: &()) {
		app.add_systems(Update, tick_test.in_set(GenerationModeSystems::<TestMode>::default()));
	}
}

impl FurnishingScheme<StubFurnishing> for OtherMode {
	fn install(app: &mut App, _config: &()) {
		app.add_systems(Update, tick_other.in_set(GenerationModeSystems::<OtherMode>::default()));
	}
}

#[test]
fn generation_runs_only_in_subscribed_modes() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		StatesPlugin,
		GenerationModePlugin::<TestMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
		BaseTerrainGenerationCore::<StubGround>::default(),
		FurnishingGenerationPlugin::<TestMode, StubFurnishing>::default(),
		FurnishingGenerationPlugin::<OtherMode, StubFurnishing>::default(),
	));
	app.init_resource::<ModeTicks>();
	app.finish();
	app.update();
	anyhow::ensure!(app.world().resource::<ModeTicks>().test == 1, "subscribed mode generates");
	anyhow::ensure!(app.world().resource::<ModeTicks>().other == 0, "other mode stays idle");

	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<OtherMode>());
	app.update();
	anyhow::ensure!(app.world().resource::<ModeTicks>().test == 1, "left mode stops");
	anyhow::ensure!(app.world().resource::<ModeTicks>().other == 1, "entered mode generates");
	Ok(())
}

#[test]
#[should_panic(expected = "requires terrain_layer_model::generation::BaseTerrainGenerationCore")]
fn generation_finish_names_the_missing_ground() {
	FurnishingGenerationPlugin::<TestMode, StubFurnishing>::default().finish(&mut App::new());
}
