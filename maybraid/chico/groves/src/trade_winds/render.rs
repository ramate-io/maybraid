//! [`RenderItem`] for populated Trade Winds groves ([#337](https://github.com/ramate-io/maybraid/issues/337)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sbs_geometry::{HonuBanyanSbs, SopesBanyanSbs, StorybookTreeSbs, WaialeaPalmSbs};
use chico_sbs_trees::honu_banyan::HonuBanyan;
use chico_sbs_trees::sopes_banyan::SopesBanyan;
use chico_sbs_trees::storybook_tree::StorybookTree;
use chico_sbs_trees::waialea_palm::WaialeaPalm;
use chico_sbs_trees::{
	SkippedInnerLeafMeshMaterial, SkippedOuterLeafMeshMaterial, SkippedStickMeshMaterial,
};
use chico_tree_components::{SkippedBodyMeshMaterial, SkippedFoliageMeshMaterial};
use chico_vegetation_shaders::{ChicoLeafMaterial, ChicoStickMaterial};
use clap::Args;
use procedural_common::{
	noise_params_from_scalar_str, BuildWithNoise, NoiseConfig, NoiseParams, UnitRange,
};
use render_item::{CascadeChunk, RenderItem};

use crate::grove::{
	patch_spawned_leaf_material, placement_noise, FlatTerrainSample, GroveExtent, GroveFrontend,
	GroveCellVariant, GroveWorldSample, WithPalette, DEFAULT_GROVE_EXTENT_XZ,
};
use crate::skipped_mesh_material::{
	SkippedLeafMeshMaterial, SkippedStickMeshMaterial as GroveSkippedStickMeshMaterial,
};
use crate::trade_winds::{
	definition, TradeWindsBanyan, TradeWindsCell, TradeWindsItem, TradeWindsStorybook,
	TradeWindsWaialeaPalm,
};

/// Honu template (material slots match playground [`RenderHonuBanyan`]).
pub type TwHonu = HonuBanyan<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	ChicoLeafMaterial,
	SkippedInnerLeafMeshMaterial<ChicoLeafMaterial>,
	ChicoLeafMaterial,
	SkippedOuterLeafMeshMaterial<ChicoLeafMaterial>,
	ChicoStickMaterial,
	SkippedBodyMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedFoliageMeshMaterial<StandardMaterial>,
>;

/// Sope template (material slots match playground [`RenderSopesBanyan`]).
pub type TwSope = SopesBanyan<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	ChicoLeafMaterial,
	SkippedLeafMeshMaterial<ChicoLeafMaterial>,
>;

/// Typical [`ChicoStickMaterial`] / [`StandardMaterial`] Trade Winds instance.
pub type TradeWindsStd = TradeWinds<
	ChicoStickMaterial,
	GroveSkippedStickMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
	FlatTerrainSample,
>;

/// Trade Winds grove preview (low-density tropical Storybook, banyan, and palm forms).
#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct TradeWinds<StickM, StickS, LeafM, LeafS, Terrain = FlatTerrainSample>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: GroveWorldSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	#[command(flatten, next_help_heading = "Grove")]
	pub grove: GroveFrontend,

	#[command(flatten, next_help_heading = "Stick Material")]
	pub stick_material: StickS,

	#[command(flatten, next_help_heading = "Leaf Material")]
	pub leaf_material: LeafS,

	#[arg(skip)]
	pub honu_template: TwHonu,

	#[arg(skip)]
	pub sope_template: TwSope,

	#[arg(
		long,
		default_value = "0,1.0,1.0,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
		help_heading = "The noise applied to the chains of sticks in trees and banyans",
	)]
	pub tree_chain_noise: NoiseParams,

	#[arg(
		long,
		default_value = "0,1.0,0.05,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
		help_heading = "Stick Surface Noise",
	)]
	pub stick_surface_noise: NoiseParams,

	#[arg(
		long,
		default_value = "0,1.0,0.06,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
		help_heading = "Leaf Surface Noise",
	)]
	pub leaf_surface_noise: NoiseParams,

	#[arg(skip)]
	pub extent: GroveExtent,

	#[command(flatten, next_help_heading = "Terrain")]
	pub terrain: Terrain,

	#[arg(skip)]
	resolved_placements: Option<Vec<GroveCellVariant<TradeWindsCell>>>,

	#[arg(skip)]
	__marker: PhantomData<fn() -> (StickM, LeafM)>,
}

