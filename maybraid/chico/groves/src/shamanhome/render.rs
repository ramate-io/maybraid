//! [`RenderItem`] for populated Shamanhome groves ([#324](https://github.com/ramate-io/maybraid/issues/324)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sbs_geometry::{BraidOakTreeSbs, DatePalmSbs, SopesBanyanSbs};
use chico_sbs_trees::braid_oak_tree::BraidOakTree;
use chico_sbs_trees::date_palm::DatePalm;
use chico_sbs_trees::sopes_banyan::SopesBanyan;
use chico_sbs_trees::SkippedStickMeshMaterial;
use chico_vegetation_shaders::{ChicoLeafMaterial, ChicoStickMaterial};
use clap::Args;
use procedural_common::{
	noise_params_from_scalar_str, BuildWithNoise, NoiseConfig, NoiseParams, UnitRange,
};
use render_item::{CascadeChunk, RenderItem};

use crate::grove::{
	patch_spawned_leaf_material, placement_noise, FlatTerrainSample, GroveExtent, GroveFrontend,
	GrovePlacedCell, TerrainSample, WithPalette, DEFAULT_GROVE_EXTENT_XZ,
};
use crate::shamanhome::{
	definition, ShamanhomeBanyan, ShamanhomeBraidOak, ShamanhomeCell, ShamanhomeDatePalm,
	ShamanhomeItem,
};
use crate::skipped_mesh_material::{
	SkippedLeafMeshMaterial, SkippedStickMeshMaterial as GroveSkippedStickMeshMaterial,
};

/// Sope template for banyan placements (material slots match playground [`RenderSopesBanyan`]).
pub type ShamanhomeSope = SopesBanyan<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	ChicoLeafMaterial,
	SkippedLeafMeshMaterial<ChicoLeafMaterial>,
>;

/// Typical [`ChicoStickMaterial`] / [`StandardMaterial`] Shamanhome instance.
pub type ShamanhomeStd = Shamanhome<
	ChicoStickMaterial,
	GroveSkippedStickMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
	FlatTerrainSample,
>;

/// Shamanhome grove preview (Braid Oak, ritual Date Palm, and Sope Banyan accents).
#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct Shamanhome<StickM, StickS, LeafM, LeafS, Terrain = FlatTerrainSample>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: TerrainSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	#[command(flatten, next_help_heading = "Grove")]
	pub grove: GroveFrontend,

	#[command(flatten, next_help_heading = "Stick Material")]
	pub stick_material: StickS,

	#[command(flatten, next_help_heading = "Leaf Material")]
	pub leaf_material: LeafS,

	#[arg(skip)]
	pub sope_template: ShamanhomeSope,

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
	resolved_placements: Option<Vec<GrovePlacedCell<ShamanhomeCell>>>,

	#[arg(skip)]
	__marker: PhantomData<fn() -> (StickM, LeafM)>,
}

