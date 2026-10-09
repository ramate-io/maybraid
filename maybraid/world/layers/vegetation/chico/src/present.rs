//! Grow and present [`ChicoGrove`] tiles against a caller-supplied world sample.

use std::collections::{HashMap, HashSet, VecDeque};

use bevy::log::info_span;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures::FutureExt;
use lod::gen::{Id, SpatialIndex, Version};
use lod::lod_ref::LodRef;
use lod::{hide_lod_tree, LodScene};
use vegetation_components::spawn_lod_scene_host_with_lod_ref;
use vegetation_groves::GroveWorldSample;

use crate::packed::{PackMode, PackedGroveCell};
use crate::{ChicoGrove, ChicoGroveHost, ForestGroveTile, ForestIndex, ForestLayer};

const MAX_GROVE_GROWTH_TASKS: usize = 4;
const GROVE_HOSTS_PER_QUANTUM: usize = 1;

#[derive(Resource, Default)]
pub struct ForestPresenterState {
	presented: HashMap<Id, PresentedGrove>,
	growing: HashMap<Id, GrowingGrove>,
	/// Replaced hosts waiting for the present-cull despawn budget. FIFO batches
	/// (one prior grove's entities per slot). `handle` never despawns.
	pending_despawn: VecDeque<Vec<Entity>>,
}

struct PresentedGrove {
	version: Version,
	entities: Vec<Entity>,
	hidden: bool,
}

struct GrowingGrove {
	version: Version,
	layer: ForestLayer,
	task: Option<Task<GroveGrowthResult>>,
	ready: VecDeque<ForestGroveTile>,
	entities: Vec<Entity>,
}

struct GroveGrowthResult {
	tiles: Vec<ForestGroveTile>,
}

impl ForestPresenterState {
	pub fn clear(&mut self, commands: &mut Commands) {
		for presented in self.presented.values() {
			for entity in &presented.entities {
				commands.entity(*entity).try_despawn();
			}
		}
		self.presented.clear();
		let growing: Vec<Id> = self.growing.keys().copied().collect();
		for id in growing {
			if let Some(pending) = self.growing.remove(&id) {
				for entity in pending.entities {
					commands.entity(entity).try_despawn();
				}
			}
		}
		self.flush_pending_despawn(commands);
	}

	fn flush_pending_despawn(&mut self, commands: &mut Commands) {
		for entities in self.pending_despawn.drain(..) {
			for entity in entities {
				commands.entity(entity).try_despawn();
			}
		}
	}

	fn retire(&mut self, id: Id) -> Option<PresentedGrove> {
		self.presented.remove(&id)
	}

	/// Cancel in-flight growth, hide hosts already spawned, and queue teardown.
	fn retire_growing(&mut self, commands: &mut Commands, id: Id) {
		let Some(pending) = self.growing.remove(&id) else {
			return;
		};
		if pending.entities.is_empty() {
			return;
		}
		for entity in &pending.entities {
			hide_lod_tree(commands, *entity);
		}
		self.pending_despawn.push_back(pending.entities);
	}

	fn remove_presented(&mut self, commands: &mut Commands, ids: impl IntoIterator<Item = Id>) {
		for id in ids {
			if let Some(entry) = self.presented.remove(&id) {
				for entity in entry.entities {
					commands.entity(entity).try_despawn();
				}
			}
		}
	}

	pub fn presented_version(&self, id: Id) -> Option<Version> {
		self.presented.get(&id).map(|entry| entry.version)
	}

	pub fn hide(&mut self, commands: &mut Commands, id: Id) {
		if let Some(entry) = self.presented.get_mut(&id) {
			entry.hidden = true;
			for entity in &entry.entities {
				hide_lod_tree(commands, *entity);
			}
		}
	}

	pub fn is_hidden(&self, id: Id) -> bool {
		self.presented.get(&id).is_some_and(|entry| entry.hidden)
	}

