//! System-local multi-type spatial index for Durham terrain generation.

use crate::terrain::base_noise::BaseTerrainNoise;
use crate::terrain::cell::{
	BootstrapJerseyStampCellLayout, BootstrapTerrainCellLayout, JerseyStampCellLayout,
	TerrainCellLayout,
};
use crate::terrain::cell_noise::CellTerrainNoise;
use crate::terrain::jersey_compose::JerseyModulations;
use crate::terrain::jersey_configs::{BootstrapJerseyLayerConfigs, JerseyLayerConfigs};
use crate::terrain::jersey_layers::{
	CanyonLayer, PlateauCapLayer, PocketWaterLayer, RollingGroundLayer, RuggedMassifLayer,
	ValleyBasinLayer,
};
use crate::terrain::presentation::{
	BootstrapTerrainPresentationAssets, TerrainPresentationAssets,
};
use crate::terrain::Terrain;
use avian3d::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::prelude::*;
use lod::gen::{Id, SpatialIndex, StorageStatus, TrackedId, Version};
use lod::lod_ref::LodRef;
use std::collections::HashMap;

/// Marks a bookkeeping entity as a tracked terrain cell.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerrainCellId(pub Id);

#[derive(Debug, Clone)]
pub(crate) struct StoredEntry<T> {
	pub(crate) value: T,
	pub(crate) bounds: Aabb3d,
	pub(crate) version: Version,
	pub(crate) entity: Option<Entity>,
}

/// Side table for all Durham terrain generation layers.
#[derive(Resource, Default)]
pub struct TerrainEntryStore {
	next_version: u64,
	pub(crate) terrain: HashMap<Id, StoredEntry<Terrain>>,
	pub(crate) base_noise: HashMap<Id, StoredEntry<BaseTerrainNoise>>,
	pub(crate) cell_noise: HashMap<Id, StoredEntry<CellTerrainNoise>>,
	pub(crate) valley_basin: HashMap<Id, StoredEntry<ValleyBasinLayer>>,
	pub(crate) plateau_cap: HashMap<Id, StoredEntry<PlateauCapLayer>>,
	pub(crate) rugged_massif: HashMap<Id, StoredEntry<RuggedMassifLayer>>,
	pub(crate) canyon: HashMap<Id, StoredEntry<CanyonLayer>>,
	pub(crate) pocket_water: HashMap<Id, StoredEntry<PocketWaterLayer>>,
	pub(crate) rolling_ground: HashMap<Id, StoredEntry<RollingGroundLayer>>,
	pub(crate) jersey_modulations: HashMap<Id, StoredEntry<JerseyModulations>>,
	pub(crate) jersey_configs: HashMap<Id, StoredEntry<JerseyLayerConfigs>>,
	pub(crate) jersey_layout: HashMap<Id, StoredEntry<JerseyStampCellLayout>>,
	pub(crate) cell_layout: HashMap<Id, StoredEntry<TerrainCellLayout>>,
	pub(crate) presentation: HashMap<Id, StoredEntry<TerrainPresentationAssets>>,
	entity_to_id: HashMap<Entity, Id>,
}

impl TerrainEntryStore {
	fn next_version(&mut self) -> Version {
		self.next_version += 1;
		Version(self.next_version)
	}

	pub fn len(&self) -> usize {
		self.terrain.len()
	}

	pub fn is_empty(&self) -> bool {
		self.terrain.is_empty()
			&& self.base_noise.is_empty()
			&& self.cell_noise.is_empty()
			&& self.valley_basin.is_empty()
			&& self.plateau_cap.is_empty()
			&& self.rugged_massif.is_empty()
			&& self.canyon.is_empty()
			&& self.pocket_water.is_empty()
			&& self.rolling_ground.is_empty()
			&& self.jersey_modulations.is_empty()
			&& self.jersey_configs.is_empty()
			&& self.jersey_layout.is_empty()
			&& self.cell_layout.is_empty()
			&& self.presentation.is_empty()
	}

	pub fn base_noise(&self) -> Option<&BaseTerrainNoise> {
		self.base_noise.get(&Id::Universal).map(|e| &e.value)
	}

