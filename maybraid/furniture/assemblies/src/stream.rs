//! Neighborhood generate / present for 50 m [`FurnitureCell`] hosts.
//!
//! Reads Richmond [`BuiltDevelopment`]s and bins High slots into furniture cells.
//! Load ring is a 50 m-cell neighborhood — not the Richmond host tree.

use std::collections::{HashMap, HashSet, VecDeque};

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::{GenerationScheme, Id, OriginalId, SpatialIndex, StorageStatus, TrackedId, Version};
use lod::lod_ref::LodRef;
use lod::presentation::{LodPresentKeepRegion, LodPresentRegion};
use lod::scene::{LodRefreshRegions, LodRefreshRegionsStatus};
use lod::LodGenerateKeepRegion;
use lod::{
	LodPresentRegionPlugin, LodRefreshCorePlugin, LodSceneRefreshRegion,
	LodSceneRefreshRegionPlugin,
};
use lod_gimme::GimmeLodSceneRefreshPlugin;
use richmond_development_models::{BuiltDevelopment, DevelopmentEntryStore, DevelopmentHosts};

use crate::cell::{
	intersects_xz, world_slot, xz_radius_aabb, FurnitureCellExtent, FURNITURE_GENERATE_RADIUS,
	FURNITURE_PRESENT_RADIUS,
};
use crate::colliders::FurnitureWalkColliderPlugin;
use crate::host::{spawn_furniture_cell, FurnitureCell};

/// Cell id of a presented 50 m host. Crate restock keys off this, not the entity,
/// because [`FurniturePresenterState::remove_stale`] despawns the host outside the present ring.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentedFurnitureCellId(pub Id);

/// One 50 m host begin / rebuild per frame so fulfill can drain kits.
const FURNITURE_GENERATE_CELLS_PER_FRAME: usize = 1;
const FURNITURE_PRESENT_CELLS_PER_FRAME: usize = 1;

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

struct CachedDevelopmentSlots {
	version: Version,
	slots: Vec<richmond_building_components::FurnitureNode>,
}

/// Generated 50 m furniture cells plus the current world-space slot list.
#[derive(Resource, Default)]
pub struct FurnitureIndex {
	next_version: u64,
	cells: HashMap<Id, StoredFurnitureCell>,
	slots: Vec<richmond_building_components::FurnitureNode>,
	development_slots: HashMap<Id, CachedDevelopmentSlots>,
	slots_fingerprint: Vec<(Id, Version)>,
	slots_region_cell: Option<(i32, i32)>,
}

impl FurnitureIndex {
	fn next_version(&mut self) -> Version {
		self.next_version += 1;
		Version(self.next_version)
	}

	/// Generated 50 m host count (HUD).
	pub fn cell_count(&self) -> usize {
		self.cells.len()
	}

	/// Flattened High slots currently in the generate ring (HUD).
	pub fn slot_count(&self) -> usize {
		self.slots.len()
	}

	fn refresh_slots(&mut self, developments: &DevelopmentEntryStore, region: Aabb3d) {
		let tracked = developments.developments_overlapping_tracked(region);
		let mut fingerprint: Vec<_> =
			tracked.iter().map(|(id, version, _)| (*id, *version)).collect();
		fingerprint.sort();
		let region_cell = FurnitureCellExtent::cell_index_containing(Vec3::new(
			(region.min.x + region.max.x) * 0.5,
			0.0,
			(region.min.z + region.max.z) * 0.5,
		));
		if fingerprint == self.slots_fingerprint && Some(region_cell) == self.slots_region_cell {
			return;
		}
		self.slots.clear();
		for (id, version, development) in tracked {
			let cached = self.development_slots.entry(id).or_insert_with(|| {
				CachedDevelopmentSlots { version: Version(0), slots: Vec::new() }
			});
			if cached.version != version {
				cached.slots = world_slots_of(development);
				cached.version = version;
			}
			self.slots.extend(
				cached
					.slots
					.iter()
					.filter(|slot| {
						let p = slot.placement.translation;
						p.x >= region.min.x
							&& p.x <= region.max.x
							&& p.z >= region.min.z
							&& p.z <= region.max.z
					})
					.cloned(),
			);
		}
		self.slots_fingerprint = fingerprint;
		self.slots_region_cell = Some(region_cell);
	}
}

fn world_slots_of(
	development: &BuiltDevelopment,
) -> Vec<richmond_building_components::FurnitureNode> {
	let mut out = Vec::new();
	for host in development.hosts() {
		let transform = host.transform();
		for node in host.furniture_nodes() {
			out.push(world_slot(transform, node));
		}
		for node in furniture_usage_areas::expand_usages(host.furniture_usage_nodes()) {
			out.push(world_slot(transform, node));
		}
	}
	out
}

