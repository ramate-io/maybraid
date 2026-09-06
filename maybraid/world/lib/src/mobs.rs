//! Streamed 400 m mob cells over Richmond and Chico selection models.

use std::collections::{HashMap, HashSet, VecDeque};

use bevy::ecs::system::SystemParam;
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::prelude::*;
use chico_forests::{ForestExtent, ForestIndex, LayeringKind, SelectedLayers};
use chico_vegetation_on_terrain_playground::WorldBaseTerrain;
use durham_terrain_models::{TerrainCellLayout, TerrainEntryStore};
use lod::gen::{
	GenerationScheme, Id, LodGenerateKeepRegion, LodGenerateRegion, OriginalId, SpatialIndex,
	StorageStatus, TrackedId, Version,
};
use lod::lod_ref::LodRef;
use lod::presentation::{LodPresentKeepRegion, LodPresentRegion, RegionPresenter};
use lod::scene::{LodRefreshRegions, LodRefreshRegionsStatus};
use lod::{
	LodGeneratePlugin, LodGenerateRegionPlugin, LodGenerateSystems, LodPresentCullPlugin,
	LodPresentPlugin, LodPresentRegionPlugin, LodPresentSystems, LodViewer,
};
use mob_groups::{GroupKind, MobEnvironmentSample, MobGroup, MobGroupsPlugin, MobWorldSample};
use procedural_common::NoiseParams;
use richmond_development_models::DevelopmentEntryStore;
use richmond_urbanization::{UrbanizationIndex, UrbanizationKind};

const MOB_CELL_EXTENT: f32 = 400.0;
const MOB_GENERATE_RADIUS: f32 = 3_000.0;
const MOB_PRESENT_RADIUS: f32 = 1_000.0;
const MOB_WORLD_SEED: u64 = 42;
const MOB_CELL_OCCUPANCY_PERCENT: u64 = 35;

#[derive(Clone, Copy, Debug, PartialEq)]
struct MobCellExtent {
	min: Vec3,
	max: Vec3,
}

impl MobCellExtent {
	fn from_cell_index(ix: i32, iz: i32) -> Self {
		let half = MOB_CELL_EXTENT * 0.5;
		Self {
			min: Vec3::new(
				ix as f32 * MOB_CELL_EXTENT - half,
				0.0,
				iz as f32 * MOB_CELL_EXTENT - half,
			),
			max: Vec3::new(
				ix as f32 * MOB_CELL_EXTENT + half,
				1.0,
				iz as f32 * MOB_CELL_EXTENT + half,
			),
		}
	}

	fn from_id(id: Id) -> Option<Self> {
		let bounds = id.origin_cell_bounds()?;
		let width = bounds.max.x - bounds.min.x;
		let depth = bounds.max.z - bounds.min.z;
		if (width - MOB_CELL_EXTENT).abs() > 1e-3 || (depth - MOB_CELL_EXTENT).abs() > 1e-3 {
			return None;
		}
		Some(Self { min: bounds.min.into(), max: bounds.max.into() })
	}

	fn cells_overlapping(region: Aabb3d) -> Vec<Self> {
		let min = Self::cell_index_containing(Vec3::new(region.min.x, 0.0, region.min.z));
		let max = Self::cell_index_containing(Vec3::new(
			(region.max.x - 1e-3).max(region.min.x),
			0.0,
			(region.max.z - 1e-3).max(region.min.z),
		));
		(min.0.min(max.0)..=min.0.max(max.0))
			.flat_map(|ix| {
				(min.1.min(max.1)..=min.1.max(max.1)).map(move |iz| Self::from_cell_index(ix, iz))
			})
			.collect()
	}

	fn cell_index_containing(position: Vec3) -> (i32, i32) {
		let half = MOB_CELL_EXTENT * 0.5;
		(
			((position.x + half) / MOB_CELL_EXTENT).floor() as i32,
			((position.z + half) / MOB_CELL_EXTENT).floor() as i32,
		)
	}

	fn center(self) -> Vec3 {
		(self.min + self.max) * 0.5
	}

	fn aabb(self) -> Aabb3d {
		Aabb3d::from_min_max(self.min, self.max)
	}

	fn id(self) -> Id {
		Id::from_cell(self.aabb())
	}

