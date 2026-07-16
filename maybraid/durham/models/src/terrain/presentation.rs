//! Basic [`RegionPresenter`] for terrain cells.
//!
//! Generation and presentation stay separate: present reads the entry store and
//! spawns each cell's [`lod::gen::LodScene`] via [`Commands::spawn_scene`].

use crate::terrain::cell::TerrainCellLayout;
use crate::terrain::index::TerrainEntryStore;
use crate::terrain::sdf::ComposedTerrain;
use crate::terrain::Terrain;
use bevy::ecs::system::SystemParam;
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::prelude::*;
use bevy::scene::Scene;
use durham_terrain::shaders::DurhamTerrainShader;
use lod::gen::{Id, RegionPresenter, SpatialIndex, StorageStatus, TrackedId, Version};
use lod::lod_ref::LodRef;
use std::collections::{HashMap, HashSet};

/// Shared SDF / material / mesh resolution used when building terrain instances.
#[derive(Resource, Clone)]
pub struct TerrainPresentationAssets {
	pub sdf: ComposedTerrain,
	pub material: Handle<DurhamTerrainShader>,
	pub res_2: u8,
}

/// Types that can supply [`TerrainPresentationAssets`] during generation.
pub trait HasTerrainPresentationAssets {
	fn presentation_assets(&self) -> &TerrainPresentationAssets;
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

/// Read-only spatial-index view over the entry store for presentation.
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
			.entries
			.iter()
			.filter(|(_, entry)| region.intersects(&entry.bounds))
			.map(|(id, _)| TrackedId(*id))
			.collect()
	}

	fn storage_status(&self, id: Id) -> StorageStatus {
		if self.store.entries.contains_key(&id) {
			StorageStatus::TrackedWithin
		} else {
			StorageStatus::NotTracked
		}
	}

	fn get(&self, id: Id) -> Option<&Terrain> {
		self.store.entries.get(&id).map(|e| &e.value)
	}

	fn get_bounds(&self, id: Id) -> Option<Aabb3d> {
		self.store.entries.get(&id).map(|e| e.bounds)
	}

	fn version(&self, id: Id) -> Option<Version> {
		self.store.entries.get(&id).map(|e| e.version)
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
