//! Neighborhood generate / present for 50 m [`FurnitureCell`] hosts.
//!
//! Reads Richmond [`BuiltDevelopment`]s and bins High slots into furniture cells.
//! Load ring is a 50 m-cell neighborhood — not the Richmond host tree.

use std::collections::{HashMap, HashSet, VecDeque};

use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::prelude::*;
use lod::gen::{
	GenerationScheme, Id, OriginalId, SpatialIndex, StorageStatus, TrackedId, Version,
};
use lod::LodGenerateKeepRegion;
use lod::lod_ref::LodRef;
use lod::presentation::{LodPresentKeepRegion, LodPresentRegion};
use lod::scene::{LodRefreshRegions, LodRefreshRegionsStatus};
use lod::{LodPresentRegionPlugin, LodRefreshCorePlugin, LodSceneRefreshRegionPlugin};
use lod_gimme::GimmeLodSceneRefreshPlugin;
use richmond_development_models::{BuiltDevelopment, DevelopmentEntryStore, DevelopmentHosts};

use crate::cell::{
	world_slot, xz_radius_aabb, FurnitureCellExtent, FURNITURE_GENERATE_RADIUS,
	FURNITURE_PRESENT_RADIUS,
};
use crate::host::{spawn_furniture_cell, FurnitureCell};

/// Channel marker for furniture generate / present / refresh.
#[derive(Debug, Clone, Copy, Default)]
pub struct FurnitureLodChan;

/// Shared produce domain for furniture host refresh.
#[derive(Debug, Clone, Copy, Default)]
pub struct FurnitureRefresh;

#[derive(SystemSet, Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum FurnitureStreamSystems {
	Generate,
	Present,
}

#[derive(Clone)]
struct StoredFurnitureCell {
	value: FurnitureCell,
	bounds: Aabb3d,
	version: Version,
}

/// Generated 50 m furniture cells plus the current world-space slot list.
#[derive(Resource, Default)]
pub struct FurnitureIndex {
	next_version: u64,
	cells: HashMap<Id, StoredFurnitureCell>,
	slots: Vec<richmond_building_components::FurnitureNode>,
}

impl FurnitureIndex {
	fn next_version(&mut self) -> Version {
		self.next_version += 1;
		Version(self.next_version)
	}

	fn refresh_slots(&mut self, developments: &DevelopmentEntryStore, region: Aabb3d) {
		self.slots.clear();
		for development in developments.developments_overlapping(region) {
			self.slots.extend(world_slots_in_region(development, region));
		}
	}
}

fn world_slots_in_region(
	development: &BuiltDevelopment,
	region: Aabb3d,
) -> Vec<richmond_building_components::FurnitureNode> {
	let mut out = Vec::new();
	for host in development.hosts() {
		let transform = host.transform();
		for node in host.furniture_nodes() {
			let world = world_slot(transform, node);
			let p = world.placement.translation;
			if p.x >= region.min.x
				&& p.x <= region.max.x
				&& p.z >= region.min.z
				&& p.z <= region.max.z
			{
				out.push(world);
			}
		}
	}
	out
}

impl SpatialIndex<FurnitureCell> for FurnitureIndex {
	fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId> {
		self.cells
			.iter()
			.filter(|(_, entry)| region.intersects(&entry.bounds))
			.map(|(id, _)| TrackedId(*id))
			.collect()
	}

	fn storage_status(&self, id: Id) -> StorageStatus {
		if self.cells.contains_key(&id) {
			StorageStatus::TrackedWithin
		} else {
			StorageStatus::NotTracked
		}
	}

	fn get(&self, id: Id) -> Option<&FurnitureCell> {
		self.cells.get(&id).map(|entry| &entry.value)
	}

	fn get_bounds(&self, id: Id) -> Option<Aabb3d> {
		self.cells.get(&id).map(|entry| entry.bounds)
	}

	fn version(&self, id: Id) -> Option<Version> {
		self.cells.get(&id).map(|entry| entry.version)
	}

	fn membership_revision(&self) -> u64 {
		self.next_version
	}

