//! System-local multi-type spatial index for Durham terrain generation.

use crate::terrain::base_noise::BaseTerrainNoise;
use crate::terrain::cell::{BootstrapTerrainCellLayout, TerrainCellLayout};
use crate::terrain::jersey::{
	BootstrapCanyonControllerLayout, BootstrapJerseyStampConfigs, BootstrapMassifControllerLayout,
	BootstrapPlateauControllerLayout, BootstrapPocketWaterControllerLayout,
	BootstrapRollingControllerLayout, BootstrapValleyControllerLayout, CanyonControllerCell,
	CanyonControllerLayout, CanyonStampCell, JerseyStampConfigs, MassifControllerCell,
	MassifControllerLayout, MassifStampCell, PlateauControllerCell, PlateauControllerLayout,
	PlateauStampCell, PocketWaterControllerCell, PocketWaterControllerLayout, PocketWaterStampCell,
	RollingControllerCell, RollingControllerLayout, RollingStampCell, ValleyControllerCell,
	ValleyControllerLayout, ValleyStampCell,
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
	pub(crate) cell_layout: HashMap<Id, StoredEntry<TerrainCellLayout>>,
	pub(crate) presentation: HashMap<Id, StoredEntry<TerrainPresentationAssets>>,
	pub(crate) jersey_configs: HashMap<Id, StoredEntry<JerseyStampConfigs>>,
	pub(crate) plateau_layout: HashMap<Id, StoredEntry<PlateauControllerLayout>>,
	pub(crate) plateau_controller: HashMap<Id, StoredEntry<PlateauControllerCell>>,
	pub(crate) plateau_stamp: HashMap<Id, StoredEntry<PlateauStampCell>>,
	pub(crate) massif_layout: HashMap<Id, StoredEntry<MassifControllerLayout>>,
	pub(crate) massif_controller: HashMap<Id, StoredEntry<MassifControllerCell>>,
	pub(crate) massif_stamp: HashMap<Id, StoredEntry<MassifStampCell>>,
	pub(crate) canyon_layout: HashMap<Id, StoredEntry<CanyonControllerLayout>>,
	pub(crate) canyon_controller: HashMap<Id, StoredEntry<CanyonControllerCell>>,
	pub(crate) canyon_stamp: HashMap<Id, StoredEntry<CanyonStampCell>>,
	pub(crate) pocket_water_layout: HashMap<Id, StoredEntry<PocketWaterControllerLayout>>,
	pub(crate) pocket_water_controller: HashMap<Id, StoredEntry<PocketWaterControllerCell>>,
	pub(crate) pocket_water_stamp: HashMap<Id, StoredEntry<PocketWaterStampCell>>,
	pub(crate) rolling_layout: HashMap<Id, StoredEntry<RollingControllerLayout>>,
	pub(crate) rolling_controller: HashMap<Id, StoredEntry<RollingControllerCell>>,
	pub(crate) rolling_stamp: HashMap<Id, StoredEntry<RollingStampCell>>,
	pub(crate) valley_layout: HashMap<Id, StoredEntry<ValleyControllerLayout>>,
	pub(crate) valley_controller: HashMap<Id, StoredEntry<ValleyControllerCell>>,
	pub(crate) valley_stamp: HashMap<Id, StoredEntry<ValleyStampCell>>,
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
			&& self.cell_layout.is_empty()
			&& self.presentation.is_empty()
			&& self.jersey_configs.is_empty()
			&& self.plateau_layout.is_empty()
			&& self.plateau_controller.is_empty()
			&& self.plateau_stamp.is_empty()
			&& self.massif_layout.is_empty()
			&& self.massif_controller.is_empty()
			&& self.massif_stamp.is_empty()
			&& self.canyon_layout.is_empty()
			&& self.canyon_controller.is_empty()
			&& self.canyon_stamp.is_empty()
			&& self.pocket_water_layout.is_empty()
			&& self.pocket_water_controller.is_empty()
			&& self.pocket_water_stamp.is_empty()
			&& self.rolling_layout.is_empty()
			&& self.rolling_controller.is_empty()
			&& self.rolling_stamp.is_empty()
			&& self.valley_layout.is_empty()
			&& self.valley_controller.is_empty()
			&& self.valley_stamp.is_empty()
	}

	pub fn base_noise(&self) -> Option<&BaseTerrainNoise> {
		self.base_noise.get(&Id::Universal).map(|e| &e.value)
	}
}

