//! Basic [`RegionPresenter`] for terrain cells.
//!
//! Generation and presentation stay separate: present reads the entry store and
//! spawns each cell's [`lod::gen::LodScene`] via [`Commands::spawn_scene`].

use crate::terrain::cell::{universal_bounds, TerrainCellLayout};
use crate::terrain::config::TerrainConfig;
use crate::terrain::index::TerrainEntryStore;
use crate::terrain::{Terrain, TerrainColliderHost, TERRAIN_CELL_SIZE};
use bevy::ecs::system::SystemParam;
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value};
use durham_terrain::shaders::DurhamTerrainShader;
use lod::gen::{
	GenerationScheme, Id, LodScene, OriginalId, RegionPresenter, SpatialIndex, StorageStatus,
	TrackedId, Version,
};
use lod::lod_host_scene_pending;
use lod::lod_ref::LodRef;
use render_item::sdf::cpu_shot::WallFaces;
use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;

/// One concentric mesh-LOD band on the fine (base-sized) cell grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainMeshLodBand {
	/// Inclusive Chebyshev cell-index radius for this band.
	pub max_radius_cells: i32,
	/// Cascade `res_2` (`2^res_2` samples along each axis).
	pub res_2: u8,
}

/// Config / material / mesh resolution used when building terrain instances.
///
/// Materialized once under [`Id::Universal`] via [`GenerationScheme`].
///
/// Fine-grid LOD: first [`TerrainMeshLodBand`] with `radius ≤ max_radius_cells`
/// wins ([`Self::lod_bands`] must be sorted ascending by radius). Radii past the
/// last band reuse that band's `res_2`.
///
/// When [`Self::outer_add_walls`] is set, CpuShot skirts are emitted on faces
/// shared with a neighbor whose LOD `res_2` differs, and on nested macro
/// seams listed in [`Self::macro_seam_half_extents`].
///
/// Cells whose XZ edge is at least [`Self::macro_cell_min_size`] skip the fine
/// bands and use [`Self::macro_res_2`] (macro outer-ring tiles).
#[derive(Resource, Clone)]
pub struct TerrainPresentationAssets {
	pub config: TerrainConfig,
	pub material: Handle<DurhamTerrainShader>,
	/// Concentric fine-grid LOD bands (ascending `max_radius_cells`).
	pub lod_bands: Vec<TerrainMeshLodBand>,
	/// When true, enable per-face CpuShot walls on LOD / fine–macro boundaries.
	pub outer_add_walls: bool,
	/// Inclusive Chebyshev radius of the fine grid (for fine→macro wall faces).
	pub fine_grid_max_radius: Option<i32>,
	/// World half-extents of nested footprints that macro faces may abut
	/// (fine edge, 2×→4× edge, …), for macro→inner wall faces.
	pub macro_seam_half_extents: Vec<f32>,
	/// XZ edge length at/above which a cell is treated as a macro outer tile.
	pub macro_cell_min_size: Option<f32>,
	/// Mesh resolution for macro outer tiles. Defaults to 3 when unset.
	pub macro_res_2: Option<u8>,
}

impl TerrainPresentationAssets {
	fn res_2_for_radius(&self, radius: i32) -> u8 {
		for band in &self.lod_bands {
			if radius <= band.max_radius_cells {
				return band.res_2;
			}
		}
		self.lod_bands.last().map(|b| b.res_2).unwrap_or(0)
	}

	fn wall_toward_neighbor(&self, my_r: i32, my_res: u8, n_r: i32) -> bool {
		if self.res_2_for_radius(n_r) != my_res {
			return true;
		}
		if let Some(fine_max) = self.fine_grid_max_radius {
			let my_in = my_r <= fine_max;
			let n_in = n_r <= fine_max;
			if my_in != n_in {
				return true;
			}
		}
		false
	}

	fn wall_faces_for_fine_cell(&self, ix: i32, iz: i32) -> WallFaces {
		if !self.outer_add_walls || self.lod_bands.is_empty() {
			return WallFaces::NONE;
		}
		let my_r = ix.abs().max(iz.abs());
		let mine = self.res_2_for_radius(my_r);
		WallFaces {
			neg_x: self.wall_toward_neighbor(my_r, mine, (ix - 1).abs().max(iz.abs())),
			pos_x: self.wall_toward_neighbor(my_r, mine, (ix + 1).abs().max(iz.abs())),
			neg_z: self.wall_toward_neighbor(my_r, mine, ix.abs().max((iz - 1).abs())),
			pos_z: self.wall_toward_neighbor(my_r, mine, ix.abs().max((iz + 1).abs())),
		}
	}