	fn index(self) -> (i32, i32) {
		Self::cell_index_containing(self.center())
	}
}

#[derive(Clone, Debug)]
struct WorldMobCell {
	extent: MobCellExtent,
	groups: Vec<MobGroup>,
}

#[derive(Clone)]
struct StoredMobCell {
	value: WorldMobCell,
	bounds: Aabb3d,
	version: Version,
}

#[derive(Resource, Clone, Default)]
struct WorldMobIndex {
	next_version: u64,
	cells: HashMap<Id, StoredMobCell>,
	forest_noise: NoiseParams,
	forest_layering: Option<LayeringKind>,
	urbanization_noise: NoiseParams,
	urbanization_kind: Option<UrbanizationKind>,
	models_ready: bool,
}

impl WorldMobIndex {
	fn next_version(&mut self) -> Version {
		self.next_version += 1;
		Version(self.next_version)
	}

	fn configure_from(&mut self, forest: &ForestIndex, urbanization: &UrbanizationIndex) {
		self.forest_noise = forest.noise;
		self.forest_layering = forest.layering;
		self.urbanization_noise = urbanization.noise;
		self.urbanization_kind = urbanization.kind;
		self.models_ready = true;
	}

	fn selected_layers(&self, xz: Vec2) -> SelectedLayers {
		let position = Vec3::new(xz.x, 0.0, xz.y);
		let (ix, iz) = ForestExtent::cell_index_containing(position);
		let extent = ForestExtent::from_cell_index(ix, iz);
		match self.forest_layering {
			Some(layering) => layering.layering().typical_layers(),
			None => chico_forests::select_cell(extent, self.forest_noise),
		}
	}

	fn urbanization_kind_at(&self, xz: Vec2) -> UrbanizationKind {
		if let Some(kind) = self.urbanization_kind {
			return kind;
		}
		let position = Vec3::new(xz.x, 0.0, xz.y);
		let (ix, iz) = richmond_urbanization::UrbanizationExtent::cell_index_containing(position);
		richmond_urbanization::select_kind(
			richmond_urbanization::UrbanizationExtent::from_cell_index(ix, iz),
			self.urbanization_noise,
		)
	}

	fn group_kind_at(&self, xz: Vec2, seed: u64) -> GroupKind {
		match self.urbanization_kind_at(xz) {
			UrbanizationKind::None => GroupKind::Wild,
			UrbanizationKind::RuralLife => GroupKind::Peaceful,
			UrbanizationKind::Townships | UrbanizationKind::Frontier => GroupKind::Frontier,
			UrbanizationKind::Colony => GroupKind::Warfront,
			UrbanizationKind::MixedAgeCity => {
				if mixed(seed) & 1 == 0 {
					GroupKind::Warfront
				} else {
					GroupKind::Dystopian
				}
			}
			UrbanizationKind::ModernCity => GroupKind::Dystopian,
		}
	}
}

impl MobWorldSample for WorldMobIndex {
	fn sample_mobs(&self, xz: Vec2) -> MobEnvironmentSample {
		let layers = self.selected_layers(xz);
		let vegetation = [layers.tufts, layers.understory, layers.lower_canopy, layers.upper_canopy]
			.into_iter()
			.filter(Option::is_some)
			.count() as f32
			/ 4.0;
		let urbanization = match self.urbanization_kind_at(xz) {
			UrbanizationKind::None => 0.0,
			UrbanizationKind::RuralLife => 0.2,
			UrbanizationKind::Frontier => 0.4,
			UrbanizationKind::Townships => 0.55,
			UrbanizationKind::Colony => 0.7,
			UrbanizationKind::MixedAgeCity => 0.85,
			UrbanizationKind::ModernCity => 1.0,
		};
		MobEnvironmentSample { elevation: Some(0.0), urbanization, vegetation }
	}
}

impl SpatialIndex<WorldMobCell> for WorldMobIndex {
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

	fn get(&self, id: Id) -> Option<&WorldMobCell> {
		self.cells.get(&id).map(|entry| &entry.value)
	}

	fn get_bounds(&self, id: Id) -> Option<Aabb3d> {
		self.cells.get(&id).map(|entry| entry.bounds)
	}

	fn version(&self, id: Id) -> Option<Version> {
		self.cells.get(&id).map(|entry| entry.version)
	}

