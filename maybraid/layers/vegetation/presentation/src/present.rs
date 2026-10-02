//! Generic grove and canopy bump-out presenters over a [`TerrainModel`].

use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;

use bevy::ecs::system::{ParamSet, SystemParam};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use chico_bumpout::{BumpOut, BumpOutNeighborhood, BumpOutStyle};
use chico_forests::{
	BumpOutLodChan, BumpOutPresentBullseye, CanopyBumpOut, ChicoGrove, ForestIndex,
	ForestLodChan, ForestPresentBullseye, ForestPresenterState, MediumBumpOutLodChan,
	MediumCanopyBumpOut, MEDIUM_BUMP_OUT_CELL_XZ,
};
use chico_groves::GroveWorldSample;
use durham_terrain_models::{cascade_chunk_for_cell, TerrainMeshBuilder, TERRAIN_CELL_SIZE};
use lod::gen::{Id, SpatialIndex, Version};
use lod::lod_ref::LodRef;
use lod::presentation::{LodPresentKeepRegion, LodPresentQueue, RegionPresenter};
use lod::hide_lod_tree;
use lod_cascade::Chunk;
use procedural_common::NoiseParams;
use render_item::mesh::IdentifiedMesh;
use render_item::NormalizeChunk;
use terrain_chunk_ref::{TerrainChunkKey, TerrainChunkRef};
use terrain_layer_model::{HeightField, ModeSubscription, TerrainCell, TerrainModel, TerrainView};
use vegetation_layer_model::VegetationLayerConfig;

use crate::VegetationPresent;

/// Urbanized medium bump-outs accept a padded cell only this close to
/// [`MEDIUM_BUMP_OUT_CELL_XZ`]. Fine bump-outs pass `None` and take any size.
const MEDIUM_OVERLAY_SIZE_TOLERANCE: f32 = 1e-2;

/// Grove sample over any [`HeightField`].
///
/// Height is the stored sample, or [`HeightField::fallback_height_at`] when the
/// covering cell is not generated. Steepness is a 1 m forward difference on
/// that height, the same difference `GroveTerrain::steepness_at` and
/// `ModulatedGroveSample::steepness_at` use.
#[derive(Clone)]
pub struct GroundGroveSample<S> {
	field: S,
}

impl<S: HeightField> GroundGroveSample<S> {
	pub fn new(field: S) -> Self {
		Self { field }
	}

	fn height(&self, xz: Vec2) -> f32 {
		self.field.height_at(xz).unwrap_or_else(|| self.field.fallback_height_at(xz))
	}
}

impl<S> GroveWorldSample for GroundGroveSample<S>
where
	S: HeightField + Clone + Send + Sync + 'static,
{
	fn height_at(&self, position: Vec3) -> f32 {
		self.height(Vec2::new(position.x, position.z))
	}

	fn steepness_at(&self, position: Vec3) -> f32 {
		const EPS: f32 = 1.0;
		let h = self.height_at(position);
		let hx = self.height_at(position + Vec3::new(EPS, 0.0, 0.0));
		let hz = self.height_at(position + Vec3::new(0.0, 0.0, EPS));
		let dx = (hx - h) / EPS;
		let dz = (hz - h) / EPS;
		(dx * dx + dz * dz).sqrt()
	}
}

/// Grove presenter over any ground model.
///
/// Prepare runs in a [`ParamSet`] ahead of [`TerrainView`] because urbanization's
/// prepare needs `&mut DevelopmentIndex` while the view reads that store.
#[derive(SystemParam)]
pub struct GroundForestPresenter<'w, 's, G>
where
	G: TerrainModel,
{
	commands: Commands<'w, 's>,
	state: ResMut<'w, ForestPresenterState>,
	access: ParamSet<'w, 's, (<G as TerrainModel>::Prepare, TerrainView<'w, 's, G>)>,
}

