//! Durham as a [`TerrainModel`]: GET-only reads over [`TerrainEntryStore`].

use bevy::ecs::system::{SystemParam, SystemParamItem};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use durham_terrain::shaders::DurhamTerrainShader;
use lod::gen::Id;
use terrain_layer_model::{
	BaseTerrainGenerationPlugin, HeightField, RequireLayer, TerrainCell, TerrainGeneration,
	TerrainModel,
};

use crate::terrain::cell::TerrainCellLayout;
use crate::terrain::host::{
	Durham, TerrainCoverage, TerrainPlugin, WorldBaseTerrain, WORLD_FINE_HALF_EXTENT_CELLS,
};
use crate::terrain::index::{TerrainEntryStore, TerrainHeightSnapshot};
use crate::terrain::{Terrain, TerrainMeshBuilder};

/// Resources behind [`Durham`]'s [`TerrainModel::Read`].
#[derive(SystemParam)]
pub struct DurhamRead<'w> {
	store: Res<'w, TerrainEntryStore>,
	layout: Res<'w, TerrainCellLayout>,
	base: Res<'w, WorldBaseTerrain>,
}

/// Owned composed heights for grove grow. Shares cell SDFs by `Arc`.
#[derive(Clone)]
pub struct DurhamHeightSnapshot {
	terrain: TerrainHeightSnapshot,
	layout: TerrainCellLayout,
}

impl HeightField for DurhamHeightSnapshot {
	fn height_at(&self, xz: Vec2) -> Option<f32> {
		self.terrain.composed_height_at(&self.layout, xz.x, xz.y)
	}
}

impl TerrainCell for Terrain {
	type Mesh = TerrainMeshBuilder;
	type Material = DurhamTerrainShader;

	fn bounds(&self) -> Aabb3d {
		self.cell
	}

	fn mesh_builder(&self) -> TerrainMeshBuilder {
		Terrain::mesh_builder(self)
	}

	fn material(&self) -> Handle<DurhamTerrainShader> {
		self.material.clone()
	}

	fn chunk_pose(&self) -> Transform {
		Terrain::chunk_pose(self)
	}

	fn seeds_collision(&self) -> bool {
		Terrain::seeds_collision(self)
	}
}

impl TerrainModel for Durham {
	type Cell = Terrain;
	type Read = DurhamRead<'static>;
	type Snapshot = DurhamHeightSnapshot;

	fn height_at(read: &SystemParamItem<'_, '_, Self::Read>, xz: Vec2) -> Option<f32> {
		read.store.composed_height_at(&read.layout, xz.x, xz.y)
	}

	fn fallback_height_at(read: &SystemParamItem<'_, '_, Self::Read>, xz: Vec2) -> f32 {
		read.base.0.height_at(xz.x, xz.y)
	}

	fn cell_ids_overlapping(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> Vec<Id> {
		read.store.terrain_ids_overlapping(region)
	}

	fn cell<'a>(read: &'a SystemParamItem<'_, '_, Self::Read>, id: Id) -> Option<&'a Terrain> {
		read.store.terrain(id)
	}

	/// Whole-store snapshot (cheap `Arc` clones), matching today's grove grow.
	fn snapshot(read: &SystemParamItem<'_, '_, Self::Read>, _region: Aabb3d) -> Self::Snapshot {
		DurhamHeightSnapshot { terrain: read.store.height_snapshot(), layout: read.layout.clone() }
	}

	fn require_generation(app: &App) {
		app.require_layer::<BaseTerrainGenerationPlugin<Durham>, Durham>();
	}
}

/// Seed and coverage for `BaseTerrainGenerationPlugin<Durham>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DurhamTerrainConfig {
	pub seed: u32,
	pub coverage: TerrainCoverage,
	pub terrain_radius: i32,
}

impl DurhamTerrainConfig {
	pub fn playable_world() -> Self {
		Self {
			seed: 42,
			coverage: TerrainCoverage::PlayableWorld,
			terrain_radius: WORLD_FINE_HALF_EXTENT_CELLS,
		}
	}

	pub fn fine_patch(terrain_radius: i32) -> Self {
		Self {
			seed: 42,
			coverage: TerrainCoverage::FinePatch,
			terrain_radius: terrain_radius.max(1),
		}
	}
}

impl TerrainGeneration for Durham {
	type Config = DurhamTerrainConfig;

	/// Still the full [`TerrainPlugin`] with raw present off. Splitting its
	/// generate half out is the terrain layer issue; callers already see the
	/// final plugin type.
	fn install_generation(app: &mut App, config: &DurhamTerrainConfig) {
		let mut plugin = TerrainPlugin::<Durham>::fine_patch(config.terrain_radius);
		plugin.seed = config.seed;
		plugin.coverage = config.coverage;
		plugin.present = false;
		app.add_plugins(plugin);
	}
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::SystemState;
	use terrain_layer_model::{OnTerrain, TerrainView};

	use super::*;
	use crate::terrain::base_noise::BaseTerrainNoise;
	use crate::terrain::config::TerrainConfig;

	fn empty_durham_world() -> World {
		let mut world = World::new();
		world.insert_resource(TerrainEntryStore::default());
		world.insert_resource(TerrainCellLayout::default());
		world.insert_resource(WorldBaseTerrain(BaseTerrainNoise::from_config(
			&TerrainConfig::new(42),
		)));
		world
	}

	#[test]
	fn missing_cells_are_none_and_fallback_is_base_noise() -> anyhow::Result<()> {
		let mut world = empty_durham_world();
		let base = BaseTerrainNoise::from_config(&TerrainConfig::new(42)).height_at(12.0, -7.0);
		let mut state = SystemState::<TerrainView<OnTerrain<Durham>>>::new(&mut world);
		let view = state.get(&world)?;

		assert_eq!(view.height_at(Vec2::new(12.0, -7.0)), None);
		assert_eq!(view.height_or_fallback(Vec2::new(12.0, -7.0)), base);
		assert!(view
			.cell_ids_overlapping(Aabb3d::new(Vec3::ZERO, Vec3::splat(1_000.0)))
			.is_empty());
		Ok(())
	}
}
