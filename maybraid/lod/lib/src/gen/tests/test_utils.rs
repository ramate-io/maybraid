use crate::gen::{
	BaseSpatialIndex, BuildWithIdLod, GeneratingSpatialIndex, Id, LodScene, Materialize,
	MaterializeStatus, OriginalId, OriginalIds, SceneLoader, ScenePatchStatus, SceneSpawner,
	StorageStatus, TrackedId,
};
use crate::lod_ref::LodRef;
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::scene::{ResolveContext, ResolvedScene, Scene, SceneFunction};
use bevy::{math::Vec3, prelude::Entity};
use std::collections::HashMap;
use std::marker::PhantomData;

pub fn empty_scene(_: &mut ResolveContext, _: &mut ResolvedScene) {}

pub fn stub_scene() -> impl Scene + 'static {
	SceneFunction(empty_scene)
}

pub fn cell(x: f32) -> Aabb3d {
	Aabb3d::from_min_max(Vec3::new(x, 0.0, 0.0), Vec3::new(x + 1.0, 1.0, 1.0))
}

pub fn tree_id(veg_id: Id) -> Id {
	child_id(veg_id, 1)
}

pub fn leaf_id(tree: Id) -> Id {
	child_id(tree, 2)
}

pub fn moss_id(leaf: Id) -> Id {
	child_id(leaf, 3)
}

fn child_id(parent: Id, depth: u8) -> Id {
	match parent {
		Id::OriginCell(crate::gen::OriginCell(crate::gen::Cell(bounds))) => {
			let mut bytes = [0u8; 32];
			bytes[0] = bounds.min.x as u8;
			bytes[1] = depth;
			Id::Bytes(crate::gen::Bytes(bytes))
		}
		Id::Bytes(bytes) => {
			let mut child = bytes.0;
			child[1] = depth;
			Id::Bytes(crate::gen::Bytes(child))
		}
	}
}

pub struct TestLod {
	pub entity: Entity,
	prev: bevy::prelude::Transform,
	cur: bevy::prelude::Transform,
	pub bounds: Aabb3d,
}

impl TestLod {
	pub fn new(bounds: Aabb3d) -> Self {
		Self {
			entity: Entity::from_raw_u32(1).expect("test entity"),
			prev: bevy::prelude::Transform::IDENTITY,
			cur: bevy::prelude::Transform::IDENTITY,
			bounds,
		}
	}

