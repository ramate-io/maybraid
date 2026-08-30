//! Canopy bump-outs as a Lod generate / present layer.
//!
//! Generate stores [`CanopyBumpOut`] on [`ForestIndex`] (selection neighborhood, no grow).
//! Present looks up the matching Durham 160 m cell and spawns [`BumpOut`] with
//! the same [`TerrainChunkRef<WorldTerrainBuilder>`] identity Durham fill uses
//! (`Terrain::mesh_builder`), so overlay copies the cached mesh handle.

use std::collections::{HashMap, HashSet};

use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use chico_bumpout::{BumpOut, BumpOutNeighborhood, BumpOutStyle};
use chico_forests::{
	BumpOutGenerateBullseye, BumpOutLodChan, BumpOutPresentBullseye, CanopyBumpOut, ForestExtent,
	ForestIndex, BUMP_OUT_OUTER_RADIUS_M,
};
use durham_terrain_models::{
	cascade_chunk_for_cell, Terrain, TerrainCellLayout, TerrainEntryStore, TerrainMeshBuilder,
	TerrainStoreView, TERRAIN_CELL_SIZE,
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
use terrain_chunk_ref::TerrainChunkRef;

use crate::ForestStreamSpec;
use crate::PlaygroundConfig;

pub(crate) type WorldTerrainBuilder = TerrainMeshBuilder;

/// Presenter bookkeeping for spawned bump-out entities.
#[derive(Resource, Default)]
pub(crate) struct CanopyBumpOutPresenterState {
	presented: HashMap<Id, PresentedBumpOut>,
}

struct PresentedBumpOut {
	version: Version,
	entity: Entity,
	hidden: bool,
}

impl CanopyBumpOutPresenterState {
	fn clear(&mut self, commands: &mut Commands) {
		for presented in self.presented.values() {
			commands.entity(presented.entity).despawn();
		}
		self.presented.clear();
	}

	fn presented_version(&self, id: Id) -> Option<Version> {
		self.presented.get(&id).map(|entry| entry.version)
	}

	fn hide(&mut self, commands: &mut Commands, id: Id) {
		if let Some(entry) = self.presented.get_mut(&id) {
			entry.hidden = true;
			commands.entity(entry.entity).insert(Visibility::Hidden);
		}
	}

	fn is_hidden(&self, id: Id) -> bool {
		self.presented.get(&id).is_some_and(|entry| entry.hidden)
	}

	fn presented_ids(&self) -> Vec<Id> {
		self.presented.keys().copied().collect()
	}

	fn remove_stale(&mut self, commands: &mut Commands, wanted: &HashSet<Id>) {
		let stale: Vec<Id> =
			self.presented.keys().copied().filter(|id| !wanted.contains(id)).collect();
		for id in stale {
			if let Some(entry) = self.presented.remove(&id) {
				commands.entity(entry.entity).despawn();
			}
		}
	}

	fn present<T: Send + Sync + 'static>(
		&mut self,
		commands: &mut Commands,
		id: Id,
		version: Version,
		bump_out: BumpOut,
		terrain_ref: TerrainChunkRef<T>,
	) {
		if let Some(previous) = self.presented.remove(&id) {
			commands.entity(previous.entity).despawn();
		}
		let entity = bump_out.spawn(commands, terrain_ref);
		self.presented.insert(id, PresentedBumpOut { version, entity, hidden: false });
	}
}

/// Durham-backed presenter: generic spawn path over [`WorldTerrainBuilder`].
#[derive(SystemParam)]
pub struct DurhamCanopyBumpOutPresenter<'w, 's> {
	commands: Commands<'w, 's>,
	state: ResMut<'w, CanopyBumpOutPresenterState>,
	store: Res<'w, TerrainEntryStore>,
	layout: Res<'w, TerrainCellLayout>,
	forest: Res<'w, ForestIndex>,
}

impl RegionPresenter<CanopyBumpOut, ForestIndex> for DurhamCanopyBumpOutPresenter<'_, '_> {
	fn presented_version(&self, id: Id) -> Option<Version> {
		self.state.presented_version(id)
	}

	fn handle(&mut self, id: Id, version: Version, cell: &CanopyBumpOut, _lod_ref: &LodRef) {
		let Some(bump_out) = bump_out_from_cell(cell, bump_out_noise(&self.forest.noise)) else {
			return;
		};
		let view = TerrainStoreView::new(&self.store, &self.layout);
		let Some(terrain) = fine_terrain_for(&view, cell.bounds) else {
			return;
		};
		self.state
			.present(&mut self.commands, id, version, bump_out, terrain_chunk_ref(terrain));
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
pub fn register_bump_out_lod<Pr>(app: &mut App)
where
	Pr: SystemParam + 'static,
	for<'w, 's> Pr::Item<'w, 's>: RegionPresenter<CanopyBumpOut, ForestIndex>,
{
	app.init_resource::<CanopyBumpOutPresenterState>()
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
	generate_regions: MessageWriter<'w, LodGenerateRegion<BumpOutLodChan>>,
	present_regions: MessageWriter<'w, LodPresentRegion<BumpOutLodChan>>,
	generate_keep: ResMut<'w, LodGenerateKeepRegion<BumpOutLodChan>>,
	keep: ResMut<'w, LodPresentKeepRegion<BumpOutLodChan>>,
}

impl BumpOutStreamLod<'_> {
	pub fn apply_spec(
		&mut self,
		commands: &mut Commands,
		spec: Option<&ForestStreamSpec>,
		camera: Option<Vec3>,
		last_key: &mut Option<String>,
	) {
		let Some(spec) = spec else {
			self.generate.enabled = false;
			self.present.enabled = false;
			self.generate_keep.region = None;
			self.keep.region = None;
			self.generate_queue.pending.clear();
			self.present_queue.pending.clear();
			self.presenter.clear(commands);
			last_key.take();
			return;
		};

		let key = spec.key();
		let key_changed = last_key.as_ref() != Some(&key);
		if key_changed {
			self.generate_queue.pending.clear();
			self.present_queue.pending.clear();
			self.presenter.clear(commands);
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
	}
}

pub fn stream_canopy_bump_outs(
	mut commands: Commands,
	config: Res<PlaygroundConfig>,
	camera: Query<&Transform, With<Camera3d>>,
	mut lod: BumpOutStreamLod,
	mut last_key: Local<Option<String>>,
) {
	let cam = camera.single().ok().map(|t| t.translation);
	lod.apply_spec(&mut commands, config.forest.as_ref(), cam, &mut last_key);
}

fn fine_terrain_for<'a>(view: &'a TerrainStoreView<'a>, bounds: Aabb3d) -> Option<&'a Terrain> {
	let mut best: Option<(f32, &'a Terrain)> = None;
	for TrackedId(id) in view.tracked_ids_for(bounds) {
		let Some(terrain) = view.get(id) else {
			continue;
		};
		let Some(cell) = view.get_bounds(id) else {
			continue;
		};
		let size = (cell.max.x - cell.min.x).max(1e-3);
		if size > TERRAIN_CELL_SIZE * 1.5 {
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

fn bump_out_from_cell(cell: &CanopyBumpOut, noise: NoiseParams) -> Option<BumpOut> {
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

fn bump_out_noise(forest: &NoiseParams) -> NoiseParams {
	NoiseParams {
		seed: forest.seed.wrapping_add(307),
		frequency: 0.045,
		amplitude: forest.amplitude,
		octaves: 3,
		..*forest
	}
}

fn terrain_chunk_ref(terrain: &Terrain) -> TerrainChunkRef<WorldTerrainBuilder> {
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
