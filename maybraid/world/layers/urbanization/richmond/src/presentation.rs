//! Present padded terrain cells.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use durham::{stream_banded_draws, PresentedWaterScene, TerrainVisualHost, Water};
use lod::gen::{Id, LodScene, LodSceneLevel, Version};
use lod::hcsg::HcsgStorage;
use lod::lod_ref::LodRef;
use std::collections::HashMap;
use std::collections::HashSet;

use crate::ground::RichmondGround;
use crate::padded::{PaddedTerrain, PresentedPaddedTerrainScene, TerrainWithPads};

#[derive(Debug, Clone, Copy)]
struct PresentedEntry {
	version: Version,
	water_version: Option<Version>,
	entity: Entity,
	water: Option<Entity>,
	level: LodSceneLevel,
}

/// Runtime presentation bookkeeping for [`TerrainWithPads`].
#[derive(Resource, Default)]
pub struct PaddedTerrainPresenterState {
	presented: HashMap<Id, PresentedEntry>,
}

impl PaddedTerrainPresenterState {
	pub fn clear(&mut self, commands: &mut Commands) {
		for entry in self.presented.values() {
			commands.entity(entry.entity).try_despawn();
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

	pub fn remove_stale(&mut self, wanted: &HashSet<Id>) {
		let stale: Vec<(Id, Entity)> = self
			.state
			.presented
			.iter()
			.filter(|(id, _)| !wanted.contains(id))
			.map(|(id, entry)| (*id, entry.entity))
			.collect();
		for (id, entity) in stale {
			self.commands.entity(entity).try_despawn();
			self.state.presented.remove(&id);
		}
	}

	fn attach_water(&mut self, id: Id, parent: Entity, water: &Water) -> Entity {
		let entity = water.spawn_fill(&mut self.commands, Transform::IDENTITY);
		self.commands.entity(entity).insert((PresentedWaterScene(id), ChildOf(parent)));
		entity
	}

	fn replace_water(
		&mut self,
		id: Id,
		parent: Entity,
		previous: Option<Entity>,
		water: Option<&Water>,
	) -> Option<Entity> {
		if let Some(previous) = previous {
			self.commands.entity(previous).try_despawn();
		}
		water.map(|water| self.attach_water(id, parent, water))
	}

	fn spawn_cell(
		&mut self,
		id: Id,
		value: &TerrainWithPads,
		draw: bool,
		water: Option<&Water>,
	) -> (Entity, Option<Entity>) {
		let visibility = if draw { Visibility::Inherited } else { Visibility::Hidden };
		let entity = value.spawn_fill(&mut self.commands, visibility, value.seeds_collision());
		self.commands.entity(entity).insert((
			Name::new("Padded terrain cell"),
			PresentedPaddedTerrainScene(id),
			TerrainVisualHost,
		));
		let water_entity = water.filter(|_| draw).map(|w| self.attach_water(id, entity, w));
		(entity, water_entity)
	}

	/// Present stored padded cells of `tracked` that draw or seed Near collision.
	///
	/// A padded cell replaces the ground cell under the same id, so it
	/// carries that cell's water.
	pub fn present_tracked<G: RichmondGround>(
		&mut self,
		storage: &HcsgStorage,
		tracked: &HashSet<Id>,
		lod_ref: &LodRef,
	) {
		let wanted: HashSet<Id> = tracked
			.iter()
			.copied()
			.filter(|&id| {
				storage.get::<PaddedTerrain<G>>(id).is_some_and(|padded| {
					let value = &padded.surface;
					let level = value.scene_lod_level(lod_ref);
					stream_banded_draws(value, level) || value.seeds_collision()
				})
			})
			.collect();

		for id in &wanted {
			let Some(entry) = storage.entry::<PaddedTerrain<G>>(*id) else {
				continue;
			};
			let (value, version) = (&entry.value.surface, entry.version);
			let level = value.scene_lod_level(lod_ref);
			let draw = stream_banded_draws(value, level);
			let water = storage.entry::<Water>(*id);
			let water_version = water.map(|water| water.version);
			let water = water.filter(|_| draw).map(|water| &water.value);
			if let Some(shown) = self.state.presented.get(id).copied() {
				if shown.version == version {
					if shown.level != level {
						self.commands.entity(shown.entity).insert(if draw {
							Visibility::Inherited
						} else {
							Visibility::Hidden
						});
						let water_entity = self.replace_water(*id, shown.entity, shown.water, water);
						if let Some(shown) = self.state.presented.get_mut(id) {
							shown.level = level;
							shown.water_version = water_version;
							shown.water = water_entity;
						}
						continue;
					}
					if shown.water_version != water_version {
						let water_entity = self.replace_water(*id, shown.entity, shown.water, water);
						if let Some(shown) = self.state.presented.get_mut(id) {
							shown.water_version = water_version;
							shown.water = water_entity;
						}
					}
					continue;
				}
			}
			if let Some(previous) = self.state.presented.remove(id) {
				self.commands.entity(previous.entity).try_despawn();
			}
			let (entity, water_entity) = self.spawn_cell(*id, value, draw, water);
			self.state.presented.insert(
				*id,
				PresentedEntry { version, water_version, entity, water: water_entity, level },
			);
		}

		self.remove_stale(&wanted);
	}
}
