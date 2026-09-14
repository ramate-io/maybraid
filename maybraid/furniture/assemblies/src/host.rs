//! Flattened furniture kits under one 50 m [`FurnitureCell`] host.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, Scene};
use furniture_components::assembly_scene;
use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::lod_host_scene_pending;
use lod::lod_ref::LodRef;
use lod::SceneChunk;
use richmond_building_components::{
	scene_children, FurnitureNode, FLATTENED_KIT_CHUNK_WEIGHT,
};

use crate::cell::FurnitureCellExtent;
use crate::fill::posed_assembly;

/// One 50 m furniture host: flattened painted / wireframe kits, no nested hosts.
#[derive(Component, Clone, Debug)]
pub struct FurnitureCell {
	pub extent: FurnitureCellExtent,
	pub slots: Vec<FurnitureNode>,
}

impl FurnitureCell {
	pub fn new(extent: FurnitureCellExtent, slots: Vec<FurnitureNode>) -> Self {
		Self { extent, slots }
	}

	pub fn bounds(&self) -> Aabb3d {
		let mut min = Vec3::new(self.extent.min.x, 0.0, self.extent.min.z);
		let mut max = Vec3::new(self.extent.max.x, 4.0, self.extent.max.z);
		for slot in &self.slots {
			let bounds = slot.scene_bounds();
			min = min.min(Vec3::from(bounds.min));
			max = max.max(Vec3::from(bounds.max));
		}
		Aabb3d::from_min_max(min, max)
	}
}

fn furniture_kit_scene(node: &FurnitureNode) -> Box<dyn Scene> {
	if let Some(parts) = posed_assembly(node) {
		Box::new(assembly_scene(&parts))
	} else {
		let identity = Transform::IDENTITY;
		let bounds = node.scene_bounds();
		let lod_ref = LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: &identity,
			current_transform: &identity,
			bounds: &bounds,
		};
		Box::new(node.scene_with_level(&lod_ref, LodSceneLevel::High))
	}
}

impl LodScene for FurnitureCell {
	fn scene_lod_level(&self, _lod_ref: &LodRef) -> LodSceneLevel {
		LodSceneLevel::High
	}

	fn scene_lod_status(&self, _lod_ref: &LodRef) -> LodSceneStatus {
		LodSceneStatus::Unchanged
	}

	fn scene_lod_culls(&self, _lod_ref: &LodRef, _current: LodSceneLevel) -> LodSceneCulls {
		LodSceneCulls::None
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		if !matches!(level, LodSceneLevel::High) {
			return Box::new(scene_children(Vec::new())) as Box<dyn Scene>;
		}
		let children: Vec<Box<dyn Scene>> =
			self.slots.iter().map(|slot| furniture_kit_scene(slot)).collect();
		Box::new(scene_children(children)) as Box<dyn Scene>
	}

	fn scene_chunks_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		if !matches!(level, LodSceneLevel::High) || self.slots.is_empty() {
			return SceneChunk::primitive(scene_children(Vec::new()));
		}
		let slots = self.slots.clone();
		let n = slots.len();
		let kit_w = FLATTENED_KIT_CHUNK_WEIGHT;
		let mut index = 0usize;
		SceneChunk::lazy(n as u32 * kit_w, n, move || {
			if index >= slots.len() {
				return None;
			}
			let scene = furniture_kit_scene(&slots[index]);
			index += 1;
			Some(SceneChunk::weighted(kit_w, scene))
		})
	}

	fn scene_bounds(&self) -> Aabb3d {
		self.bounds()
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let level = self.scene_lod_level(lod_ref);
		lod_host_scene_pending(level, self.scene_bounds())
	}
}

/// Spawn a pending flattened furniture host. Slot placements are already world-space.
pub fn spawn_furniture_cell(commands: &mut Commands, cell: FurnitureCell) -> Entity {
	let bounds = cell.bounds();
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &bounds,
	};
	let level = cell.scene_lod_level(&lod_ref);
	let entity = commands
		.spawn_scene((
			lod_host_scene_pending(level, bounds),
			bsn! {
				Transform::default()
				Visibility::default()
			},
		))
		.id();
	commands.entity(entity).insert(cell);
	entity
}
