//! Presented-cell bookkeeping and the `Last` despawn of hosts and members.

use std::collections::{HashMap, VecDeque};

use bevy::prelude::*;
use lod::gen::{Id, Version};
use mob_intelligence::MemberOf;

#[derive(Resource, Default)]
pub struct MobPresenterState {
	presented: HashMap<Id, PresentedCell>,
	pending_despawn: VecDeque<Vec<Entity>>,
}

struct PresentedCell {
	version: Version,
	entities: Vec<Entity>,
	hidden: bool,
}

/// Host the presenter spawned for this generated cell.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentedMobCell(pub Id);

impl MobPresenterState {
	pub fn presented_version(&self, id: Id) -> Option<Version> {
		self.presented.get(&id).map(|entry| entry.version)
	}

	pub fn presents(&self, id: Id) -> bool {
		self.presented.contains_key(&id)
	}

	/// Drop the cell's record and return the entities the caller hides and queues.
	pub fn retire(&mut self, id: Id) -> Option<Vec<Entity>> {
		self.presented.remove(&id).map(|entry| entry.entities)
	}

	pub fn queue(&mut self, entities: Vec<Entity>) {
		self.pending_despawn.push_back(entities);
	}

	pub fn remember(&mut self, id: Id, version: Version, entities: Vec<Entity>) {
		self.presented.insert(id, PresentedCell { version, entities, hidden: false });
	}

	/// Mark the cell hidden and return the entities the caller hides.
	pub fn hide(&mut self, id: Id) -> Vec<Entity> {
		let Some(entry) = self.presented.get_mut(&id) else {
			return Vec::new();
		};
		entry.hidden = true;
		entry.entities.clone()
	}

	pub fn is_hidden(&self, id: Id) -> bool {
		self.presented.get(&id).is_some_and(|entry| entry.hidden)
	}

	pub fn presented_ids(&self) -> Vec<Id> {
		self.presented.keys().copied().collect()
	}

	pub fn queue_remove(&mut self, id: Id) {
		if let Some(presented) = self.presented.remove(&id) {
			self.pending_despawn.push_back(presented.entities);
		}
	}
}

/// Last-schedule despawn [`crate::MobPresentationCore`] uses.
pub fn install_mob_cell_teardown(app: &mut App) {
	app.init_resource::<MobPresenterState>()
		.add_systems(Last, drain_retired_mob_cells);
}

/// Combat, threat, and mob systems queue inserts on hosts and members through
/// `PostUpdate`. Despawn in `Last` so those commands still find their targets.
pub fn drain_retired_mob_cells(
	mut presented: ResMut<MobPresenterState>,
	members: Query<(Entity, &MemberOf)>,
	mut commands: Commands,
) {
	while let Some(entities) = presented.pending_despawn.pop_front() {
		let hosts: std::collections::HashSet<Entity> = entities.iter().copied().collect();
		for (entity, member) in &members {
			if hosts.contains(&member.mob) {
				commands.entity(entity).try_despawn();
			}
		}
		for entity in entities {
			commands.entity(entity).try_despawn();
		}
	}
}