	pub fn presents(&self, id: Id) -> bool {
		self.presented.contains_key(&id)
	}

	pub fn insert_presented(&mut self, id: Id, entities: Vec<Entity>) {
		self.presented
			.insert(id, PresentedGrove { version: Version(1), entities, hidden: false });
	}

	#[cfg(test)]
	pub(crate) fn insert_growing_hosts(&mut self, id: Id, entities: Vec<Entity>) {
		self.growing.insert(
			id,
			GrowingGrove {
				version: Version(1),
				layer: ForestLayer::UpperCanopy,
				task: None,
				ready: VecDeque::new(),
				entities,
			},
		);
	}

	pub fn presented_ids(&self) -> Vec<Id> {
		self.presented.keys().copied().collect()
	}

	pub fn remove_stale(&mut self, commands: &mut Commands, wanted: &HashSet<Id>) {
		let stale_growing: Vec<Id> =
			self.growing.keys().copied().filter(|id| !wanted.contains(id)).collect();
		for id in stale_growing {
			self.retire_growing(commands, id);
		}
		let stale_presented: Vec<Id> =
			self.presented.keys().copied().filter(|id| !wanted.contains(id)).collect();
		self.remove_presented(commands, stale_presented);
		// Gate close (`wanted` empty) cannot wait on present-cull admission.
		if wanted.is_empty() {
			self.flush_pending_despawn(commands);
		}
	}

	/// Grow off-thread, then spawn a bounded number of host trees per present slot.
	pub fn present_with_world<W>(
		&mut self,
		commands: &mut Commands,
		id: Id,
		version: Version,
		grove: &ChicoGrove,
		lod_ref: &LodRef,
		world: W,
	) -> Vec<Entity>
	where
		W: GroveWorldSample + Clone + Send + Sync + 'static,
	{
		if self.growing.get(&id).is_some_and(|pending| pending.version == version)
			|| self.presented.get(&id).is_some_and(|presented| presented.version == version)
		{
			// Keep polling / spawning the in-flight version.
		} else if let Some(previous) = self.retire(id) {
			for entity in &previous.entities {
				hide_lod_tree(commands, *entity);
			}
			self.pending_despawn.push_back(previous.entities);
		}

		if self.growing.get(&id).is_some_and(|pending| pending.version != version) {
			self.retire_growing(commands, id);
		}
		if !self.growing.contains_key(&id) {
			if self.presented.get(&id).is_some_and(|presented| presented.version == version) {
				return Vec::new();
			}
			if self.growing.values().filter(|pending| pending.task.is_some()).count()
				>= MAX_GROVE_GROWTH_TASKS
			{
				return Vec::new();
			}
			let grove = grove.clone();
			let layer = grove.layer;
			let world = world.clone();
			let task = if let Some(pool) = AsyncComputeTaskPool::try_get() {
				pool.spawn(async move {
					let _span = info_span!("chico_grove_growth").entered();
					let tiles = grove.ensure_grown(&world).to_vec();
					GroveGrowthResult { tiles }
				})
			} else {
				let _span = info_span!("chico_grove_growth").entered();
				let tiles = grove.ensure_grown(&world).to_vec();
				return self.finish_grown(commands, id, version, layer, tiles, lod_ref);
			};
			self.growing.insert(
				id,
				GrowingGrove {
					version,
					layer,
					task: Some(task),
					ready: VecDeque::new(),
					entities: Vec::new(),
				},
			);
			return Vec::new();
		}

		let pending = self.growing.get_mut(&id).expect("inserted above");
		if let Some(task) = pending.task.as_mut() {
			let Some(result) = (&mut *task).now_or_never() else {
				return Vec::new();
			};
			pending.task = None;
			pending.ready = result.tiles.into();
		}

		self.spawn_ready_hosts(commands, id, lod_ref)
	}

