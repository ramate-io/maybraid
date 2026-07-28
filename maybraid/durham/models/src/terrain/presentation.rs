//! Basic [`RegionPresenter`] for terrain cells.
//!
//! Generation and presentation stay separate: present reads the entry store and
//! spawns each cell's [`lod::gen::LodScene`] via [`Commands::spawn_scene`].

use crate::terrain::cell::{universal_bounds, TerrainCellLayout};
use crate::terrain::config::TerrainConfig;
use crate::terrain::index::TerrainEntryStore;
use crate::terrain::Terrain;
use bevy::ecs::system::SystemParam;
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::prelude::*;
use bevy::scene::Scene;
use durham_terrain::shaders::DurhamTerrainShader;
use lod::gen::{GenerationScheme, Id, OriginalId, RegionPresenter, SpatialIndex, StorageStatus, TrackedId, Version};
use lod::lod_ref::LodRef;
use std::collections::{HashMap, HashSet};

/// Config / material / mesh resolution used when building terrain instances.
///
/// Materialized once under [`Id::Universal`] via [`GenerationScheme`].
///
/// Optional concentric rings (Chebyshev cell index from world origin):
/// - `radius ≤ fine_radius_cells` → [`Self::res_2`]
/// - `fine < radius ≤ mid_radius_cells` → [`Self::mid_res_2`] (when set)
/// - beyond that → [`Self::outer_res_2`] (when set)
#[derive(Resource, Clone)]
pub struct TerrainPresentationAssets {
	pub config: TerrainConfig,
	pub material: Handle<DurhamTerrainShader>,
	/// Mesh resolution (`2^res_2` samples) for cells inside [`Self::fine_radius_cells`].
	pub res_2: u8,
	/// Inclusive Chebyshev radius for the fine [`Self::res_2`] ring.
	pub fine_radius_cells: i32,
	/// Inclusive Chebyshev radius for the mid ring (must be ≥ fine when mid is used).
	pub mid_radius_cells: i32,
	/// Resolution for `fine_radius_cells < radius ≤ mid_radius_cells`. `None` skips the mid ring.
	pub mid_res_2: Option<u8>,
	/// Resolution beyond [`Self::mid_radius_cells`] (or beyond fine if mid is unset).
	/// `None` keeps the previous ring's resolution for the remainder.
	pub outer_res_2: Option<u8>,
	/// When true, mid and outer cells enable CpuShot edge height walls.
	pub outer_add_walls: bool,
}

impl TerrainPresentationAssets {
	/// `(res_2, add_walls)` for a terrain origin cell AABB.
	pub fn mesh_params_for_cell(&self, bounds: Aabb3d) -> (u8, bool) {
		let min = Vec3::from(bounds.min);
		let max = Vec3::from(bounds.max);
		let cell_size = (max.x - min.x).max(1e-3);
		let ix = (min.x / cell_size).floor() as i32;
		let iz = (min.z / cell_size).floor() as i32;
		let radius = ix.abs().max(iz.abs());
		if radius <= self.fine_radius_cells {
			return (self.res_2, false);
		}
		if let Some(mid) = self.mid_res_2 {
			if radius <= self.mid_radius_cells {
				return (mid, self.outer_add_walls);
			}
		}
		match self.outer_res_2 {
			Some(outer) => (outer, self.outer_add_walls),
			None => (self.mid_res_2.unwrap_or(self.res_2), self.outer_add_walls),
		}
	}
}

/// Bootstrap source used only when first materializing [`TerrainPresentationAssets`]
/// at [`Id::Universal`]. Consumers should depend on
/// [`lod::gen::GeneratingSpatialIndex`]`<TerrainPresentationAssets>` instead.
pub trait BootstrapTerrainPresentationAssets {
	fn bootstrap_terrain_presentation_assets(&self) -> TerrainPresentationAssets;
}