	fn insert(&mut self, id: Id, value: WorldMobCell, bounds: Aabb3d, _lod_ref: &LodRef) {
		let version = self.next_version();
		self.cells.insert(id, StoredMobCell { value, bounds, version });
	}
}

impl GenerationScheme<WorldMobIndex> for WorldMobCell {
	fn original_ids_for(index: &mut WorldMobIndex, region: Aabb3d) -> Vec<OriginalId> {
		if !index.models_ready {
			return Vec::new();
		}
		MobCellExtent::cells_overlapping(region)
			.into_iter()
			.map(|extent| OriginalId(extent.id()))
			.collect()
	}

	fn build_with_id(
		index: &mut WorldMobIndex,
		id: Id,
		_lod_ref: &LodRef,
	) -> Option<(Self, Aabb3d)> {
		if !index.models_ready {
			return None;
		}
		let extent = MobCellExtent::from_id(id)?;
		let (ix, iz) = extent.index();
		let seed = cell_seed(ix, iz);
		let occupied = (ix == 0 && iz == 0)
			|| mixed(seed ^ 0x6d6f_622d_6365_6c6c) % 100 < MOB_CELL_OCCUPANCY_PERCENT;
		let groups = if occupied {
			let origin = Vec2::new(extent.center().x, extent.center().z);
			let kind = index.group_kind_at(origin, seed);
			vec![MobGroup::generate(kind, seed, origin, index)]
		} else {
			Vec::new()
		};
		Some((Self { extent, groups }, extent.aabb()))
	}
}

#[derive(Resource, Clone, Copy, Debug, PartialEq)]
struct MobGenerateBullseye {
	radius_m: f32,
	enabled: bool,
}

impl Default for MobGenerateBullseye {
	fn default() -> Self {
		Self { radius_m: MOB_GENERATE_RADIUS, enabled: true }
	}
}

impl LodRefreshRegions for MobGenerateBullseye {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		refresh_status(self.enabled, self.radius_m, lod_ref)
	}
}

#[derive(Resource, Clone, Copy, Debug, PartialEq)]
struct MobPresentBullseye {
	radius_m: f32,
	enabled: bool,
}

impl Default for MobPresentBullseye {
	fn default() -> Self {
		Self { radius_m: MOB_PRESENT_RADIUS, enabled: true }
	}
}

impl LodRefreshRegions for MobPresentBullseye {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		refresh_status(self.enabled, self.radius_m, lod_ref)
	}
}

fn refresh_status(enabled: bool, radius: f32, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
	if !enabled {
		return LodRefreshRegionsStatus::Unchanged;
	}
	let previous = MobCellExtent::cell_index_containing(lod_ref.previous_transform.translation);
	let current = MobCellExtent::cell_index_containing(lod_ref.current_transform.translation);
	if previous == current {
		LodRefreshRegionsStatus::Unchanged
	} else {
		LodRefreshRegionsStatus::Changed(xz_radius_aabb(
			lod_ref.current_transform.translation,
			radius,
		))
	}
}

#[derive(Clone, Copy, Debug, Default)]
struct MobLodChan;

#[derive(Resource, Default)]
struct WorldMobPresenterState {
	presented: HashMap<Id, PresentedMobCell>,
	pending_despawn: VecDeque<Vec<Entity>>,
}

struct PresentedMobCell {
	version: Version,
	entities: Vec<Entity>,
	hidden: bool,
}

#[derive(Component, Clone, Copy, Debug)]
struct WorldMobCellRoot;

#[derive(Component, Clone, Copy, Debug)]
struct WorldMobGroupRoot;

impl WorldMobPresenterState {
	fn retire(&mut self, id: Id) -> Option<PresentedMobCell> {
		self.presented.remove(&id)
	}

	fn remove(&mut self, commands: &mut Commands, id: Id) {
		if let Some(presented) = self.presented.remove(&id) {
			for entity in presented.entities {
				commands.entity(entity).despawn();
			}
		}
	}
}

#[derive(SystemParam)]
struct WorldMobPresenter<'w, 's> {
	commands: Commands<'w, 's>,
	state: ResMut<'w, WorldMobPresenterState>,
	terrain: Res<'w, TerrainEntryStore>,
	layout: Res<'w, TerrainCellLayout>,
	base: Res<'w, WorldBaseTerrain>,
	developments: Res<'w, DevelopmentEntryStore>,
}

