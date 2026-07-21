//! System-local multi-type spatial index for Durham terrain generation.

use crate::terrain::base_noise::BaseTerrainNoise;
use crate::terrain::cell::{cell_bounds, BootstrapTerrainCellLayout, TerrainCellLayout};
use crate::terrain::jersey::{
	BootstrapCanyonHighPassControllerLayout, BootstrapCanyonLowPassControllerLayout,
	BootstrapJerseyStampConfigs, BootstrapMassifHighPassControllerLayout,
	BootstrapMassifLowPassControllerLayout, BootstrapPlateauHighPassControllerLayout,
	BootstrapPlateauLowPassControllerLayout, BootstrapPocketWaterHighPassControllerLayout,
	BootstrapPocketWaterLowPassControllerLayout, BootstrapRollingHighPassControllerLayout,
	BootstrapRollingLowPassControllerLayout, BootstrapValleyHighPassControllerLayout,
	BootstrapValleyLowPassControllerLayout, CanyonHighPassControllerCell,
	CanyonHighPassControllerLayout, CanyonHighPassStampCell, CanyonLowPassControllerCell,
	CanyonLowPassControllerLayout, CanyonLowPassStampCell, JerseyControllerLayouts,
	JerseyStampConfigs, MassifHighPassControllerCell, MassifHighPassControllerLayout,
	MassifHighPassStampCell, MassifLowPassControllerCell, MassifLowPassControllerLayout,
	MassifLowPassStampCell, PlateauHighPassControllerCell, PlateauHighPassControllerLayout,
	PlateauHighPassStampCell, PlateauLowPassControllerCell, PlateauLowPassControllerLayout,
	PlateauLowPassStampCell, PocketWaterHighPassControllerCell, PocketWaterHighPassControllerLayout,
	PocketWaterHighPassStampCell, PocketWaterLowPassControllerCell,
	PocketWaterLowPassControllerLayout, PocketWaterLowPassStampCell, RollingHighPassControllerCell,
	RollingHighPassControllerLayout, RollingHighPassStampCell, RollingLowPassControllerCell,
	RollingLowPassControllerLayout, RollingLowPassStampCell, ValleyHighPassControllerCell,
	ValleyHighPassControllerLayout, ValleyHighPassStampCell, ValleyLowPassControllerCell,
	ValleyLowPassControllerLayout, ValleyLowPassStampCell,
};
use crate::terrain::marazion::{
	BootstrapMarazionWatershedConfigs, BootstrapPrePocketHighPassLayout,
	BootstrapPrePocketLowPassLayout, MarazionLakeHighPassCell, MarazionLakeLowPassCell,
	MarazionWatershedConfigs, PocketHighPassCell, PocketLowPassCell, PrePocketHighPassCell,
	PrePocketHighPassLayout, PrePocketLowPassCell, PrePocketLowPassLayout,
};
use crate::terrain::presentation::{
	BootstrapTerrainPresentationAssets, TerrainPresentationAssets,
};
use crate::terrain::{PreWatershedTerrain, Terrain};
use crate::water::{BootstrapWaterPresentationAssets, Water, WaterPresentationAssets};
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
	pub(crate) pre_watershed: HashMap<Id, StoredEntry<PreWatershedTerrain>>,
	pub(crate) water: HashMap<Id, StoredEntry<Water>>,
	pub(crate) base_noise: HashMap<Id, StoredEntry<BaseTerrainNoise>>,
	pub(crate) cell_layout: HashMap<Id, StoredEntry<TerrainCellLayout>>,
	pub(crate) presentation: HashMap<Id, StoredEntry<TerrainPresentationAssets>>,
	pub(crate) water_presentation: HashMap<Id, StoredEntry<WaterPresentationAssets>>,
	pub(crate) jersey_configs: HashMap<Id, StoredEntry<JerseyStampConfigs>>,
	pub(crate) marazion_configs: HashMap<Id, StoredEntry<MarazionWatershedConfigs>>,
	pub(crate) pre_pocket_low_pass_layout: HashMap<Id, StoredEntry<PrePocketLowPassLayout>>,
	pub(crate) pre_pocket_low_pass_cell: HashMap<Id, StoredEntry<PrePocketLowPassCell>>,
	pub(crate) pocket_low_pass_cell: HashMap<Id, StoredEntry<PocketLowPassCell>>,
	pub(crate) marazion_lake_low_pass_cell: HashMap<Id, StoredEntry<MarazionLakeLowPassCell>>,
	pub(crate) pre_pocket_high_pass_layout: HashMap<Id, StoredEntry<PrePocketHighPassLayout>>,
	pub(crate) pre_pocket_high_pass_cell: HashMap<Id, StoredEntry<PrePocketHighPassCell>>,
	pub(crate) pocket_high_pass_cell: HashMap<Id, StoredEntry<PocketHighPassCell>>,
	pub(crate) marazion_lake_high_pass_cell: HashMap<Id, StoredEntry<MarazionLakeHighPassCell>>,
	pub(crate) plateau_low_pass_layout: HashMap<Id, StoredEntry<PlateauLowPassControllerLayout>>,
	pub(crate) plateau_low_pass_controller: HashMap<Id, StoredEntry<PlateauLowPassControllerCell>>,
	pub(crate) plateau_low_pass_stamp: HashMap<Id, StoredEntry<PlateauLowPassStampCell>>,
	pub(crate) plateau_high_pass_layout: HashMap<Id, StoredEntry<PlateauHighPassControllerLayout>>,
	pub(crate) plateau_high_pass_controller: HashMap<Id, StoredEntry<PlateauHighPassControllerCell>>,
	pub(crate) plateau_high_pass_stamp: HashMap<Id, StoredEntry<PlateauHighPassStampCell>>,
	pub(crate) massif_low_pass_layout: HashMap<Id, StoredEntry<MassifLowPassControllerLayout>>,
	pub(crate) massif_low_pass_controller: HashMap<Id, StoredEntry<MassifLowPassControllerCell>>,
	pub(crate) massif_low_pass_stamp: HashMap<Id, StoredEntry<MassifLowPassStampCell>>,
	pub(crate) massif_high_pass_layout: HashMap<Id, StoredEntry<MassifHighPassControllerLayout>>,
	pub(crate) massif_high_pass_controller: HashMap<Id, StoredEntry<MassifHighPassControllerCell>>,
	pub(crate) massif_high_pass_stamp: HashMap<Id, StoredEntry<MassifHighPassStampCell>>,
	pub(crate) canyon_low_pass_layout: HashMap<Id, StoredEntry<CanyonLowPassControllerLayout>>,
	pub(crate) canyon_low_pass_controller: HashMap<Id, StoredEntry<CanyonLowPassControllerCell>>,
	pub(crate) canyon_low_pass_stamp: HashMap<Id, StoredEntry<CanyonLowPassStampCell>>,
	pub(crate) canyon_high_pass_layout: HashMap<Id, StoredEntry<CanyonHighPassControllerLayout>>,
	pub(crate) canyon_high_pass_controller: HashMap<Id, StoredEntry<CanyonHighPassControllerCell>>,
	pub(crate) canyon_high_pass_stamp: HashMap<Id, StoredEntry<CanyonHighPassStampCell>>,
	pub(crate) pocket_water_low_pass_layout:
		HashMap<Id, StoredEntry<PocketWaterLowPassControllerLayout>>,
	pub(crate) pocket_water_low_pass_controller:
		HashMap<Id, StoredEntry<PocketWaterLowPassControllerCell>>,
	pub(crate) pocket_water_low_pass_stamp: HashMap<Id, StoredEntry<PocketWaterLowPassStampCell>>,
	pub(crate) pocket_water_high_pass_layout:
		HashMap<Id, StoredEntry<PocketWaterHighPassControllerLayout>>,
	pub(crate) pocket_water_high_pass_controller:
		HashMap<Id, StoredEntry<PocketWaterHighPassControllerCell>>,
	pub(crate) pocket_water_high_pass_stamp: HashMap<Id, StoredEntry<PocketWaterHighPassStampCell>>,
	pub(crate) rolling_low_pass_layout: HashMap<Id, StoredEntry<RollingLowPassControllerLayout>>,
	pub(crate) rolling_low_pass_controller: HashMap<Id, StoredEntry<RollingLowPassControllerCell>>,
	pub(crate) rolling_low_pass_stamp: HashMap<Id, StoredEntry<RollingLowPassStampCell>>,
	pub(crate) rolling_high_pass_layout: HashMap<Id, StoredEntry<RollingHighPassControllerLayout>>,
	pub(crate) rolling_high_pass_controller: HashMap<Id, StoredEntry<RollingHighPassControllerCell>>,
	pub(crate) rolling_high_pass_stamp: HashMap<Id, StoredEntry<RollingHighPassStampCell>>,
	pub(crate) valley_low_pass_layout: HashMap<Id, StoredEntry<ValleyLowPassControllerLayout>>,
	pub(crate) valley_low_pass_controller: HashMap<Id, StoredEntry<ValleyLowPassControllerCell>>,
	pub(crate) valley_low_pass_stamp: HashMap<Id, StoredEntry<ValleyLowPassStampCell>>,
	pub(crate) valley_high_pass_layout: HashMap<Id, StoredEntry<ValleyHighPassControllerLayout>>,
	pub(crate) valley_high_pass_controller: HashMap<Id, StoredEntry<ValleyHighPassControllerCell>>,
	pub(crate) valley_high_pass_stamp: HashMap<Id, StoredEntry<ValleyHighPassStampCell>>,
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
	}

	pub fn base_noise(&self) -> Option<&BaseTerrainNoise> {
		self.base_noise.get(&Id::Universal).map(|e| &e.value)
	}

	/// Composed terrain height (jersey + Marazion) at `(x, z)`, if that cell is stored.
	pub fn composed_height_at(&self, layout: &TerrainCellLayout, x: f32, z: f32) -> Option<f32> {
		let size = layout.cell_size.max(1e-3);
		let ix = (x / size).floor() as i32;
		let iz = (z / size).floor() as i32;
		let cell = cell_bounds(ix, iz, size, layout.vertical_half_extent);
		let id = Id::from_cell(cell);
		self.terrain
			.get(&id)
			.map(|e| e.value.sdf.terrain.height_at_with_all_modulations(x, z))
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
	jersey_layouts: ResMut<'w, JerseyControllerLayouts>,
	marazion_configs: Res<'w, MarazionWatershedConfigs>,
	pre_pocket_low_pass_layout: ResMut<'w, PrePocketLowPassLayout>,
	pre_pocket_high_pass_layout: ResMut<'w, PrePocketHighPassLayout>,
	presentation: Res<'w, TerrainPresentationAssets>,
	water_presentation: Res<'w, WaterPresentationAssets>,
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

impl<'w, 's> BootstrapMarazionWatershedConfigs for AvianTerrainIndex<'w, 's> {
	fn bootstrap_marazion_watershed_configs(&self) -> MarazionWatershedConfigs {
		self.marazion_configs.clone()
	}
}

impl<'w, 's> BootstrapPrePocketLowPassLayout for AvianTerrainIndex<'w, 's> {
	fn bootstrap_pre_pocket_low_pass_layout(&self) -> PrePocketLowPassLayout {
		self.pre_pocket_low_pass_layout.clone()
	}
}

impl<'w, 's> BootstrapPrePocketHighPassLayout for AvianTerrainIndex<'w, 's> {
	fn bootstrap_pre_pocket_high_pass_layout(&self) -> PrePocketHighPassLayout {
		self.pre_pocket_high_pass_layout.clone()
	}
}

macro_rules! impl_bootstrap_layout {
	($trait:ident, $method:ident, $field:ident, $ty:ty) => {
		impl<'w, 's> $trait for AvianTerrainIndex<'w, 's> {
			fn $method(&self) -> $ty {
				self.jersey_layouts.$field.clone()
			}
		}
	};
}

impl_bootstrap_layout!(
	BootstrapPlateauLowPassControllerLayout,
	bootstrap_plateau_low_pass_controller_layout,
	plateau_low_pass,
	PlateauLowPassControllerLayout
);
impl_bootstrap_layout!(
	BootstrapPlateauHighPassControllerLayout,
	bootstrap_plateau_high_pass_controller_layout,
	plateau_high_pass,
	PlateauHighPassControllerLayout
);
impl_bootstrap_layout!(
	BootstrapMassifLowPassControllerLayout,
	bootstrap_massif_low_pass_controller_layout,
	massif_low_pass,
	MassifLowPassControllerLayout
);
impl_bootstrap_layout!(
	BootstrapMassifHighPassControllerLayout,
	bootstrap_massif_high_pass_controller_layout,
	massif_high_pass,
	MassifHighPassControllerLayout
);
impl_bootstrap_layout!(
	BootstrapCanyonLowPassControllerLayout,
	bootstrap_canyon_low_pass_controller_layout,
	canyon_low_pass,
	CanyonLowPassControllerLayout
);
impl_bootstrap_layout!(
	BootstrapCanyonHighPassControllerLayout,
	bootstrap_canyon_high_pass_controller_layout,
	canyon_high_pass,
	CanyonHighPassControllerLayout
);
impl_bootstrap_layout!(
	BootstrapPocketWaterLowPassControllerLayout,
	bootstrap_pocket_water_low_pass_controller_layout,
	pocket_water_low_pass,
	PocketWaterLowPassControllerLayout
);
impl_bootstrap_layout!(
	BootstrapPocketWaterHighPassControllerLayout,
	bootstrap_pocket_water_high_pass_controller_layout,
	pocket_water_high_pass,
	PocketWaterHighPassControllerLayout
);
impl_bootstrap_layout!(
	BootstrapRollingLowPassControllerLayout,
	bootstrap_rolling_low_pass_controller_layout,
	rolling_low_pass,
	RollingLowPassControllerLayout
);
impl_bootstrap_layout!(
	BootstrapRollingHighPassControllerLayout,
	bootstrap_rolling_high_pass_controller_layout,
	rolling_high_pass,
	RollingHighPassControllerLayout
);
impl_bootstrap_layout!(
	BootstrapValleyLowPassControllerLayout,
	bootstrap_valley_low_pass_controller_layout,
	valley_low_pass,
	ValleyLowPassControllerLayout
);
impl_bootstrap_layout!(
	BootstrapValleyHighPassControllerLayout,
	bootstrap_valley_high_pass_controller_layout,
	valley_high_pass,
	ValleyHighPassControllerLayout
);

impl<'w, 's> BootstrapTerrainPresentationAssets for AvianTerrainIndex<'w, 's> {
	fn bootstrap_terrain_presentation_assets(&self) -> TerrainPresentationAssets {
		self.presentation.clone()
	}
}

impl<'w, 's> BootstrapWaterPresentationAssets for AvianTerrainIndex<'w, 's> {
	fn bootstrap_water_presentation_assets(&self) -> WaterPresentationAssets {
		self.water_presentation.clone()
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

	/// Composed terrain height at `(x, z)` when the cell is in the store.
	pub fn composed_height_at(&self, x: f32, z: f32) -> Option<f32> {
		self.store.composed_height_at(&self.layout, x, z)
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
					StoredEntry {
						value,
						bounds,
						version,
						entity: None,
					},
				);
			}
		}
	};
}

