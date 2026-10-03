//! Spawn flattened hosts for the present neighborhood.

use std::collections::{HashMap, HashSet, VecDeque};

use bevy::prelude::*;
use furnishing_layer_model::FurnishingGenerationSystems;
use furniture_assemblies::FurnitureAssembliesPlugin;
use furniture_shaders::FurnitureShadersPlugin;
use layer_stack::LodPresentGateSync;
use lod::gen::{Id, SpatialIndex, Version};
use lod::presentation::LodPresentKeepRegion;
use lod::LodPresentGate;
use lod::{
	LodPresentRegionPlugin, LodRefreshCorePlugin, LodSceneRefreshRegion,
	LodSceneRefreshRegionPlugin,
};
use lod_gimme::GimmeLodSceneRefreshPlugin;

use crate::colliders::FurnitureWalkColliderPlugin;
use crate::host::{spawn_furniture_cell, FurnitureCell};
use crate::index::FurnitureIndex;
use crate::stream::FurniturePresentBullseye;

/// Cell id of a presented 50 m host. Crate restock keys off this, not the entity,
/// because [`FurniturePresenterState::remove_stale`] despawns the host outside the present ring.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentedFurnitureCellId(pub Id);

/// One flattened host spawn per frame so fulfill can drain kits.
const FURNITURE_PRESENT_CELLS_PER_FRAME: usize = 1;

/// Channel marker for furniture generate / present / refresh.
#[derive(Debug, Clone, Copy, Default)]
pub struct FurnitureLodChan;

/// Shared produce domain for furniture host refresh.
#[derive(Debug, Clone, Copy, Default)]
pub struct FurnitureRefresh;

#[derive(Resource, Default)]
pub(crate) struct FurniturePresenterState {
	presented: HashMap<Id, PresentedFurnitureCell>,
	pending_despawn: VecDeque<Vec<Entity>>,
}

struct PresentedFurnitureCell {
	version: Version,
	entities: Vec<Entity>,
}

impl FurniturePresenterState {
	fn retire(&mut self, id: Id) -> Option<PresentedFurnitureCell> {
		self.presented.remove(&id)
	}

	fn remove_stale(&mut self, commands: &mut Commands, wanted: &HashSet<Id>) {
		let stale: Vec<_> =
			self.presented.keys().copied().filter(|id| !wanted.contains(id)).collect();
		for id in stale {
			if let Some(entry) = self.presented.remove(&id) {
				self.pending_despawn.push_back(entry.entities);
			}
		}
		if let Some(entities) = self.pending_despawn.pop_front() {
			for entity in entities {
				commands.entity(entity).despawn();
			}
		}
	}

	pub(crate) fn take_entities(&mut self) -> Vec<Entity> {
		let mut out = Vec::new();
		for (_, entry) in self.presented.drain() {
			out.extend(entry.entities);
		}
		while let Some(entities) = self.pending_despawn.pop_front() {
			out.extend(entities);
		}
		out
	}

	fn despawn_all(&mut self, commands: &mut Commands) {
		for entity in self.take_entities() {
			commands.entity(entity).despawn();
		}
	}
}

/// Spawn flattened hosts for the present neighborhood; despawn the rest.
fn present_furniture_cells(
	mut commands: Commands,
	gate: Res<LodPresentGate<FurnitureLodChan>>,
	camera: Query<&Transform, With<Camera3d>>,
	present_keep: Res<LodPresentKeepRegion<FurnitureLodChan>>,
	index: Res<FurnitureIndex>,
	mut state: ResMut<FurniturePresenterState>,
	mut refresh_regions: MessageWriter<LodSceneRefreshRegion<FurnitureRefresh>>,
) {
	if !gate.open {
		if gate.is_changed() {
			state.despawn_all(&mut commands);
		}
		return;
	}
	let Some(region) = present_keep.region else {
		return;
	};
	let origin = camera.single().map(|transform| transform.translation).unwrap_or(Vec3::ZERO);
	let mut wanted = HashSet::new();
	let mut missing = Vec::new();
	for tracked in SpatialIndex::<FurnitureCell>::tracked_ids_for(&*index, region) {
		let id = tracked.0;
		let Some(cell) = index.get(id) else {
			continue;
		};
		let Some(version) = index.version(id) else {
			continue;
		};
		wanted.insert(id);
		if state.presented.get(&id).is_some_and(|entry| entry.version == version) {
			continue;
		}
		let center = cell.extent.center();
		let distance = Vec2::new(center.x, center.z).distance(origin.xz());
		missing.push((id, distance, version, cell.clone()));
	}
	missing.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
	let mut spawned = false;
	for (id, _, version, cell) in missing.into_iter().take(FURNITURE_PRESENT_CELLS_PER_FRAME) {
		if let Some(previous) = state.retire(id) {
			state.pending_despawn.push_back(previous.entities);
		}
		let entity = spawn_furniture_cell(&mut commands, cell);
		commands.entity(entity).insert(PresentedFurnitureCellId(id));
		state
			.presented
			.insert(id, PresentedFurnitureCell { version, entities: vec![entity] });
		spawned = true;
	}
	if spawned {
		refresh_regions.write(LodSceneRefreshRegion::new(region));
	}
	state.remove_stale(&mut commands, &wanted);
}

pub(crate) fn install_maputo_presentation(app: &mut App) {
	if !app.is_plugin_added::<FurnitureShadersPlugin>() {
		app.add_plugins(FurnitureShadersPlugin);
	}
	if !app.is_plugin_added::<FurnitureAssembliesPlugin>() {
		app.add_plugins(FurnitureAssembliesPlugin);
	}
	if !app.is_plugin_added::<LodRefreshCorePlugin>() {
		app.add_plugins(LodRefreshCorePlugin);
	}
	if !app.is_plugin_added::<FurnitureWalkColliderPlugin>() {
		app.add_plugins(FurnitureWalkColliderPlugin);
	}
	app.init_resource::<FurniturePresenterState>()
		.add_plugins(LodPresentRegionPlugin::<
			FurniturePresentBullseye,
			With<Camera3d>,
			FurnitureLodChan,
		>::default())
		.add_plugins(LodSceneRefreshRegionPlugin::<
			FurniturePresentBullseye,
			With<Camera>,
			FurnitureRefresh,
		>::default())
		.add_plugins(
			GimmeLodSceneRefreshPlugin::<FurnitureCell, FurnitureRefresh, With<Camera>>::without_full_scan_cull(),
		)
		.add_systems(
			Update,
			present_furniture_cells
				.after(LodPresentGateSync)
				.after(FurnishingGenerationSystems),
		);
}