impl<StickM, StickS, LeafM, LeafS, Terrain> Default
	for TradeWinds<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static + Default,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static + Default,
	Terrain: GroveWorldSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	fn default() -> Self {
		Self {
			grove: GroveFrontend::default(),
			stick_material: StickS::default(),
			leaf_material: LeafS::default(),
			honu_template: TwHonu::default(),
			sope_template: TwSope::default(),
			tree_chain_noise: NoiseParams::from_scalar(0.0, 1.0, 1.0, 1),
			stick_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
			leaf_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
			extent: GroveExtent::new(
				Vec3::ZERO,
				Vec3::new(DEFAULT_GROVE_EXTENT_XZ, 1.0, DEFAULT_GROVE_EXTENT_XZ),
			),
			terrain: Terrain::default(),
			resolved_placements: None,
			__marker: PhantomData,
		}
	}
}

impl<StickM, StickS, LeafM, LeafS, Terrain> TradeWinds<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: GroveWorldSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	pub fn with_resolved_placements(
		resolved_placements: Vec<GroveCellVariant<TradeWindsCell>>,
		terrain: Terrain,
		tree_chain_noise: NoiseParams,
		stick_surface_noise: NoiseParams,
		leaf_surface_noise: NoiseParams,
		stick_material: StickS,
		leaf_material: LeafS,
	) -> Self {
		Self {
			grove: GroveFrontend::default(),
			stick_material,
			leaf_material,
			honu_template: TwHonu::default(),
			sope_template: TwSope::default(),
			tree_chain_noise,
			stick_surface_noise,
			leaf_surface_noise,
			extent: GroveExtent::new(
				Vec3::ZERO,
				Vec3::new(DEFAULT_GROVE_EXTENT_XZ, 1.0, DEFAULT_GROVE_EXTENT_XZ),
			),
			terrain,
			resolved_placements: Some(resolved_placements),
			__marker: PhantomData,
		}
	}

	pub fn with_extent(mut self, extent: GroveExtent) -> Self {
		self.extent = extent;
		self
	}

	pub fn with_terrain(mut self, terrain: Terrain) -> Self {
		self.terrain = terrain;
		self
	}

	pub fn cell_extent_xz(&self) -> Vec2 {
		self.grove.definition(definition()).cell_extent_xz
	}

	pub fn placement_cells(&self) -> Vec<gimme_gen::Cell> {
		self.extent.subdivide_xz(self.cell_extent_xz())
	}

	pub fn placements(&self) -> Vec<GroveCellVariant<TradeWindsCell>> {
		if let Some(ref resolved) = self.resolved_placements {
			return resolved.clone();
		}
		self.grove.assemble(definition()).populate(&self.extent, &self.terrain)
	}
}

fn sample_f32(config: &NoiseConfig, range: UnitRange, salt: f32) -> f32 {
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
}

fn span_fraction(canopy_spread: f32, height: f32) -> f32 {
	(canopy_spread / height.max(0.5)).clamp(0.35, 1.20)
}

const TRADE_WINDS_RING_SPACING_SCALE: f32 = 1.25;
const TRADE_WINDS_ANCHORS_PER_RING: u32 = 5;

fn trade_winds_ring_spacing(base: f32) -> f32 {
	base * TRADE_WINDS_RING_SPACING_SCALE
}

impl BuildWithNoise<StorybookTreeSbs> for TradeWindsStorybook {
	fn build_with_noise(&self, noise: NoiseParams) -> StorybookTreeSbs {
		let config = NoiseConfig::new(noise);
		let height =
			sample_f32(&config, self.height, 1.0).max(self.height.start.min(self.height.end));
		let stalk_radius = sample_f32(&config, self.stalk_radius, 1.5);
		let canopy_spread = sample_f32(&config, self.canopy_spread, 2.0);
		let canopy_density = sample_f32(&config, self.canopy_density, 4.0);
		let span = span_fraction(canopy_spread, height);

		let mut geometry = StorybookTreeSbs::default();
		geometry.scale.tree_height = height;
		geometry.scale.stalk_base_radius = Some(stalk_radius);
		geometry.rings.spacing = trade_winds_ring_spacing(geometry.rings.spacing);
		geometry.rings.anchors_per_ring =
			TRADE_WINDS_ANCHORS_PER_RING + (canopy_density * 2.0).round() as u32;
		geometry.projection.span_fraction_of_height = UnitRange::new(span * 0.82, span * 1.05);
		geometry.rings.height_range = UnitRange::new(0.58, 1.0);
		geometry.canopy_noise = noise;
		geometry
	}
}