	fn wall_faces_for_macro_cell(&self, bounds: Aabb3d) -> WallFaces {
		if !self.outer_add_walls {
			return WallFaces::NONE;
		}
		if self.macro_seam_half_extents.is_empty() {
			return WallFaces::ALL;
		}
		let min = Vec3::from(bounds.min);
		let max = Vec3::from(bounds.max);
		let eps = 1.0;
		let mut faces = WallFaces::NONE;
		for &half in &self.macro_seam_half_extents {
			faces.neg_x |= (min.x - half).abs() < eps;
			faces.pos_x |= (max.x - (-half)).abs() < eps;
			faces.neg_z |= (min.z - half).abs() < eps;
			faces.pos_z |= (max.z - (-half)).abs() < eps;
		}
		faces
	}

	/// `(res_2, wall_faces)` for a terrain origin cell AABB.
	pub fn mesh_params_for_cell(&self, bounds: Aabb3d) -> (u8, WallFaces) {
		let min = Vec3::from(bounds.min);
		let max = Vec3::from(bounds.max);
		let cell_size = (max.x - min.x).max(1e-3);
		if let Some(macro_min) = self.macro_cell_min_size {
			if cell_size + 1e-3 >= macro_min {
				return (self.macro_res_2.unwrap_or(3), self.wall_faces_for_macro_cell(bounds));
			}
		}
		let ix = (min.x / cell_size).floor() as i32;
		let iz = (min.z / cell_size).floor() as i32;
		let radius = ix.abs().max(iz.abs());
		(self.res_2_for_radius(radius), self.wall_faces_for_fine_cell(ix, iz))
	}
}

/// Bootstrap source used only when first materializing [`TerrainPresentationAssets`]
/// at [`Id::Universal`]. Consumers should depend on
/// [`lod::gen::GeneratingSpatialIndex`]`<TerrainPresentationAssets>` instead.
pub trait BootstrapTerrainPresentationAssets {
	fn bootstrap_terrain_presentation_assets(&self) -> TerrainPresentationAssets;
}

impl<S> GenerationScheme<S> for TerrainPresentationAssets
where
	S: BootstrapTerrainPresentationAssets,
{
	fn original_ids_for(_spatial_index: &mut S, _region: Aabb3d) -> Vec<OriginalId> {
		vec![OriginalId::universal()]
	}

	fn build_with_id(spatial_index: &mut S, id: Id, _lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		if id != Id::Universal {
			return None;
		}
		Some((spatial_index.bootstrap_terrain_presentation_assets(), universal_bounds()))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}

/// Runtime presentation bookkeeping: last presented version and root entity per id.
#[derive(Resource, Default)]
pub struct TerrainPresenterState {
	presented: HashMap<Id, PresentedEntry>,
}

#[derive(Debug, Clone, Copy)]
struct PresentedEntry {
	version: Version,
	entity: Entity,
}

/// Marks a spawned terrain scene root as belonging to a presented id.
#[derive(Component, Debug, Clone, Copy)]
pub struct PresentedTerrainScene(pub Id);

/// Near-stream terrain host (160 m cells, render + stable collision).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TerrainNear;

/// Far-stream terrain host (320 m render-only cells).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TerrainFar;

/// Background-stream terrain host (640 m render-only cells).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TerrainBackground;

/// Marker policy for one moving terrain scale presenter.
pub trait TerrainStreamMarker: Component + Default {
	const CELL_SIZE_MULTIPLE: f32;
	const COLLIDER: bool;
}

impl TerrainStreamMarker for TerrainNear {
	const CELL_SIZE_MULTIPLE: f32 = 1.0;
	const COLLIDER: bool = true;
}

impl TerrainStreamMarker for TerrainFar {
	const CELL_SIZE_MULTIPLE: f32 = 2.0;
	const COLLIDER: bool = false;
}

impl TerrainStreamMarker for TerrainBackground {
	const CELL_SIZE_MULTIPLE: f32 = 4.0;
	const COLLIDER: bool = false;
}

impl TerrainPresenterState {
	pub fn clear(&mut self, commands: &mut Commands) {
		for entry in self.presented.values() {
			commands.entity(entry.entity).despawn();
		}
		self.presented.clear();
	}
}

/// Independent runtime bookkeeping for one moving terrain scale.
#[derive(Resource)]
pub struct TerrainStreamPresenterState<M: TerrainStreamMarker> {
	presented: HashMap<Id, PresentedEntry>,
	_marker: PhantomData<M>,
}

impl<M: TerrainStreamMarker> Default for TerrainStreamPresenterState<M> {
	fn default() -> Self {
		Self { presented: HashMap::new(), _marker: PhantomData }
	}
}

impl<M: TerrainStreamMarker> TerrainStreamPresenterState<M> {
	fn clear(&mut self, commands: &mut Commands) {
		for entry in self.presented.values() {
			commands.entity(entry.entity).despawn();
		}
		self.presented.clear();
	}
}

/// Read-only spatial-index view over the terrain entry map for presentation.
///
/// Insert is unsupported; generate through [`crate::terrain::AvianTerrainIndex`] first.
pub struct TerrainStoreView<'a> {
	store: &'a TerrainEntryStore,
	_layout: &'a TerrainCellLayout,
}

