use crate::gen::generation::{BuildWithIdLod, GeneratingSpatialIndex, MaterializeStatus};
use crate::gen::id::Id;
use crate::gen::scene::{LodScene, SceneSpawner};
use crate::gen::spatial_index::{BaseSpatialIndex, SpatialIndex};
use crate::lod_ref::LodRef;
use bevy::math::bounding::Aabb3d;
use std::collections::HashSet;
use std::marker::PhantomData;

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
	fn tracked_ids_for(&self, region: Aabb3d) -> Vec<crate::gen::id::TrackedId> {
		self.spatial_index().tracked_ids_for(region)
	}

	fn storage_status(&self, id: Id) -> crate::gen::id::StorageStatus {
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
