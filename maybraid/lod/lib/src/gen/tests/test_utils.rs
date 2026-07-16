use crate::gen::{
	GeneratingSpatialIndex, GenerationScheme, Id, LodScene, OriginalId, RegionPresenter,
	SpatialIndex, StorageStatus, TrackedId, Version,
};
use crate::lod_ref::LodRef;
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::scene::{ResolveContext, ResolvedScene, Scene, SceneFunction};
use bevy::{math::Vec3, prelude::Entity};
use std::collections::{HashMap, HashSet};

pub fn empty_scene(_: &mut ResolveContext, _: &mut ResolvedScene) {}

pub fn stub_scene() -> impl Scene + 'static {
	SceneFunction(empty_scene)
}

pub fn cell(x: f32) -> Aabb3d {
	Aabb3d::from_min_max(Vec3::new(x, 0.0, 0.0), Vec3::new(x + 1.0, 1.0, 1.0))
}

pub fn span(min_x: f32, max_x: f32) -> Aabb3d {
	Aabb3d::from_min_max(Vec3::new(min_x, 0.0, 0.0), Vec3::new(max_x, 1.0, 1.0))
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
		Id::Universal => Id::Universal,
	}
}

fn cell_from_bytes(bytes: crate::gen::Bytes) -> Aabb3d {
	cell(bytes.0[0] as f32)
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

// -----------------------------------------------------------------------------
// Asset hierarchy: Vegetation depends on Terrain; descendants Tree → Leaf → Moss.
// -----------------------------------------------------------------------------

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
		}
	};
}

impl_lod_scene!(Terrain);
impl_lod_scene!(Vegetation);
impl_lod_scene!(Tree);
impl_lod_scene!(Leaf);
impl_lod_scene!(Moss);

// -----------------------------------------------------------------------------
// Storage
// -----------------------------------------------------------------------------

pub struct StoredEntry<T> {
	pub value: T,
	pub bounds: Aabb3d,
	pub version: Version,
}

#[derive(Default)]
pub struct WorldIndex {
	next_version: u64,
	pub terrain: HashMap<Id, StoredEntry<Terrain>>,
	pub vegetation: HashMap<Id, StoredEntry<Vegetation>>,
	pub trees: HashMap<Id, StoredEntry<Tree>>,
	pub leaves: HashMap<Id, StoredEntry<Leaf>>,
	pub moss: HashMap<Id, StoredEntry<Moss>>,
}

impl WorldIndex {
	fn next_version(&mut self) -> Version {
		self.next_version += 1;
		Version(self.next_version)
	}
}

macro_rules! impl_spatial_index {
	($ty:ty, $field:ident) => {
		impl SpatialIndex<$ty> for WorldIndex {
			fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId> {
				self.$field
					.iter()
					.filter(|(_, entry)| region.intersects(&entry.bounds))
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
				self.$field.get(&id).map(|entry| &entry.value)
			}

			fn get_bounds(&self, id: Id) -> Option<Aabb3d> {
				self.$field.get(&id).map(|entry| entry.bounds)
			}

			fn version(&self, id: Id) -> Option<Version> {
				self.$field.get(&id).map(|entry| entry.version)
			}

			fn insert(&mut self, id: Id, value: $ty, bounds: Aabb3d, _lod_ref: &LodRef) {
				let version = self.next_version();
				self.$field.insert(id, StoredEntry { value, bounds, version });
			}
		}
	};
}

impl_spatial_index!(Terrain, terrain);
impl_spatial_index!(Vegetation, vegetation);
impl_spatial_index!(Tree, trees);
impl_spatial_index!(Leaf, leaves);
impl_spatial_index!(Moss, moss);

// -----------------------------------------------------------------------------
// Generation schemes
// -----------------------------------------------------------------------------

impl<S> GenerationScheme<S> for Terrain
where
	S: SpatialIndex<Terrain>,
{
	fn original_ids_for(_spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		let min_x = region.min.x.floor() as i32;
		let max_x = region.max.x.ceil() as i32;
		(min_x..=max_x)
			.map(|x| OriginalId(Id::from_cell(cell(x as f32))))
			.filter(|OriginalId(id)| id.origin_cell_bounds().is_some_and(|b| region.intersects(&b)))
			.collect()
	}

	fn build_with_id(_spatial_index: &mut S, id: Id, _lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;
		Some((Self { cell: bounds }, bounds))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}

impl<S> GenerationScheme<S> for Vegetation
where
	S: GeneratingSpatialIndex<Terrain> + GeneratingSpatialIndex<Tree>,
{
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		<Terrain as GenerationScheme<S>>::original_ids_for(spatial_index, region)
	}

	fn build_with_id(spatial_index: &mut S, id: Id, lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;
		GeneratingSpatialIndex::<Terrain>::get_or_generate(
			spatial_index,
			Id::from_cell(bounds),
			lod_ref,
		)?;
		Some((Self { cell: bounds }, bounds))
	}

	fn descendants_with_lod(id: Id, spatial_index: &mut S, lod_ref: &LodRef) {
		GeneratingSpatialIndex::<Tree>::get_or_generate(spatial_index, tree_id(id), lod_ref);
	}
}

