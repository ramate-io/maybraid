//! Grow and present [`ChicoGrove`] tiles against a caller-supplied world sample.

use std::collections::{HashMap, HashSet, VecDeque};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use chico_groves::GroveWorldSample;
use chico_vegetation_components::spawn_lod_scene_host_with_lod_ref;
use lod::gen::{Id, SpatialIndex, Version};
use lod::lod_ref::LodRef;
use lod::presentation::RegionPresenter;
use lod::LodScene;

use crate::{
	forest_world_sample, ChicoGrove, ChicoGroveHost, ForestGroveTile, ForestIndex, ForestLayer,
};

#[derive(Resource, Default)]
pub struct ForestPresenterState {
	presented: HashMap<Id, PresentedGrove>,
	/// Replaced hosts waiting for the present-cull despawn budget. FIFO batches
	/// (one prior grove's entities per slot). `handle` never despawns.
	pending_despawn: VecDeque<Vec<Entity>>,
}

struct PresentedGrove {
	version: Version,
	entities: Vec<Entity>,
	hidden: bool,
}

impl ForestPresenterState {
	pub fn clear(&mut self, commands: &mut Commands) {
		for presented in self.presented.values() {
			for entity in &presented.entities {
				commands.entity(*entity).despawn();
			}
		}
		self.presented.clear();
		for entities in self.pending_despawn.drain(..) {
			for entity in entities {
				commands.entity(entity).despawn();
			}
		}
	}

	fn retire(&mut self, id: Id) -> Option<PresentedGrove> {
		self.presented.remove(&id)
	}

	pub fn presented_version(&self, id: Id) -> Option<Version> {
		self.presented.get(&id).map(|entry| entry.version)
	}

	pub fn hide(&mut self, commands: &mut Commands, id: Id) {
		if let Some(entry) = self.presented.get_mut(&id) {
			entry.hidden = true;
			for entity in &entry.entities {
				commands.entity(*entity).insert(Visibility::Hidden);
			}
		}
	}

	pub fn is_hidden(&self, id: Id) -> bool {
		self.presented.get(&id).is_some_and(|entry| entry.hidden)
	}

	pub fn presented_ids(&self) -> Vec<Id> {
		self.presented.keys().copied().collect()
	}

	pub fn remove_stale(&mut self, commands: &mut Commands, wanted: &HashSet<Id>) {
		let stale: Vec<Id> =
			self.presented.keys().copied().filter(|id| !wanted.contains(id)).collect();
		for id in stale {
			if let Some(entry) = self.presented.remove(&id) {
				for entity in entry.entities {
					commands.entity(entity).despawn();
				}
			}
		}
	}

	/// Grow (or spawn) with `world`. Returns hosts spawned this slot (empty when
	/// this call only grew).
	pub fn present_with_world(
		&mut self,
		commands: &mut Commands,
		id: Id,
		version: Version,
		grove: &ChicoGrove,
		lod_ref: &LodRef,
		world: &impl GroveWorldSample,
	) -> Vec<Entity> {
		if let Some(previous) = self.retire(id) {
			for entity in &previous.entities {
				commands.entity(*entity).insert(Visibility::Hidden);
			}
			self.pending_despawn.push_back(previous.entities);
		}
		let Some(tiles) = grove.tiles_ready_to_present(world) else {
			return Vec::new();
		};
		let mut entities = Vec::new();
		for tile in tiles {
			entities.extend(spawn_forest_grove_tile(commands, tile, grove.layer, lod_ref));
		}
		self.presented
			.insert(id, PresentedGrove { version, entities: entities.clone(), hidden: false });
		entities
	}

	pub fn cull(
		&mut self,
		commands: &mut Commands,
		spatial_index: &ForestIndex,
		keep: &HashSet<Id>,
		mut despawn_budget: u32,
	) -> u32 {
		while despawn_budget > 0 {
			let Some(entities) = self.pending_despawn.pop_front() else {
				break;
			};
			for entity in entities {
				commands.entity(entity).despawn();
			}
			despawn_budget -= 1;
		}
		let stale: Vec<Id> = self
			.presented_ids()
			.into_iter()
			.filter(|id| !keep.contains(id))
			.filter(|id| SpatialIndex::<ChicoGrove>::get_bounds(spatial_index, *id).is_some())
			.collect();
		let mut to_remove = HashSet::new();
		for id in stale {
			if !self.is_hidden(id) {
				self.hide(commands, id);
			}
			if despawn_budget > 0 {
				to_remove.insert(id);
				despawn_budget -= 1;
			}
		}
		if !to_remove.is_empty() {
			let wanted: HashSet<Id> =
				self.presented_ids().into_iter().filter(|id| !to_remove.contains(&id)).collect();
			self.remove_stale(commands, &wanted);
		}
		despawn_budget
	}
}

fn spawn_grove_host(
	commands: &mut Commands,
	host: &ChicoGroveHost,
	lod_ref: &LodRef,
) -> Vec<Entity> {
	spawn_lod_scene_host_with_lod_ref(
		commands,
		host,
		Transform::IDENTITY,
		host.scene_bounds(),
		lod_ref,
	)
}

fn spawn_forest_grove_tile(
	commands: &mut Commands,
	tile: &ForestGroveTile,
	layer: ForestLayer,
	lod_ref: &LodRef,
) -> Vec<Entity> {
	spawn_grove_host(commands, &ChicoGroveHost::new(tile.clone(), layer), lod_ref)
}

/// Flat-ground presenter. Does not stamp playground-only markers such as `ShowRoot`.
#[derive(SystemParam)]
pub struct FlatForestPresenter<'w, 's> {
	commands: Commands<'w, 's>,
	state: ResMut<'w, ForestPresenterState>,
}

impl RegionPresenter<ChicoGrove, ForestIndex> for FlatForestPresenter<'_, '_> {
	fn presented_version(&self, id: Id) -> Option<Version> {
		self.state.presented_version(id)
	}

	fn handle(&mut self, id: Id, version: Version, grove: &ChicoGrove, lod_ref: &LodRef) {
		self.state.present_with_world(
			&mut self.commands,
			id,
			version,
			grove,
			lod_ref,
			&forest_world_sample(),
		);
	}

	fn hide(&mut self, id: Id) {
		self.state.hide(&mut self.commands, id);
	}

	fn is_hidden(&self, id: Id) -> bool {
		self.state.is_hidden(id)
	}

	fn presented_ids(&self) -> Vec<Id> {
		self.state.presented_ids()
	}

	fn remove_stale(&mut self, wanted: &HashSet<Id>) {
		self.state.remove_stale(&mut self.commands, wanted);
	}

	fn cull(
		&mut self,
		spatial_index: &ForestIndex,
		keep: &HashSet<Id>,
		despawn_budget: u32,
	) -> u32 {
		self.state.cull(&mut self.commands, spatial_index, keep, despawn_budget)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use bevy::math::bounding::Aabb3d;
	use bevy::math::Vec3;

	#[test]
	fn retire_queues_previous_hosts_without_dropping_them() -> Result<()> {
		let mut state = ForestPresenterState::default();
		let id = Id::from_cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE));
		let entity = Entity::from_raw_u32(7).expect("test entity");
		state.presented.insert(
			id,
			PresentedGrove { version: Version(1), entities: vec![entity], hidden: false },
		);
		let previous = state.retire(id).ok_or_else(|| anyhow::anyhow!("retired"))?;
		assert!(state.presented.is_empty());
		state.pending_despawn.push_back(previous.entities);
		assert_eq!(state.pending_despawn.len(), 1);
		assert_eq!(state.pending_despawn[0], vec![entity]);
		Ok(())
	}
}
