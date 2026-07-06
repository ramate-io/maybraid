//! LOD spatial loading sketch.
//!
//! Main idea:
//!
//! - `SpatialIndex<T>` owns spatial truth.
//! - `GeneratingSpatialIndex<T>` materializes missing values.
//! - `SceneLoader` is middleware over generation that can also spawn/heal.
//!
//! Pitfall avoided:
//! spawning from `insert()` is too implicit. It can make descendant generation
//! accidentally spawn visuals and can miss moved assets that need healing first.

use crate::lod_ref::LodRef;
use bevy::{math::bounding::Aabb3d, scene::Scene};
use std::collections::HashSet;
use std::marker::PhantomData;

// -----------------------------------------------------------------------------
// IDs
// -----------------------------------------------------------------------------

/// Spatial cell bounds for procedural origin ids.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell(pub Aabb3d);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OriginCell(pub Cell);

impl Eq for Cell {}

impl core::hash::Hash for Cell {
	fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
		self.0.min.x.to_bits().hash(state);
		self.0.min.y.to_bits().hash(state);
		self.0.min.z.to_bits().hash(state);
		self.0.max.x.to_bits().hash(state);
		self.0.max.y.to_bits().hash(state);
		self.0.max.z.to_bits().hash(state);
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bytes(pub [u8; 32]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Id {
	/// Some entities have a custom ID.
	Bytes(Bytes),

	/// Some entities, particularly procedural ones, are identified by their
	/// origin cell.
	OriginCell(OriginCell),
}

impl Id {
	pub fn from_cell(bounds: Aabb3d) -> Self {
		Self::OriginCell(OriginCell(Cell(bounds)))
	}

	pub fn origin_cell_bounds(self) -> Option<Aabb3d> {
		match self {
			Self::OriginCell(OriginCell(Cell(bounds))) => Some(bounds),
			Self::Bytes(_) => None,
		}
	}
}

/// Ids that originate in the region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OriginalId(pub Id);

/// Ids that are tracked in the region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TrackedId(pub Id);

/// Whether or not a given id is tracked in the region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StorageStatus {
	NotTracked,
	TrackedWithin,
	TrackedOutside,
}

// -----------------------------------------------------------------------------
// Spatial index
// -----------------------------------------------------------------------------

pub trait BaseSpatialIndex<T> {
	fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId>;

	fn storage_status(&self, id: Id) -> StorageStatus;

	fn get(&self, id: Id) -> Option<&T>;

	fn get_bounds(&self, id: Id) -> Option<Aabb3d>;

	fn get_with_bounds(&self, id: Id) -> Option<(&T, Aabb3d)> {
		self.get(id).and_then(|t| self.get_bounds(id).map(|b| (t, b)))
	}

	/// Inserts the type into the spatial index. Must not spawn scenes.
	fn insert(&mut self, id: Id, t: T, bounds: Aabb3d, lod_ref: &LodRef);
}

pub trait SpatialIndex<T>: BaseSpatialIndex<T> {
	fn original_ids_for(&mut self, region: Aabb3d) -> Vec<OriginalId>;

	fn all_ids_for(&mut self, region: Aabb3d) -> Vec<Id> {
		self.original_ids_for(region)
			.into_iter()
			.map(|id| id.0)
			.chain(self.tracked_ids_for(region).into_iter().map(|id| id.0))
			.collect()
	}

	fn fresh_ids_for(&mut self, region: Aabb3d) -> Vec<OriginalId> {
		self.original_ids_for(region)
			.into_iter()
			.filter(|id| self.storage_status(id.0) == StorageStatus::NotTracked)
			.collect()
	}

	fn ids_for(&mut self, region: Aabb3d) -> Vec<Id> {
		self.fresh_ids_for(region)
			.into_iter()
			.map(|id| id.0)
			.chain(self.tracked_ids_for(region).into_iter().map(|id| id.0))
			.collect()
	}

	fn deduplicated_ids_for(&mut self, region: Aabb3d) -> Vec<Id> {
		self.ids_for(region).into_iter().collect::<HashSet<_>>().into_iter().collect()
	}
}

pub trait OriginalIds<S> {
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId>;
}

impl<T, S> SpatialIndex<T> for S
where
	S: BaseSpatialIndex<T>,
	T: OriginalIds<S>,
{
	fn original_ids_for(&mut self, region: Aabb3d) -> Vec<OriginalId> {
		T::original_ids_for(self, region)
	}
}

// -----------------------------------------------------------------------------
// Generation
// -----------------------------------------------------------------------------

pub trait BuildWithIdLod<S>: Sized {
	fn build_with_id(spatial_index: &mut S, id: Id, lod_ref: &LodRef) -> Option<(Self, Aabb3d)>;