impl<'a> TerrainStoreView<'a> {
	pub fn new(store: &'a TerrainEntryStore, layout: &'a TerrainCellLayout) -> Self {
		Self { store, _layout: layout }
	}
}

impl SpatialIndex<Terrain> for TerrainStoreView<'_> {
	fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId> {
		self.store
			.terrain
			.iter()
			.filter(|(_, entry)| region.intersects(&entry.bounds))
			.map(|(id, _)| TrackedId(*id))
			.collect()
	}

	fn storage_status(&self, id: Id) -> StorageStatus {
		if self.store.terrain.contains_key(&id) {
			StorageStatus::TrackedWithin
		} else {
			StorageStatus::NotTracked
		}
	}

	fn get(&self, id: Id) -> Option<&Terrain> {
		self.store.terrain.get(&id).map(|e| &e.value)
	}

	fn get_bounds(&self, id: Id) -> Option<Aabb3d> {
		self.store.terrain.get(&id).map(|e| e.bounds)
	}

	fn version(&self, id: Id) -> Option<Version> {
		self.store.terrain.get(&id).map(|e| e.version)
	}

	fn insert(&mut self, _id: Id, _t: Terrain, _bounds: Aabb3d, _lod_ref: &LodRef) {
		panic!("TerrainStoreView is read-only; insert via AvianTerrainIndex");
	}
}

/// System-local presenter: spawns terrain `bsn!` scenes and tracks versions.
#[derive(SystemParam)]
pub struct TerrainRegionPresenter<'w, 's> {
	commands: Commands<'w, 's>,
	state: ResMut<'w, TerrainPresenterState>,
}

/// Scale-filtered presenter used by moving near / far / background streams.
#[derive(SystemParam)]
pub struct TerrainStreamRegionPresenter<'w, 's, M: TerrainStreamMarker> {
	commands: Commands<'w, 's>,
	state: ResMut<'w, TerrainStreamPresenterState<M>>,
}

pub type TerrainNearRegionPresenter<'w, 's> = TerrainStreamRegionPresenter<'w, 's, TerrainNear>;
pub type TerrainFarRegionPresenter<'w, 's> = TerrainStreamRegionPresenter<'w, 's, TerrainFar>;
pub type TerrainBackgroundRegionPresenter<'w, 's> =
	TerrainStreamRegionPresenter<'w, 's, TerrainBackground>;

impl<'w, 's> TerrainRegionPresenter<'w, 's> {
	pub fn clear_presented(&mut self) {
		self.state.clear(&mut self.commands);
	}
}

impl<M: TerrainStreamMarker> TerrainStreamRegionPresenter<'_, '_, M> {
	fn matches(value: &Terrain) -> bool {
		let size = Vec3::from(value.cell.max - value.cell.min).x;
		(size - M::CELL_SIZE_MULTIPLE * TERRAIN_CELL_SIZE).abs() < 1e-3
	}

	/// Present this scale's generated cells and retire hosts outside its keep.
	pub fn present(&mut self, store: &TerrainEntryStore, region: Aabb3d, lod_ref: &LodRef) {
		let wanted: HashSet<Id> = store
			.terrain
			.iter()
			.filter(|(_, entry)| region.intersects(&entry.bounds) && Self::matches(&entry.value))
			.map(|(id, _)| *id)
			.collect();

		for id in &wanted {
			let Some(entry) = store.terrain.get(id) else {
				continue;
			};
			if self.state.presented.get(id).is_some_and(|shown| shown.version == entry.version) {
				continue;
			}
			if let Some(previous) = self.state.presented.remove(id) {
				self.commands.entity(previous.entity).despawn();
			}
			let mut value = entry.value.clone();
			value.presented_water = store.water(*id).cloned();
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
				.insert((value, PresentedTerrainScene(*id), M::default()))
				.id();
			if M::COLLIDER {
				self.commands.entity(entity).insert(TerrainColliderHost);
			}
			self.state
				.presented
				.insert(*id, PresentedEntry { version: entry.version, entity });
		}

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

	pub fn clear_presented(&mut self) {
		self.state.clear(&mut self.commands);
	}
}

impl<'a, 'w, 's> RegionPresenter<Terrain, TerrainStoreView<'a>> for TerrainRegionPresenter<'w, 's> {
	fn presented_version(&self, id: Id) -> Option<Version> {
		self.state.presented.get(&id).map(|e| e.version)
	}

	fn handle(&mut self, id: Id, version: Version, value: &Terrain, lod_ref: &LodRef) {
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
			.insert((value.clone(), PresentedTerrainScene(id), TerrainColliderHost))
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