impl<S> GenerationScheme<S> for Tree
where
	S: SpatialIndex<Vegetation> + GeneratingSpatialIndex<Leaf>,
{
	fn original_ids_for(_spatial_index: &mut S, _region: Aabb3d) -> Vec<OriginalId> {
		Vec::new()
	}

	fn build_with_id(spatial_index: &mut S, id: Id, _lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let bounds = match id {
			Id::Bytes(bytes) => cell_from_bytes(bytes),
			Id::OriginCell(crate::gen::OriginCell(crate::gen::Cell(bounds))) => bounds,
			Id::Universal => return None,
		};
		let parent = Id::from_cell(bounds);
		if SpatialIndex::<Vegetation>::get(spatial_index, parent).is_none() {
			return None;
		}
		Some((Self { parent, cell: bounds }, bounds))
	}

	fn descendants_with_lod(id: Id, spatial_index: &mut S, lod_ref: &LodRef) {
		GeneratingSpatialIndex::<Leaf>::get_or_generate(spatial_index, leaf_id(id), lod_ref);
	}
}

impl<S> GenerationScheme<S> for Leaf
where
	S: SpatialIndex<Tree> + GeneratingSpatialIndex<Moss>,
{
	fn original_ids_for(_spatial_index: &mut S, _region: Aabb3d) -> Vec<OriginalId> {
		Vec::new()
	}

	fn build_with_id(spatial_index: &mut S, id: Id, _lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let bounds = match id {
			Id::Bytes(bytes) => cell_from_bytes(bytes),
			Id::OriginCell(crate::gen::OriginCell(crate::gen::Cell(bounds))) => bounds,
			Id::Universal => return None,
		};
		let parent = tree_id(Id::from_cell(bounds));
		if SpatialIndex::<Tree>::get(spatial_index, parent).is_none() {
			return None;
		}
		Some((Self { parent, cell: bounds }, bounds))
	}

	fn descendants_with_lod(id: Id, spatial_index: &mut S, lod_ref: &LodRef) {
		GeneratingSpatialIndex::<Moss>::get_or_generate(spatial_index, moss_id(id), lod_ref);
	}
}

impl<S> GenerationScheme<S> for Moss
where
	S: SpatialIndex<Leaf>,
{
	fn original_ids_for(_spatial_index: &mut S, _region: Aabb3d) -> Vec<OriginalId> {
		Vec::new()
	}

	fn build_with_id(spatial_index: &mut S, id: Id, _lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let bounds = match id {
			Id::Bytes(bytes) => cell_from_bytes(bytes),
			Id::OriginCell(crate::gen::OriginCell(crate::gen::Cell(bounds))) => bounds,
			Id::Universal => return None,
		};
		let parent = leaf_id(tree_id(Id::from_cell(bounds)));
		if SpatialIndex::<Leaf>::get(spatial_index, parent).is_none() {
			return None;
		}
		Some((Self { parent, cell: bounds }, bounds))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}

// -----------------------------------------------------------------------------
// Presenter
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum PresenterOp {
	Handle(Id, Version),
	/// Ids actually removed by a `remove_stale` call.
	RemoveStale(HashSet<Id>),
}

/// Records presentation per layer. Each layer keeps its own id → version map,
/// mirroring how `remove_stale`'s wanted set is scoped to one layer's ids.
#[derive(Default)]
pub struct RecordingPresenter {
	pub terrain: HashMap<Id, Version>,
	pub vegetation: HashMap<Id, Version>,
	pub trees: HashMap<Id, Version>,
	pub leaves: HashMap<Id, Version>,
	pub moss: HashMap<Id, Version>,
	pub ops: Vec<PresenterOp>,
	/// Ids flagged for repair even when storage version is unchanged.
	pub repair_ids: HashSet<Id>,
}

macro_rules! presenter_methods {
	($field:ident) => {
		fn presented_version(&self, id: Id) -> Option<Version> {
			self.$field.get(&id).copied()
		}

		fn needs_repair(&self, _region: Aabb3d, id: Id, _version: Version) -> bool {
			self.repair_ids.contains(&id)
		}

		fn handle(&mut self, id: Id, version: Version, _scene: impl Scene, _lod_ref: &LodRef) {
			self.$field.insert(id, version);
			self.ops.push(PresenterOp::Handle(id, version));
		}

		fn remove_stale(&mut self, _region: Aabb3d, wanted: &HashSet<Id>) {
			// Test presenter treats its whole layer map as in-region; a real
			// implementation scopes removal to ids presented within `region`.
			let removed: HashSet<Id> =
				self.$field.keys().copied().filter(|id| !wanted.contains(id)).collect();
			for id in &removed {
				self.$field.remove(id);
			}
			self.ops.push(PresenterOp::RemoveStale(removed));
		}
	};
}

macro_rules! impl_presenter {
	($ty:ty, $field:ident) => {
		impl RegionPresenter<$ty, WorldIndex> for RecordingPresenter {
			presenter_methods!($field);
		}
	};
}

impl_presenter!(Terrain, terrain);
impl_presenter!(Tree, trees);
impl_presenter!(Leaf, leaves);
impl_presenter!(Moss, moss);

/// Vegetation is the index type for this hierarchy: `present_with_descendants`
/// composes every layer; `present_all` uses the default and forwards here.
impl RegionPresenter<Vegetation, WorldIndex> for RecordingPresenter {
	presenter_methods!(vegetation);

	fn present_with_descendants(&mut self, spatial_index: &WorldIndex, region: Aabb3d, lod_ref: &LodRef) {
		RegionPresenter::<Terrain, WorldIndex>::present(self, spatial_index, region, lod_ref);
		RegionPresenter::<Vegetation, WorldIndex>::present(self, spatial_index, region, lod_ref);
		RegionPresenter::<Tree, WorldIndex>::present(self, spatial_index, region, lod_ref);
		RegionPresenter::<Leaf, WorldIndex>::present(self, spatial_index, region, lod_ref);
		RegionPresenter::<Moss, WorldIndex>::present(self, spatial_index, region, lod_ref);
	}
}
