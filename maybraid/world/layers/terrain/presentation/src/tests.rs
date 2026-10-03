use std::collections::HashMap;

use bevy::app::App;
use bevy::ecs::system::{Res, SystemParam, SystemParamItem};
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::math::{Vec2, Vec3};
use bevy::prelude::{Local, MinimalPlugins, NextState, ResMut, Resource, Update, World};
use bevy::state::app::StatesPlugin;
use bevy::transform::components::Transform;
use layer_stack::{
	subscribe_mode, ActiveGenerationMode, Generate, GenerationMode, GenerationModePlugin,
	LayerGenerationCore, ModeSubscription, Present, RequireLayer, Scheme,
};
use lod::gen::Id;
use lod::lod_ref::LodRef;
use terrain_layer_model::{
	HeightField, OnTerrain, TerrainCell, TerrainGeneration, TerrainModel,
};

use crate::TerrainPresenter;

struct Flat;

#[derive(Clone)]
struct FlatCell {
	bounds: Aabb3d,
	height: f32,
}

impl FlatCell {
	fn contains_xz(&self, xz: Vec2) -> bool {
		xz.x >= self.bounds.min.x
			&& xz.x < self.bounds.max.x
			&& xz.y >= self.bounds.min.z
			&& xz.y < self.bounds.max.z
	}
}

impl TerrainCell for FlatCell {
	type Mesh = f32;

	fn bounds(&self) -> Aabb3d {
		self.bounds
	}

	fn mesh_builder(&self) -> f32 {
		self.height
	}

	fn chunk_pose(&self) -> Transform {
		Transform::from_translation(Vec3::from(self.bounds.min))
	}

	fn seeds_collision(&self) -> bool {
		true
	}

	fn res_2(&self) -> u8 {
		0
	}
}

#[derive(Resource, Default)]
struct FlatStore {
	cells: HashMap<Id, FlatCell>,
	fallback: f32,
}

#[derive(Clone)]
struct FlatSnapshot {
	cells: Vec<FlatCell>,
	fallback: f32,
}

impl HeightField for FlatSnapshot {
	fn height_at(&self, xz: Vec2) -> Option<f32> {
		self.cells.iter().find(|cell| cell.contains_xz(xz)).map(|cell| cell.height)
	}

	fn fallback_height_at(&self, _xz: Vec2) -> f32 {
		self.fallback
	}
}

#[derive(SystemParam)]
struct FlatRead<'w> {
	store: Res<'w, FlatStore>,
}

impl TerrainModel for Flat {
	type Base = Self;
	type Cell = FlatCell;
	type Read = FlatRead<'static>;
	type Snapshot = FlatSnapshot;
	type Prepare = ();

	fn prepare(
		_prepare: &mut SystemParamItem<'_, '_, Self::Prepare>,
		_bounds: Aabb3d,
		_lod_ref: &LodRef,
	) {
	}

	fn height_at(read: &SystemParamItem<'_, '_, Self::Read>, xz: Vec2) -> Option<f32> {
		read.store
			.cells
			.values()
			.find(|cell| cell.contains_xz(xz))
			.map(|cell| cell.height)
	}

	fn fallback_height_at(read: &SystemParamItem<'_, '_, Self::Read>, _xz: Vec2) -> f32 {
		read.store.fallback
	}

	fn overlay_cell<'a>(
		_read: &'a SystemParamItem<'_, '_, Self::Read>,
		_bounds: Aabb3d,
		_target_size: f32,
		_overlay_size_tolerance: Option<f32>,
	) -> Option<&'a dyn TerrainCell<Mesh = f32>> {
		None
	}

	fn snapshot(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> FlatSnapshot {
		FlatSnapshot {
			cells: read
				.store
				.cells
				.values()
				.filter(|cell| region.intersects(&cell.bounds))
				.cloned()
				.collect(),
			fallback: read.store.fallback,
		}
	}

	fn require_generation(app: &App) {
		app.require_layer::<LayerGenerationCore<OnTerrain<Flat>>, Flat>();
	}
}

impl TerrainGeneration for Flat {
	const LABEL: &'static str = "stub";
	type Config = f32;

	fn install_generation(app: &mut App) {
		app.init_resource::<FlatStore>();
	}

	fn apply_generation(world: &mut World, config: &f32) {
		world.resource_mut::<FlatStore>().fallback = *config;
	}
	fn install_presentation(app: &mut App) {
		FlatPresenter::install(app);
	}
}

struct TestMode;

impl GenerationMode for TestMode {}

impl Scheme<OnTerrain<Flat>> for TestMode {
	fn install(_app: &mut App, _config: &f32) {}
}

#[derive(Resource)]
struct FlatPresentInstalled;

#[derive(Resource, Default)]
struct InstallCount(u32);

struct FlatPresenter;

impl TerrainPresenter<OnTerrain<Flat>> for FlatPresenter {
	fn install(app: &mut App) {
		{
			let mut count = app.world_mut().get_resource_or_insert_with(InstallCount::default);
			count.0 += 1;
		}
		app.insert_resource(FlatPresentInstalled);
	}
}

struct OtherMode;

impl GenerationMode for OtherMode {}

#[test]
fn presenter_installs_and_finish_requires_generation() {
	let mut app = App::new();
	app.add_plugins((
		GenerationModePlugin::<TestMode>::initial(),
		Generate::<TestMode, OnTerrain<Flat>>::new(2.5),
	))
	.add_plugins(Present::<TestMode, OnTerrain<Flat>>::default());
	app.finish();
	app.update();

	assert_eq!(app.world().get_resource::<FlatStore>().map(|store| store.fallback), Some(2.5));
	assert!(app.world().contains_resource::<FlatPresentInstalled>());
}

#[test]
#[should_panic(expected = "LayerGenerationCore")]
fn presentation_without_generation_names_the_missing_plugin() {
	let mut app = App::new();
	app.add_plugins(
		Present::<TestMode, OnTerrain<Flat>>::default(),
	);
	app.finish();
}

#[test]
fn two_modes_install_the_core_once() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins((
		GenerationModePlugin::<TestMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
		Generate::<TestMode, OnTerrain<Flat>>::new(2.5),
		Present::<TestMode, OnTerrain<Flat>>::default(),
		Present::<OtherMode, OnTerrain<Flat>>::default(),
	));
	app.finish();
	let count = app.world().resource::<InstallCount>().0;
	anyhow::ensure!(count == 1, "core install ran {count} times");
	Ok(())
}

#[derive(Resource)]
struct Shown(bool);

fn sync_shown(
	subscription: ModeSubscription<OnTerrain<Flat>>,
	mut was: Local<bool>,
	mut shown: ResMut<Shown>,
) {
	if *was && !subscription.active() {
		shown.0 = false;
	} else if subscription.active() {
		shown.0 = true;
	}
	*was = subscription.active();
}

#[test]
fn losing_subscription_clears_and_returning_presents() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		StatesPlugin,
		GenerationModePlugin::<TestMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
	));
	subscribe_mode::<OnTerrain<Flat>, TestMode>(&mut app);
	app.insert_resource(Shown(false));
	app.add_systems(Update, sync_shown);
	app.update();
	anyhow::ensure!(app.world().resource::<Shown>().0, "subscribed mode presents");

	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<OtherMode>());
	app.update();
	anyhow::ensure!(!app.world().resource::<Shown>().0, "unsubscribed mode retires");

	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<TestMode>());
	app.update();
	anyhow::ensure!(app.world().resource::<Shown>().0, "return presents again");
	Ok(())
}