impl_map_spatial_index!(BaseTerrainNoise, base_noise);
impl_map_spatial_index!(TerrainCellLayout, cell_layout);
impl_map_spatial_index!(TerrainPresentationAssets, presentation);
impl_map_spatial_index!(WaterPresentationAssets, water_presentation);
impl_map_spatial_index!(JerseyStampConfigs, jersey_configs);
impl_map_spatial_index!(MarazionWatershedConfigs, marazion_configs);
impl_map_spatial_index!(PrePocketLowPassLayout, pre_pocket_low_pass_layout);
impl_map_spatial_index!(PrePocketLowPassCell, pre_pocket_low_pass_cell);
impl_map_spatial_index!(PocketLowPassCell, pocket_low_pass_cell);
impl_map_spatial_index!(MarazionLakeLowPassCell, marazion_lake_low_pass_cell);
impl_map_spatial_index!(PrePocketHighPassLayout, pre_pocket_high_pass_layout);
impl_map_spatial_index!(PrePocketHighPassCell, pre_pocket_high_pass_cell);
impl_map_spatial_index!(PocketHighPassCell, pocket_high_pass_cell);
impl_map_spatial_index!(MarazionLakeHighPassCell, marazion_lake_high_pass_cell);
impl_map_spatial_index!(PreWatershedTerrain, pre_watershed);
impl_map_spatial_index!(Water, water);