impl<S> GenerationScheme<S> for TerrainPresentationAssets
where
	S: BootstrapTerrainPresentationAssets,
{
	fn original_ids_for(_spatial_index: &mut S, _region: Aabb3d) -> Vec<OriginalId> {
		vec![OriginalId::universal()]
	}

	fn build_with_id(spatial_index: &mut S, id: Id, _lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		if id != Id::Universal {
			return None;
		}
		Some((
			spatial_index.bootstrap_terrain_presentation_assets(),
			universal_bounds(),
		))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}

/// Runtime presentation bookkeeping: last presented version and root entity per id.
#[derive(Resource, Default)]
pub struct TerrainPresenterState {
	presented: HashMap<Id, PresentedEntry>,
}

#[derive(Debug, Clone, Copy)]
struct PresentedEntry {
	version: Version,
	entity: Entity,
}

/// Marks a spawned terrain scene root as belonging to a presented id.
#[derive(Component, Debug, Clone, Copy)]
pub struct PresentedTerrainScene(pub Id);

impl TerrainPresenterState {
	pub fn clear(&mut self, commands: &mut Commands) {
		for entry in self.presented.values() {
			commands.entity(entry.entity).despawn();
		}
		self.presented.clear();
	}
}

/// Read-only spatial-index view over the terrain entry map for presentation.
///
/// Insert is unsupported; generate through [`crate::terrain::AvianTerrainIndex`] first.
pub struct TerrainStoreView<'a> {
	store: &'a TerrainEntryStore,
	_layout: &'a TerrainCellLayout,
}

impl<'a> TerrainStoreView<'a> {
	pub fn new(store: &'a TerrainEntryStore, layout: &'a TerrainCellLayout) -> Self {
		Self { store, _layout: layout }
	}
}

impl SpatialIndex<Terrain> for TerrainStoreView<'_> {
	fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId> {
		self.store
			.terrain
			.iter()
			.filter(|(_, entry)| region.intersects(&entry.bounds))
			.map(|(id, _)| TrackedId(*id))
			.collect()
	}

	fn storage_status(&self, id: Id) -> StorageStatus {
		if self.store.terrain.contains_key(&id) {
			StorageStatus::TrackedWithin
		} else {
			StorageStatus::NotTracked
		}
	}

	fn get(&self, id: Id) -> Option<&Terrain> {
		self.store.terrain.get(&id).map(|e| &e.value)
	}

	fn get_bounds(&self, id: Id) -> Option<Aabb3d> {
		self.store.terrain.get(&id).map(|e| e.bounds)
	}

	fn version(&self, id: Id) -> Option<Version> {
		self.store.terrain.get(&id).map(|e| e.version)
	}

	fn insert(&mut self, _id: Id, _t: Terrain, _bounds: Aabb3d, _lod_ref: &LodRef) {
		panic!("TerrainStoreView is read-only; insert via AvianTerrainIndex");
	}
}

/// System-local presenter: spawns terrain `bsn!` scenes and tracks versions.
#[derive(SystemParam)]
pub struct TerrainRegionPresenter<'w, 's> {
	commands: Commands<'w, 's>,
	state: ResMut<'w, TerrainPresenterState>,
}

impl<'w, 's> TerrainRegionPresenter<'w, 's> {
	pub fn clear_presented(&mut self) {
		self.state.clear(&mut self.commands);
	}
}

impl<'a, 'w, 's> RegionPresenter<Terrain, TerrainStoreView<'a>> for TerrainRegionPresenter<'w, 's> {
	fn presented_version(&self, id: Id) -> Option<Version> {
		self.state.presented.get(&id).map(|e| e.version)
	}

	fn handle(&mut self, id: Id, version: Version, scene: impl Scene, _lod_ref: &LodRef) {
		if let Some(previous) = self.state.presented.remove(&id) {
			self.commands.entity(previous.entity).despawn();
		}
		let entity = self
			.commands
			.spawn_scene(scene)
			.insert(PresentedTerrainScene(id))
			.id();
		self.state.presented.insert(id, PresentedEntry { version, entity });
	}

	fn remove_stale(&mut self, _region: Aabb3d, wanted: &HashSet<Id>) {
		let stale: Vec<(Id, Entity)> = self
			.state
			.presented
			.iter()
			.filter(|(id, _)| !wanted.contains(id))
			.map(|(id, entry)| (*id, entry.entity))
			.collect();

		for (id, entity) in stale {
			self.commands.entity(entity).despawn();
			self.state.presented.remove(&id);
		}
	}
}
