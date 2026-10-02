//! Durham as a [`TerrainModel`]: GET-only reads over [`TerrainEntryStore`].

use bevy::ecs::system::{SystemParam, SystemParamItem};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::Id;
use lod::lod_ref::LodRef;
use terrain_layer_model::{BaseTerrainGenerationCore, HeightField, TerrainCell, TerrainGeneration, TerrainModel};
use layer_stack::{RequireLayer};

use crate::terrain::cell::TerrainCellLayout;
use crate::terrain::host::{
	install_durham_generation, Durham, TerrainCoverage, WorldBaseTerrain,
	WORLD_FINE_HALF_EXTENT_CELLS,
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
///
/// `fallback` is the base-noise clone grove samples use where no cell is stored
/// (the old owned Durham grove sample carried the same clone).
#[derive(Clone)]
pub struct DurhamHeightSnapshot {
	terrain: TerrainHeightSnapshot,
	layout: TerrainCellLayout,
	fallback: crate::terrain::base_noise::BaseTerrainNoise,
}

impl HeightField for DurhamHeightSnapshot {
	fn height_at(&self, xz: Vec2) -> Option<f32> {
		self.terrain.composed_height_at(&self.layout, xz.x, xz.y)
	}

	fn fallback_height_at(&self, xz: Vec2) -> f32 {
		self.fallback.height_at(xz.x, xz.y)
	}
}

impl TerrainCell for Terrain {
	type Mesh = TerrainMeshBuilder;

	fn bounds(&self) -> Aabb3d {
		self.cell
	}

	fn mesh_builder(&self) -> TerrainMeshBuilder {
		Terrain::mesh_builder(self)
	}

	fn chunk_pose(&self) -> Transform {
		Terrain::chunk_pose(self)
	}

	fn seeds_collision(&self) -> bool {
		Terrain::seeds_collision(self)
	}

	fn res_2(&self) -> u8 {
		self.res_2
	}
}

impl TerrainModel for Durham {
	type Cell = Terrain;
	type Read = DurhamRead<'static>;
	type Snapshot = DurhamHeightSnapshot;
	type Prepare = ();

	fn prepare(
		_prepare: &mut SystemParamItem<'_, '_, Self::Prepare>,
		_bounds: Aabb3d,
		_lod_ref: &LodRef,
	) {
	}

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

	/// Best-sized raw cell. Durham has no padded replacement, so
	/// `overlay_size_tolerance` is unused.
	fn overlay_cell<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		bounds: Aabb3d,
		target_size: f32,
		_overlay_size_tolerance: Option<f32>,
	) -> Option<&'a dyn TerrainCell<Mesh = TerrainMeshBuilder>> {
		raw_cell_for_size(read, bounds, target_size).map(|terrain| terrain as _)
	}

	/// Whole-store snapshot (cheap `Arc` clones), matching today's grove grow.
	fn snapshot(read: &SystemParamItem<'_, '_, Self::Read>, _region: Aabb3d) -> Self::Snapshot {
		DurhamHeightSnapshot {
			terrain: read.store.height_snapshot(),
			layout: read.layout.clone(),
			fallback: read.base.0.clone(),
		}
	}

	fn require_generation(app: &App) {
		app.require_layer::<BaseTerrainGenerationCore<Durham>, Durham>();
	}
}

/// Width band of the old `fine_terrain_for` / `medium_terrain_for` helpers.
const RAW_CELL_SIZE_BAND: f32 = 0.25;

fn raw_cell_for_size<'a>(
	read: &'a SystemParamItem<'_, '_, DurhamRead<'static>>,
	bounds: Aabb3d,
	target_size: f32,
) -> Option<&'a Terrain> {
	let mut best: Option<(f32, &'a Terrain)> = None;
	for id in read.store.terrain_ids_overlapping(bounds) {
		let Some(terrain) = read.store.terrain(id) else {
			continue;
		};
		let cell = terrain.bounds();
		let size = (cell.max.x - cell.min.x).max(1e-3);
		if (size - target_size).abs() > target_size * RAW_CELL_SIZE_BAND {
			continue;
		}
		let overlap = xz_overlap_area(bounds, cell);
		if overlap <= 1e-3 {
			continue;
		}
		if best.is_none_or(|(best_overlap, _)| overlap > best_overlap) {
			best = Some((overlap, terrain));
		}
	}
	best.map(|(_, terrain)| terrain)
}

fn xz_overlap_area(a: Aabb3d, b: Aabb3d) -> f32 {
	let x = (a.max.x.min(b.max.x) - a.min.x.max(b.min.x)).max(0.0);
	let z = (a.max.z.min(b.max.z) - a.min.z.max(b.min.z)).max(0.0);
	x * z
}

/// Seed is shared across modes; coverage and radius are per mode.
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
	type SharedConfig = u32;

	fn shared_config(config: &Self::Config) -> u32 {
		config.seed
	}

	fn install_generation(app: &mut App, config: &DurhamTerrainConfig) {
		install_durham_generation(app, config.seed, config.coverage, config.terrain_radius);
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

	#[test]
	fn overlay_cell_picks_the_best_sized_raw_cell() -> anyhow::Result<()> {
		use crate::terrain::TERRAIN_CELL_SIZE;

		let mut world = empty_durham_world();
		let base = BaseTerrainNoise::from_config(&TerrainConfig::new(42));
		let fine = TerrainCellLayout::default();
		let medium = TerrainCellLayout {
			cell_size: 2.0 * TERRAIN_CELL_SIZE,
			..TerrainCellLayout::default()
		};
		{
			let mut store = world.resource_mut::<TerrainEntryStore>();
			store.insert_base_terrain_for_test(&fine, 0, 0, base.clone());
			store.insert_base_terrain_for_test(&medium, 0, 0, base);
		}
		let query = Aabb3d::from_min_max(Vec3::new(1.0, -10.0, 1.0), Vec3::new(20.0, 10.0, 20.0));
		let mut state = SystemState::<TerrainView<OnTerrain<Durham>>>::new(&mut world);
		let view = state.get(&world)?;
		let fine_cell = view
			.overlay_cell(query, TERRAIN_CELL_SIZE, None)
			.ok_or_else(|| anyhow::anyhow!("fine cell"))?;
		let medium_cell = view
			.overlay_cell(query, 2.0 * TERRAIN_CELL_SIZE, Some(1e-2))
			.ok_or_else(|| anyhow::anyhow!("medium cell"))?;
		let fine_width = fine_cell.bounds().max.x - fine_cell.bounds().min.x;
		let medium_width = medium_cell.bounds().max.x - medium_cell.bounds().min.x;
		assert!((fine_width - TERRAIN_CELL_SIZE).abs() < 1e-3);
		assert!((medium_width - 2.0 * TERRAIN_CELL_SIZE).abs() < 1e-3);
		Ok(())
	}
}
