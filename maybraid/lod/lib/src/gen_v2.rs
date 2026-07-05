//! LOD spatial loading sketch.
//!
//! Main idea:
//!
//! - `SpatialIndex<T>` owns spatial truth.
//! - `GeneratingSpatialIndex<T>` materializes missing values.
//! - `SceneLoader<T>` is middleware over generation that can also spawn/heal.
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

	/// Some entities, particularly procedural ones, are identified by their
	/// origin cell.
	OriginCell(OriginCell),
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
	/// Produces the tracked ids for the type in the given region.
	///
	/// These are ids whose current bounds put them in the region.
	fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId>;

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
	/// Important:
	/// this should only mutate the spatial index. It should not spawn scenes.
	///
	/// Pitfall avoided:
	/// hiding scene spawning inside insertion makes it hard to heal moved ids
	/// and hard to control whether descendants should merely be generated or
	/// also visually presented.
	fn insert(&mut self, id: Id, t: T, bounds: Aabb3d, lod_ref: &LodRef);
}

pub trait SpatialIndex<T>: BaseSpatialIndex<T> {
	/// Produce the ids stored or originating in the given region.
	///
	/// Needs to be mutable because we allow generating and inserting
	/// dependencies that determine original ids.
	///
	/// This returns a `Vec` rather than `impl Iterator` because the operation may
	/// borrow/mutate the spatial index. Returning lazy iterators from mutating
	/// methods tends to produce long-lived borrows and painful borrow-checker
	/// failures.
	fn original_ids_for(&mut self, region: Aabb3d) -> Vec<OriginalId>;

	/// All ids that are original or tracked in the region.
	fn all_ids_for(&mut self, region: Aabb3d) -> Vec<Id> {
		self.original_ids_for(region)
			.into_iter()
			.map(|id| id.0)
			.chain(self.tracked_ids_for(region).into_iter().map(|id| id.0))
			.collect()
	}

	/// The ids which originate in a region and are not tracked elsewhere.
	fn fresh_ids_for(&mut self, region: Aabb3d) -> Vec<OriginalId> {
		self.original_ids_for(region)
			.into_iter()
			.filter(|id| self.storage_status(id.0) == StorageStatus::NotTracked)
			.collect()
	}

	/// All ids which are either fresh or tracked in the region.
	fn ids_for(&mut self, region: Aabb3d) -> Vec<Id> {
		self.fresh_ids_for(region)
			.into_iter()
			.map(|id| id.0)
			.chain(self.tracked_ids_for(region).into_iter().map(|id| id.0))
			.collect()
	}

	/// Deduplicated ids which are either fresh or tracked in the region.
	fn deduplicated_ids_for(&mut self, region: Aabb3d) -> Vec<Id> {
		self.ids_for(region).into_iter().collect::<HashSet<_>>().into_iter().collect()
	}
}

/// Reverses the trait requirements for producing original ids
/// s.t. downstream types can determine how to produce original ids over a given
/// spatial index.
pub trait OriginalIds<S> {
	/// Produces the original ids for a given spatial index.
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

/// The LOD specific build path.
///
/// In theory, we could just make a single build path and use marker types to
/// differentiate implementation behavior.
pub trait BuildWithIdLod<S>: Sized {
	/// Builds the instance.
	fn build_with_id(spatial_index: &mut S, id: Id, lod_ref: &LodRef) -> Option<(Self, Aabb3d)>;

	/// Builds and populates the spatial index with any descendants for the given
	/// LOD.
	///
	/// This receives `S`, which may be either a raw spatial index or a scene
	/// loading middleware. That is the key trick: descendants can choose to call
	/// back into the middleware path when they should also be spawned/healed.
	fn descendants_with_lod(&self, spatial_index: &mut S, lod_ref: &LodRef);
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
	/// Generates the type for the given id, if missing.
	///
	/// This should populate the spatial index but should not itself imply visual
	/// spawning unless `Self` is a middleware that intentionally adds that
	/// behavior.
	fn get_or_generate(&mut self, id: Id, lod_ref: &LodRef) -> Option<MaterializeStatus> {
		if self.get(id).is_some() {
			return Some(MaterializeStatus::Existing);
		}

		let (instance, bounds) = T::build_with_id(self, id, lod_ref)?;
		self.insert(id, instance, bounds, lod_ref);

		// Pitfall avoided:
		// do not implement this with `self.get(id).or_else(|| { ... })`.
		// The immutable borrow from `get` and mutable borrow inside the closure
		// will usually fight each other.
		//
		// We insert first, then reborrow.
		let instance = self.get(id)?;
		instance.descendants_with_lod(self, lod_ref);

		Some(MaterializeStatus::Created)
	}

	/// Gets or generates the region.
	///
	/// Returns ids/bounds rather than `&T` to avoid holding references into the
	/// index while the caller may want to keep mutating the loader/spawner.
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

pub enum SceneWithLod<S> {
	/// The asset believes the scene has changed and should be spawned or patched.
	Changed(S),

	/// The asset believes the scene is unchanged.
	///
	/// This still carries the scene because the spawner may know this does not
	/// match runtime reality. For example, the spawner may track ids and where
	/// their scenes are currently spawned. If an id was healed away, moved to a
	/// different region, or newly materialized, `Unchanged` is only the asset's
	/// local opinion; the spawner may still need the scene to spawn, reparent, or
	/// repair the ECS state.
	Unchanged(S),
}

pub trait LodScene {
	type Scene: Scene;

