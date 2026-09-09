//! Present padded terrain cells.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use durham_terrain_models::{
	stream_banded_draws, PresentedWaterScene, TerrainEntryStore, TerrainVisualHost, Water,
};
use lod::gen::{Id, LodScene, LodSceneLevel, RegionPresenter, SpatialIndex, Version};
use lod::lod_ref::LodRef;
use std::collections::HashMap;
use std::collections::HashSet;

use crate::index::PaddedStoreView;
use crate::padded::{PresentedPaddedTerrainScene, TerrainWithPads};

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
	terrain_store: Res<'w, TerrainEntryStore>,
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
			self.commands.entity(entity).despawn();
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
			self.commands.entity(previous).despawn();
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

	/// Present keep-region pads that draw or seed Near collision.
	pub fn present_banded(
		&mut self,
		view: &PaddedStoreView<'_>,
		region: bevy::math::bounding::Aabb3d,
		lod_ref: &LodRef,
	) {
		let wanted: HashSet<Id> = SpatialIndex::<TerrainWithPads>::tracked_ids_for(view, region)
			.into_iter()
			.filter_map(|tracked| {
				let value = SpatialIndex::<TerrainWithPads>::get(view, tracked.0)?;
				let level = value.scene_lod_level(lod_ref);
				(stream_banded_draws(value, level) || value.seeds_collision()).then_some(tracked.0)
			})
			.collect();

		for id in &wanted {
			let Some(value) = SpatialIndex::<TerrainWithPads>::get(view, *id) else {
				continue;
			};
			let Some(version) = SpatialIndex::<TerrainWithPads>::version(view, *id) else {
				continue;
			};
			let level = value.scene_lod_level(lod_ref);
			let draw = stream_banded_draws(value, level);
			let water_version = self.terrain_store.water_version(*id);
			let water = draw.then(|| self.terrain_store.water(*id).cloned()).flatten();
			if let Some(shown) = self.state.presented.get(id).copied() {
				if shown.version == version {
					if shown.level != level {
						self.commands.entity(shown.entity).insert(if draw {
							Visibility::Inherited
						} else {
							Visibility::Hidden
						});
						let water_entity =
							self.replace_water(*id, shown.entity, shown.water, water.as_ref());
						if let Some(shown) = self.state.presented.get_mut(id) {
							shown.level = level;
							shown.water_version = water_version;
							shown.water = water_entity;
						}
						continue;
					}
					if shown.water_version != water_version {
						let water_entity =
							self.replace_water(*id, shown.entity, shown.water, water.as_ref());
						if let Some(shown) = self.state.presented.get_mut(id) {
							shown.water_version = water_version;
							shown.water = water_entity;
						}
					}
					continue;
				}
			}
			if let Some(previous) = self.state.presented.remove(id) {
				self.commands.entity(previous.entity).despawn();
			}
			let (entity, water_entity) = self.spawn_cell(*id, value, draw, water.as_ref());
			self.state.presented.insert(
				*id,
				PresentedEntry { version, water_version, entity, water: water_entity, level },
			);
		}

		self.remove_stale(&wanted);
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
		let level = value.scene_lod_level(lod_ref);
		// FinePatch own-terrain presents water via [`durham_terrain_models::WaterRegionPresenter`].
		let (entity, water) = self.spawn_cell(id, value, true, None);
		self.state
			.presented
			.insert(id, PresentedEntry { version, water_version: None, entity, water, level });
	}

	fn presented_ids(&self) -> Vec<Id> {
		self.state.presented.keys().copied().collect()
	}

	fn remove_stale(&mut self, wanted: &HashSet<Id>) {
		PaddedTerrainPresenter::remove_stale(self, wanted);
	}
}