impl WorldMobPresenter<'_, '_> {
	fn surface_height(&self, xz: Vec2) -> f32 {
		let raw = self
			.terrain
			.composed_height_at(&self.layout, xz.x, xz.y)
			.unwrap_or_else(|| self.base.0.height_at(xz.x, xz.y));
		let probe = Aabb3d::from_min_max(
			Vec3::new(xz.x - 0.5, -10_000.0, xz.y - 0.5),
			Vec3::new(xz.x + 0.5, 10_000.0, xz.y + 0.5),
		);
		self.developments.merged_pad_complex(probe).modify_elevation(raw, xz.x, xz.y)
	}
}

impl RegionPresenter<WorldMobCell, WorldMobIndex> for WorldMobPresenter<'_, '_> {
	fn presented_version(&self, id: Id) -> Option<Version> {
		self.state.presented.get(&id).map(|entry| entry.version)
	}

	fn handle(&mut self, id: Id, version: Version, cell: &WorldMobCell, _lod_ref: &LodRef) {
		if let Some(previous) = self.state.retire(id) {
			for entity in &previous.entities {
				self.commands.entity(*entity).insert(Visibility::Hidden);
			}
			self.state.pending_despawn.push_back(previous.entities);
		}
		let cell_index = cell.extent.index();
		let cell_root = self
			.commands
			.spawn((
				Name::new(format!("mob-cell {},{}", cell_index.0, cell_index.1)),
				WorldMobCellRoot,
				Transform::default(),
				Visibility::default(),
			))
			.id();
		for group in &cell.groups {
			let group_root = self
				.commands
				.spawn((
					Name::new(format!("{:?} mob group", group.kind)),
					WorldMobGroupRoot,
					ChildOf(cell_root),
					Transform::default(),
					Visibility::default(),
				))
				.id();
			for placed in &group.mobs {
				let mut transform = placed.transform;
				let xz = Vec2::new(transform.translation.x, transform.translation.z);
				transform.translation.y = self.surface_height(xz);
				let mob = placed.scene.spawn(&mut self.commands, transform);
				self.commands.entity(mob).insert(ChildOf(group_root));
			}
		}
		self.state
			.presented
			.insert(id, PresentedMobCell { version, entities: vec![cell_root], hidden: false });
	}

	fn hide(&mut self, id: Id) {
		if let Some(entry) = self.state.presented.get_mut(&id) {
			entry.hidden = true;
			for entity in &entry.entities {
				self.commands.entity(*entity).insert(Visibility::Hidden);
			}
		}
	}

	fn is_hidden(&self, id: Id) -> bool {
		self.state.presented.get(&id).is_some_and(|entry| entry.hidden)
	}

	fn presented_ids(&self) -> Vec<Id> {
		self.state.presented.keys().copied().collect()
	}

	fn remove_stale(&mut self, wanted: &HashSet<Id>) {
		let stale: Vec<_> = self
			.state
			.presented_ids()
			.into_iter()
			.filter(|id| !wanted.contains(id))
			.collect();
		for id in stale {
			self.state.remove(&mut self.commands, id);
		}
		while let Some(entities) = self.state.pending_despawn.pop_front() {
			for entity in entities {
				self.commands.entity(entity).despawn();
			}
		}
	}
}

impl WorldMobPresenterState {
	fn presented_ids(&self) -> Vec<Id> {
		self.presented.keys().copied().collect()
	}
}

#[derive(SystemParam)]
struct WorldMobStream<'w> {
	generate: ResMut<'w, MobGenerateBullseye>,
	present: ResMut<'w, MobPresentBullseye>,
	generate_regions: MessageWriter<'w, LodGenerateRegion<MobLodChan>>,
	present_regions: MessageWriter<'w, LodPresentRegion<MobLodChan>>,
	generate_keep: ResMut<'w, LodGenerateKeepRegion<MobLodChan>>,
	present_keep: ResMut<'w, LodPresentKeepRegion<MobLodChan>>,
}

fn sync_world_mob_models(
	forest: Res<ForestIndex>,
	urbanization: Res<UrbanizationIndex>,
	mut mobs: ResMut<WorldMobIndex>,
) {
	if !mobs.models_ready {
		mobs.configure_from(&forest, &urbanization);
	}
}

