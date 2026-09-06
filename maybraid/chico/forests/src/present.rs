//! Grow and present [`ChicoGrove`] tiles against a caller-supplied world sample.

use std::collections::{HashMap, HashSet, VecDeque};

use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::log::info_span;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use chico_groves::GroveWorldSample;
use chico_vegetation_components::spawn_lod_scene_host_with_lod_ref;
use futures::FutureExt;
use lod::gen::{Id, SpatialIndex, Version};
use lod::lod_ref::LodRef;
use lod::presentation::RegionPresenter;
use lod::LodScene;
use lod_first_load::{FirstLoadActivity, FirstLoadPermit};

use crate::world::GroveWorldSource;
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
	_permit: Option<FirstLoadPermit>,
}

impl ForestPresenterState {
	pub fn clear(&mut self, commands: &mut Commands) {
		for presented in self.presented.values() {
			for entity in &presented.entities {
				commands.entity(*entity).despawn();
			}
		}
		self.presented.clear();
		self.growing.clear();
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
		self.growing.retain(|id, _| wanted.contains(id));
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

	/// Grow off-thread, then spawn a bounded number of host trees per present slot.
	pub fn present_with_world<W>(
		&mut self,
		commands: &mut Commands,
		id: Id,
		version: Version,
		grove: &ChicoGrove,
		lod_ref: &LodRef,
		world: impl FnOnce() -> W,
		activity: Option<&FirstLoadActivity>,
	) -> Vec<Entity>
	where
		W: GroveWorldSample + Clone + Send + Sync + 'static,
	{
		if let Some(previous) = self.retire(id) {
			for entity in &previous.entities {
				commands.entity(*entity).insert(Visibility::Hidden);
			}
			self.pending_despawn.push_back(previous.entities);
		}

		if self.growing.get(&id).is_some_and(|pending| pending.version != version) {
			self.growing.remove(&id);
		}
		if !self.growing.contains_key(&id) {
			if self.growing.values().filter(|pending| pending.task.is_some()).count()
				>= MAX_GROVE_GROWTH_TASKS
			{
				return Vec::new();
			}
			let grove = grove.clone();
			let layer = grove.layer;
			let permit = activity.map(FirstLoadActivity::begin);
			let world = world();
			let task = AsyncComputeTaskPool::get().spawn(async move {
				let _span = info_span!("chico_grove_growth").entered();
				let tiles = grove.ensure_grown(&world).to_vec();
				GroveGrowthResult { tiles, _permit: permit }
			});
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

		let mut spawned = Vec::new();
		for _ in 0..GROVE_HOSTS_PER_QUANTUM {
			let Some(tile) = pending.ready.pop_front() else {
				break;
			};
			let _span = info_span!("chico_grove_host_spawn").entered();
			spawned.extend(spawn_forest_grove_tile(commands, &tile, pending.layer, lod_ref));
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
		self.growing.retain(|id, _| keep.contains(id));
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

/// Present [`ChicoGrove`] via a composed [`GroveWorldSource`].
#[derive(SystemParam)]
pub struct ForestPresenter<'w, 's, S: SystemParam + 'static> {
	commands: Commands<'w, 's>,
	state: ResMut<'w, ForestPresenterState>,
	source: StaticSystemParam<'w, 's, S>,
	activity: Option<Res<'w, FirstLoadActivity>>,
}

impl<S: SystemParam + 'static> RegionPresenter<ChicoGrove, ForestIndex>
	for ForestPresenter<'_, '_, S>
where
	for<'a, 'b> S::Item<'a, 'b>: GroveWorldSource,
{
	fn presented_version(&self, id: Id) -> Option<Version> {
		self.state.presented_version(id)
	}

	fn handle(&mut self, id: Id, version: Version, grove: &ChicoGrove, lod_ref: &LodRef) {
		let Some(world) = self.source.sample(grove, lod_ref) else {
			return;
		};
		self.state.present_with_world(
			&mut self.commands,
			id,
			version,
			grove,
			lod_ref,
			move || world,
			self.activity.as_deref(),
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

/// Flat-ground forest presenter.
pub type FlatForestPresenter<'w, 's> = ForestPresenter<'w, 's, crate::world::FlatWorld<'s>>;

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use bevy::math::bounding::Aabb3d;

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