struct HonuBanyanSamples {
	geometry: HonuBanyanSbs,
	growth_spawn_fraction: f32,
}

impl BuildWithNoise<HonuBanyanSamples> for TradeWindsBanyan {
	fn build_with_noise(&self, noise: NoiseParams) -> HonuBanyanSamples {
		let config = NoiseConfig::new(noise);
		let height =
			sample_f32(&config, self.height, 1.0).max(self.height.start.min(self.height.end));
		let stalk_radius = sample_f32(&config, self.stalk_radius, 1.5);
		let canopy_spread = sample_f32(&config, self.canopy_spread, 2.0);
		let descender_threshold = sample_f32(&config, self.descender_density, 3.0);
		let canopy_density = sample_f32(&config, self.canopy_density, 4.0);
		let span = span_fraction(canopy_spread, height);

		let mut geometry = HonuBanyanSbs::default();
		geometry.scale.tree_height = height;
		geometry.scale.stalk_radius_fraction = (stalk_radius / height).clamp(0.04, 0.08);
		geometry.projection.length_fraction_of_height = UnitRange::new(span * 0.85, span);
		geometry.growth.descender_threshold = descender_threshold;
		geometry.canopy_noise = noise;

		HonuBanyanSamples { geometry, growth_spawn_fraction: canopy_density }
	}
}

struct SopeBanyanSamples {
	geometry: SopesBanyanSbs,
}

impl BuildWithNoise<SopeBanyanSamples> for TradeWindsBanyan {
	fn build_with_noise(&self, noise: NoiseParams) -> SopeBanyanSamples {
		let config = NoiseConfig::new(noise);
		let height =
			sample_f32(&config, self.height, 1.0).max(self.height.start.min(self.height.end));
		let stalk_radius = sample_f32(&config, self.stalk_radius, 1.5);
		let canopy_spread = sample_f32(&config, self.canopy_spread, 2.0);
		let descender_threshold = sample_f32(&config, self.descender_density, 3.0);
		let canopy_density = sample_f32(&config, self.canopy_density, 4.0);
		let span = span_fraction(canopy_spread, height);

		let mut geometry = SopesBanyanSbs::default();
		geometry.scale.stalk_height = height;
		geometry.scale.canopy_height = height * 2.0;
		geometry.scale.stalk_base_radius = stalk_radius;
		geometry.projection.length_fraction_of_height =
			UnitRange::new(span * 0.05, span * 0.18);
		geometry.growth.descender_threshold = descender_threshold;
		geometry.leaf_ball_factor = 0.25 + canopy_density * 0.35;
		geometry.canopy_noise = noise;

		SopeBanyanSamples { geometry }
	}
}

fn waialea_height_floor(palm: &TradeWindsWaialeaPalm) -> f32 {
	palm.height.start.min(palm.height.end)
}

impl BuildWithNoise<WaialeaPalmSbs> for TradeWindsWaialeaPalm {
	fn build_with_noise(&self, noise: NoiseParams) -> WaialeaPalmSbs {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(waialea_height_floor(self));
		let crown_density = sample_f32(&config, self.crown_density, 2.0);

		let mut geometry = WaialeaPalmSbs::default();
		geometry.scale.stalk_height = height;
		geometry.crown.ring_count = 2 + (crown_density * 2.0).round() as u32;
		geometry.crown.fronds_per_ring = 8 + (crown_density * 7.0).round() as u32;
		geometry.frond_world_scale = 0.55 + crown_density * 0.35;
		geometry.trunk_noise = noise;
		geometry
	}
}

fn placement_transform<V>(placed: &GroveCellVariant<V>) -> Transform {
	Transform {
		translation: placed.position,
		rotation: Quat::IDENTITY,
		scale: Vec3::splat(placed.scale.max(1e-4)),
	}
}