	fn insert(&mut self, id: Id, value: FurnitureCell, bounds: Aabb3d, _lod_ref: &LodRef) {
		let version = self.next_version();
		self.cells.insert(id, StoredFurnitureCell { value, bounds, version });
	}
}

impl GenerationScheme<FurnitureIndex> for FurnitureCell {
	fn original_ids_for(index: &mut FurnitureIndex, region: Aabb3d) -> Vec<OriginalId> {
		let mut ids = HashSet::new();
		for slot in &index.slots {
			let p = slot.placement.translation;
			if p.x < region.min.x || p.x > region.max.x || p.z < region.min.z || p.z > region.max.z
			{
				continue;
			}
			ids.insert(FurnitureCellExtent::from_cell_index(
				FurnitureCellExtent::cell_index_containing(p).0,
				FurnitureCellExtent::cell_index_containing(p).1,
			)
			.id());
		}
		ids.into_iter().map(OriginalId).collect()
	}

	fn build_with_id(
		index: &mut FurnitureIndex,
		id: Id,
		_lod_ref: &LodRef,
	) -> Option<(Self, Aabb3d)> {
		let extent = FurnitureCellExtent::from_id(id)?;
		let slots: Vec<_> = index
			.slots
			.iter()
			.filter(|slot| extent.contains_xz(slot.placement.translation))
			.cloned()
			.collect();
		if slots.is_empty() {
			return None;
		}
		let cell = FurnitureCell::new(extent, slots);
		let bounds = cell.bounds();
		Some((cell, bounds))
	}
}

#[derive(Resource, Clone, Copy, Debug, PartialEq)]
struct FurnitureGenerateBullseye {
	radius_m: f32,
	enabled: bool,
}

impl Default for FurnitureGenerateBullseye {
	fn default() -> Self {
		Self { radius_m: FURNITURE_GENERATE_RADIUS, enabled: true }
	}
}

impl LodRefreshRegions for FurnitureGenerateBullseye {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		refresh_status(self.enabled, self.radius_m, lod_ref)
	}
}

#[derive(Resource, Clone, Copy, Debug, PartialEq)]
struct FurniturePresentBullseye {
	radius_m: f32,
	enabled: bool,
}

impl Default for FurniturePresentBullseye {
	fn default() -> Self {
		Self { radius_m: FURNITURE_PRESENT_RADIUS, enabled: true }
	}
}

impl LodRefreshRegions for FurniturePresentBullseye {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		refresh_status(self.enabled, self.radius_m, lod_ref)
	}
}

fn refresh_status(enabled: bool, radius: f32, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
	if !enabled {
		return LodRefreshRegionsStatus::Unchanged;
	}
	let previous =
		FurnitureCellExtent::cell_index_containing(lod_ref.previous_transform.translation);
	let current = FurnitureCellExtent::cell_index_containing(lod_ref.current_transform.translation);
	if previous == current {
		LodRefreshRegionsStatus::Unchanged
	} else {
		LodRefreshRegionsStatus::Changed(xz_radius_aabb(
			lod_ref.current_transform.translation,
			radius,
		))
	}
}

#[derive(Resource, Default)]
struct FurniturePresenterState {
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
		while let Some(entities) = self.pending_despawn.pop_front() {
			for entity in entities {
				commands.entity(entity).despawn();
			}
		}
	}
}

/// Drive generate / present keep from the camera. Crossing a 50 m cell re-emits regions.
fn stream_furniture_keep(
	camera: Query<&Transform, With<Camera3d>>,
	mut generate: ResMut<FurnitureGenerateBullseye>,
	mut present: ResMut<FurniturePresentBullseye>,
	mut generate_keep: ResMut<LodGenerateKeepRegion<FurnitureLodChan>>,
	mut present_keep: ResMut<LodPresentKeepRegion<FurnitureLodChan>>,
	mut present_regions: MessageWriter<LodPresentRegion<FurnitureLodChan>>,
	mut previous_cell: Local<Option<(i32, i32)>>,
) {
	let Ok(camera) = camera.single() else {
		return;
	};
	generate.enabled = true;
	present.enabled = true;
	let generate_aabb = xz_radius_aabb(camera.translation, generate.radius_m);
	let present_aabb = xz_radius_aabb(camera.translation, present.radius_m);
	generate_keep.region = Some(generate_aabb);
	present_keep.region = Some(present_aabb);
	let current = FurnitureCellExtent::cell_index_containing(camera.translation);
	if previous_cell.as_ref() == Some(&current) {
		return;
	}
	present_regions.write(LodPresentRegion::new(present_aabb));
	*previous_cell = Some(current);
}

