//! Basic [`RegionPresenter`] for terrain cells.
//!
//! Generation and presentation stay separate: present reads the entry store and
//! spawns a posed fill entity ([`Terrain::spawn_fill`]). [`Mesh3d`] and the
//! Near trimesh land on that same entity.

use crate::terrain::cell::{expand_aabb_xz, TerrainCellLayout, TERRAIN_CELL_SIZE};
use crate::terrain::config::TerrainConfig;
use crate::terrain::Terrain;
use crate::water::{PresentedWaterScene, Water};
use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use lod::gen::{Id, LodScene, RegionPresenter, Version};
use lod::hcsg::{HcsgStorage, StoredEntry};
use lod::lod_ref::LodRef;
use lod::LodSceneLevel;
use render_item::sdf::cpu_shot::WallFaces;
use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use std::sync::Arc;
use terrain_shaders::TerrainShader;

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
/// Materialized once under [`Id::Universal`] via [`lod::gen::GenerationScheme`].
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
	pub material: Handle<TerrainShader>,
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

	fn wall_faces_for_fine_cell(&self, ix: i32, iz: i32, layout: &TerrainCellLayout) -> WallFaces {
		if !self.outer_add_walls || self.lod_bands.is_empty() {
			return WallFaces::NONE;
		}
		let my_r = layout.fine_cell_radius(ix, iz);
		let mine = self.res_2_for_radius(my_r);
		WallFaces {
			neg_x: self.wall_toward_neighbor(my_r, mine, layout.fine_cell_radius(ix - 1, iz)),
			pos_x: self.wall_toward_neighbor(my_r, mine, layout.fine_cell_radius(ix + 1, iz)),
			neg_z: self.wall_toward_neighbor(my_r, mine, layout.fine_cell_radius(ix, iz - 1)),
			pos_z: self.wall_toward_neighbor(my_r, mine, layout.fine_cell_radius(ix, iz + 1)),
		}
	}

	fn macro_inner_footprints(layout: &TerrainCellLayout) -> Vec<Aabb3d> {
		let mut inners = vec![layout.fine_request_region()];
		let mut covered = layout.fine_request_region();
		for (i, outer) in layout.outer_rings.iter().enumerate() {
			if outer.rows <= 0 {
				continue;
			}
			covered = expand_aabb_xz(covered, outer.rows as f32 * outer.cell_size.max(1e-3));
			if i + 1 < layout.outer_rings.len() {
				inners.push(covered);
			}
		}
		inners
	}

	fn wall_faces_for_macro_cell(&self, bounds: Aabb3d, layout: &TerrainCellLayout) -> WallFaces {
		if !self.outer_add_walls {
			return WallFaces::NONE;
		}
		let inners = Self::macro_inner_footprints(layout);
		if inners.is_empty() {
			return WallFaces::ALL;
		}
		let min = Vec3::from(bounds.min);
		let max = Vec3::from(bounds.max);
		let eps = 1.0;
		let mut faces = WallFaces::NONE;
		for inner in inners {
			let inner_min = Vec3::from(inner.min);
			let inner_max = Vec3::from(inner.max);
			faces.neg_x |= (min.x - inner_max.x).abs() < eps || (min.x - inner_min.x).abs() < eps;
			faces.pos_x |= (max.x - inner_min.x).abs() < eps || (max.x - inner_max.x).abs() < eps;
			faces.neg_z |= (min.z - inner_max.z).abs() < eps || (min.z - inner_min.z).abs() < eps;
			faces.pos_z |= (max.z - inner_min.z).abs() < eps || (max.z - inner_max.z).abs() < eps;
		}
		faces
	}

	fn wall_faces_for_stream_cell(
		&self,
		layout: &TerrainCellLayout,
		ring: crate::terrain::cell::TerrainCellRing,
	) -> WallFaces {
		if !self.outer_add_walls {
			return WallFaces::NONE;
		}
		// Inner holes and Near/Far rims sit under the next-finer stream (draw
		// overlap). Only the outermost Background skirt faces empty space, and
		// any of its cells can be on the edge wherever the stream is anchored.
		if layout.is_outermost_stream_ring(ring) {
			WallFaces::ALL
		} else {
			WallFaces::NONE
		}
	}

	/// `(res_2, wall_faces)` for a terrain origin cell AABB.
	///
	/// Fine-grid LOD radius is Chebyshev distance from the current layout window
	/// center, so newly admitted cells use the sliding stream's bands.
	pub fn mesh_params_for_cell(
		&self,
		bounds: Aabb3d,
		layout: &TerrainCellLayout,
	) -> (u8, WallFaces) {
		let min = Vec3::from(bounds.min);
		let max = Vec3::from(bounds.max);
		let cell_size = (max.x - min.x).max(1e-3);
		if let Some(ring) = layout.stream_ring_for_cell_size(cell_size) {
			return (ring.res_2, self.wall_faces_for_stream_cell(layout, ring));
		}
		if let Some(macro_min) = self.macro_cell_min_size {
			if cell_size + 1e-3 >= macro_min {
				return (
					self.macro_res_2.unwrap_or(3),
					self.wall_faces_for_macro_cell(bounds, layout),
				);
			}
		}
		let ix = (min.x / cell_size).floor() as i32;
		let iz = (min.z / cell_size).floor() as i32;
		let radius = layout.fine_cell_radius(ix, iz);
		(self.res_2_for_radius(radius), self.wall_faces_for_fine_cell(ix, iz, layout))
	}
}

