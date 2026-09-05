//! Canopy bump-outs as a Lod generate / present layer.
//!
//! Generate stores [`CanopyBumpOut`] on [`ForestIndex`] (selection neighborhood, no grow).
//! Present asks [`TerrainMeshSource`] for the matching terrain mesh (160 m near /
//! 320 m far) and spawns [`BumpOut`] with that [`TerrainChunkRef<WorldTerrainBuilder>`]
//! identity so overlay copies the cached mesh handle.

use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;

use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use chico_bumpout::{BumpOut, BumpOutNeighborhood, BumpOutStyle};
use chico_forests::{
	BumpOutGenerateBullseye, BumpOutLodChan, BumpOutPresentBullseye, CanopyBumpOut, ForestExtent,
	ForestIndex, MediumBumpOutLodChan, MediumCanopyBumpOut, OnTerrain, BUMP_OUT_CELL_XZ,
	BUMP_OUT_OUTER_RADIUS_M, MEDIUM_BUMP_OUT_ANCHOR_STEP_M, MEDIUM_BUMP_OUT_CELL_XZ,
	MEDIUM_BUMP_OUT_OUTER_RADIUS_M,
};
use durham_terrain_models::{
	cascade_chunk_for_cell, Terrain, TerrainMeshBuilder, TerrainStoreView, TERRAIN_CELL_SIZE,
};
use lod::gen::{
	Id, LodGenerateKeepRegion, LodGenerateQueue, LodGenerateRegion, SpatialIndex, TrackedId,
	Version,
};
use lod::lod_ref::LodRef;
use lod::presentation::{LodPresentKeepRegion, LodPresentQueue, LodPresentRegion, RegionPresenter};
use lod::{
	LodGeneratePlugin, LodGenerateRegionPlugin, LodGenerateSystems, LodPresentCullPlugin,
	LodPresentPlugin, LodPresentRegionPlugin, LodPresentSystems, LodViewer,
};
use lod_cascade::Chunk;
use procedural_common::NoiseParams;
use render_item::mesh::IdentifiedMesh;
use render_item::NormalizeChunk;
use terrain_chunk_ref::{TerrainChunkKey, TerrainChunkRef};

use crate::ForestStreamSpec;
use crate::PlaygroundConfig;

pub type WorldTerrainBuilder = TerrainMeshBuilder;

/// Terrain mesh identity for a bump cell of `cell_size` (160 m near / 320 m far).
///
/// Skip this tick when the matching cell is not in the store yet — same as forest
/// `sample` returning `None`. Do not wrap terrain LOD plugins here.
pub trait TerrainMeshSource {
	fn mesh_for(
		&self,
		bounds: Aabb3d,
		cell_size: f32,
	) -> Option<TerrainChunkRef<WorldTerrainBuilder>>;
}

impl<H: SystemParam + 'static> TerrainMeshSource for OnTerrain<'_, '_, H>
where
	for<'a, 'b> H::Item<'a, 'b>: TerrainMeshSource,
{
	fn mesh_for(
		&self,
		bounds: Aabb3d,
		cell_size: f32,
	) -> Option<TerrainChunkRef<WorldTerrainBuilder>> {
		self.height.mesh_for(bounds, cell_size)
	}
}

/// Presenter bookkeeping for spawned bump-out entities.
#[derive(Resource)]
pub struct BumpOutPresenterState<M: Send + Sync + 'static> {
	presented: HashMap<Id, PresentedBumpOut>,
	_marker: PhantomData<fn() -> M>,
}

impl<M: Send + Sync + 'static> Default for BumpOutPresenterState<M> {
	fn default() -> Self {
		Self { presented: HashMap::new(), _marker: PhantomData }
	}
}

pub type CanopyBumpOutPresenterState = BumpOutPresenterState<CanopyBumpOut>;
pub type MediumCanopyBumpOutPresenterState = BumpOutPresenterState<MediumCanopyBumpOut>;

struct PresentedBumpOut {
	version: Version,
	entity: Entity,
	hidden: bool,
	terrain_key: TerrainChunkKey,
}

