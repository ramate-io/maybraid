//! 400 m mob cells and the spatial index that materializes them.

use std::collections::HashMap;

use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::prelude::*;
use chico::LayeringKind;
use lod::gen::{GenerationScheme, Id, OriginalId, SpatialIndex, StorageStatus, TrackedId, Version};
use procedural_common::NoiseParams;
use urbanization_cells::UrbanizationKind;

use crate::generation::{
	GroupKind, MobEnvironmentSample, MobGroup, MobPlantHost, MobWorldHosts, MobWorldSample,
};
#[cfg(test)]
use crate::sample::{chico_layers_at, richmond_kind_at};

pub const MOB_CELL_EXTENT: f32 = 400.0;
pub const MOB_WORLD_SEED: u64 = 42;
pub const MOB_CELL_OCCUPANCY_PERCENT: u64 = 35;

pub(crate) type ForestLayersAt = fn(NoiseParams, Option<LayeringKind>, Vec2) -> u8;
pub(crate) type UrbanKindAt = fn(NoiseParams, Option<UrbanizationKind>, Vec2) -> UrbanizationKind;

fn idle_layers(_noise: NoiseParams, _layering: Option<LayeringKind>, _xz: Vec2) -> u8 {
	0
}

fn idle_kind(_noise: NoiseParams, pinned: Option<UrbanizationKind>, _xz: Vec2) -> UrbanizationKind {
	pinned.unwrap_or(UrbanizationKind::None)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MobCellExtent {
	min: Vec3,
	max: Vec3,
}

impl MobCellExtent {
	pub fn from_cell_index(ix: i32, iz: i32) -> Self {
		let half = MOB_CELL_EXTENT * 0.5;
		Self {
			min: Vec3::new(
				ix as f32 * MOB_CELL_EXTENT - half,
				0.0,
				iz as f32 * MOB_CELL_EXTENT - half,
			),
			max: Vec3::new(
				ix as f32 * MOB_CELL_EXTENT + half,
				1.0,
				iz as f32 * MOB_CELL_EXTENT + half,
			),
		}
	}

	/// Scheme-placed cell. Grid generate still uses [`Self::from_cell_index`].
	pub fn from_bounds(min: Vec3, max: Vec3) -> Self {
		Self { min, max }
	}

	pub fn from_id(id: Id) -> Option<Self> {
		let bounds = id.origin_cell_bounds()?;
		let width = bounds.max.x - bounds.min.x;
		let depth = bounds.max.z - bounds.min.z;
		if (width - MOB_CELL_EXTENT).abs() > 1e-3 || (depth - MOB_CELL_EXTENT).abs() > 1e-3 {
			return None;
		}
		Some(Self { min: bounds.min.into(), max: bounds.max.into() })
	}

	pub fn cells_overlapping(region: Aabb3d) -> Vec<Self> {
		let min = Self::cell_index_containing(Vec3::new(region.min.x, 0.0, region.min.z));
		let max = Self::cell_index_containing(Vec3::new(
			(region.max.x - 1e-3).max(region.min.x),
			0.0,
			(region.max.z - 1e-3).max(region.min.z),
		));
		(min.0.min(max.0)..=min.0.max(max.0))
			.flat_map(|ix| {
				(min.1.min(max.1)..=min.1.max(max.1)).map(move |iz| Self::from_cell_index(ix, iz))
			})
			.collect()
	}

	pub fn cell_index_containing(position: Vec3) -> (i32, i32) {
		let half = MOB_CELL_EXTENT * 0.5;
		(
			((position.x + half) / MOB_CELL_EXTENT).floor() as i32,
			((position.z + half) / MOB_CELL_EXTENT).floor() as i32,
		)
	}

	pub fn center(self) -> Vec3 {
		(self.min + self.max) * 0.5
	}

	pub fn aabb(self) -> Aabb3d {
		Aabb3d::from_min_max(self.min, self.max)
	}

	pub fn id(self) -> Id {
		Id::from_cell(self.aabb())
	}

	pub fn index(self) -> (i32, i32) {
		Self::cell_index_containing(self.center())
	}
}

#[derive(Clone, Debug)]
pub struct MobCell {
	pub extent: MobCellExtent,
	pub groups: Vec<MobGroup>,
}

#[derive(Clone)]
struct StoredMobCell {
	value: MobCell,
	bounds: Aabb3d,
	version: Version,
}

#[derive(Resource, Clone)]
pub struct MobIndex {
	next_version: u64,
	cells: HashMap<Id, StoredMobCell>,
	forest_noise: NoiseParams,
	forest_layering: Option<LayeringKind>,
	pub(crate) urbanization_noise: NoiseParams,
	pub(crate) urbanization_kind: Option<UrbanizationKind>,
	layers_at: ForestLayersAt,
	kind_at: UrbanKindAt,
	pub(crate) models_ready: bool,
	pub(crate) plant_hosts: Vec<MobPlantHost>,
}

impl Default for MobIndex {
	fn default() -> Self {
		Self {
			next_version: 0,
			cells: HashMap::new(),
			forest_noise: NoiseParams::default(),
			forest_layering: None,
			urbanization_noise: NoiseParams::default(),
			urbanization_kind: None,
			layers_at: idle_layers,
			kind_at: idle_kind,
			models_ready: false,
			plant_hosts: Vec::new(),
		}
	}
}

impl MobIndex {
	#[cfg(test)]
	pub(crate) fn ready() -> Self {
		Self {
			models_ready: true,
			layers_at: chico_layers_at,
			kind_at: richmond_kind_at,
			..Self::default()
		}
	}

	#[cfg(test)]
	pub(crate) fn ready_frontier(plant_hosts: Vec<MobPlantHost>) -> Self {
		Self {
			models_ready: true,
			urbanization_kind: Some(UrbanizationKind::Frontier),
			plant_hosts,
			layers_at: chico_layers_at,
			kind_at: richmond_kind_at,
			..Self::default()
		}
	}

	fn next_version(&mut self) -> Version {
		self.next_version += 1;
		Version(self.next_version)
	}

	pub(crate) fn configure_from(
		&mut self,
		forest_noise: NoiseParams,
		forest_layering: Option<LayeringKind>,
		urbanization_noise: NoiseParams,
		urbanization_kind: Option<UrbanizationKind>,
		layers_at: ForestLayersAt,
		kind_at: UrbanKindAt,
	) {
		self.forest_noise = forest_noise;
		self.forest_layering = forest_layering;
		self.urbanization_noise = urbanization_noise;
		self.urbanization_kind = urbanization_kind;
		self.layers_at = layers_at;
		self.kind_at = kind_at;
		self.models_ready = true;
	}

	/// Write one cell the scheme owns. [`crate::MobCellWrites::insert`]
	/// announces it; membership only drives stale removal.
	pub(crate) fn insert_cell(&mut self, cell: MobCell) -> Id {
		let id = cell.extent.id();
		let bounds = cell.extent.aabb();
		let version = self.next_version();
		self.cells.insert(id, StoredMobCell { value: cell, bounds, version });
		id
	}

	/// Drop one cell the scheme owns. Bumps membership so presenters retire it.
	pub(crate) fn remove_cell(&mut self, id: Id) -> Option<MobCell> {
		let removed = self.cells.remove(&id).map(|entry| entry.value);
		if removed.is_some() {
			let _ = self.next_version();
		}
		removed
	}

	/// Drop every cell. Bumps membership when the index was not already empty.
	pub fn clear(&mut self) {
		if self.cells.is_empty() {
			return;
		}
		self.cells.clear();
		let _ = self.next_version();
	}

	pub fn is_empty(&self) -> bool {
		self.cells.is_empty()
	}

	pub(crate) fn models_match(
		&self,
		forest_noise: NoiseParams,
		forest_layering: Option<LayeringKind>,
		urbanization_noise: NoiseParams,
		urbanization_kind: Option<UrbanizationKind>,
	) -> bool {
		self.models_ready
			&& self.forest_noise == forest_noise
			&& self.forest_layering == forest_layering
			&& self.urbanization_noise == urbanization_noise
			&& self.urbanization_kind == urbanization_kind
	}

	fn vegetation_at(&self, xz: Vec2) -> f32 {
		(self.layers_at)(self.forest_noise, self.forest_layering, xz) as f32 / 4.0
	}

	fn urbanization_kind_at(&self, xz: Vec2) -> UrbanizationKind {
		(self.kind_at)(self.urbanization_noise, self.urbanization_kind, xz)
	}

	fn group_kind_at(&self, xz: Vec2, seed: u64) -> GroupKind {
		match self.urbanization_kind_at(xz) {
			UrbanizationKind::None => GroupKind::Wild,
			UrbanizationKind::RuralLife => GroupKind::Peaceful,
			UrbanizationKind::Townships | UrbanizationKind::Frontier => GroupKind::Frontier,
			UrbanizationKind::Colony => GroupKind::Warfront,
			UrbanizationKind::MixedAgeCity => {
				if mixed(seed) & 1 == 0 {
					GroupKind::Warfront
				} else {
					GroupKind::Dystopian
				}
			}
			UrbanizationKind::ModernCity => GroupKind::Dystopian,
		}
	}
}

impl MobWorldSample for MobIndex {
	fn sample_mobs(&self, xz: Vec2) -> MobEnvironmentSample {
		let vegetation = self.vegetation_at(xz);
		let urbanization = match self.urbanization_kind_at(xz) {
			UrbanizationKind::None => 0.0,
			UrbanizationKind::RuralLife => 0.2,
			UrbanizationKind::Frontier => 0.4,
			UrbanizationKind::Townships => 0.55,
			UrbanizationKind::Colony => 0.7,
			UrbanizationKind::MixedAgeCity => 0.85,
			UrbanizationKind::ModernCity => 1.0,
		};
		MobEnvironmentSample { elevation: Some(0.0), urbanization, vegetation }
	}
}

impl MobWorldHosts for MobIndex {
	fn plant_hosts(&self, origin: Vec2, extent: f32) -> Vec<MobPlantHost> {
		let half = extent * 0.5;
		self.plant_hosts
			.iter()
			.copied()
			.filter(|host| {
				(host.xz.x - origin.x).abs() <= half + host.arrival_radius
					&& (host.xz.y - origin.y).abs() <= half + host.arrival_radius
			})
			.collect()
	}
}

impl SpatialIndex<MobCell> for MobIndex {
	fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId> {
		self.cells
			.iter()
			.filter(|(_, entry)| region.intersects(&entry.bounds))
			.map(|(id, _)| TrackedId(*id))
			.collect()
	}

	fn storage_status(&self, id: Id) -> StorageStatus {
		if self.cells.contains_key(&id) {
			StorageStatus::TrackedWithin
		} else {
			StorageStatus::NotTracked
		}
	}

	fn get(&self, id: Id) -> Option<&MobCell> {
		self.cells.get(&id).map(|entry| &entry.value)
	}

	fn get_bounds(&self, id: Id) -> Option<Aabb3d> {
		self.cells.get(&id).map(|entry| entry.bounds)
	}

	fn version(&self, id: Id) -> Option<Version> {
		self.cells.get(&id).map(|entry| entry.version)
	}

	fn membership_revision(&self) -> u64 {
		self.next_version
	}

	fn insert(&mut self, id: Id, value: MobCell, bounds: Aabb3d) {
		let version = self.next_version();
		self.cells.insert(id, StoredMobCell { value, bounds, version });
	}
}

impl GenerationScheme<MobIndex> for MobCell {
	fn original_ids_for(index: &mut MobIndex, region: Aabb3d) -> Vec<OriginalId> {
		if !index.models_ready {
			return Vec::new();
		}
		MobCellExtent::cells_overlapping(region)
			.into_iter()
			.map(|extent| OriginalId(extent.id()))
			.collect()
	}

	fn build_with_id(index: &mut MobIndex, id: Id) -> Option<(Self, Aabb3d)> {
		if !index.models_ready {
			return None;
		}
		let extent = MobCellExtent::from_id(id)?;
		let (ix, iz) = extent.index();
		let seed = cell_seed(ix, iz);
		let occupied = (ix == 0 && iz == 0)
			|| mixed(seed ^ 0x6d6f_622d_6365_6c6c) % 100 < MOB_CELL_OCCUPANCY_PERCENT;
		let groups = if occupied {
			let origin = Vec2::new(extent.center().x, extent.center().z);
			let kind = index.group_kind_at(origin, seed);
			vec![MobGroup::generate(kind, seed, origin, index)]
		} else {
			Vec::new()
		};
		Some((Self { extent, groups }, extent.aabb()))
	}
}

pub fn cell_seed(ix: i32, iz: i32) -> u64 {
	mixed(MOB_WORLD_SEED ^ (ix as u32 as u64).rotate_left(17) ^ (iz as u32 as u64).rotate_left(43))
}

pub fn mixed(mut value: u64) -> u64 {
	value ^= value >> 30;
	value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
	value ^= value >> 27;
	value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
	value ^ (value >> 31)
}

pub fn xz_radius_aabb(center: Vec3, radius: f32) -> Aabb3d {
	Aabb3d::from_min_max(
		Vec3::new(center.x - radius, 0.0, center.z - radius),
		Vec3::new(center.x + radius, 1.0, center.z + radius),
	)
}

pub fn urban_leaf_arrival_radius(bounds: Aabb3d) -> f32 {
	((bounds.max.x - bounds.min.x).min(bounds.max.z - bounds.min.z) * 0.25).clamp(8.0, 128.0)
}