/// Materialize occupied 50 m cells from current Richmond developments.
fn generate_furniture_cells(
	developments: Res<DevelopmentEntryStore>,
	generate_keep: Res<LodGenerateKeepRegion<FurnitureLodChan>>,
	present_keep: Res<LodPresentKeepRegion<FurnitureLodChan>>,
	mut index: ResMut<FurnitureIndex>,
) {
	let Some(region) = generate_keep.region.or(present_keep.region) else {
		return;
	};
	index.refresh_slots(&developments, region);
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &region,
	};
	for OriginalId(id) in FurnitureCell::original_ids_for(&mut index, region) {
		if index.get(id).is_some() {
			continue;
		}
		if let Some((cell, bounds)) = FurnitureCell::build_with_id(&mut index, id, &lod_ref) {
			index.insert(id, cell, bounds, &lod_ref);
		}
	}
}

/// Spawn flattened hosts for the present neighborhood; despawn the rest.
fn present_furniture_cells(
	mut commands: Commands,
	present_keep: Res<LodPresentKeepRegion<FurnitureLodChan>>,
	index: Res<FurnitureIndex>,
	mut state: ResMut<FurniturePresenterState>,
) {
	let Some(region) = present_keep.region else {
		return;
	};
	let mut wanted = HashSet::new();
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
		if let Some(previous) = state.retire(id) {
			state.pending_despawn.push_back(previous.entities);
		}
		let entity = spawn_furniture_cell(&mut commands, cell.clone());
		state.presented.insert(id, PresentedFurnitureCell { version, entities: vec![entity] });
	}
	state.remove_stale(&mut commands, &wanted);
}

/// Generate + present + flattened-host refresh for the 50 m furniture neighborhood.
pub struct FurnitureStreamPlugin;

impl Plugin for FurnitureStreamPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<LodRefreshCorePlugin>() {
			app.add_plugins(LodRefreshCorePlugin);
		}
		app.init_resource::<FurnitureIndex>()
			.init_resource::<FurniturePresenterState>()
			.init_resource::<FurnitureGenerateBullseye>()
			.init_resource::<FurniturePresentBullseye>()
			.init_resource::<LodGenerateKeepRegion<FurnitureLodChan>>()
			.add_plugins(LodPresentRegionPlugin::<
				FurniturePresentBullseye,
				With<Camera3d>,
				FurnitureLodChan,
			>::default())
			.add_plugins(
				LodSceneRefreshRegionPlugin::<
					FurniturePresentBullseye,
					With<Camera>,
					FurnitureRefresh,
				>::default(),
			)
			.add_plugins(
				GimmeLodSceneRefreshPlugin::<FurnitureCell, FurnitureRefresh, With<Camera>>::without_full_scan_cull(),
			)
			.add_systems(
				Update,
				stream_furniture_keep.before(FurnitureStreamSystems::Generate),
			)
			.add_systems(
				Update,
				generate_furniture_cells.in_set(FurnitureStreamSystems::Generate),
			)
			.add_systems(
				Update,
				present_furniture_cells
					.in_set(FurnitureStreamSystems::Present)
					.after(FurnitureStreamSystems::Generate),
			);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use richmond_building_components::Placement;

	#[test]
	fn original_ids_bin_slots_to_cells() {
		let mut index = FurnitureIndex::default();
		index.slots.push(richmond_building_components::FurnitureNode::chest(Placement::IDENTITY));
		index.slots.push(richmond_building_components::FurnitureNode::chair(
			Placement::new(Vec3::new(60.0, 0.0, 0.0), 0.0),
		));
		let region = xz_radius_aabb(Vec3::ZERO, 200.0);
		let ids = FurnitureCell::original_ids_for(&mut index, region);
		assert_eq!(ids.len(), 2);
	}
}