lod::seeded_root!(TerrainPresentationAssets);

/// Runtime presentation bookkeeping: last presented terrain and water versions
/// and the root entity per id.
///
/// Terrain and water generate independently, so a cell can be presented before
/// its water exists. Water is reconciled against its own version, not the
/// terrain version.
#[derive(Resource, Default)]
pub struct TerrainPresenterState {
	presented: HashMap<Id, PresentedEntry>,
}

#[derive(Debug, Clone, Copy)]
struct PresentedEntry {
	version: Version,
	entity: Entity,
	water: Option<Entity>,
	water_version: Option<Version>,
	level: LodSceneLevel,
}

impl PresentedEntry {
	fn new(version: Version, entity: Entity, level: LodSceneLevel) -> Self {
		Self { version, entity, water: None, water_version: None, level }
	}

	/// Attach, replace, or drop the water child so it matches `water`.
	fn sync_water(
		&mut self,
		commands: &mut Commands,
		id: Id,
		water: Option<StoredEntry<Arc<Water>>>,
	) {
		let version = water.as_ref().map(|entry| entry.version);
		if self.water_version == version {
			return;
		}
		if let Some(previous) = self.water.take() {
			commands.entity(previous).try_despawn();
		}
		self.water = water
			.as_ref()
			.map(|entry| attach_water(commands, id, self.entity, entry.value.as_ref()));
		self.water_version = version;
	}
}

/// Marks a spawned terrain scene root as belonging to a presented id.
#[derive(Component, Debug, Clone, Copy)]
pub struct PresentedTerrainScene(pub Id);

/// Posed fill root. CpuShot verts are local; [`Transform`] is the cascade origin.
/// Near cells also carry [`crate::terrain::TerrainColliderMeshSource`] on this entity.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TerrainVisualHost;

/// Near-stream terrain host (160 m High cells; collision scale).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TerrainNear;

/// Far-stream terrain host (320 m render-only cells).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TerrainFar;

/// Background-stream terrain host (640 m render-only cells).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TerrainBackground;

/// Marker policy for one moving terrain scale presenter.
pub trait TerrainStreamMarker: Component + Default + Send + Sync + 'static {
	const CELL_SIZE_MULTIPLE: f32;
}

impl TerrainStreamMarker for TerrainNear {
	const CELL_SIZE_MULTIPLE: f32 = 1.0;
}

impl TerrainStreamMarker for TerrainFar {
	const CELL_SIZE_MULTIPLE: f32 = 2.0;
}

impl TerrainStreamMarker for TerrainBackground {
	const CELL_SIZE_MULTIPLE: f32 = 4.0;
}

fn fill_visibility(draw: bool) -> Visibility {
	if draw {
		Visibility::Inherited
	} else {
		Visibility::Hidden
	}
}

fn attach_water(commands: &mut Commands, id: Id, parent: Entity, water: &Water) -> Entity {
	let entity = water.spawn_fill(commands, Transform::IDENTITY);
	commands.entity(entity).insert((PresentedWaterScene(id), ChildOf(parent)));
	entity
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
			commands.entity(entry.entity).try_despawn();
		}
		self.presented.clear();
	}
}

impl TerrainPresenterState {
	pub fn clear(&mut self, commands: &mut Commands) {
		for entry in self.presented.values() {
			commands.entity(entry.entity).try_despawn();
		}
		self.presented.clear();
	}

	#[cfg(test)]
	pub(crate) fn insert_for_test(&mut self, id: Id, version: Version, entity: Entity) {
		self.presented
			.insert(id, PresentedEntry::new(version, entity, LodSceneLevel::High));
	}

	#[cfg(test)]
	pub(crate) fn presented_version(&self, id: Id) -> Option<Version> {
		self.presented.get(&id).map(|entry| entry.version)
	}

	#[cfg(test)]
	pub(crate) fn presented_water(&self, id: Id) -> Option<(Entity, Version)> {
		let entry = self.presented.get(&id)?;
		entry.water.zip(entry.water_version)
	}
}

/// System-local presenter: spawns terrain `bsn!` scenes and tracks versions.
#[derive(SystemParam)]
pub struct TerrainRegionPresenter<'w, 's> {
	commands: Commands<'w, 's>,
	state: ResMut<'w, TerrainPresenterState>,
	store: Res<'w, HcsgStorage>,
}

impl<'w, 's> TerrainRegionPresenter<'w, 's> {
	pub fn clear_presented(&mut self) {
		self.state.clear(&mut self.commands);
	}

