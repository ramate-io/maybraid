//! Grow and present [`TerrainOutcropping`] placements against a caller-supplied height sample.

use std::collections::{HashMap, HashSet};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lod::gen::{Id, Version};
use lod::lod_ref::LodRef;
use lod::presentation::RegionPresenter;

use crate::{
	RockComponent, RockPlacement, TerrainDetailIndex, TerrainDetailWorldSample, TerrainOutcropping,
};

/// Shared unit meshes and rock material. Initialized by the plugin.
#[derive(Resource, Clone)]
pub struct RockMeshCache {
	pub round: Handle<Mesh>,
	pub knob: Handle<Mesh>,
	pub sharp: Handle<Mesh>,
	pub material: Handle<StandardMaterial>,
}

/// Insert [`RockMeshCache`] once assets exist (DefaultPlugins / playground).
pub fn init_rock_mesh_cache(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	existing: Option<Res<RockMeshCache>>,
) {
	if existing.is_some() {
		return;
	}
	commands.insert_resource(RockMeshCache::from_assets(&mut meshes, &mut materials));
}

impl RockMeshCache {
	pub fn from_assets(
		meshes: &mut Assets<Mesh>,
		materials: &mut Assets<StandardMaterial>,
	) -> Self {
		Self {
			round: meshes.add(RockComponent::RoundRock.unit_mesh()),
			knob: meshes.add(RockComponent::RockKnob.unit_mesh()),
			sharp: meshes.add(RockComponent::SharpRock.unit_mesh()),
			material: materials.add(crate::rock_material()),
		}
	}

	pub fn mesh(&self, component: RockComponent) -> Handle<Mesh> {
		match component {
			RockComponent::RoundRock => self.round.clone(),
			RockComponent::RockKnob => self.knob.clone(),
			RockComponent::SharpRock => self.sharp.clone(),
		}
	}
}

/// Spawn one scaled unit rock. No collider in v1.
pub fn spawn_rock(
	commands: &mut Commands,
	cache: &RockMeshCache,
	placement: RockPlacement,
) -> Entity {
	commands
		.spawn((
			Mesh3d(cache.mesh(placement.component)),
			MeshMaterial3d(cache.material.clone()),
			placement.transform(),
			Visibility::default(),
		))
		.id()
}

#[derive(Resource, Default)]
pub struct TerrainDetailPresenterState {
	presented: HashMap<Id, PresentedOutcropping>,
}

struct PresentedOutcropping {
	version: Version,
	entities: Vec<Entity>,
	hidden: bool,
}

impl TerrainDetailPresenterState {
	pub fn clear(&mut self, commands: &mut Commands) {
		for presented in self.presented.values() {
			for entity in &presented.entities {
				commands.entity(*entity).despawn();
			}
		}
		self.presented.clear();
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

	/// Grow on one tick and spawn on the next so a dense cell does not own both costs.
	pub fn present_with_world<W>(
		&mut self,
		commands: &mut Commands,
		cache: &RockMeshCache,
		id: Id,
		version: Version,
		outcropping: &TerrainOutcropping,
		world: W,
	) -> Vec<Entity>
	where
		W: TerrainDetailWorldSample,
	{
		if self.presented.get(&id).is_some_and(|presented| presented.version == version) {
			return Vec::new();
		}
		if let Some(previous) = self.presented.remove(&id) {
			for entity in previous.entities {
				commands.entity(entity).despawn();
			}
		}
		let Some(placements) = outcropping.placements_ready_to_present(&world) else {
			return Vec::new();
		};
		let entities: Vec<Entity> = placements
			.iter()
			.copied()
			.map(|placement| spawn_rock(commands, cache, placement))
			.collect();
		self.presented.insert(
			id,
			PresentedOutcropping { version, entities: entities.clone(), hidden: false },
		);
		entities
	}
}

/// Flat-ground presenter for isolated tests and `/show` without Durham.
#[derive(SystemParam)]
pub struct FlatTerrainDetailPresenter<'w, 's> {
	commands: Commands<'w, 's>,
	state: ResMut<'w, TerrainDetailPresenterState>,
	cache: Res<'w, RockMeshCache>,
}

impl RegionPresenter<TerrainOutcropping, TerrainDetailIndex>
	for FlatTerrainDetailPresenter<'_, '_>
{
	fn presented_version(&self, id: Id) -> Option<Version> {
		self.state.presented_version(id)
	}

	fn handle(
		&mut self,
		id: Id,
		version: Version,
		outcropping: &TerrainOutcropping,
		_lod_ref: &LodRef,
	) {
		self.state.present_with_world(
			&mut self.commands,
			&self.cache,
			id,
			version,
			outcropping,
			crate::FlatTerrainDetailSample::default(),
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
		spatial_index: &TerrainDetailIndex,
		keep: &HashSet<Id>,
		despawn_budget: u32,
	) -> u32 {
		self.state.cull(&mut self.commands, spatial_index, keep, despawn_budget)
	}
}

impl TerrainDetailPresenterState {
	pub fn cull(
		&mut self,
		commands: &mut Commands,
		_spatial_index: &TerrainDetailIndex,
		keep: &HashSet<Id>,
		mut despawn_budget: u32,
	) -> u32 {
		let stale: Vec<Id> =
			self.presented_ids().into_iter().filter(|id| !keep.contains(id)).collect();
		for id in stale {
			if despawn_budget == 0 {
				self.hide(commands, id);
				continue;
			}
			if let Some(entry) = self.presented.remove(&id) {
				for entity in entry.entities {
					commands.entity(entity).despawn();
				}
				despawn_budget -= 1;
			}
		}
		despawn_budget
	}
}
