use crate::lod_ref::LodRef;
use bevy::{math::bounding::Aabb3d, scene::Scene};
use std::collections::HashSet;
use std::marker::PhantomData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cell {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OriginCell(pub Cell);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bytes(pub [u8; 32]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Id {
	/// Some entities have a custom ID.
	Bytes(Bytes),
	/// Some entities (particularly procedural ones), are identified by their origin cell.
	OriginCell(OriginCell),
}

/// Ids the originate in the region
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OriginalId(pub Id);

/// Ids that are tracked in th region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TrackedId(pub Id);

/// Whether or not a given id is tracked in the region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StorageStatus {
	NotTracked,
	TrackedWithin,
	TrackedOutside,
}

pub trait BaseSpatialIndex<T> {
	/// Produces the tracked ids for the type in the given region.
	fn tracked_ids_for(&self, region: Aabb3d) -> impl Iterator<Item = TrackedId>;

	/// Whether or not a given id is tracked in the region.
	fn storage_status(&self, id: Id) -> StorageStatus;

	/// Gets the type for a given id.
	fn get(&self, id: Id) -> Option<&T>;

	/// Gets the bounds for a given id.
	fn get_bounds(&self, id: Id) -> Option<Aabb3d>;

	/// Gets the type for a given id with bounds.
	fn get_with_bounds(&self, id: Id) -> Option<(&T, Aabb3d)> {
		self.get(id).and_then(|t| self.get_bounds(id).map(|b| (t, b)))
	}

	/// Inserts the type into the spatial index.
	///
	/// For the most part, the developer will ignore the lod ref.
	/// However, scene spawners may use it to determine how to spawn the scene.
	fn insert(&mut self, id: Id, t: T, bounds: Aabb3d, lod_ref: &LodRef);
}

pub trait SpatialIndex<T>: BaseSpatialIndex<T> {
	/// Produce the ids stored or originating in the given region.
	///
	/// Needs to be mut because we allow generating and inserting dependencies that determine
	/// original ids.
	fn original_ids_for(&mut self, region: Aabb3d) -> impl Iterator<Item = OriginalId>;

	/// All ids that are orignal or tracked in the region.
	///
	/// We may want to add deduplication here. But, right now we give a specific method.
	fn all_ids_for(&mut self, region: Aabb3d) -> impl Iterator<Item = Id> {
		self.original_ids_for(region)
			.map(|id| id.0)
			.chain(self.tracked_ids_for(region).map(|id| id.0))
	}

	/// The ids which originate in a region and are not tracked elsewhere.
	fn fresh_ids_for(&mut self, region: Aabb3d) -> impl Iterator<Item = OriginalId> {
		self.original_ids_for(region)
			.filter(|id| self.storage_status(id.0) == StorageStatus::NotTracked)
	}

	/// All ids which are either fresh or tracked in the region.
	fn ids_for(&mut self, region: Aabb3d) -> impl Iterator<Item = Id> {
		self.fresh_ids_for_region(region)
			.map(|id| id.0)
			.chain(self.tracked_ids_for(region).map(|id| id.0))
	}

	/// Deduplicated ids which are either fresh or tracked in the region.
	fn deduplicated_ids_for(&mut self, region: Aabb3d) -> impl Iterator<Item = Id> {
		self.fresh_ids_for(region)
			.map(|id| id.0)
			.chain(self.tracked_ids_for(region).map(|id| id.0))
			.collect::<HashSet<_>>()
			.into_iter()
	}
}

/// Reverses the trait requirements for producing original ids
/// s.t. downstream types can determine how to produce
/// original ids over a given spatial index.
pub trait OriginalIds<S> {
	/// Produces the original ids for a given spatial index.
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> impl Iterator<Item = OriginalId>;
}

impl<T, S> SpatialIndex<T> for S
where
	S: BaseSpatialIndex<T>,
	T: OriginalIds<S>,
{
	fn original_ids_for(&mut self, region: Aabb3d) -> impl Iterator<Item = OriginalId> {
		T::original_ids_for(self, region)
	}
}

/// The LOD specific build path.
///
/// In theory, we could just make a single build path
/// and use marker types to differentiate implementation behavior.
pub trait BuildWithIdLod<S>: Sized {
	/// Builds the instance.
	fn build_with_id(spatial_index: &mut S, id: Id) -> Option<(Self, Aabb3d)>;

	/// Builds and populates the spatial index with any descendants for the give LOD
	fn descendants_with_lod(&self, spatial_index: &mut S, lod_ref: &LodRef);
}

pub trait LodScene {
	/// Produces a scene for the given lod reference.
	fn scene_for_lod(&self, lod_ref: &LodRef) -> impl Scene;
}

pub trait GeneratingSpatialIndex<T>: SpatialIndex<T> {
	/// Generates the type for the given id.
	fn get_or_generate(&mut self, id: Id, lod_ref: &LodRef) -> Option<&T>;

	/// Gets or generates the region
	fn get_or_generate_region(
		&mut self,
		region: Aabb3d,
		lod_ref: &LodRef,
	) -> impl Iterator<Item = (Id, &T, Aabb3d)> {
		self.deduplicated_ids_for(region)
			.map(|id| {
				self.get_or_generate(id, lod_ref)
					.and_then(|t| self.get_bounds(id).map(|b| (id, t, b)))
			})
			.filter_map(|x| x)
	}
}

impl<T, S> GeneratingSpatialIndex<T> for S
where
	S: SpatialIndex<T>,
	T: BuildWithIdLod<S>,
{
	fn get_or_generate(&mut self, id: Id, lod_ref: &LodRef) -> Option<&T> {
		self.get(id).or_else(|| {
			let (instance, bounds) = T::build_with_id(self, id)?;
			self.insert(id, instance, bounds, &lod_ref);

			// It'd be nice if we could find a way to short-circuit this.
			// But, we need the instance in the spatial index
			// in order for the spatial index to populate the dependencies.
			if let Some(instance) = self.get(id) {
				instance.descendants_with_lod(self, lod_ref);
				return Some(instance);
			} else {
				None
			}
		})
	}
}

pub trait SceneSpawner<T> {
	/// Do we need this phanton data or should we just use as expressions?
	fn spawn_scene(&mut self, id: Id, scene: impl Scene, marker: PhantomData<T>);
}

/// Scene loader mostly serves as middleware on the spatial index
/// to load the scene in via the spawner.
pub trait SceneLoader<
	T: LodScene,
	SpatialIndex: GeneratingSpatialIndex<T>,
	Spawner: SceneSpawner<T>,
>
{
	fn spatial_index(&self) -> &SpatialIndex;

	fn spatial_index_mut(&mut self) -> &mut SpatialIndex;

	fn borrow_parts_mut(&mut self) -> (&mut SpatialIndex, &mut Spawner);
}

impl<
		T: LodScene,
		SpatialIndex: GeneratingSpatialIndex<T> + 'static,
		Spawner: SceneSpawner<T> + 'static,
		Loader: SceneLoader<T, SpatialIndex, Spawner>,
	> BaseSpatialIndex<T> for Loader
{
	fn tracked_ids_for(&self, region: Aabb3d) -> impl Iterator<Item = TrackedId> {
		self.spatial_index().tracked_ids_for(region)
	}

	fn storage_status(&self, id: Id) -> StorageStatus {
		self.spatial_index().storage_status(id)
	}

	fn get(&self, id: Id) -> Option<&T> {
		self.spatial_index().get(id)
	}

	fn get_bounds(&self, id: Id) -> Option<Aabb3d> {
		self.spatial_index().get_bounds(id)
	}

	/// Overwrites the insertion system to use the scene spawner.
	///
	/// This means that get_or_generate and get_or_generate_region will use the scene spawner to load the scene.
	fn insert(&mut self, id: Id, t: T, bounds: Aabb3d, lod_ref: &LodRef) {
		let (spatial_index, spawner) = self.borrow_parts_mut();
		spawner.spawn_scene(id, t.scene_for_lod(lod_ref), PhantomData::<T>);
		spatial_index.insert(id, t, bounds, lod_ref);
	}
}