	fn finish_grown(
		&mut self,
		commands: &mut Commands,
		id: Id,
		version: Version,
		layer: ForestLayer,
		tiles: Vec<ForestGroveTile>,
		lod_ref: &LodRef,
	) -> Vec<Entity> {
		self.growing.insert(
			id,
			GrowingGrove { version, layer, task: None, ready: tiles.into(), entities: Vec::new() },
		);
		self.spawn_ready_hosts(commands, id, lod_ref)
	}

	fn spawn_ready_hosts(
		&mut self,
		commands: &mut Commands,
		id: Id,
		lod_ref: &LodRef,
	) -> Vec<Entity> {
		let pending = self.growing.get_mut(&id).expect("growing grove");
		let version = pending.version;
		let mut spawned = Vec::new();
		for _ in 0..GROVE_HOSTS_PER_QUANTUM {
			let Some(tile) = pending.ready.pop_front() else {
				break;
			};
			let _span = info_span!("chico_grove_host_spawn").entered();
			spawned.extend(spawn_forest_grove_tile(
				commands,
				&tile,
				pending.layer,
				lod_ref,
				id,
				version,
			));
		}
		pending.entities.extend(spawned.iter().copied());
		if pending.task.is_none() && pending.ready.is_empty() {
			let complete = self.growing.remove(&id).expect("pending grove");
			self.presented
				.insert(id, PresentedGrove { version, entities: complete.entities, hidden: false });
		}
		spawned
	}

