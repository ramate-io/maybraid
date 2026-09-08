//! Near High ground-cover bump-outs as a Lod generate / present layer.
//!
//! Sibling of the canopy presenters: own keep disk, own channel, own spawned
//! set. Generate stores [`GroundCoverBumpOut`] on [`ForestIndex`]. Present GETs
//! the matching 160 m Durham cell (or skip) and spawns [`BumpOut`] with
//! centimetre recipes — never canopy-metre heights.

use std::collections::HashSet;

use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use chico_bumpout::{BumpOut, BumpOutNeighborhood, BumpOutStyle};
use chico_forests::{
	ForestExtent, ForestIndex, ForestStreamSpec, GroundCoverBumpOut, GroundCoverGenerateBullseye,
	GroundCoverKind, GroundCoverLodChan, GroundCoverPresentBullseye, GROUND_COVER_ANCHOR_STEP_M,
	GROUND_COVER_FAR_OVERLAP_M, GROUND_COVER_RADIUS_M,
};
use durham_terrain_models::{TerrainEntryStore, TerrainStoreView};
use lod::gen::{Id, LodGenerateKeepRegion, LodGenerateQueue, LodGenerateRegion, Version};
use lod::lod_ref::LodRef;
use lod::presentation::{LodPresentKeepRegion, LodPresentQueue, LodPresentRegion, RegionPresenter};
use lod::{
	LodGeneratePlugin, LodGenerateRegionPlugin, LodGenerateSystems, LodPresentCullPlugin,
	LodPresentPlugin, LodPresentRegionPlugin, LodPresentSystems, LodViewer,
};
use procedural_common::NoiseParams;

use crate::bump_out::{fine_terrain_for, terrain_chunk_ref, BumpOutPresenterState};
use crate::PlaygroundConfig;

pub type GroundCoverBumpOutPresenterState = BumpOutPresenterState<GroundCoverBumpOut>;

/// Durham-backed presenter for the Near High floor-color disk.
#[derive(SystemParam)]
pub struct DurhamGroundCoverBumpOutPresenter<'w, 's> {
	commands: Commands<'w, 's>,
	state: ResMut<'w, GroundCoverBumpOutPresenterState>,
	store: Res<'w, TerrainEntryStore>,
	layout: Res<'w, durham_terrain_models::TerrainCellLayout>,
	forest: Res<'w, ForestIndex>,
}