	/// Lookup a materialized jersey modulation cell by id (debug / inspection).
	pub fn jersey_modulation(&self, id: Id) -> Option<&JerseyModulations> {
		self.jersey_modulations.get(&id).map(|e| &e.value)
	}

	/// Iterate materialized jersey modulation cells (debug / inspection).
	pub fn iter_jersey_modulations(
		&self,
	) -> impl Iterator<Item = (Id, &JerseyModulations)> + '_ {
		self.jersey_modulations
			.iter()
			.map(|(id, entry)| (*id, &entry.value))
	}
}

/// System-local wrapper used as `S` for [`lod::gen::GeneratingSpatialIndex`].
///
/// Bevy Resources remain the bootstrap source for universal layout / presentation;
/// once materialized they live in [`TerrainEntryStore`] under [`Id::Universal`].
#[derive(SystemParam)]
pub struct AvianTerrainIndex<'w, 's> {
	commands: Commands<'w, 's>,
	spatial: SpatialQuery<'w, 's>,
	store: ResMut<'w, TerrainEntryStore>,
	layout: ResMut<'w, TerrainCellLayout>,
	jersey_layout: ResMut<'w, JerseyStampCellLayout>,
	jersey_configs: Res<'w, JerseyLayerConfigs>,
	presentation: Res<'w, TerrainPresentationAssets>,
}

impl<'w, 's> BootstrapTerrainCellLayout for AvianTerrainIndex<'w, 's> {
	fn bootstrap_terrain_cell_layout(&self) -> TerrainCellLayout {
		self.layout.clone()
	}
}

impl<'w, 's> BootstrapJerseyStampCellLayout for AvianTerrainIndex<'w, 's> {
	fn bootstrap_jersey_stamp_cell_layout(&self) -> JerseyStampCellLayout {
		self.jersey_layout.clone()
	}
}

impl<'w, 's> BootstrapJerseyLayerConfigs for AvianTerrainIndex<'w, 's> {
	fn bootstrap_jersey_layer_configs(&self) -> JerseyLayerConfigs {
		self.jersey_configs.clone()
	}
}

impl<'w, 's> BootstrapTerrainPresentationAssets for AvianTerrainIndex<'w, 's> {
	fn bootstrap_terrain_presentation_assets(&self) -> TerrainPresentationAssets {
		self.presentation.clone()
	}
}

impl<'w, 's> AvianTerrainIndex<'w, 's> {
	fn region_to_collider_aabb(region: Aabb3d) -> ColliderAabb {
		ColliderAabb::from_min_max(Vec3::from(region.min), Vec3::from(region.max))
	}

	fn spawn_cell_entity(&mut self, id: Id, terrain: &Terrain, bounds: Aabb3d) -> Entity {
		let min = Vec3::from(bounds.min);
		let max = Vec3::from(bounds.max);
		let center = (min + max) * 0.5;
		self.commands
			.spawn((
				Name::new("TerrainCell"),
				TerrainCellId(id),
				terrain.clone(),
				Transform::from_translation(center),
				GlobalTransform::default(),
			))
			.id()
	}

	/// Despawn terrain bookkeeping entities and clear all generation layers.
	pub fn clear(&mut self) {
		let entities: Vec<Entity> = self
			.store
			.terrain
			.values()
			.filter_map(|e| e.entity)
			.collect();
		for entity in entities {
			self.commands.entity(entity).despawn();
		}
		self.store.terrain.clear();
		self.store.base_noise.clear();
		self.store.cell_noise.clear();
		self.store.valley_basin.clear();
		self.store.plateau_cap.clear();
		self.store.rugged_massif.clear();
		self.store.canyon.clear();
		self.store.pocket_water.clear();
		self.store.rolling_ground.clear();
		self.store.jersey_modulations.clear();
		self.store.jersey_configs.clear();
		self.store.jersey_layout.clear();
		self.store.cell_layout.clear();
		self.store.presentation.clear();
		self.store.entity_to_id.clear();
	}

	pub fn set_layout(&mut self, layout: TerrainCellLayout) {
		*self.layout = layout;
	}