	pub fn cull(
		&mut self,
		commands: &mut Commands,
		spatial_index: &ForestIndex,
		keep: &HashSet<Id>,
		mut despawn_budget: u32,
	) -> u32 {
		let stale_growing: Vec<Id> = self
			.growing
			.keys()
			.copied()
			.filter(|id| {
				SpatialIndex::<ChicoGrove>::get_bounds(spatial_index, *id).is_none()
					|| !keep.contains(id)
			})
			.collect();
		for id in stale_growing {
			self.retire_growing(commands, id);
		}
		while despawn_budget > 0 {
			let Some(entities) = self.pending_despawn.pop_front() else {
				break;
			};
			for entity in entities {
				commands.entity(entity).try_despawn();
			}
			despawn_budget -= 1;
		}
		let mut missing = Vec::new();
		let mut leaving = Vec::new();
		for id in self.presented_ids() {
			if SpatialIndex::<ChicoGrove>::get_bounds(spatial_index, id).is_none() {
				missing.push(id);
			} else if !keep.contains(&id) {
				leaving.push(id);
			}
		}
		if !missing.is_empty() {
			self.remove_presented(commands, missing);
		}
		let mut to_remove = HashSet::new();
		for id in leaving {
			if !self.is_hidden(id) {
				self.hide(commands, id);
			}
			if despawn_budget > 0 {
				to_remove.insert(id);
				despawn_budget -= 1;
			}
		}
		if !to_remove.is_empty() {
			self.remove_presented(commands, to_remove);
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
	id: Id,
	version: Version,
) -> Vec<Entity> {
	let entities = spawn_grove_host(commands, &ChicoGroveHost::new(tile.clone(), layer), lod_ref);
	if PackMode::current().packs_orchard() && tile.as_orchard().is_some() {
		for entity in &entities {
			commands.entity(*entity).insert(PackedGroveCell { id, version });
		}
	}
	entities
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use bevy::ecs::world::CommandQueue;
	use bevy::math::bounding::Aabb3d;
	use bevy::math::Vec3;

	fn cell_id(x: f32) -> Id {
		Id::from_cell(Aabb3d::from_min_max(Vec3::new(x, 0.0, 0.0), Vec3::new(x + 1.0, 1.0, 1.0)))
	}

	fn growing(version: Version, entities: Vec<Entity>) -> GrowingGrove {
		GrowingGrove {
			version,
			layer: ForestLayer::UpperCanopy,
			task: None,
			ready: VecDeque::new(),
			entities,
		}
	}

	fn with_commands(
		state: &mut ForestPresenterState,
		world: &mut World,
		f: impl FnOnce(&mut ForestPresenterState, &mut Commands),
	) {
		let mut queue = CommandQueue::default();
		let mut commands = Commands::new(&mut queue, world);
		f(state, &mut commands);
		drop(commands);
		queue.apply(world);
	}

	#[test]
	fn retire_queues_previous_hosts_without_dropping_them() -> Result<()> {
		let mut state = ForestPresenterState::default();
		let id = cell_id(0.0);
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

	#[test]
	fn retire_growing_hides_and_queues_spawned_hosts() -> Result<()> {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		let id = cell_id(0.0);
		let mut state = ForestPresenterState::default();
		state.growing.insert(id, growing(Version(1), vec![entity]));
		with_commands(&mut state, &mut world, |state, commands| {
			state.retire_growing(commands, id);
		});
		anyhow::ensure!(state.growing.is_empty(), "growth is cancelled");
		anyhow::ensure!(state.pending_despawn == vec![vec![entity]], "hosts wait on teardown");
		Ok(())
	}

	#[test]
	fn remove_stale_empty_retires_growing_hosts() -> Result<()> {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		let id = cell_id(0.0);
		let mut state = ForestPresenterState::default();
		state.growing.insert(id, growing(Version(1), vec![entity]));
		with_commands(&mut state, &mut world, |state, commands| {
			state.remove_stale(commands, &HashSet::new());
		});
		anyhow::ensure!(state.growing.is_empty(), "gate close retires in-flight growth");
		anyhow::ensure!(state.pending_despawn.is_empty(), "gate close flushes teardown");
		anyhow::ensure!(world.get_entity(entity).is_err(), "retired hosts are gone");
		Ok(())
	}

	#[test]
	fn cull_does_not_cancel_growing_ids_still_in_keep() -> Result<()> {
		use vegetation_groves::GroveExtent;

		let growing_bounds =
			Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(100.0, 1.0, 100.0));
		let presented_bounds =
			Aabb3d::from_min_max(Vec3::new(200.0, 0.0, 0.0), Vec3::new(300.0, 1.0, 100.0));
		let growing_id = Id::from_cell(growing_bounds);
		let presented_id = Id::from_cell(presented_bounds);
		let mut index = ForestIndex::default();
		SpatialIndex::<ChicoGrove>::insert(
			&mut index,
			growing_id,
			ChicoGrove::selected(
				GroveExtent::new(Vec3::from(growing_bounds.min), Vec3::from(growing_bounds.max)),
				ForestLayer::UpperCanopy,
				Vec::new(),
			),
			growing_bounds,
		);

		let mut world = World::new();
		let growing_entity = world.spawn_empty().id();
		let presented_entity = world.spawn_empty().id();
		let mut state = ForestPresenterState::default();
		state.growing.insert(growing_id, growing(Version(1), vec![growing_entity]));
		state.presented.insert(
			presented_id,
			PresentedGrove { version: Version(1), entities: vec![presented_entity], hidden: false },
		);
		let keep = HashSet::from([growing_id]);
		with_commands(&mut state, &mut world, |state, commands| {
			state.cull(commands, &index, &keep, 8);
		});
		anyhow::ensure!(state.growing.contains_key(&growing_id), "in-keep growth survives");
		anyhow::ensure!(!state.presents(presented_id), "leaving presented is removed");
		Ok(())
	}

	#[test]
	fn version_replace_retires_the_previous_growing_hosts() -> Result<()> {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		let id = cell_id(0.0);
		let mut state = ForestPresenterState::default();
		state.growing.insert(id, growing(Version(1), vec![entity]));
		with_commands(&mut state, &mut world, |state, commands| {
			if state.growing.get(&id).is_some_and(|pending| pending.version != Version(2)) {
				state.retire_growing(commands, id);
			}
		});
		anyhow::ensure!(state.growing.is_empty());
		anyhow::ensure!(state.pending_despawn == vec![vec![entity]]);
		Ok(())
	}
}