impl_map_spatial_index!(PlateauLowPassControllerLayout, plateau_low_pass_layout);
impl_map_spatial_index!(PlateauLowPassControllerCell, plateau_low_pass_controller);
impl_map_spatial_index!(PlateauLowPassStampCell, plateau_low_pass_stamp);
impl_map_spatial_index!(PlateauHighPassControllerLayout, plateau_high_pass_layout);
impl_map_spatial_index!(PlateauHighPassControllerCell, plateau_high_pass_controller);
impl_map_spatial_index!(PlateauHighPassStampCell, plateau_high_pass_stamp);

impl_map_spatial_index!(MassifLowPassControllerLayout, massif_low_pass_layout);
impl_map_spatial_index!(MassifLowPassControllerCell, massif_low_pass_controller);
impl_map_spatial_index!(MassifLowPassStampCell, massif_low_pass_stamp);
impl_map_spatial_index!(MassifHighPassControllerLayout, massif_high_pass_layout);
impl_map_spatial_index!(MassifHighPassControllerCell, massif_high_pass_controller);
impl_map_spatial_index!(MassifHighPassStampCell, massif_high_pass_stamp);

impl_map_spatial_index!(CanyonLowPassControllerLayout, canyon_low_pass_layout);
impl_map_spatial_index!(CanyonLowPassControllerCell, canyon_low_pass_controller);
impl_map_spatial_index!(CanyonLowPassStampCell, canyon_low_pass_stamp);
impl_map_spatial_index!(CanyonHighPassControllerLayout, canyon_high_pass_layout);
impl_map_spatial_index!(CanyonHighPassControllerCell, canyon_high_pass_controller);
impl_map_spatial_index!(CanyonHighPassStampCell, canyon_high_pass_stamp);