impl<G: TerrainModel> RegionPresenter<ChicoGrove, ForestIndex> for GroundForestPresenter<'_, '_, G>
where
	G::Snapshot: Clone + Send + Sync,
{
	fn presented_version(&self, id: Id) -> Option<Version> {
		self.state.presented_version(id)
	}

	fn handle(&mut self, id: Id, version: Version, grove: &ChicoGrove, lod_ref: &LodRef) {
		let bounds = grove.aabb();
		G::prepare(&mut self.access.p0(), bounds, lod_ref);
		let world = GroundGroveSample::new(self.access.p1().snapshot(bounds));
		self.state
			.present_with_world(&mut self.commands, id, version, grove, lod_ref, world);
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
		spatial_index: &ForestIndex,
		keep: &HashSet<Id>,
		despawn_budget: u32,
	) -> u32 {
		self.state.cull(&mut self.commands, spatial_index, keep, despawn_budget)
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
			hide_lod_tree(commands, entry.entity);
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

pub(crate) fn chunk_ref(
	cell: &dyn TerrainCell<Mesh = TerrainMeshBuilder>,
) -> TerrainChunkRef<TerrainMeshBuilder> {
	let cascade = cascade_chunk_for_cell(cell.bounds(), cell.res_2());
	let extent = cascade.extent.unwrap_or(Vec3::splat(cascade.size));
	let chunk = Chunk::from_min_max(cascade.origin, cascade.origin + extent, None);
	TerrainChunkRef::new(cell.mesh_builder(), chunk, cell.res_2())
}

/// Fine canopy overlays. Padded cells of any size win; otherwise the best raw
/// fine cell. [`presented_version`](RegionPresenter::presented_version) follows
/// the development presenters' terrain-key rule for every model.
#[derive(SystemParam)]
pub struct GroundCanopyBumpOutPresenter<'w, 's, G: TerrainModel> {
	commands: Commands<'w, 's>,
	state: ResMut<'w, CanopyBumpOutPresenterState>,
	ground: TerrainView<'w, 's, G>,
	forest: Res<'w, ForestIndex>,
}

impl<G> GroundCanopyBumpOutPresenter<'_, '_, G>
where
	G: TerrainModel<Cell: TerrainCell<Mesh = TerrainMeshBuilder>>,
{
	fn terrain_ref_for(&self, bounds: Aabb3d) -> Option<TerrainChunkRef<TerrainMeshBuilder>> {
		self.ground.overlay_cell(bounds, TERRAIN_CELL_SIZE, None).map(chunk_ref)
	}
}

impl<G> RegionPresenter<CanopyBumpOut, ForestIndex> for GroundCanopyBumpOutPresenter<'_, '_, G>
where
	G: TerrainModel<Cell: TerrainCell<Mesh = TerrainMeshBuilder>>,
{
	fn presented_version(&self, id: Id) -> Option<Version> {
		let cell = SpatialIndex::<CanopyBumpOut>::get(&*self.forest, id)?;
		let terrain_ref = self.terrain_ref_for(cell.bounds)?;
		self.state.presented_version_for_terrain(id, terrain_ref.key())
	}

	fn handle(&mut self, id: Id, version: Version, cell: &CanopyBumpOut, _lod_ref: &LodRef) {
		let Some(bump_out) = bump_out_from_cell(cell, bump_out_noise(&self.forest.noise)) else {
			return;
		};
		let Some(terrain_ref) = self.terrain_ref_for(cell.bounds) else {
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

/// Medium canopy overlays. A padded cell is used only when its width is within
/// [`MEDIUM_OVERLAY_SIZE_TOLERANCE`] of [`MEDIUM_BUMP_OUT_CELL_XZ`].
#[derive(SystemParam)]
pub struct GroundMediumCanopyBumpOutPresenter<'w, 's, G: TerrainModel> {
	commands: Commands<'w, 's>,
	state: ResMut<'w, MediumCanopyBumpOutPresenterState>,
	ground: TerrainView<'w, 's, G>,
	forest: Res<'w, ForestIndex>,
}

impl<G> GroundMediumCanopyBumpOutPresenter<'_, '_, G>
where
	G: TerrainModel<Cell: TerrainCell<Mesh = TerrainMeshBuilder>>,
{
	fn terrain_ref_for(&self, bounds: Aabb3d) -> Option<TerrainChunkRef<TerrainMeshBuilder>> {
		self.ground
			.overlay_cell(bounds, MEDIUM_BUMP_OUT_CELL_XZ, Some(MEDIUM_OVERLAY_SIZE_TOLERANCE))
			.map(chunk_ref)
	}
}

impl<G> RegionPresenter<MediumCanopyBumpOut, ForestIndex>
	for GroundMediumCanopyBumpOutPresenter<'_, '_, G>
where
	G: TerrainModel<Cell: TerrainCell<Mesh = TerrainMeshBuilder>>,
{
	fn presented_version(&self, id: Id) -> Option<Version> {
		let cell = SpatialIndex::<MediumCanopyBumpOut>::get(&*self.forest, id)?;
		let terrain_ref = self.terrain_ref_for(cell.0.bounds)?;
		self.state.presented_version_for_terrain(id, terrain_ref.key())
	}

	fn handle(&mut self, id: Id, version: Version, cell: &MediumCanopyBumpOut, _lod_ref: &LodRef) {
		let Some(bump_out) = bump_out_from_cell(&cell.0, bump_out_noise(&self.forest.noise)) else {
			return;
		};
		let Some(terrain_ref) = self.terrain_ref_for(cell.0.bounds) else {
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

/// Clear presenter hosts when the forest spec is absent, its key changes, or
/// the active mode is not subscribed.
///
/// The generation stream used to clear these inside `apply_spec`. Keeping the
/// clear here means the model crate does not despawn. It still runs after
/// [`VegetationGenerationSystems`](vegetation_layer_model::VegetationGenerationSystems)
/// and before present produce, under `terrain_streaming_enabled`.
///
/// While unsubscribed, present bullseyes, keep regions, and queues are also
/// dropped so `LodPresentSystems::Produce` cannot respawn the hosts.
#[allow(clippy::too_many_arguments)]
pub fn retire_vegetation_presenters<G: TerrainModel>(
	mut commands: Commands,
	config: Res<VegetationLayerConfig>,
	subscription: ModeSubscription<(G, VegetationPresent)>,
	mut forest: ResMut<ForestPresenterState>,
	mut bump_outs: ResMut<CanopyBumpOutPresenterState>,
	mut medium: ResMut<MediumCanopyBumpOutPresenterState>,
	forest_present: Option<ResMut<ForestPresentBullseye>>,
	bump_present: Option<ResMut<BumpOutPresentBullseye>>,
	forest_keep: Option<ResMut<LodPresentKeepRegion<ForestLodChan>>>,
	bump_keep: Option<ResMut<LodPresentKeepRegion<BumpOutLodChan>>>,
	medium_keep: Option<ResMut<LodPresentKeepRegion<MediumBumpOutLodChan>>>,
	forest_queue: Option<ResMut<LodPresentQueue<ChicoGrove>>>,
	bump_queue: Option<ResMut<LodPresentQueue<CanopyBumpOut>>>,
	medium_queue: Option<ResMut<LodPresentQueue<MediumCanopyBumpOut>>>,
	mut last_key: Local<Option<String>>,
) {
	if !subscription.active() {
		forest.clear(&mut commands);
		bump_outs.clear(&mut commands);
		medium.clear(&mut commands);
		last_key.take();
		if let Some(mut bullseye) = forest_present {
			bullseye.enabled = false;
		}
		if let Some(mut bullseye) = bump_present {
			bullseye.enabled = false;
		}
		if let Some(mut keep) = forest_keep {
			keep.region = None;
		}
		if let Some(mut keep) = bump_keep {
			keep.region = None;
		}
		if let Some(mut keep) = medium_keep {
			keep.region = None;
		}
		if let Some(mut queue) = forest_queue {
			queue.clear();
		}
		if let Some(mut queue) = bump_queue {
			queue.clear();
		}
		if let Some(mut queue) = medium_queue {
			queue.clear();
		}
		return;
	}
	let Some(spec) = config.forest.as_ref() else {
		forest.clear(&mut commands);
		bump_outs.clear(&mut commands);
		medium.clear(&mut commands);
		last_key.take();
		return;
	};
	let key = spec.key();
	if last_key.as_ref() != Some(&key) {
		forest.clear(&mut commands);
		bump_outs.clear(&mut commands);
		medium.clear(&mut commands);
		*last_key = Some(key);
	}
}