impl<M: Send + Sync + 'static> BumpOutPresenterState<M> {
	pub fn clear(&mut self, commands: &mut Commands) {
		for presented in self.presented.values() {
			commands.entity(presented.entity).despawn();
		}
		self.presented.clear();
	}

	pub fn presented_version(&self, id: Id) -> Option<Version> {
		self.presented.get(&id).map(|entry| entry.version)
	}

	pub fn presented_version_for_terrain(
		&self,
		id: Id,
		terrain_key: &TerrainChunkKey,
	) -> Option<Version> {
		self.presented
			.get(&id)
			.filter(|entry| &entry.terrain_key == terrain_key)
			.map(|entry| entry.version)
	}

	pub fn hide(&mut self, commands: &mut Commands, id: Id) {
		if let Some(entry) = self.presented.get_mut(&id) {
			entry.hidden = true;
			commands.entity(entry.entity).insert(Visibility::Hidden);
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
				commands.entity(entry.entity).despawn();
			}
		}
	}

	pub fn present<T>(
		&mut self,
		commands: &mut Commands,
		id: Id,
		version: Version,
		bump_out: BumpOut,
		terrain_ref: TerrainChunkRef<T>,
	) where
		T: IdentifiedMesh + NormalizeChunk + Send + Sync + 'static,
	{
		if let Some(previous) = self.presented.remove(&id) {
			commands.entity(previous.entity).despawn();
		}
		let terrain_key = terrain_ref.key().clone();
		let entity = bump_out.spawn(commands, terrain_ref);
		self.presented
			.insert(id, PresentedBumpOut { version, entity, hidden: false, terrain_key });
	}
}

trait BumpCell: Send + Sync + 'static {
	fn bump_cell(&self) -> &CanopyBumpOut;
	fn terrain_cell_size() -> f32;
}

impl BumpCell for CanopyBumpOut {
	fn bump_cell(&self) -> &CanopyBumpOut {
		self
	}

	fn terrain_cell_size() -> f32 {
		BUMP_OUT_CELL_XZ
	}
}

impl BumpCell for MediumCanopyBumpOut {
	fn bump_cell(&self) -> &CanopyBumpOut {
		&self.0
	}

	fn terrain_cell_size() -> f32 {
		MEDIUM_BUMP_OUT_CELL_XZ
	}
}

/// Spawn bump-outs by looking up a terrain mesh from composed source `S`.
#[derive(SystemParam)]
pub struct BumpOutPresenter<'w, 's, S: SystemParam + 'static, M: Send + Sync + 'static> {
	commands: Commands<'w, 's>,
	state: ResMut<'w, BumpOutPresenterState<M>>,
	source: StaticSystemParam<'w, 's, S>,
	forest: Res<'w, ForestIndex>,
}

pub type CanopyBumpOutPresenter<'w, 's, S> = BumpOutPresenter<'w, 's, S, CanopyBumpOut>;
pub type MediumCanopyBumpOutPresenter<'w, 's, S> = BumpOutPresenter<'w, 's, S, MediumCanopyBumpOut>;