impl RegionPresenter<GroundCoverBumpOut, ForestIndex>
	for DurhamGroundCoverBumpOutPresenter<'_, '_>
{
	fn presented_version(&self, id: Id) -> Option<Version> {
		self.state.presented_version(id)
	}

	fn handle(&mut self, id: Id, version: Version, cell: &GroundCoverBumpOut, _lod_ref: &LodRef) {
		let Some(bump_out) = ground_cover_from_cell(cell, &self.forest.noise) else {
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

/// Independent ground-cover generate / present / cull on [`ForestIndex`].
pub fn register_ground_cover_lod<Pr>(app: &mut App)
where
	Pr: SystemParam + 'static,
	for<'w, 's> Pr::Item<'w, 's>: RegionPresenter<GroundCoverBumpOut, ForestIndex>,
{
	app.init_resource::<GroundCoverBumpOutPresenterState>()
		.add_plugins(LodGenerateRegionPlugin::<
			GroundCoverGenerateBullseye,
			With<LodViewer>,
			GroundCoverLodChan,
		>::default())
		.add_plugins(LodGeneratePlugin::<
			GroundCoverBumpOut,
			ForestIndex,
			GroundCoverLodChan,
			With<LodViewer>,
		>::default())
		.add_plugins(LodPresentRegionPlugin::<
			GroundCoverPresentBullseye,
			With<LodViewer>,
			GroundCoverLodChan,
		>::default())
		.add_plugins(LodPresentPlugin::<
			GroundCoverBumpOut,
			ForestIndex,
			Pr,
			GroundCoverLodChan,
			With<LodViewer>,
		>::default())
		.add_plugins(LodPresentCullPlugin::<
			GroundCoverBumpOut,
			ForestIndex,
			Pr,
			GroundCoverLodChan,
		>::default())
		.configure_sets(Update, LodPresentSystems::Produce.after(LodGenerateSystems::Drain));
}

/// Keep / queue / bullseye resources the forest stream drives for ground cover.
#[derive(SystemParam)]
pub struct GroundCoverStreamLod<'w> {
	generate: ResMut<'w, GroundCoverGenerateBullseye>,
	present: ResMut<'w, GroundCoverPresentBullseye>,
	generate_queue: ResMut<'w, LodGenerateQueue<GroundCoverBumpOut>>,
	present_queue: ResMut<'w, LodPresentQueue<GroundCoverBumpOut>>,
	presenter: ResMut<'w, GroundCoverBumpOutPresenterState>,
	generate_regions: MessageWriter<'w, LodGenerateRegion<GroundCoverLodChan>>,
	present_regions: MessageWriter<'w, LodPresentRegion<GroundCoverLodChan>>,
	generate_keep: ResMut<'w, LodGenerateKeepRegion<GroundCoverLodChan>>,
	keep: ResMut<'w, LodPresentKeepRegion<GroundCoverLodChan>>,
}

impl GroundCoverStreamLod<'_> {
	pub fn apply_spec(
		&mut self,
		commands: &mut Commands,
		spec: Option<&ForestStreamSpec>,
		camera: Option<Vec3>,
		last_key: &mut Option<String>,
		last_region: &mut Option<Aabb3d>,
	) {
		let Some(spec) = spec else {
			self.generate.enabled = false;
			self.present.enabled = false;
			self.generate_keep.region = None;
			self.keep.region = None;
			self.generate_queue.clear();
			self.present_queue.clear();
			self.presenter.clear(commands);
			last_key.take();
			last_region.take();
			return;
		};

		let key = format!("ground-cover:{}", spec.key());
		let key_changed = last_key.as_ref() != Some(&key);
		if key_changed {
			self.generate_queue.clear();
			self.present_queue.clear();
			self.presenter.clear(commands);
			*last_key = Some(key);
		}

		self.generate.radius_m = GROUND_COVER_RADIUS_M;
		self.generate.enabled = true;
		self.present.radius_m = GROUND_COVER_RADIUS_M;
		self.present.enabled = true;

		let Some(cam) = camera else {
			return;
		};
		let step = GROUND_COVER_ANCHOR_STEP_M;
		let anchor = Vec3::new((cam.x / step).round() * step, 0.0, (cam.z / step).round() * step);
		let keep_radius = GROUND_COVER_RADIUS_M + GROUND_COVER_FAR_OVERLAP_M;
		let region = ForestExtent::xz_radius_aabb(anchor, keep_radius);
		let region_changed = *last_region != Some(region);
		self.generate_keep.region = Some(region);
		self.keep.region = Some(region);
		if key_changed || region_changed {
			self.generate_regions.write(LodGenerateRegion::new(region));
			self.present_regions.write(LodPresentRegion::new(region));
			*last_region = Some(region);
		}
	}
}

pub fn stream_ground_cover_bump_outs(
	mut commands: Commands,
	config: Res<PlaygroundConfig>,
	camera: Query<&Transform, With<Camera3d>>,
	mut lod: GroundCoverStreamLod,
	mut last_key: Local<Option<String>>,
	mut last_region: Local<Option<Aabb3d>>,
) {
	let cam = camera.single().ok().map(|t| t.translation);
	lod.apply_spec(&mut commands, config.forest.as_ref(), cam, &mut last_key, &mut last_region);
}

pub fn ground_cover_from_cell(cell: &GroundCoverBumpOut, forest: &NoiseParams) -> Option<BumpOut> {
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
	let kind = cell.center_kind().unwrap_or(GroundCoverKind::MossyBumps);
	let style = kind.style_values();
	Some(
		BumpOut::from_neighborhood(
			neighborhood,
			cell.blended_palette(),
			ground_cover_noise(kind, forest),
		)
		.with_style(
			BumpOutStyle::new(style.coverage_softness, style.roughness, style.normal_soften)
				.with_cheese(style.cheese_amount, style.cheese_scale)
				.with_fragment_height(
					style.fragment_height_frequency,
					style.fragment_height_amplitude,
				)
				.with_boundary_rough(
					BumpOutStyle::GROUND_COVER_BOUNDARY_ROUGH_M,
					(cell.bounds.max.x - cell.bounds.min.x).max(1.0),
				),
		),
	)
}

fn ground_cover_noise(kind: GroundCoverKind, forest: &NoiseParams) -> NoiseParams {
	let style = kind.style_values();
	NoiseParams {
		seed: forest.seed.wrapping_add(811),
		frequency: style.noise_frequency,
		amplitude: forest.amplitude,
		octaves: style.noise_octaves,
		..*forest
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use chico_forests::GroundCoverSample;
	use durham_terrain_models::TERRAIN_CELL_SIZE;

	#[test]
	fn ground_cover_keep_matches_near_high_plus_far_overlap() {
		assert!((GROUND_COVER_RADIUS_M - 8.0 * TERRAIN_CELL_SIZE).abs() < 1e-3);
		assert!((GROUND_COVER_FAR_OVERLAP_M - 2.0 * TERRAIN_CELL_SIZE).abs() < 1e-3);
	}

	#[test]
	fn empty_neighborhood_does_not_build_an_overlay() {
		let cell = GroundCoverBumpOut {
			bounds: Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(160.0, 1.0, 160.0)),
			samples: [GroundCoverSample::empty(); 9],
		};
		assert!(ground_cover_from_cell(&cell, &NoiseParams::default()).is_none());
	}

	#[test]
	fn moss_recipe_stays_centimetre_scale() -> anyhow::Result<()> {
		let mut samples = [GroundCoverSample::empty(); 9];
		samples[4] = GroundCoverSample::from_kind(GroundCoverKind::MossyBumps, 0.5);
		let cell = GroundCoverBumpOut {
			bounds: Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(160.0, 1.0, 160.0)),
			samples,
		};
		let bump = ground_cover_from_cell(&cell, &NoiseParams::default())
			.ok_or_else(|| anyhow::anyhow!("moss overlay"))?;
		assert!(bump.max_vertical_displacement < 0.5);
		assert!(bump.min_vertical_displacement > -0.2);
		Ok(())
	}
}
