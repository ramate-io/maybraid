//! Present padded terrain cells.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value};
use lod::gen::{Id, LodScene, RegionPresenter, Version};
use lod::lod_host_scene_pending;
use lod::lod_ref::LodRef;
use std::collections::HashMap;
use std::collections::HashSet;

use crate::index::PaddedStoreView;
use crate::padded::{PresentedPaddedTerrainScene, TerrainWithPads};

#[derive(Debug, Clone, Copy)]
struct PresentedEntry {
	version: Version,
	entity: Entity,
}

/// Runtime presentation bookkeeping for [`TerrainWithPads`].
#[derive(Resource, Default)]
pub struct PaddedTerrainPresenterState {
	presented: HashMap<Id, PresentedEntry>,
}

impl PaddedTerrainPresenterState {
	pub fn clear(&mut self, commands: &mut Commands) {
		for entry in self.presented.values() {
			commands.entity(entry.entity).despawn();
		}
		self.presented.clear();
	}
}

/// System-local presenter for padded terrain meshes.
#[derive(SystemParam)]
pub struct PaddedTerrainPresenter<'w, 's> {
	commands: Commands<'w, 's>,
	state: ResMut<'w, PaddedTerrainPresenterState>,
}

impl PaddedTerrainPresenter<'_, '_> {
	pub fn clear_presented(&mut self) {
		self.state.clear(&mut self.commands);
	}
}

impl<'a> RegionPresenter<TerrainWithPads, PaddedStoreView<'a>> for PaddedTerrainPresenter<'_, '_> {
	fn presented_version(&self, id: Id) -> Option<Version> {
		self.state.presented.get(&id).map(|e| e.version)
	}

	fn handle(&mut self, id: Id, version: Version, value: &TerrainWithPads, lod_ref: &LodRef) {
		if let Some(previous) = self.state.presented.remove(&id) {
			self.commands.entity(previous.entity).despawn();
		}
		let min = Vec3::from(value.cell.min);
		let max = Vec3::from(value.cell.max);
		let transform = Transform::from_translation((min + max) * 0.5);
		let level = value.scene_lod_level(lod_ref);
		let entity = self
			.commands
			.spawn_scene((
				lod_host_scene_pending(level, value.scene_bounds()),
				bsn! {
					template_value(transform)
					Visibility::default()
				},
			))
			.insert((value.clone(), PresentedPaddedTerrainScene(id)))
			.id();
		self.state.presented.insert(id, PresentedEntry { version, entity });
	}

	fn presented_ids(&self) -> Vec<Id> {
		self.state.presented.keys().copied().collect()
	}

	fn remove_stale(&mut self, wanted: &HashSet<Id>) {
		let stale: Vec<(Id, Entity)> = self
			.state
			.presented
			.iter()
			.filter(|(id, _)| !wanted.contains(id))
			.map(|(id, entry)| (*id, entry.entity))
			.collect();

		for (id, entity) in stale {
			self.commands.entity(entity).despawn();
			self.state.presented.remove(&id);
		}
	}
}
