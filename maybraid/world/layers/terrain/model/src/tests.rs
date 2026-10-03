use std::collections::HashMap;

use bevy::app::App;
use bevy::ecs::system::{Res, SystemParam, SystemParamItem, SystemState};
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::math::{Vec2, Vec3};
use bevy::prelude::{Resource, World};
use bevy::transform::components::Transform;
use lod::gen::Id;
use lod::lod_ref::LodRef;

use crate::{
	BaseTerrainGenerationCore, BaseTerrainGenerationPlugin, BaseTerrainScheme, HeightField,
	OnTerrain, TerrainCell, TerrainGeneration, TerrainModel, TerrainView,
};
use layer_stack::{GenerationMode, GenerationModePlugin, RequireLayer};

/// Flat test model: stored cells carry a constant height; fallback is configured.
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

impl FlatStore {
	fn insert(&mut self, min: Vec2, size: f32, height: f32) -> Id {
		let bounds = Aabb3d::from_min_max(
			Vec3::new(min.x, -1.0, min.y),
			Vec3::new(min.x + size, 1.0, min.y + size),
		);
		let id = Id::from_cell(bounds);
		self.cells.insert(id, FlatCell { bounds, height });
		id
	}
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
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		bounds: Aabb3d,
		target_size: f32,
		_overlay_size_tolerance: Option<f32>,
	) -> Option<&'a dyn TerrainCell<Mesh = f32>> {
		let mut best: Option<(f32, &'a FlatCell)> = None;
		for cell in read.store.cells.values() {
			let size = (cell.bounds.max.x - cell.bounds.min.x).max(1e-3);
			if (size - target_size).abs() > target_size * 0.25 || !bounds.intersects(&cell.bounds) {
				continue;
			}
			let overlap = (bounds.max.x.min(cell.bounds.max.x)
				- bounds.min.x.max(cell.bounds.min.x))
			.max(0.0) * (bounds.max.z.min(cell.bounds.max.z)
				- bounds.min.z.max(cell.bounds.min.z))
			.max(0.0);
			if overlap <= 1e-3 {
				continue;
			}
			if best.is_none_or(|(best_overlap, _)| overlap > best_overlap) {
				best = Some((overlap, cell));
			}
		}
		best.map(|(_, cell)| cell as _)
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
		app.require_layer::<BaseTerrainGenerationCore<Flat>, Flat>();
	}
}

impl TerrainGeneration for Flat {
	type Config = f32;

	fn install_generation(app: &mut App) {
		app.init_resource::<FlatStore>();
	}

	fn apply_generation(world: &mut World, config: &f32) {
		world.resource_mut::<FlatStore>().fallback = *config;
	}
}

struct TestMode;

impl GenerationMode for TestMode {}

impl BaseTerrainScheme<Flat> for TestMode {
	fn install(_app: &mut App, _config: &f32) {}
}

fn world_with_one_cell() -> (World, Id) {
	let mut world = World::new();
	let mut store = FlatStore { fallback: -3.0, ..FlatStore::default() };
	let id = store.insert(Vec2::ZERO, 10.0, 7.0);
	world.insert_resource(store);
	(world, id)
}

#[test]
fn view_reads_stored_cells_and_opts_into_fallback() -> anyhow::Result<()> {
	let (mut world, _id) = world_with_one_cell();
	let mut state = SystemState::<TerrainView<Flat>>::new(&mut world);
	let view = state.get(&world)?;

	assert_eq!(view.height_at(Vec2::new(5.0, 5.0)), Some(7.0));
	assert_eq!(view.height_at(Vec2::new(50.0, 5.0)), None);
	assert_eq!(view.height_or_fallback(Vec2::new(50.0, 5.0)), -3.0);
	let region = Aabb3d::from_min_max(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
	assert!(view.overlay_cell(region, 10.0, None).is_some());
	assert_eq!(view.snapshot(region).height_at(Vec2::new(1.0, 1.0)), Some(7.0));
	Ok(())
}

#[test]
fn on_terrain_is_transparent() -> anyhow::Result<()> {
	let (mut world, _id) = world_with_one_cell();
	let mut state = SystemState::<TerrainView<OnTerrain<Flat>>>::new(&mut world);
	let view = state.get(&world)?;

	assert_eq!(view.height_at(Vec2::new(5.0, 5.0)), Some(7.0));
	assert_eq!(view.height_or_fallback(Vec2::new(50.0, 5.0)), -3.0);
	let region = Aabb3d::from_min_max(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
	assert!(view.overlay_cell(region, 10.0, None).is_some());
	Ok(())
}

#[test]
fn base_generation_installs_model() {
	let mut app = App::new();
	app.add_plugins((
		GenerationModePlugin::<TestMode>::initial(),
		BaseTerrainGenerationPlugin::<TestMode, Flat>::new(2.5),
	));
	app.finish();
	app.update();

	assert_eq!(app.world().get_resource::<FlatStore>().map(|store| store.fallback), Some(2.5));
}