impl_map_spatial_index!(PocketWaterLowPassControllerLayout, pocket_water_low_pass_layout);
impl_map_spatial_index!(PocketWaterLowPassControllerCell, pocket_water_low_pass_controller);
impl_map_spatial_index!(PocketWaterLowPassStampCell, pocket_water_low_pass_stamp);
impl_map_spatial_index!(PocketWaterHighPassControllerLayout, pocket_water_high_pass_layout);
impl_map_spatial_index!(PocketWaterHighPassControllerCell, pocket_water_high_pass_controller);
impl_map_spatial_index!(PocketWaterHighPassStampCell, pocket_water_high_pass_stamp);

impl_map_spatial_index!(RollingLowPassControllerLayout, rolling_low_pass_layout);
impl_map_spatial_index!(RollingLowPassControllerCell, rolling_low_pass_controller);
impl_map_spatial_index!(RollingLowPassStampCell, rolling_low_pass_stamp);
impl_map_spatial_index!(RollingHighPassControllerLayout, rolling_high_pass_layout);
impl_map_spatial_index!(RollingHighPassControllerCell, rolling_high_pass_controller);
impl_map_spatial_index!(RollingHighPassStampCell, rolling_high_pass_stamp);

impl_map_spatial_index!(ValleyLowPassControllerLayout, valley_low_pass_layout);
impl_map_spatial_index!(ValleyLowPassControllerCell, valley_low_pass_controller);
impl_map_spatial_index!(ValleyLowPassStampCell, valley_low_pass_stamp);
impl_map_spatial_index!(ValleyHighPassControllerLayout, valley_high_pass_layout);
impl_map_spatial_index!(ValleyHighPassControllerCell, valley_high_pass_controller);
impl_map_spatial_index!(ValleyHighPassStampCell, valley_high_pass_stamp);

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
			StoredEntry {
				value: t,
				bounds,
				version,
				entity: Some(entity),
			},
		);
	}
}