impl<StickM, StickS, LeafM, LeafS, Terrain> Default
	for Shamanhome<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static + Default,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static + Default,
	Terrain: TerrainSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	fn default() -> Self {
		Self {
			grove: GroveFrontend::default(),
			stick_material: StickS::default(),
			leaf_material: LeafS::default(),
			sope_template: ShamanhomeSope::default(),
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

impl<StickM, StickS, LeafM, LeafS, Terrain> Shamanhome<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: TerrainSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	pub fn with_resolved_placements(
		resolved_placements: Vec<GrovePlacedCell<ShamanhomeCell>>,
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
			sope_template: ShamanhomeSope::default(),
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

	pub fn placements(&self) -> Vec<GrovePlacedCell<ShamanhomeCell>> {
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

impl BuildWithNoise<BraidOakTreeSbs> for ShamanhomeBraidOak {
	fn build_with_noise(&self, noise: NoiseParams) -> BraidOakTreeSbs {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(3.5);
		let canopy_spread = sample_f32(&config, self.canopy_spread, 2.0);
		let span = span_fraction(canopy_spread, height);

		let mut geometry = BraidOakTreeSbs::default();
		geometry.apply_braid_preset();
		geometry.scale.tree_height = height;
		geometry.projection.span_fraction_of_height = UnitRange::new(span * 0.82, span * 1.02);
		geometry.canopy_noise = noise;
		geometry
	}
}

impl BuildWithNoise<DatePalmSbs> for ShamanhomeDatePalm {
	fn build_with_noise(&self, noise: NoiseParams) -> DatePalmSbs {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(3.5);
		let crown_density = sample_f32(&config, self.crown_density, 2.0);

		let mut geometry = DatePalmSbs::default();
		geometry.scale.stalk_height = height;
		geometry.crown.ring_count = 2 + (crown_density * 2.0).round() as u32;
		geometry.crown.fronds_per_ring = 5 + (crown_density * 5.0).round() as u32;
		geometry.frond_world_scale = 0.28 + crown_density * 0.24;
		geometry.crown_tuft_scale_factor = 0.04 + crown_density * 0.02;
		geometry.trunk_noise = noise;
		geometry
	}
}

struct SopeBanyanSamples {
	geometry: SopesBanyanSbs,
}

impl BuildWithNoise<SopeBanyanSamples> for ShamanhomeBanyan {
	fn build_with_noise(&self, noise: NoiseParams) -> SopeBanyanSamples {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(4.5);
		let stalk_radius = sample_f32(&config, self.stalk_radius, 1.5);
		let canopy_spread = sample_f32(&config, self.canopy_spread, 2.0);
		let descender_threshold = sample_f32(&config, self.descender_density, 3.0);
		let canopy_density = sample_f32(&config, self.canopy_density, 4.0);
		let span = span_fraction(canopy_spread, height);

		let mut geometry = SopesBanyanSbs::default();
		geometry.scale.stalk_height = height;
		geometry.scale.canopy_height = height * 2.0;
		geometry.scale.stalk_base_radius = stalk_radius;
		geometry.projection.length_fraction_of_height = UnitRange::new(span * 0.05, span * 0.18);
		geometry.growth.descender_threshold = descender_threshold;
		geometry.leaf_ball_factor = 0.25 + canopy_density * 0.35;
		geometry.canopy_noise = noise;

		SopeBanyanSamples { geometry }
	}
}

fn placement_transform<V>(placed: &GrovePlacedCell<V>) -> Transform {
	Transform {
		translation: placed.position,
		rotation: Quat::IDENTITY,
		scale: Vec3::splat(placed.scale.max(1e-4)),
	}
}

impl<StickM, StickS, LeafM, LeafS, Terrain> RenderItem
	for Shamanhome<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material + WithPalette + Default + Send + Sync + 'static,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static + Default,
	LeafM: Material + WithPalette + Default + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static + Default,
	Terrain: TerrainSample + Clone + Send + Sync + 'static + Default + clap::Args,
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
				ShamanhomeItem::BraidOak(oak) => {
					log::info!("building braid oak");
					let geometry = oak.build_with_noise(build_noise);
					let mut tree =
						BraidOakTree::<StickM, StickS, LeafM, LeafS, LeafM, LeafS>::default();
					tree.geometry = geometry;
					tree.stick_material = self.stick_material.clone();
					tree.inner_leaf_material = self.leaf_material.clone();
					tree.outer_leaf_material = self.leaf_material.clone();
					tree.stick_surface_noise =
						placement_noise(self.stick_surface_noise, placed.position);
					tree.inner_leaf_surface_noise = foliage_noise;
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
				ShamanhomeItem::DatePalm(palm) => {
					let geometry = palm.build_with_noise(build_noise);
					let mut tree = DatePalm::<StickM, StickS, LeafM, LeafS>::default();
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
				ShamanhomeItem::SopeBanyan(banyan) => {
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

		for cell in [
			ShamanhomeCell::ShamanBraidOak,
			ShamanhomeCell::RedRitualBraidOak,
			ShamanhomeCell::GnarledElderBraidOak,
			ShamanhomeCell::SilverShrineBraidOak,
			ShamanhomeCell::CopperBranchBraidOak,
		] {
			let ShamanhomeItem::BraidOak(oak) = cell.item() else {
				anyhow::bail!("expected braid oak item for {cell:?}");
			};
			let oak_geom = oak.build_with_noise(noise);
			assert!(oak_geom.scale.tree_height >= oak.height.start.min(oak.height.end));
			assert!(oak_geom.scale.tree_height <= oak.height.start.max(oak.height.end));
		}

		let ShamanhomeItem::DatePalm(palm) = ShamanhomeCell::RitualDatePalm.item() else {
			anyhow::bail!("expected date palm item");
		};
		let palm_geom = palm.build_with_noise(noise);
		assert!(palm_geom.scale.stalk_height >= palm.height.start.min(palm.height.end));
		assert!(palm_geom.scale.stalk_height <= palm.height.start.max(palm.height.end));

		let ShamanhomeItem::SopeBanyan(banyan) = ShamanhomeCell::SmallSopeBanyan.item() else {
			anyhow::bail!("expected sope banyan item");
		};
		let banyan_geom = banyan.build_with_noise(noise);
		assert!(
			banyan_geom.geometry.scale.stalk_height >= banyan.height.start.min(banyan.height.end)
		);
		assert!(
			banyan_geom.geometry.scale.stalk_height <= banyan.height.start.max(banyan.height.end)
		);
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			ShamanhomeCell::ShamanBraidOak,
			ShamanhomeCell::RedRitualBraidOak,
			ShamanhomeCell::GnarledElderBraidOak,
			ShamanhomeCell::SilverShrineBraidOak,
			ShamanhomeCell::CopperBranchBraidOak,
			ShamanhomeCell::RitualDatePalm,
			ShamanhomeCell::SmallSopeBanyan,
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
		let placement =
			GrovePlacedCell::new(ShamanhomeCell::ShamanBraidOak, Vec3::new(1.0, 0.0, 2.0), 1.0);
		let item = ShamanhomeStd::with_resolved_placements(
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
	fn default_weights_yield_moderate_placements_in_preview_grid() -> Result<()> {
		let span = DEFAULT_GROVE_EXTENT_XZ;
		let grove = ShamanhomeStd::default()
			.with_terrain(FlatTerrainSample { elevation: 0.35, steepness: 0.15 })
			.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span)));
		let cells = grove.placement_cells().len();
		let placements = grove.placements();
		let placed_share = placements.len() as f32 / cells as f32;
		assert!(
			(0.18..=0.48).contains(&placed_share),
			"expected moderate shamanhome fill, got {placed_share} ({}/{cells})",
			placements.len()
		);
		assert!(!placements.is_empty());
		Ok(())
	}
}