	/// Bring every presented cell's water up to the stored [`Water`] version.
	/// Run after [`RegionPresenter::present`]; that pass only compares terrain
	/// versions, so water generated later would otherwise never attach.
	pub fn sync_water(&mut self) {
		for (id, shown) in &mut self.state.presented {
			shown.sync_water(&mut self.commands, *id, self.store.entry::<Water>(*id));
		}
	}
}

/// Scale-filtered presenter used by moving near / far / background streams.
#[derive(SystemParam)]
pub struct TerrainStreamRegionPresenter<'w, 's, M: TerrainStreamMarker> {
	commands: Commands<'w, 's>,
	state: ResMut<'w, TerrainStreamPresenterState<M>>,
	store: Res<'w, HcsgStorage>,
}

pub type TerrainNearRegionPresenter<'w, 's> = TerrainStreamRegionPresenter<'w, 's, TerrainNear>;
pub type TerrainFarRegionPresenter<'w, 's> = TerrainStreamRegionPresenter<'w, 's, TerrainFar>;
pub type TerrainBackgroundRegionPresenter<'w, 's> =
	TerrainStreamRegionPresenter<'w, 's, TerrainBackground>;

impl<M: TerrainStreamMarker> TerrainStreamRegionPresenter<'_, '_, M> {
	fn matches(value: &Terrain) -> bool {
		let size = Vec3::from(value.cell.max - value.cell.min).x;
		(size - M::CELL_SIZE_MULTIPLE * TERRAIN_CELL_SIZE).abs() < 1e-3
	}

	/// Present this scale's keep-region cells. The annulus hole is LodScene
	/// banding (High = mesh, Medium / Low = empty), not a presenter High filter.
	pub fn present(
		&mut self,
		store: &HcsgStorage,
		layout: &TerrainCellLayout,
		region: Aabb3d,
		lod_ref: &LodRef,
	) {
		if layout
			.stream_ring_for_cell_size(M::CELL_SIZE_MULTIPLE * TERRAIN_CELL_SIZE)
			.is_none()
		{
			return;
		};
		let wanted: HashSet<Id> = store
			.overlapping::<Terrain>(region)
			.into_iter()
			.filter(|id| {
				store.get::<Terrain>(*id).is_some_and(|value| {
					let level = value.scene_lod_level(lod_ref);
					Self::matches(value.as_ref())
						&& (crate::terrain::stream_lod::stream_banded_draws(value.as_ref(), level)
							|| value.seeds_collision())
				})
			})
			.collect();

		for id in &wanted {
			let Some(entry) = store.entry::<Terrain>(*id) else {
				continue;
			};
			let level = entry.value.scene_lod_level(lod_ref);
			let draw =
				crate::terrain::stream_lod::stream_banded_draws(entry.value.as_ref(), level);
			let water = draw.then(|| self.store.entry::<Water>(*id)).flatten();
			if let Some(shown) = self.state.presented.get_mut(id) {
				if shown.version == entry.version {
					if shown.level != level {
						self.commands.entity(shown.entity).insert(fill_visibility(draw));
						shown.level = level;
					}
					shown.sync_water(&mut self.commands, *id, water);
					continue;
				}
			}
			if let Some(previous) = self.state.presented.remove(id) {
				self.commands.entity(previous.entity).try_despawn();
			}
			let entity = entry.value.as_ref().spawn_fill(
				&mut self.commands,
				fill_visibility(draw),
				entry.value.seeds_collision(),
			);
			self.commands.entity(entity).insert((
				Name::new("Terrain cell"),
				PresentedTerrainScene(*id),
				TerrainVisualHost,
				M::default(),
			));
			let mut shown = PresentedEntry::new(entry.version, entity, level);
			shown.sync_water(&mut self.commands, *id, water);
			self.state.presented.insert(*id, shown);
		}

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

	pub fn clear_presented(&mut self) {
		self.state.clear(&mut self.commands);
	}
}

/// Keep fill pose on the cascade origin if something wipes [`Transform`].
pub fn sync_visual_terrain_host_pose(
	mut hosts: Query<(&CascadeChunk, &mut Transform), With<TerrainVisualHost>>,
) {
	for (chunk, mut transform) in &mut hosts {
		if transform.translation != chunk.origin {
			transform.translation = chunk.origin;
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn visual_host_restamps_identity_from_its_chunk() -> Result<()> {
		let mut app = App::new();
		app.add_systems(Update, sync_visual_terrain_host_pose);

		let origin = Vec3::new(160.0, -40.0, -320.0);
		let host = app
			.world_mut()
			.spawn((
				TerrainVisualHost,
				PresentedTerrainScene(Id::from_cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE))),
				CascadeChunk { origin, size: 160.0, ..CascadeChunk::unit_chunk() },
				Transform::IDENTITY,
			))
			.id();

		app.update();

		let transform = app
			.world()
			.get::<Transform>(host)
			.copied()
			.ok_or_else(|| anyhow::anyhow!("visual host lost Transform"))?;
		assert_eq!(transform.translation, origin);
		assert_ne!(transform, Transform::IDENTITY);
		Ok(())
	}
}