	/// Produces a scene for the given lod reference, along with the asset's local
	/// opinion about whether the scene changed.
	///
	/// Correctness should not depend entirely on this method. The spawner also
	/// receives `MaterializeStatus` and may maintain its own runtime id/entity
	/// tracking, so it can correct mismatches between the asset's opinion and
	/// the actual ECS world.
	fn scene_with_lod(&self, lod_ref: &LodRef) -> SceneWithLod<Self::Scene>;
}

pub trait SceneSpawner<T: LodScene> {
	/// Spawns or patches a scene for the id.
	///
	/// The spawner receives both statuses:
	///
	/// - `MaterializeStatus`: whether the spatial value was newly created or
	///   already existed.
	/// - `SceneWithLod`: whether the asset believes its scene changed.
	///
	/// The spawner should treat these as inputs, not commands. Since the spawner
	/// may track which ids are currently spawned and under which parent/region,
	/// it can decide that `Unchanged(scene)` still needs to be spawned, moved, or
	/// repaired.
	fn spawn_or_patch_scene(
		&mut self,
		id: Id,
		materialize_status: MaterializeStatus,
		scene_status: SceneWithLod<T::Scene>,
		marker: PhantomData<T>,
	);

	/// Heals the current visual representation of a region.
	///
	/// The usual implementation checks the ECS tree/runtime state for the region
	/// and removes or detaches visual entities whose ids are no longer wanted.
	///
	/// This is what handles moved assets:
	///
	/// - the old region sees the id is no longer wanted and removes it
	/// - the new region sees the id is wanted and spawns/patches it
	fn heal_region(&mut self, region: Aabb3d, wanted: &HashSet<Id>, marker: PhantomData<T>);
}

// -----------------------------------------------------------------------------
// Scene loading middleware
// -----------------------------------------------------------------------------

/// Scene loader serves as middleware over the spatial index.
///
/// Unlike the earlier design, this does not override `insert()` to spawn scenes.
/// Instead, spawning and healing happen explicitly in `get_or_generate`,
/// `spawn_existing`, and `get_or_generate_region`.
pub trait SceneLoader<T, Index, Spawner>
where
	T: LodScene,
	Index: GeneratingSpatialIndex<T>,
	Spawner: SceneSpawner<T>,
{
	fn spatial_index(&self) -> &Index;

	fn spatial_index_mut(&mut self) -> &mut Index;

	fn spawner_mut(&mut self) -> &mut Spawner;

	fn borrow_parts_mut(&mut self) -> (&mut Index, &mut Spawner);

	/// Attempts to spawn or patch an already-materialized id.
	fn spawn_existing(&mut self, id: Id, materialize_status: MaterializeStatus, lod_ref: &LodRef) {
		let (index, spawner) = self.borrow_parts_mut();

		let Some(instance) = index.get(id) else {
			return;
		};

		let scene_status = instance.scene_with_lod(lod_ref);

		spawner.spawn_or_patch_scene(id, materialize_status, scene_status, PhantomData::<T>);
	}

	/// Heals the visual state for a region against the ids that should currently
	/// be present in that region.
	fn heal_region(&mut self, region: Aabb3d, wanted: &HashSet<Id>) {
		self.spawner_mut().heal_region(region, wanted, PhantomData::<T>);
	}

	/// Gets or generates an id, then gives the spawner a chance to present it.
	///
	/// This is the middleware point that replaces the old `insert()` override.
	///
	/// Important:
	/// even if `scene_with_lod` returns `Unchanged(scene)`, the spawner also gets
	/// the materialization status and can compare against its own id/entity
	/// tracking. This is what lets the world heal from stale or missing visuals
	/// without forcing every `LodScene` implementation to know global runtime
	/// state.
	fn get_or_generate_and_spawn(&mut self, id: Id, lod_ref: &LodRef) -> Option<MaterializeStatus> {
		let status = self.spatial_index_mut().get_or_generate(id, lod_ref)?;
		self.spawn_existing(id, status, lod_ref);
		Some(status)
	}

	/// Gets or generates a region, heals stale visuals, and spawns/patches wanted
	/// visuals.
	fn get_or_generate_region_and_spawn(
		&mut self,
		region: Aabb3d,
		lod_ref: &LodRef,
	) -> Vec<(Id, Aabb3d)> {
		let ids = self.spatial_index_mut().deduplicated_ids_for(region);
		let wanted = ids.iter().copied().collect::<HashSet<_>>();

		// Healing happens before spawning so moved ids are removed from old
		// parents/regions before they are inserted under the correct region.
		self.heal_region(region, &wanted);

		let mut loaded = Vec::new();

		for id in ids {
			self.get_or_generate_and_spawn(id, lod_ref);

			if let Some(bounds) = self.spatial_index().get_bounds(id) {
				loaded.push((id, bounds));
			}
		}

		loaded
	}
}

// -----------------------------------------------------------------------------
// Middleware delegation
// -----------------------------------------------------------------------------

impl<T, Index, Spawner, Loader> BaseSpatialIndex<T> for Loader
where
	T: LodScene + BuildWithIdLod<Loader>,
	Index: GeneratingSpatialIndex<T>,
	Spawner: SceneSpawner<T>,
	Loader: SceneLoader<T, Index, Spawner>,
{
	fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId> {
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

	/// Inserts into the underlying spatial index only.
	///
	/// Important:
	/// even for the middleware, insertion does not spawn. Spawning is explicit
	/// through `get_or_generate_and_spawn` or region loading.
	fn insert(&mut self, id: Id, t: T, bounds: Aabb3d, lod_ref: &LodRef) {
		self.spatial_index_mut().insert(id, t, bounds, lod_ref);
	}
}