/// System-local wrapper used as `S` for [`lod::gen::GeneratingSpatialIndex`].
#[derive(SystemParam)]
pub struct AvianTerrainIndex<'w, 's> {
	commands: Commands<'w, 's>,
	spatial: SpatialQuery<'w, 's>,
	store: ResMut<'w, TerrainEntryStore>,
	layout: ResMut<'w, TerrainCellLayout>,
	jersey_configs: Res<'w, JerseyStampConfigs>,
	plateau_layout: ResMut<'w, PlateauControllerLayout>,
	massif_layout: ResMut<'w, MassifControllerLayout>,
	canyon_layout: ResMut<'w, CanyonControllerLayout>,
	pocket_water_layout: ResMut<'w, PocketWaterControllerLayout>,
	rolling_layout: ResMut<'w, RollingControllerLayout>,
	valley_layout: ResMut<'w, ValleyControllerLayout>,
	presentation: Res<'w, TerrainPresentationAssets>,
}

impl<'w, 's> BootstrapTerrainCellLayout for AvianTerrainIndex<'w, 's> {
	fn bootstrap_terrain_cell_layout(&self) -> TerrainCellLayout {
		self.layout.clone()
	}
}

impl<'w, 's> BootstrapJerseyStampConfigs for AvianTerrainIndex<'w, 's> {
	fn bootstrap_jersey_stamp_configs(&self) -> JerseyStampConfigs {
		self.jersey_configs.clone()
	}
}

impl<'w, 's> BootstrapPlateauControllerLayout for AvianTerrainIndex<'w, 's> {
	fn bootstrap_plateau_controller_layout(&self) -> PlateauControllerLayout {
		self.plateau_layout.clone()
	}
}

impl<'w, 's> BootstrapMassifControllerLayout for AvianTerrainIndex<'w, 's> {
	fn bootstrap_massif_controller_layout(&self) -> MassifControllerLayout {
		self.massif_layout.clone()
	}
}

impl<'w, 's> BootstrapCanyonControllerLayout for AvianTerrainIndex<'w, 's> {
	fn bootstrap_canyon_controller_layout(&self) -> CanyonControllerLayout {
		self.canyon_layout.clone()
	}
}

impl<'w, 's> BootstrapPocketWaterControllerLayout for AvianTerrainIndex<'w, 's> {
	fn bootstrap_pocket_water_controller_layout(&self) -> PocketWaterControllerLayout {
		self.pocket_water_layout.clone()
	}
}

impl<'w, 's> BootstrapRollingControllerLayout for AvianTerrainIndex<'w, 's> {
	fn bootstrap_rolling_controller_layout(&self) -> RollingControllerLayout {
		self.rolling_layout.clone()
	}
}

impl<'w, 's> BootstrapValleyControllerLayout for AvianTerrainIndex<'w, 's> {
	fn bootstrap_valley_controller_layout(&self) -> ValleyControllerLayout {
		self.valley_layout.clone()
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
		*self.store = TerrainEntryStore::default();
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
impl_map_spatial_index!(TerrainCellLayout, cell_layout);
impl_map_spatial_index!(TerrainPresentationAssets, presentation);
impl_map_spatial_index!(JerseyStampConfigs, jersey_configs);
impl_map_spatial_index!(PlateauControllerLayout, plateau_layout);
impl_map_spatial_index!(PlateauControllerCell, plateau_controller);
impl_map_spatial_index!(PlateauStampCell, plateau_stamp);
impl_map_spatial_index!(MassifControllerLayout, massif_layout);
impl_map_spatial_index!(MassifControllerCell, massif_controller);
impl_map_spatial_index!(MassifStampCell, massif_stamp);
impl_map_spatial_index!(CanyonControllerLayout, canyon_layout);
impl_map_spatial_index!(CanyonControllerCell, canyon_controller);
impl_map_spatial_index!(CanyonStampCell, canyon_stamp);
impl_map_spatial_index!(PocketWaterControllerLayout, pocket_water_layout);
impl_map_spatial_index!(PocketWaterControllerCell, pocket_water_controller);
impl_map_spatial_index!(PocketWaterStampCell, pocket_water_stamp);
impl_map_spatial_index!(RollingControllerLayout, rolling_layout);
impl_map_spatial_index!(RollingControllerCell, rolling_controller);
impl_map_spatial_index!(RollingStampCell, rolling_stamp);
impl_map_spatial_index!(ValleyControllerLayout, valley_layout);
impl_map_spatial_index!(ValleyControllerCell, valley_controller);
impl_map_spatial_index!(ValleyStampCell, valley_stamp);

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
