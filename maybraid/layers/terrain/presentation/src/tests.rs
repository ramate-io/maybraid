use std::collections::HashMap;

use bevy::app::App;
use bevy::ecs::system::{Res, SystemParam, SystemParamItem};
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::math::{Vec2, Vec3};
use bevy::prelude::Resource;
use bevy::transform::components::Transform;
use lod::gen::Id;
use terrain_layer_model::{
	BaseTerrainGenerationPlugin, HeightField, OnTerrain, RequireLayer, TerrainCell,
	TerrainGeneration, TerrainModel,
};

use crate::{TerrainPresentationPlugin, TerrainPresenter};

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
}

#[derive(Resource, Default)]
struct FlatStore {
	cells: HashMap<Id, FlatCell>,
	fallback: f32,
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

struct FlatPresenter;

impl TerrainPresenter<OnTerrain<Flat>> for FlatPresenter {
	fn install(app: &mut App) {
		app.insert_resource(FlatPresentInstalled);
	}
}

#[test]
fn presenter_installs_and_finish_requires_generation() {
	let mut app = App::new();
	app.add_plugins(BaseTerrainGenerationPlugin::<Flat>::new(2.5))
		.add_plugins(TerrainPresentationPlugin::<OnTerrain<Flat>, FlatPresenter>::default());
	app.finish();

	assert_eq!(app.world().get_resource::<FlatStore>().map(|store| store.fallback), Some(2.5));
	assert!(app.world().contains_resource::<FlatPresentInstalled>());
}

#[test]
#[should_panic(expected = "requires terrain_layer_model::generation::BaseTerrainGenerationPlugin")]
fn presentation_without_generation_names_the_missing_plugin() {
	let mut app = App::new();
	app.add_plugins(TerrainPresentationPlugin::<OnTerrain<Flat>, FlatPresenter>::default());
	app.finish();
}
