//! System-local Avian-backed [`SpatialIndex`] for terrain cells.

use crate::terrain::cell::{HasTerrainCellLayout, TerrainCellLayout};
use crate::terrain::presentation::{HasTerrainPresentationAssets, TerrainPresentationAssets};
use crate::terrain::Terrain;
use avian3d::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::{Id, SpatialIndex, StorageStatus, TrackedId, Version};
use lod::lod_ref::LodRef;
use std::collections::HashMap;

/// Marks an Avian collider entity as a tracked terrain cell.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerrainCellId(pub Id);

#[derive(Debug, Clone)]
pub(crate) struct StoredEntry {
	pub(crate) value: Terrain,
	pub(crate) bounds: Aabb3d,
	pub(crate) version: Version,
	pub(crate) entity: Entity,
}

/// Side table for terrain values / versions / entity mapping.
///
/// ECS + Avian colliders are the spatial truth for region queries; this resource
/// holds payloads that [`SpatialIndex::get`] must return by reference.
#[derive(Resource, Default)]
pub struct TerrainEntryStore {
	next_version: u64,
	pub(crate) entries: HashMap<Id, StoredEntry>,
	entity_to_id: HashMap<Entity, Id>,
}

impl TerrainEntryStore {
	fn next_version(&mut self) -> Version {
		self.next_version += 1;
		Version(self.next_version)
	}

	pub fn len(&self) -> usize {
		self.entries.len()
	}

	pub fn is_empty(&self) -> bool {
		self.entries.is_empty()
	}
}

/// System-local wrapper: Avian [`SpatialQuery`] + commands + entry store.
///
/// Construct inside a Bevy system and use as `S` for [`lod::gen::GeneratingSpatialIndex`].
#[derive(SystemParam)]
pub struct AvianTerrainIndex<'w, 's> {
	commands: Commands<'w, 's>,
	spatial: SpatialQuery<'w, 's>,
	store: ResMut<'w, TerrainEntryStore>,
	layout: ResMut<'w, TerrainCellLayout>,
	presentation: Res<'w, TerrainPresentationAssets>,
}

impl<'w, 's> HasTerrainCellLayout for AvianTerrainIndex<'w, 's> {
	fn cell_layout(&self) -> &TerrainCellLayout {
		&self.layout
	}
}

impl<'w, 's> HasTerrainPresentationAssets for AvianTerrainIndex<'w, 's> {
	fn presentation_assets(&self) -> &TerrainPresentationAssets {
		&self.presentation
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
		// Index entities are bookkeeping only — solid trimesh colliders live on
		// presented mesh scenes. Tall sensor cuboids would fight ground checks.
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

	/// Despawn all tracked cell entities and clear the entry store.
	pub fn clear(&mut self) {
		let entities: Vec<Entity> = self.store.entries.values().map(|e| e.entity).collect();
		for entity in entities {
			self.commands.entity(entity).despawn();
		}
		self.store.entries.clear();
		self.store.entity_to_id.clear();
	}

	pub fn set_layout(&mut self, layout: TerrainCellLayout) {
		*self.layout = layout;
	}

	pub fn layout(&self) -> &TerrainCellLayout {
		&self.layout
	}
}

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

		// Same-frame inserts are not yet in Avian's BVH; include store hits by bounds.
		use bevy::math::bounding::IntersectsVolume;
		for (id, entry) in &self.store.entries {
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
		if self.store.entries.contains_key(&id) {
			StorageStatus::TrackedWithin
		} else {
			StorageStatus::NotTracked
		}
	}

	fn get(&self, id: Id) -> Option<&Terrain> {
		self.store.entries.get(&id).map(|e| &e.value)
	}

	fn get_bounds(&self, id: Id) -> Option<Aabb3d> {
		self.store.entries.get(&id).map(|e| e.bounds)
	}

	fn version(&self, id: Id) -> Option<Version> {
		self.store.entries.get(&id).map(|e| e.version)
	}

	fn insert(&mut self, id: Id, t: Terrain, bounds: Aabb3d, _lod_ref: &LodRef) {
		if let Some(existing) = self.store.entries.remove(&id) {
			self.store.entity_to_id.remove(&existing.entity);
			self.commands.entity(existing.entity).despawn();
		}

		let entity = self.spawn_cell_entity(id, &t, bounds);
		let version = self.store.next_version();
		self.store.entity_to_id.insert(entity, id);
		self.store.entries.insert(
			id,
			StoredEntry { value: t, bounds, version, entity },
		);
	}
}