impl<S, M> RegionPresenter<M, ForestIndex> for BumpOutPresenter<'_, '_, S, M>
where
	S: SystemParam + 'static,
	for<'a, 'b> S::Item<'a, 'b>: TerrainMeshSource,
	M: BumpCell,
	ForestIndex: SpatialIndex<M>,
{
	fn presented_version(&self, id: Id) -> Option<Version> {
		let cell = SpatialIndex::<M>::get(&*self.forest, id)?;
		let terrain_ref = self.source.mesh_for(cell.bump_cell().bounds, M::terrain_cell_size())?;
		self.state.presented_version_for_terrain(id, terrain_ref.key())
	}

	fn handle(&mut self, id: Id, version: Version, cell: &M, _lod_ref: &LodRef) {
		let bump_cell = cell.bump_cell();
		let Some(bump_out) = bump_out_from_cell(bump_cell, bump_out_noise(&self.forest.noise))
		else {
			return;
		};
		let Some(terrain_ref) = self.source.mesh_for(bump_cell.bounds, M::terrain_cell_size())
		else {
			return;
		};
		self.state.present(&mut self.commands, id, version, bump_out, terrain_ref);
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
}

/// Independent bump-out generate / present / cull on [`ForestIndex`].
pub fn register_bump_out_lod<Pr, MediumPr>(app: &mut App)
where
	Pr: SystemParam + 'static,
	for<'w, 's> Pr::Item<'w, 's>: RegionPresenter<CanopyBumpOut, ForestIndex>,
	MediumPr: SystemParam + 'static,
	for<'w, 's> MediumPr::Item<'w, 's>: RegionPresenter<MediumCanopyBumpOut, ForestIndex>,
{
	app.init_resource::<CanopyBumpOutPresenterState>()
		.init_resource::<MediumCanopyBumpOutPresenterState>()
		.add_plugins(LodGenerateRegionPlugin::<
			BumpOutGenerateBullseye,
			With<LodViewer>,
			BumpOutLodChan,
		>::default())
		.add_plugins(LodGeneratePlugin::<
			CanopyBumpOut,
			ForestIndex,
			BumpOutLodChan,
			With<LodViewer>,
		>::default())
		.add_plugins(LodPresentRegionPlugin::<
			BumpOutPresentBullseye,
			With<LodViewer>,
			BumpOutLodChan,
		>::default())
		.add_plugins(LodPresentPlugin::<
			CanopyBumpOut,
			ForestIndex,
			Pr,
			BumpOutLodChan,
			With<LodViewer>,
		>::default())
		.add_plugins(LodPresentCullPlugin::<
			CanopyBumpOut,
			ForestIndex,
			Pr,
			BumpOutLodChan,
		>::default())
		.add_plugins(LodGeneratePlugin::<
			MediumCanopyBumpOut,
			ForestIndex,
			MediumBumpOutLodChan,
			With<LodViewer>,
		>::default())
		.add_plugins(LodPresentPlugin::<
			MediumCanopyBumpOut,
			ForestIndex,
			MediumPr,
			MediumBumpOutLodChan,
			With<LodViewer>,
		>::default())
		.add_plugins(LodPresentCullPlugin::<
			MediumCanopyBumpOut,
			ForestIndex,
			MediumPr,
			MediumBumpOutLodChan,
		>::default())
		.configure_sets(Update, LodPresentSystems::Produce.after(LodGenerateSystems::Drain));
}

/// Keep / queue / bullseye resources the forest stream drives for bump-outs.
#[derive(SystemParam)]
pub struct BumpOutStreamLod<'w> {
	generate: ResMut<'w, BumpOutGenerateBullseye>,
	present: ResMut<'w, BumpOutPresentBullseye>,
	generate_queue: ResMut<'w, LodGenerateQueue<CanopyBumpOut>>,
	present_queue: ResMut<'w, LodPresentQueue<CanopyBumpOut>>,
	presenter: ResMut<'w, CanopyBumpOutPresenterState>,
	medium_generate_queue: ResMut<'w, LodGenerateQueue<MediumCanopyBumpOut>>,
	medium_present_queue: ResMut<'w, LodPresentQueue<MediumCanopyBumpOut>>,
	medium_presenter: ResMut<'w, MediumCanopyBumpOutPresenterState>,
	generate_regions: MessageWriter<'w, LodGenerateRegion<BumpOutLodChan>>,
	present_regions: MessageWriter<'w, LodPresentRegion<BumpOutLodChan>>,
	medium_generate_regions: MessageWriter<'w, LodGenerateRegion<MediumBumpOutLodChan>>,
	medium_present_regions: MessageWriter<'w, LodPresentRegion<MediumBumpOutLodChan>>,
	generate_keep: ResMut<'w, LodGenerateKeepRegion<BumpOutLodChan>>,
	keep: ResMut<'w, LodPresentKeepRegion<BumpOutLodChan>>,
	medium_generate_keep: ResMut<'w, LodGenerateKeepRegion<MediumBumpOutLodChan>>,
	medium_keep: ResMut<'w, LodPresentKeepRegion<MediumBumpOutLodChan>>,
}

impl BumpOutStreamLod<'_> {
	pub fn apply_spec(
		&mut self,
		commands: &mut Commands,
		spec: Option<&ForestStreamSpec>,
		camera: Option<Vec3>,
		last_key: &mut Option<String>,
		last_medium_region: &mut Option<Aabb3d>,
	) {
		let Some(spec) = spec else {
			self.generate.enabled = false;
			self.present.enabled = false;
			self.generate_keep.region = None;
			self.keep.region = None;
			self.generate_queue.clear();
			self.present_queue.clear();
			self.presenter.clear(commands);
			self.medium_generate_keep.region = None;
			self.medium_keep.region = None;
			self.medium_generate_queue.clear();
			self.medium_present_queue.clear();
			self.medium_presenter.clear(commands);
			last_key.take();
			last_medium_region.take();
			return;
		};

		let key = spec.key();
		let key_changed = last_key.as_ref() != Some(&key);
		if key_changed {
			self.generate_queue.clear();
			self.present_queue.clear();
			self.presenter.clear(commands);
			self.medium_generate_queue.clear();
			self.medium_present_queue.clear();
			self.medium_presenter.clear(commands);
			*last_key = Some(key);
		}

		self.generate.radius_m = BUMP_OUT_OUTER_RADIUS_M;
		self.generate.enabled = true;
		self.present.radius_m = BUMP_OUT_OUTER_RADIUS_M;
		self.present.enabled = true;

		let Some(cam) = camera else {
			return;
		};
		let aabb = ForestExtent::xz_radius_aabb(cam, BUMP_OUT_OUTER_RADIUS_M);
		self.generate_keep.region = Some(aabb);
		self.keep.region = Some(aabb);
		if key_changed {
			self.generate_regions.write(LodGenerateRegion::new(aabb));
			self.present_regions.write(LodPresentRegion::new(aabb));
		}

		let step = MEDIUM_BUMP_OUT_ANCHOR_STEP_M;
		let anchor = Vec3::new((cam.x / step).round() * step, 0.0, (cam.z / step).round() * step);
		let medium_region = ForestExtent::xz_radius_aabb(anchor, MEDIUM_BUMP_OUT_OUTER_RADIUS_M);
		let medium_region_changed = *last_medium_region != Some(medium_region);
		self.medium_generate_keep.region = Some(medium_region);
		self.medium_keep.region = Some(medium_region);
		if key_changed || medium_region_changed {
			self.medium_generate_regions.write(LodGenerateRegion::new(medium_region));
			self.medium_present_regions.write(LodPresentRegion::new(medium_region));
			*last_medium_region = Some(medium_region);
		}
	}
}