	pub fn layout(&self) -> &TerrainCellLayout {
		&self.layout
	}

	pub fn base_noise(&self) -> Option<&BaseTerrainNoise> {
		self.store.base_noise()
	}
}

macro_rules! impl_map_spatial_index {
	($ty:ty, $field:ident) => {
		impl<'w, 's> SpatialIndex<$ty> for AvianTerrainIndex<'w, 's> {
			fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId> {
				self.store
					.$field
					.iter()
					.filter(|(_, entry)| region.intersects(&entry.bounds))
					.map(|(id, _)| TrackedId(*id))
					.collect()
			}

			fn storage_status(&self, id: Id) -> StorageStatus {
				if self.store.$field.contains_key(&id) {
					StorageStatus::TrackedWithin
				} else {
					StorageStatus::NotTracked
				}
			}

			fn get(&self, id: Id) -> Option<&$ty> {
				self.store.$field.get(&id).map(|e| &e.value)
			}

			fn get_bounds(&self, id: Id) -> Option<Aabb3d> {
				self.store.$field.get(&id).map(|e| e.bounds)
			}

			fn version(&self, id: Id) -> Option<Version> {
				self.store.$field.get(&id).map(|e| e.version)
			}

			fn insert(&mut self, id: Id, value: $ty, bounds: Aabb3d, _lod_ref: &LodRef) {
				if let Some(existing) = self.store.$field.remove(&id) {
					if let Some(entity) = existing.entity {
						self.store.entity_to_id.remove(&entity);
						self.commands.entity(entity).despawn();
					}
				}
				let version = self.store.next_version();
				self.store.$field.insert(
					id,
					StoredEntry { value, bounds, version, entity: None },
				);
			}
		}
	};
}

impl_map_spatial_index!(BaseTerrainNoise, base_noise);
impl_map_spatial_index!(CellTerrainNoise, cell_noise);
impl_map_spatial_index!(ValleyBasinLayer, valley_basin);
impl_map_spatial_index!(PlateauCapLayer, plateau_cap);
impl_map_spatial_index!(RuggedMassifLayer, rugged_massif);
impl_map_spatial_index!(CanyonLayer, canyon);
impl_map_spatial_index!(PocketWaterLayer, pocket_water);
impl_map_spatial_index!(RollingGroundLayer, rolling_ground);
impl_map_spatial_index!(JerseyModulations, jersey_modulations);
impl_map_spatial_index!(JerseyLayerConfigs, jersey_configs);
impl_map_spatial_index!(JerseyStampCellLayout, jersey_layout);
impl_map_spatial_index!(TerrainCellLayout, cell_layout);
impl_map_spatial_index!(TerrainPresentationAssets, presentation);

impl<'w, 's> SpatialIndex<Terrain> for AvianTerrainIndex<'w, 's> {
	fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId> {
		let aabb = Self::region_to_collider_aabb(region);
		let mut ids: Vec<TrackedId> = self
			.spatial
			.aabb_intersections_with_aabb(aabb)
			.into_iter()
			.filter_map(|entity| {
				self.store.entity_to_id.get(&entity).map(|id| TrackedId(*id))
			})
			.collect();

		for (id, entry) in &self.store.terrain {
			if region.intersects(&entry.bounds) {
				let tracked = TrackedId(*id);
				if !ids.contains(&tracked) {
					ids.push(tracked);
				}
			}
		}

		ids
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

	fn insert(&mut self, id: Id, t: Terrain, bounds: Aabb3d, _lod_ref: &LodRef) {
		if let Some(existing) = self.store.terrain.remove(&id) {
			if let Some(entity) = existing.entity {
				self.store.entity_to_id.remove(&entity);
				self.commands.entity(entity).despawn();
			}
		}

		let entity = self.spawn_cell_entity(id, &t, bounds);
		let version = self.store.next_version();
		self.store.entity_to_id.insert(entity, id);
		self.store.terrain.insert(
			id,
			StoredEntry { value: t, bounds, version, entity: Some(entity) },
		);
	}
}