	pub fn lod_ref(&self) -> LodRef<'_> {
		LodRef {
			entity: self.entity,
			previous_transform: &self.prev,
			current_transform: &self.cur,
			bounds: &self.bounds,
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct Terrain {
	pub cell: Aabb3d,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Vegetation {
	pub cell: Aabb3d,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tree {
	pub parent: Id,
	pub cell: Aabb3d,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Leaf {
	pub parent: Id,
	pub cell: Aabb3d,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Moss {
	pub parent: Id,
	pub cell: Aabb3d,
}

macro_rules! impl_lod_scene {
	($ty:ty) => {
		impl LodScene for $ty {
			fn scene_with_lod(&self, _lod_ref: &LodRef) -> impl Scene + 'static {
				stub_scene()
			}

			fn scene_patch_status(&self, _lod_ref: &LodRef) -> ScenePatchStatus {
				ScenePatchStatus::Unchanged
			}
		}
	};
}

impl_lod_scene!(Terrain);
impl_lod_scene!(Vegetation);
impl_lod_scene!(Tree);
impl_lod_scene!(Leaf);
impl_lod_scene!(Moss);

#[derive(Default)]
pub struct WorldIndex {
	pub terrain: HashMap<Id, (Terrain, Aabb3d)>,
	pub vegetation: HashMap<Id, (Vegetation, Aabb3d)>,
	pub trees: HashMap<Id, (Tree, Aabb3d)>,
	pub leaves: HashMap<Id, (Leaf, Aabb3d)>,
	pub moss: HashMap<Id, (Moss, Aabb3d)>,
}

macro_rules! impl_base_spatial_index {
	($ty:ty, $field:ident) => {
		impl BaseSpatialIndex<$ty> for WorldIndex {
			fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId> {
				self.$field
					.iter()
					.filter(|(_, (_, bounds))| region.intersects(bounds))
					.map(|(id, _)| TrackedId(*id))
					.collect()
			}

			fn storage_status(&self, id: Id) -> StorageStatus {
				if self.$field.contains_key(&id) {
					StorageStatus::TrackedWithin
				} else {
					StorageStatus::NotTracked
				}
			}

			fn get(&self, id: Id) -> Option<&$ty> {
				self.$field.get(&id).map(|(value, _)| value)
			}

			fn get_bounds(&self, id: Id) -> Option<Aabb3d> {
				self.$field.get(&id).map(|(_, bounds)| *bounds)
			}

			fn insert(&mut self, id: Id, value: $ty, bounds: Aabb3d, _lod_ref: &LodRef) {
				self.$field.insert(id, (value, bounds));
			}
		}
	};
}

impl_base_spatial_index!(Terrain, terrain);
impl_base_spatial_index!(Vegetation, vegetation);
impl_base_spatial_index!(Tree, trees);
impl_base_spatial_index!(Leaf, leaves);
impl_base_spatial_index!(Moss, moss);

impl OriginalIds<WorldIndex> for Terrain {
	fn original_ids_for(_spatial_index: &mut WorldIndex, region: Aabb3d) -> Vec<OriginalId> {
		let min_x = region.min.x.floor() as i32;
		let max_x = region.max.x.ceil() as i32;
		(min_x..=max_x)
			.map(|x| OriginalId(Id::from_cell(cell(x as f32))))
			.filter(|OriginalId(id)| id.origin_cell_bounds().is_some_and(|b| region.intersects(&b)))
			.collect()
	}
}

impl OriginalIds<WorldIndex> for Vegetation {
	fn original_ids_for(spatial_index: &mut WorldIndex, region: Aabb3d) -> Vec<OriginalId> {
		Terrain::original_ids_for(spatial_index, region)
	}
}

impl OriginalIds<WorldIndex> for Tree {
	fn original_ids_for(_spatial_index: &mut WorldIndex, _region: Aabb3d) -> Vec<OriginalId> {
		Vec::new()
	}
}

impl OriginalIds<WorldIndex> for Leaf {
	fn original_ids_for(_spatial_index: &mut WorldIndex, _region: Aabb3d) -> Vec<OriginalId> {
		Vec::new()
	}
}

impl OriginalIds<WorldIndex> for Moss {
	fn original_ids_for(_spatial_index: &mut WorldIndex, _region: Aabb3d) -> Vec<OriginalId> {
		Vec::new()
	}
}

fn cell_from_bytes(bytes: crate::gen::Bytes) -> Aabb3d {
	cell(bytes.0[0] as f32)
}

impl<S> BuildWithIdLod<S> for Terrain
where
	S: BaseSpatialIndex<Terrain>,
{
	fn build_with_id(_spatial_index: &mut S, id: Id, _lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;
		Some((Self { cell: bounds }, bounds))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}

impl<S> BuildWithIdLod<S> for Vegetation
where
	S: Materialize<Terrain> + Materialize<Tree> + BaseSpatialIndex<Vegetation>,
{
	fn build_with_id(spatial_index: &mut S, id: Id, lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;
		Materialize::<Terrain>::materialize(spatial_index, Id::from_cell(bounds), lod_ref)?;
		Some((Self { cell: bounds }, bounds))
	}

	fn descendants_with_lod(id: Id, spatial_index: &mut S, lod_ref: &LodRef) {
		Materialize::<Tree>::materialize(spatial_index, tree_id(id), lod_ref);
	}
}

impl<S> BuildWithIdLod<S> for Tree
where
	S: BaseSpatialIndex<Vegetation> + BaseSpatialIndex<Tree> + Materialize<Leaf>,
{
	fn build_with_id(spatial_index: &mut S, id: Id, _lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let bounds = match id {
			Id::Bytes(bytes) => cell_from_bytes(bytes),
			Id::OriginCell(crate::gen::OriginCell(crate::gen::Cell(bounds))) => bounds,
		};
		let parent = Id::from_cell(bounds);
		if BaseSpatialIndex::<Vegetation>::get(spatial_index, parent).is_none() {
			return None;
		}
		Some((Self { parent, cell: bounds }, bounds))
	}

	fn descendants_with_lod(id: Id, spatial_index: &mut S, lod_ref: &LodRef) {
		Materialize::<Leaf>::materialize(spatial_index, leaf_id(id), lod_ref);
	}
}

impl<S> BuildWithIdLod<S> for Leaf
where
	S: BaseSpatialIndex<Tree> + BaseSpatialIndex<Leaf> + Materialize<Moss>,
{
	fn build_with_id(spatial_index: &mut S, id: Id, _lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let bounds = match id {
			Id::Bytes(bytes) => cell_from_bytes(bytes),
			Id::OriginCell(crate::gen::OriginCell(crate::gen::Cell(bounds))) => bounds,
		};
		let parent = tree_id(Id::from_cell(bounds));
		if BaseSpatialIndex::<Tree>::get(spatial_index, parent).is_none() {
			return None;
		}
		Some((Self { parent, cell: bounds }, bounds))
	}

	fn descendants_with_lod(id: Id, spatial_index: &mut S, lod_ref: &LodRef) {
		Materialize::<Moss>::materialize(spatial_index, moss_id(id), lod_ref);
	}
}

impl<S> BuildWithIdLod<S> for Moss
where
	S: BaseSpatialIndex<Leaf> + BaseSpatialIndex<Moss>,
{
	fn build_with_id(spatial_index: &mut S, id: Id, _lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let bounds = match id {
			Id::Bytes(bytes) => cell_from_bytes(bytes),
			Id::OriginCell(crate::gen::OriginCell(crate::gen::Cell(bounds))) => bounds,
		};
		let parent = leaf_id(tree_id(Id::from_cell(bounds)));
		if BaseSpatialIndex::<Leaf>::get(spatial_index, parent).is_none() {
			return None;
		}
		Some((Self { parent, cell: bounds }, bounds))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}

#[derive(Default)]
pub struct RecordingSpawner {
	pub spawns: Vec<(Id, MaterializeStatus, ScenePatchStatus)>,
	pub heals: Vec<Aabb3d>,
}

impl SceneSpawner<Vegetation> for RecordingSpawner {
	fn spawn_or_patch_scene(
		&mut self,
		id: Id,
		materialize_status: MaterializeStatus,
		scene_status: ScenePatchStatus,
		_scene: impl Scene,
		_marker: PhantomData<Vegetation>,
	) {
		self.spawns.push((id, materialize_status, scene_status));
	}

	fn heal_region(&mut self, region: Aabb3d, _wanted: &std::collections::HashSet<Id>, _marker: PhantomData<Vegetation>) {
		self.heals.push(region);
	}
}

pub struct VegetationLoader {
	pub index: WorldIndex,
	pub spawner: RecordingSpawner,
	pub descendant_spawns: Vec<Id>,
}

impl VegetationLoader {
	pub fn new() -> Self {
		Self {
			index: WorldIndex::default(),
			spawner: RecordingSpawner::default(),
			descendant_spawns: Vec::new(),
		}
	}
}

impl Materialize<Terrain> for VegetationLoader {
	fn materialize(&mut self, id: Id, lod_ref: &LodRef) -> Option<MaterializeStatus> {
		GeneratingSpatialIndex::<Terrain>::get_or_generate(&mut self.index, id, lod_ref)
	}
}

impl Materialize<Tree> for VegetationLoader {
	fn materialize(&mut self, id: Id, lod_ref: &LodRef) -> Option<MaterializeStatus> {
		let status = GeneratingSpatialIndex::<Tree>::get_or_generate(&mut self.index, id, lod_ref)?;
		self.descendant_spawns.push(id);
		// Index generation already materialized nested values; re-enter through the
		// loader so nested levels also record spawns.
		Materialize::<Leaf>::materialize(self, leaf_id(id), lod_ref);
		Some(status)
	}
}

impl Materialize<Leaf> for VegetationLoader {
	fn materialize(&mut self, id: Id, lod_ref: &LodRef) -> Option<MaterializeStatus> {
		let status = GeneratingSpatialIndex::<Leaf>::get_or_generate(&mut self.index, id, lod_ref)?;
		self.descendant_spawns.push(id);
		Materialize::<Moss>::materialize(self, moss_id(id), lod_ref);
		Some(status)
	}
}

impl Materialize<Moss> for VegetationLoader {
	fn materialize(&mut self, id: Id, lod_ref: &LodRef) -> Option<MaterializeStatus> {
		let status = GeneratingSpatialIndex::<Moss>::get_or_generate(&mut self.index, id, lod_ref)?;
		self.descendant_spawns.push(id);
		Some(status)
	}
}

impl SceneLoader for VegetationLoader {
	type Asset = Vegetation;
	type Index = WorldIndex;
	type Spawner = RecordingSpawner;

	fn spatial_index(&self) -> &Self::Index {
		&self.index
	}

	fn spatial_index_mut(&mut self) -> &mut Self::Index {
		&mut self.index
	}

	fn spawner_mut(&mut self) -> &mut Self::Spawner {
		&mut self.spawner
	}

	fn borrow_parts_mut(&mut self) -> (&mut Self::Index, &mut Self::Spawner) {
		(&mut self.index, &mut self.spawner)
	}
}