fn slots_match(
	left: &[richmond_building_components::FurnitureNode],
	right: &[richmond_building_components::FurnitureNode],
) -> bool {
	left.len() == right.len()
		&& left.iter().zip(right).all(|(a, b)| {
			a.geometry == b.geometry
				&& (a.placement.translation - b.placement.translation).length_squared() < 1e-4
		})
}

impl SpatialIndex<FurnitureCell> for FurnitureIndex {
	fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId> {
		self.cells
			.iter()
			.filter(|(_, entry)| intersects_xz(region, entry.bounds))
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
			ids.insert(
				FurnitureCellExtent::from_cell_index(
					FurnitureCellExtent::cell_index_containing(p).0,
					FurnitureCellExtent::cell_index_containing(p).1,
				)
				.id(),
			);
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
		if let Some(entities) = self.pending_despawn.pop_front() {
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
	mut refresh_regions: MessageWriter<LodSceneRefreshRegion<FurnitureRefresh>>,
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
	refresh_regions.write(LodSceneRefreshRegion::new(present_aabb));
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
	let mut built = 0usize;
	for OriginalId(id) in FurnitureCell::original_ids_for(&mut index, region) {
		let Some((cell, bounds)) = FurnitureCell::build_with_id(&mut index, id, &lod_ref) else {
			continue;
		};
		if index.get(id).is_some_and(|existing| slots_match(&existing.slots, &cell.slots)) {
			continue;
		}
		if built >= FURNITURE_GENERATE_CELLS_PER_FRAME {
			break;
		}
		index.insert(id, cell, bounds, &lod_ref);
		built += 1;
	}
}

/// Spawn flattened hosts for the present neighborhood; despawn the rest.
fn present_furniture_cells(
	mut commands: Commands,
	camera: Query<&Transform, With<Camera3d>>,
	present_keep: Res<LodPresentKeepRegion<FurnitureLodChan>>,
	index: Res<FurnitureIndex>,
	mut state: ResMut<FurniturePresenterState>,
	mut refresh_regions: MessageWriter<LodSceneRefreshRegion<FurnitureRefresh>>,
) {
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

/// Generate + present + flattened-host refresh for the 50 m furniture neighborhood.
pub struct FurnitureStreamPlugin;

impl Plugin for FurnitureStreamPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<LodRefreshCorePlugin>() {
			app.add_plugins(LodRefreshCorePlugin);
		}
		if !app.is_plugin_added::<FurnitureWalkColliderPlugin>() {
			app.add_plugins(FurnitureWalkColliderPlugin);
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
	fn matching_slots_ignore_finish_seed() {
		let a = richmond_building_components::FurnitureNode::chair(Placement::IDENTITY)
			.with_finish_seed(1);
		let b = richmond_building_components::FurnitureNode::chair(Placement::IDENTITY)
			.with_finish_seed(9);
		assert!(slots_match(&[a.clone()], &[b]));
		let moved =
			richmond_building_components::FurnitureNode::chair(Placement::new(Vec3::X, 0.0));
		assert!(!slots_match(&[a], &[moved]));
	}

	#[test]
	fn original_ids_bin_slots_to_cells() {
		let mut index = FurnitureIndex::default();
		index
			.slots
			.push(richmond_building_components::FurnitureNode::chest(Placement::IDENTITY));
		index
			.slots
			.push(richmond_building_components::FurnitureNode::chair(Placement::new(
				Vec3::new(60.0, 0.0, 0.0),
				0.0,
			)));
		let region = xz_radius_aabb(Vec3::ZERO, 200.0);
		let ids = FurnitureCell::original_ids_for(&mut index, region);
		assert_eq!(ids.len(), 2);
	}

	#[test]
	fn tracked_ids_find_cells_below_sea_level() {
		let mut index = FurnitureIndex::default();
		let extent = FurnitureCellExtent::from_cell_index(18, 36);
		let slot = richmond_building_components::FurnitureNode::chair(Placement::new(
			Vec3::new(extent.center().x, -141.0, extent.center().z),
			0.0,
		));
		let cell = FurnitureCell::new(extent, vec![slot]);
		let bounds = cell.bounds();
		let identity = Transform::IDENTITY;
		let lod_ref = LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: &identity,
			current_transform: &identity,
			bounds: &bounds,
		};
		index.insert(extent.id(), cell, bounds, &lod_ref);
		let camera = xz_radius_aabb(Vec3::new(extent.center().x, -141.0, extent.center().z), 125.0);
		assert_eq!(SpatialIndex::<FurnitureCell>::tracked_ids_for(&index, camera).len(), 1);
		let sea = Aabb3d::from_min_max(
			Vec3::new(extent.min.x, 0.0, extent.min.z),
			Vec3::new(extent.max.x, 1.0, extent.max.z),
		);
		assert_eq!(SpatialIndex::<FurnitureCell>::tracked_ids_for(&index, sea).len(), 1);
	}
}
