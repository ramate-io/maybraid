use std::collections::HashMap;

use bevy::app::{App, Plugin};
use bevy::asset::Handle;
use bevy::ecs::system::{Res, SystemParam, SystemParamItem, SystemState};
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::math::{Vec2, Vec3};
use bevy::pbr::StandardMaterial;
use bevy::prelude::{Resource, World};
use bevy::transform::components::Transform;
use lod::gen::Id;

use crate::{
	BaseTerrainGenerationPlugin, HeightField, OnTerrain, RequireLayer, TerrainCell,
	TerrainGeneration, TerrainModel, TerrainPresentation, TerrainView,
};

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
	type Material = StandardMaterial;

	fn bounds(&self) -> Aabb3d {
		self.bounds
	}

	fn mesh_builder(&self) -> f32 {
		self.height
	}

	fn material(&self) -> Handle<StandardMaterial> {
		Handle::default()
	}

	fn chunk_pose(&self) -> Transform {
		Transform::from_translation(Vec3::from(self.bounds.min))
	}

	fn seeds_collision(&self) -> bool {
		true
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
struct FlatSnapshot(Vec<FlatCell>);

impl HeightField for FlatSnapshot {
	fn height_at(&self, xz: Vec2) -> Option<f32> {
		self.0.iter().find(|cell| cell.contains_xz(xz)).map(|cell| cell.height)
	}
}

#[derive(SystemParam)]
struct FlatRead<'w> {
	store: Res<'w, FlatStore>,
}

impl TerrainModel for Flat {
	type Cell = FlatCell;
	type Read = FlatRead<'static>;
	type Snapshot = FlatSnapshot;

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

	fn cell_ids_overlapping(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> Vec<Id> {
		read.store
			.cells
			.iter()
			.filter(|(_, cell)| region.intersects(&cell.bounds))
			.map(|(id, _)| *id)
			.collect()
	}

	fn cell<'a>(read: &'a SystemParamItem<'_, '_, Self::Read>, id: Id) -> Option<&'a FlatCell> {
		read.store.cells.get(&id)
	}

	fn snapshot(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> FlatSnapshot {
		FlatSnapshot(
			read.store
				.cells
				.values()
				.filter(|cell| region.intersects(&cell.bounds))
				.cloned()
				.collect(),
		)
	}

	fn require_generation(app: &App) {
		app.require_layer::<BaseTerrainGenerationPlugin<Flat>, Flat>();
	}
}

impl TerrainGeneration for Flat {
	type Config = f32;

	fn install_generation(app: &mut App, config: &f32) {
		app.insert_resource(FlatStore { cells: HashMap::new(), fallback: *config });
	}
}

#[derive(Resource)]
struct FlatPresentInstalled;

impl TerrainPresentation for Flat {
	fn install_presentation(app: &mut App) {
		app.insert_resource(FlatPresentInstalled);
	}
}

/// Stand-in presentation plugin that checks its stack like real layers do.
struct FlatPresentationPlugin<M>(std::marker::PhantomData<fn() -> M>);

impl<M: TerrainPresentation> Plugin for FlatPresentationPlugin<M> {
	fn build(&self, app: &mut App) {
		M::install_presentation(app);
	}

	fn finish(&self, app: &mut App) {
		M::require_generation(app);
	}
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
	let (mut world, id) = world_with_one_cell();
	let mut state = SystemState::<TerrainView<Flat>>::new(&mut world);
	let view = state.get(&world)?;

	assert_eq!(view.height_at(Vec2::new(5.0, 5.0)), Some(7.0));
	assert_eq!(view.height_at(Vec2::new(50.0, 5.0)), None);
	assert_eq!(view.height_or_fallback(Vec2::new(50.0, 5.0)), -3.0);
	assert_eq!(view.cell(id).map(TerrainCell::mesh_builder), Some(7.0));

	let region = Aabb3d::from_min_max(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
	assert_eq!(view.cell_ids_overlapping(region), vec![id]);
	assert_eq!(view.snapshot(region).height_at(Vec2::new(1.0, 1.0)), Some(7.0));
	Ok(())
}

#[test]
fn on_terrain_is_transparent() -> anyhow::Result<()> {
	let (mut world, id) = world_with_one_cell();
	let mut state = SystemState::<TerrainView<OnTerrain<Flat>>>::new(&mut world);
	let view = state.get(&world)?;

	assert_eq!(view.height_at(Vec2::new(5.0, 5.0)), Some(7.0));
	assert_eq!(view.height_or_fallback(Vec2::new(50.0, 5.0)), -3.0);
	assert!(view.cell(id).is_some());
	Ok(())
}

#[test]
fn base_generation_installs_model_and_satisfies_requirements() {
	let mut app = App::new();
	app.add_plugins(BaseTerrainGenerationPlugin::<Flat>::new(2.5))
		.add_plugins(FlatPresentationPlugin::<OnTerrain<Flat>>(std::marker::PhantomData));
	app.finish();

	assert_eq!(app.world().get_resource::<FlatStore>().map(|store| store.fallback), Some(2.5));
	assert!(app.world().contains_resource::<FlatPresentInstalled>());
}

#[test]
#[should_panic(expected = "requires terrain_layer_model::generation::BaseTerrainGenerationPlugin")]
fn presentation_without_generation_names_the_missing_plugin() {
	let mut app = App::new();
	app.add_plugins(FlatPresentationPlugin::<OnTerrain<Flat>>(std::marker::PhantomData));
	app.finish();
}