	fn descendants_with_lod(id: Id, spatial_index: &mut S, lod_ref: &LodRef);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MaterializeStatus {
	Existing,
	Created,
}

pub trait GeneratingSpatialIndex<T>: SpatialIndex<T>
where
	T: BuildWithIdLod<Self>,
	Self: Sized,
{
	fn get_or_generate(&mut self, id: Id, lod_ref: &LodRef) -> Option<MaterializeStatus> {
		if self.get(id).is_some() {
			return Some(MaterializeStatus::Existing);
		}

		let (instance, bounds) = T::build_with_id(self, id, lod_ref)?;
		self.insert(id, instance, bounds, lod_ref);
		T::descendants_with_lod(id, self, lod_ref);

		Some(MaterializeStatus::Created)
	}

	fn get_or_generate_region(&mut self, region: Aabb3d, lod_ref: &LodRef) -> Vec<(Id, Aabb3d)> {
		let ids = self.deduplicated_ids_for(region);

		ids.into_iter()
			.filter_map(|id| {
				self.get_or_generate(id, lod_ref)?;
				self.get_bounds(id).map(|bounds| (id, bounds))
			})
			.collect()
	}
}

impl<T, S> GeneratingSpatialIndex<T> for S
where
	S: SpatialIndex<T>,
	T: BuildWithIdLod<S>,
{
}

// -----------------------------------------------------------------------------
// Scenes
// -----------------------------------------------------------------------------

/// The asset's local opinion about whether its scene changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScenePatchStatus {
	Changed,
	Unchanged,
}

pub trait LodScene {
	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static;

	fn scene_patch_status(&self, lod_ref: &LodRef) -> ScenePatchStatus;
}

pub trait SceneSpawner<T: LodScene> {
	fn spawn_or_patch_scene(
		&mut self,
		id: Id,
		materialize_status: MaterializeStatus,
		scene_status: ScenePatchStatus,
		scene: impl Scene,
		marker: PhantomData<T>,
	);

	fn heal_region(&mut self, region: Aabb3d, wanted: &HashSet<Id>, marker: PhantomData<T>);
}

// -----------------------------------------------------------------------------
// Scene loading middleware
// -----------------------------------------------------------------------------

pub trait SceneLoader: Sized {
	type Index: BaseSpatialIndex<Self::Asset> + SpatialIndex<Self::Asset>;
	type Asset: LodScene + BuildWithIdLod<Self> + BuildWithIdLod<Self::Index>;
	type Spawner: SceneSpawner<Self::Asset>;

	fn spatial_index(&self) -> &Self::Index;

	fn spatial_index_mut(&mut self) -> &mut Self::Index;

	fn spawner_mut(&mut self) -> &mut Self::Spawner;

	fn borrow_parts_mut(&mut self) -> (&mut Self::Index, &mut Self::Spawner);

	fn spawn_scene_for(&mut self, id: Id, materialize_status: MaterializeStatus, lod_ref: &LodRef) {
		let (scene_patch_status, scene) = {
			let Some(instance) = self.spatial_index().get(id) else {
				return;
			};
			(instance.scene_patch_status(lod_ref), instance.scene_with_lod(lod_ref))
		};

		self.spawner_mut().spawn_or_patch_scene(
			id,
			materialize_status,
			scene_patch_status,
			scene,
			PhantomData::<Self::Asset>,
		);
	}

	fn heal_region(&mut self, region: Aabb3d, wanted: &HashSet<Id>) {
		self.spawner_mut()
			.heal_region(region, wanted, PhantomData::<Self::Asset>);
	}

	fn get_or_generate(&mut self, id: Id, lod_ref: &LodRef) -> Option<MaterializeStatus> {
		let status = if self.spatial_index().get(id).is_some() {
			MaterializeStatus::Existing
		} else {
			let (instance, bounds) = Self::Asset::build_with_id(self, id, lod_ref)?;
			self.spatial_index_mut().insert(id, instance, bounds, lod_ref);
			MaterializeStatus::Created
		};

		self.spawn_scene_for(id, status, lod_ref);
		Self::Asset::descendants_with_lod(id, self, lod_ref);

		Some(status)
	}

	fn get_or_generate_region(&mut self, region: Aabb3d, lod_ref: &LodRef) -> Vec<(Id, Aabb3d)> {
		let ids = self.spatial_index_mut().deduplicated_ids_for(region);
		let wanted = ids.iter().copied().collect::<HashSet<_>>();

		self.heal_region(region, &wanted);

		let mut loaded = Vec::new();

		for id in ids {
			if self.get_or_generate(id, lod_ref).is_some() {
				if let Some(bounds) = self.spatial_index().get_bounds(id) {
					loaded.push((id, bounds));
				}
			}
		}

		loaded
	}
}

// -----------------------------------------------------------------------------
// Middleware helpers
// -----------------------------------------------------------------------------

pub trait Materialize<T> {
	fn materialize(&mut self, id: Id, lod_ref: &LodRef) -> Option<MaterializeStatus>;
}

impl<T, S> Materialize<T> for S
where
	S: GeneratingSpatialIndex<T>,
	T: BuildWithIdLod<S>,
{
	fn materialize(&mut self, id: Id, lod_ref: &LodRef) -> Option<MaterializeStatus> {
		self.get_or_generate(id, lod_ref)
	}
}

impl<L> BaseSpatialIndex<L::Asset> for L
where
	L: SceneLoader,
{
	fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId> {
		self.spatial_index().tracked_ids_for(region)
	}

	fn storage_status(&self, id: Id) -> StorageStatus {
		self.spatial_index().storage_status(id)
	}

	fn get(&self, id: Id) -> Option<&L::Asset> {
		self.spatial_index().get(id)
	}

	fn get_bounds(&self, id: Id) -> Option<Aabb3d> {
		self.spatial_index().get_bounds(id)
	}

	fn insert(&mut self, id: Id, t: L::Asset, bounds: Aabb3d, lod_ref: &LodRef) {
		self.spatial_index_mut().insert(id, t, bounds, lod_ref);
	}
}

#[cfg(test)]
mod tests;
