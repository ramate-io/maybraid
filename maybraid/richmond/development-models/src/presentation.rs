//! Present padded terrain cells.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use durham_terrain_models::water::PresentedWaterScene;
use durham_terrain_models::{stream_banded_draws, TerrainEntryStore};
use lod::gen::{Id, LodScene, LodSceneLevel, RegionPresenter, Version};
use lod::lod_ref::LodRef;
use std::collections::HashMap;
use std::collections::HashSet;

use crate::index::PaddedStoreView;
use crate::padded::{PresentedPaddedTerrainScene, TerrainWithPads};

#[derive(Debug, Clone, Copy)]
struct PresentedEntry {
	version: Version,
	entity: Entity,
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

	/// Present keep-region pads. Banding is [`StreamBandedLod`] on each cell.
	pub fn present_banded(
		&mut self,
		view: &PaddedStoreView<'_>,
		region: bevy::math::bounding::Aabb3d,
		lod_ref: &LodRef,
	) {
		use lod::gen::SpatialIndex;

		let wanted: HashSet<Id> = SpatialIndex::<TerrainWithPads>::tracked_ids_for(view, region)
			.into_iter()
			.map(|tracked| tracked.0)
			.collect();

		for id in &wanted {
			let Some(value) = SpatialIndex::<TerrainWithPads>::get(view, *id) else {
				continue;
			};
			let Some(version) = SpatialIndex::<TerrainWithPads>::version(view, *id) else {
				continue;
			};
			let level = value.scene_lod_level(lod_ref);
			if self
				.state
				.presented
				.get(id)
				.is_some_and(|shown| shown.version == version && shown.level == level)
			{
				continue;
			}
			if let Some(previous) = self.state.presented.remove(id) {
				self.commands.entity(previous.entity).despawn();
			}
			let entity = self
				.commands
				.spawn_scene(value.scene_with_lod(lod_ref))
				.insert(PresentedPaddedTerrainScene(*id))
				.id();
			if stream_banded_draws(value, level) {
				if let Some(water) = self.terrain_store.water(*id) {
					self.commands
						.spawn_scene(water.scene_with_lod(lod_ref))
						.insert((PresentedWaterScene(*id), bevy::prelude::ChildOf(entity)));
				}
			}
			self.state.presented.insert(*id, PresentedEntry { version, entity, level });
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
		let entity = self
			.commands
			.spawn_scene(value.scene_with_lod(lod_ref))
			.insert(PresentedPaddedTerrainScene(id))
			.id();
		if stream_banded_draws(value, level) {
			if let Some(water) = self.terrain_store.water(id) {
				self.commands
					.spawn_scene(water.scene_with_lod(lod_ref))
					.insert((PresentedWaterScene(id), ChildOf(entity)));
			}
		}
		self.state.presented.insert(id, PresentedEntry { version, entity, level });
	}

	fn presented_ids(&self) -> Vec<Id> {
		self.state.presented.keys().copied().collect()
	}

	fn remove_stale(&mut self, wanted: &HashSet<Id>) {
		PaddedTerrainPresenter::remove_stale(self, wanted);
	}
}