pub fn stream_canopy_bump_outs(
	mut commands: Commands,
	config: Res<PlaygroundConfig>,
	camera: Query<&Transform, With<Camera3d>>,
	mut lod: BumpOutStreamLod,
	mut last_key: Local<Option<String>>,
	mut last_medium_region: Local<Option<Aabb3d>>,
) {
	let cam = camera.single().ok().map(|t| t.translation);
	lod.apply_spec(
		&mut commands,
		config.forest.as_ref(),
		cam,
		&mut last_key,
		&mut last_medium_region,
	);
}

pub fn fine_terrain_for<'a>(view: &'a TerrainStoreView<'a>, bounds: Aabb3d) -> Option<&'a Terrain> {
	terrain_for_cell_size(view, bounds, TERRAIN_CELL_SIZE)
}

pub fn medium_terrain_for<'a>(
	view: &'a TerrainStoreView<'a>,
	bounds: Aabb3d,
) -> Option<&'a Terrain> {
	terrain_for_cell_size(view, bounds, MEDIUM_BUMP_OUT_CELL_XZ)
}

pub(crate) fn terrain_for_cell_size<'a>(
	view: &'a TerrainStoreView<'a>,
	bounds: Aabb3d,
	target_size: f32,
) -> Option<&'a Terrain> {
	let mut best: Option<(f32, &'a Terrain)> = None;
	for TrackedId(id) in view.tracked_ids_for(bounds) {
		let Some(terrain) = view.get(id) else {
			continue;
		};
		let Some(cell) = view.get_bounds(id) else {
			continue;
		};
		let size = (cell.max.x - cell.min.x).max(1e-3);
		if (size - target_size).abs() > target_size * 0.25 {
			continue;
		}
		let overlap = xz_overlap_area(bounds, cell);
		if overlap <= 1e-3 {
			continue;
		}
		if best.is_none_or(|(best_overlap, _)| overlap > best_overlap) {
			best = Some((overlap, terrain));
		}
	}
	best.map(|(_, terrain)| terrain)
}

pub fn bump_out_from_cell(cell: &CanopyBumpOut, noise: NoiseParams) -> Option<BumpOut> {
	let samples = cell.samples;
	let neighborhood = BumpOutNeighborhood::new(
		samples.map(|sample| sample.density),
		samples.map(|sample| sample.bite_size),
		samples.map(|sample| sample.bite_size_deviation),
		samples.map(|sample| sample.height_m),
		samples.map(|sample| sample.height_deviation_m),
	);
	if neighborhood.densities.iter().all(|density| *density <= 0.001) {
		return None;
	}
	Some(
		BumpOut::from_neighborhood(neighborhood, cell.center_palette(), noise).with_style(
			BumpOutStyle::new(0.065, 0.88, 0.18)
				.with_cheese(0.88, 1.0)
				.with_fragment_height(4.5, 0.85),
		),
	)
}

pub fn bump_out_noise(forest: &NoiseParams) -> NoiseParams {
	NoiseParams {
		seed: forest.seed.wrapping_add(307),
		frequency: 0.045,
		amplitude: forest.amplitude,
		octaves: 3,
		..*forest
	}
}

pub fn terrain_chunk_ref(terrain: &Terrain) -> TerrainChunkRef<WorldTerrainBuilder> {
	let cascade = cascade_chunk_for_cell(terrain.cell, terrain.res_2);
	let extent = cascade.extent.unwrap_or(Vec3::splat(cascade.size));
	let chunk = Chunk::from_min_max(cascade.origin, cascade.origin + extent, None);
	TerrainChunkRef::new(terrain.mesh_builder(), chunk, terrain.res_2)
}

fn xz_overlap_area(a: Aabb3d, b: Aabb3d) -> f32 {
	let x = (a.max.x.min(b.max.x) - a.min.x.max(b.min.x)).max(0.0);
	let z = (a.max.z.min(b.max.z) - a.min.z.max(b.min.z)).max(0.0);
	x * z
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn bump_out_cell_matches_terrain_fine_size() {
		assert!((chico_forests::BUMP_OUT_CELL_XZ - TERRAIN_CELL_SIZE).abs() < 1e-3);
	}
}