impl<StickM, StickS, LeafM, LeafS, Terrain> RenderItem
	for TradeWinds<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material + WithPalette + Default + Send + Sync + 'static,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static + Default,
	LeafM: Material + WithPalette + Default + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static + Default,
	Terrain: GroveWorldSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		let mut out = Vec::new();
		for placed in self.placements() {
			let local = transform.mul_transform(placement_transform(&placed));
			let foliage_noise = placement_noise(self.leaf_surface_noise, placed.position);
			let build_noise = placement_noise(self.grove.noise, placed.position);
			let chain_noise = placement_noise(self.tree_chain_noise, placed.position);
			let stick_seed = chain_noise.seed as i32;
			let canopy_seed = build_noise.seed as i32 + 31;

			let entities = match placed.variant.item() {
				TradeWindsItem::Storybook(story) => {
					let geometry = story.build_with_noise(build_noise);
					let mut tree = StorybookTree::<StickM, StickS, LeafM, LeafS>::default();
					tree.geometry = geometry;
					tree.stick_material = self.stick_material.clone();
					tree.leaf_material = self.leaf_material.clone();
					tree.stick_surface_noise =
						placement_noise(self.stick_surface_noise, placed.position);
					tree.leaf_surface_noise = foliage_noise;
					let entities = tree.spawn_render_items(commands, cascade_chunk, local);
					patch_spawned_leaf_material::<StickM>(
						&entities,
						placed.variant.stick_palette_mix(),
						stick_seed,
						commands,
					);
					patch_spawned_leaf_material::<LeafM>(
						&entities,
						placed.variant.canopy_palette_mix(),
						canopy_seed,
						commands,
					);
					entities
				}
				TradeWindsItem::Honu(banyan) => {
					let samples: HonuBanyanSamples = banyan.build_with_noise(build_noise);
					let mut tree = self.honu_template.clone();
					tree.geometry = samples.geometry;
					tree.construction.growth_spawn_fraction = samples.growth_spawn_fraction;
					tree.stick_surface_noise =
						placement_noise(self.stick_surface_noise, placed.position);
					tree.inner_leaf_surface_noise = foliage_noise;
					tree.outer_leaf_surface_noise = foliage_noise;
					tree.growth_body_noise = foliage_noise;
					tree.growth_foliage_noise = foliage_noise;
					let entities = tree.spawn_render_items(commands, cascade_chunk, local);
					patch_spawned_leaf_material::<ChicoStickMaterial>(
						&entities,
						placed.variant.stick_palette_mix(),
						stick_seed,
						commands,
					);
					patch_spawned_leaf_material::<ChicoLeafMaterial>(
						&entities,
						placed.variant.canopy_palette_mix(),
						canopy_seed,
						commands,
					);
					entities
				}
				TradeWindsItem::Sope(banyan) => {
					let samples: SopeBanyanSamples = banyan.build_with_noise(build_noise);
					let mut tree = self.sope_template.clone();
					tree.geometry = samples.geometry;
					tree.stick_surface_noise =
						placement_noise(self.stick_surface_noise, placed.position);
					tree.leaf_surface_noise = foliage_noise;
					let entities = tree.spawn_render_items(commands, cascade_chunk, local);
					patch_spawned_leaf_material::<ChicoStickMaterial>(
						&entities,
						placed.variant.stick_palette_mix(),
						stick_seed,
						commands,
					);
					patch_spawned_leaf_material::<ChicoLeafMaterial>(
						&entities,
						placed.variant.canopy_palette_mix(),
						canopy_seed,
						commands,
					);
					entities
				}
				TradeWindsItem::WaialeaPalm(palm) => {
					let geometry = palm.build_with_noise(build_noise);
					let mut tree = WaialeaPalm::<StickM, StickS, LeafM, LeafS>::default();
					tree.geometry = geometry;
					tree.stick_material = self.stick_material.clone();
					tree.leaf_material = self.leaf_material.clone();
					tree.stick_surface_noise =
						placement_noise(self.stick_surface_noise, placed.position);
					tree.foliage_noise = foliage_noise;
					let entities = tree.spawn_render_items(commands, cascade_chunk, local);
					patch_spawned_leaf_material::<StickM>(
						&entities,
						placed.variant.stick_palette_mix(),
						stick_seed,
						commands,
					);
					patch_spawned_leaf_material::<LeafM>(
						&entities,
						placed.variant.canopy_palette_mix(),
						canopy_seed,
						commands,
					);
					entities
				}
			};
			out.extend(entities);
		}
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn tree_geometry_builds_within_authored_ranges() -> Result<()> {
		let noise = placement_noise(NoiseParams::default(), Vec3::new(5.0, 0.0, 5.0));

		let TradeWindsItem::Storybook(story) = TradeWindsCell::TradeStorybook.item() else {
			anyhow::bail!("expected trade storybook item");
		};
		let story_geom = story.build_with_noise(noise);
		assert!(story_geom.scale.tree_height >= story.height.start.min(story.height.end));
		assert!(story_geom.scale.tree_height <= story.height.start.max(story.height.end));

		let TradeWindsItem::Honu(honu) = TradeWindsCell::TradeHonuBanyan.item() else {
			anyhow::bail!("expected trade honu item");
		};
		let honu_samples: HonuBanyanSamples = honu.build_with_noise(noise);
		assert!(honu_samples.geometry.scale.tree_height >= honu.height.start.min(honu.height.end));
		assert!(honu_samples.geometry.scale.tree_height <= honu.height.start.max(honu.height.end));

		let TradeWindsItem::Sope(sope) = TradeWindsCell::TradeSopesBanyan.item() else {
			anyhow::bail!("expected trade sope item");
		};
		let sope_samples: SopeBanyanSamples = sope.build_with_noise(noise);
		assert!(sope_samples.geometry.scale.stalk_height >= sope.height.start.min(sope.height.end));
		assert!(sope_samples.geometry.scale.stalk_height <= sope.height.start.max(sope.height.end));

		let TradeWindsItem::WaialeaPalm(palm) = TradeWindsCell::RareTradeWaialeaPalm.item() else {
			anyhow::bail!("expected waialea item");
		};
		let palm_geom = palm.build_with_noise(noise);
		assert!(palm_geom.scale.stalk_height >= palm.height.start.min(palm.height.end));
		assert!(palm_geom.scale.stalk_height <= palm.height.start.max(palm.height.end));
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			TradeWindsCell::TradeStorybook,
			TradeWindsCell::TradeSopesBanyan,
			TradeWindsCell::TradeHonuBanyan,
			TradeWindsCell::RareTallTradeStorybook,
			TradeWindsCell::RareTradeWaialeaPalm,
		] {
			for (palette, label) in
				[(cell.stick_palette_mix(), "stick"), (cell.canopy_palette_mix(), "canopy")]
			{
				let mut allowed = Vec::new();
				for slot in palette.slots {
					allowed.extend(slot.start.resolve());
					allowed.extend(slot.end.resolve());
				}
				assert!(!allowed.is_empty(), "unresolved {label} tokens for {cell:?}");
			}
		}
		Ok(())
	}

	#[test]
	fn with_resolved_placements_skips_live_selection() -> Result<()> {
		let placement = GroveCellVariant::new(
			TradeWindsCell::TradeStorybook,
			Vec3::new(1.0, 0.0, 2.0),
				1.0,
		);
		let item = TradeWindsStd::with_resolved_placements(
			vec![placement.clone()],
			FlatTerrainSample::default(),
			NoiseParams::default(),
			NoiseParams::default(),
			NoiseParams::default(),
			GroveSkippedStickMeshMaterial::<ChicoStickMaterial>::default(),
			SkippedLeafMeshMaterial::<StandardMaterial>::default(),
		);
		assert_eq!(item.placements(), vec![placement]);
		Ok(())
	}

	#[test]
	fn default_weights_yield_low_density_placements_in_preview_grid() -> Result<()> {
		let span = 260.0;
		let grove = TradeWindsStd::default()
			.with_terrain(FlatTerrainSample { elevation: 0.35, steepness: 0.15 })
			.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span)));
		let cells = grove.placement_cells().len();
		let placements = grove.placements();
		let placed_share = placements.len() as f32 / cells as f32;
		assert!(
			(0.08..=0.24).contains(&placed_share),
			"expected trade-winds fill, got {placed_share} ({}/{cells})",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn resolved_placements_cover_all_varietal_kinds() -> Result<()> {
		let placements = vec![
			GroveCellVariant::new(
				TradeWindsCell::TradeStorybook,
				Vec3::new(0.0, 0.0, 0.0),
				1.0,
			),
			GroveCellVariant::new(
				TradeWindsCell::TradeHonuBanyan,
				Vec3::new(4.0, 0.0, 0.0),
				1.0,
			),
			GroveCellVariant::new(
				TradeWindsCell::RareTradeWaialeaPalm,
				Vec3::new(8.0, 0.0, 0.0),
				1.0,
			),
		];
		let item = TradeWindsStd::with_resolved_placements(
			placements.clone(),
			FlatTerrainSample::default(),
			NoiseParams::default(),
			NoiseParams::default(),
			NoiseParams::default(),
			GroveSkippedStickMeshMaterial::<ChicoStickMaterial>::default(),
			SkippedLeafMeshMaterial::<StandardMaterial>::default(),
		);
		assert_eq!(item.placements().len(), 3);
		Ok(())
	}
}
