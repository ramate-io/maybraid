//! [`RegionPresenter`] for water cells (mirrors terrain presentation).

use crate::terrain::cell::{universal_bounds, TerrainCellLayout};
use crate::terrain::index::TerrainEntryStore;
use crate::water::Water;
use bevy::ecs::system::SystemParam;
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::prelude::*;
use bevy::scene::Scene;
use lod::gen::{
	GenerationScheme, Id, OriginalId, RegionPresenter, SpatialIndex, StorageStatus, TrackedId,
	Version,
};
use lod::lod_ref::LodRef;
use std::collections::{HashMap, HashSet};

/// Material used when building water instances.
///
/// Mesh `res_2` and origin-cell bounds come from the sibling [`crate::terrain::Terrain`]
/// cell / [`crate::terrain::cell::TerrainCellLayout`] — not from this resource — so
/// water and terrain always share one cascade lattice.
#[derive(Resource, Clone)]
pub struct WaterPresentationAssets {
	pub material: Handle<StandardMaterial>,
}

/// Bootstrap source used only when first materializing [`WaterPresentationAssets`]
/// at [`Id::Universal`].
pub trait BootstrapWaterPresentationAssets {
	fn bootstrap_water_presentation_assets(&self) -> WaterPresentationAssets;
}

impl<S> GenerationScheme<S> for WaterPresentationAssets
where
	S: BootstrapWaterPresentationAssets,
{
	fn original_ids_for(_spatial_index: &mut S, _region: Aabb3d) -> Vec<OriginalId> {
		vec![OriginalId::universal()]
	}

	fn build_with_id(spatial_index: &mut S, id: Id, _lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		if id != Id::Universal {
			return None;
		}
		Some((spatial_index.bootstrap_water_presentation_assets(), universal_bounds()))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}

/// Runtime presentation bookkeeping: last presented version and root entity per id.
#[derive(Resource, Default)]
pub struct WaterPresenterState {
	presented: HashMap<Id, PresentedEntry>,
}

#[derive(Debug, Clone, Copy)]
struct PresentedEntry {
	version: Version,
	entity: Entity,
}

/// Marks a spawned water scene root as belonging to a presented id.
#[derive(Component, Debug, Clone, Copy)]
pub struct PresentedWaterScene(pub Id);

impl WaterPresenterState {
	pub fn clear(&mut self, commands: &mut Commands) {
		for entry in self.presented.values() {
			commands.entity(entry.entity).despawn();
		}
		self.presented.clear();
	}
}

/// Read-only spatial-index view over the water entry map for presentation.
pub struct WaterStoreView<'a> {
	store: &'a TerrainEntryStore,
	_layout: &'a TerrainCellLayout,
}

impl<'a> WaterStoreView<'a> {
	pub fn new(store: &'a TerrainEntryStore, layout: &'a TerrainCellLayout) -> Self {
		Self { store, _layout: layout }
	}
}

impl SpatialIndex<Water> for WaterStoreView<'_> {
	fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId> {
		self.store
			.water
			.iter()
			.filter(|(_, entry)| region.intersects(&entry.bounds))
			.map(|(id, _)| TrackedId(*id))
			.collect()
	}

	fn storage_status(&self, id: Id) -> StorageStatus {
		if self.store.water.contains_key(&id) {
			StorageStatus::TrackedWithin
		} else {
			StorageStatus::NotTracked
		}
	}

	fn get(&self, id: Id) -> Option<&Water> {
		self.store.water.get(&id).map(|e| &e.value)
	}

	fn get_bounds(&self, id: Id) -> Option<Aabb3d> {
		self.store.water.get(&id).map(|e| e.bounds)
	}

	fn version(&self, id: Id) -> Option<Version> {
		self.store.water.get(&id).map(|e| e.version)
	}

	fn insert(&mut self, _id: Id, _t: Water, _bounds: Aabb3d, _lod_ref: &LodRef) {
		panic!("WaterStoreView is read-only; insert via AvianTerrainIndex");
	}
}

/// System-local presenter: spawns water `bsn!` scenes and tracks versions.
#[derive(SystemParam)]
pub struct WaterRegionPresenter<'w, 's> {
	commands: Commands<'w, 's>,
	state: ResMut<'w, WaterPresenterState>,
}

impl<'w, 's> WaterRegionPresenter<'w, 's> {
	pub fn clear_presented(&mut self) {
		self.state.clear(&mut self.commands);
	}
}

impl<'a, 'w, 's> RegionPresenter<Water, WaterStoreView<'a>> for WaterRegionPresenter<'w, 's> {
	fn presented_version(&self, id: Id) -> Option<Version> {
		self.state.presented.get(&id).map(|e| e.version)
	}

	fn handle(&mut self, id: Id, version: Version, scene: impl Scene, _lod_ref: &LodRef) {
		if let Some(previous) = self.state.presented.remove(&id) {
			self.commands.entity(previous.entity).despawn();
		}
		let entity = self.commands.spawn_scene(scene).insert(PresentedWaterScene(id)).id();
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