fn stream_world_mobs(
	camera: Query<&Transform, With<Camera3d>>,
	mut stream: WorldMobStream,
	mut previous_cell: Local<Option<(i32, i32)>>,
) {
	let Ok(camera) = camera.single() else {
		return;
	};
	stream.generate.enabled = true;
	stream.present.enabled = true;
	let generate = xz_radius_aabb(camera.translation, MOB_GENERATE_RADIUS);
	let present = xz_radius_aabb(camera.translation, MOB_PRESENT_RADIUS);
	stream.generate_keep.region = Some(generate);
	stream.present_keep.region = Some(present);
	let current = MobCellExtent::cell_index_containing(camera.translation);
	if previous_cell.as_ref() == Some(&current) {
		return;
	}
	stream.generate_regions.write(LodGenerateRegion::new(generate));
	stream.present_regions.write(LodPresentRegion::new(present));
	*previous_cell = Some(current);
}

fn xz_radius_aabb(center: Vec3, radius: f32) -> Aabb3d {
	Aabb3d::from_min_max(
		Vec3::new(center.x - radius, 0.0, center.z - radius),
		Vec3::new(center.x + radius, 1.0, center.z + radius),
	)
}

fn cell_seed(ix: i32, iz: i32) -> u64 {
	mixed(MOB_WORLD_SEED ^ (ix as u32 as u64).rotate_left(17) ^ (iz as u32 as u64).rotate_left(43))
}

fn mixed(mut value: u64) -> u64 {
	value ^= value >> 30;
	value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
	value ^= value >> 27;
	value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
	value ^ (value >> 31)
}

pub struct WorldMobsPlugin;

impl Plugin for WorldMobsPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<MobGroupsPlugin>() {
			app.add_plugins(MobGroupsPlugin);
		}
		app.init_resource::<WorldMobIndex>()
			.init_resource::<WorldMobPresenterState>()
			.init_resource::<MobGenerateBullseye>()
			.init_resource::<MobPresentBullseye>()
			.add_plugins(LodGenerateRegionPlugin::<
				MobGenerateBullseye,
				With<LodViewer>,
				MobLodChan,
			>::default())
			.add_plugins(LodGeneratePlugin::<
				WorldMobCell,
				WorldMobIndex,
				MobLodChan,
				With<LodViewer>,
			>::default())
			.add_plugins(LodPresentRegionPlugin::<
				MobPresentBullseye,
				With<LodViewer>,
				MobLodChan,
			>::default())
			.add_plugins(LodPresentPlugin::<
				WorldMobCell,
				WorldMobIndex,
				WorldMobPresenter<'_, '_>,
				MobLodChan,
				With<LodViewer>,
			>::default())
			.add_plugins(LodPresentCullPlugin::<
				WorldMobCell,
				WorldMobIndex,
				WorldMobPresenter<'_, '_>,
				MobLodChan,
			>::default())
			.configure_sets(Update, LodPresentSystems::Produce.after(LodGenerateSystems::Drain))
			.add_systems(
				Update,
				sync_world_mob_models
					.after(LodGenerateSystems::Produce)
					.before(LodGenerateSystems::Drain),
			)
			.add_systems(Update, stream_world_mobs.before(LodGenerateSystems::Produce));
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn mob_cells_cover_a_four_hundred_metre_lattice() {
		let region =
			Aabb3d::from_min_max(Vec3::new(-199.0, 0.0, -199.0), Vec3::new(201.0, 1.0, 201.0));
		assert_eq!(MobCellExtent::cells_overlapping(region).len(), 4);
	}

	#[test]
	fn origin_cell_is_always_populated_when_models_are_ready() -> anyhow::Result<()> {
		let mut index = WorldMobIndex { models_ready: true, ..default() };
		let extent = MobCellExtent::from_cell_index(0, 0);
		let transform = Transform::IDENTITY;
		let bounds = extent.aabb();
		let lod_ref = LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: &transform,
			current_transform: &transform,
			bounds: &bounds,
		};
		let (cell, _) = WorldMobCell::build_with_id(&mut index, extent.id(), &lod_ref)
			.ok_or_else(|| anyhow::anyhow!("origin mob cell did not generate"))?;
		assert!(!cell.groups.is_empty());
		Ok(())
	}
}
