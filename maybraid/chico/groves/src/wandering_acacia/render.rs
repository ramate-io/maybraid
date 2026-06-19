//! [`RenderItem`] for populated Wandering Acacia groves ([#338](https://github.com/ramate-io/maybraid/issues/338)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sbs_geometry::anchors::high_bush::{
	DEFAULT_ANCHOR_LIFT_FRACTION, DEFAULT_SEGMENT_LENGTH_FRACTION_HI,
	DEFAULT_SEGMENT_LENGTH_FRACTION_LO, DEFAULT_SEGMENT_RADIUS_FRACTION_HI,
	DEFAULT_SEGMENT_RADIUS_FRACTION_LO,
};
use chico_sbs_geometry::{KamakuraTorchSbs, PenmarchTorchSbs, SopesBanyanSbs, VaseTreeSbs};
use chico_sbs_trees::kamakura_torch::KamakuraTorch;
use chico_sbs_trees::penmarch_torch::PenmarchTorch;
use chico_sbs_trees::sopes_banyan::SopesBanyan;
use chico_sbs_trees::vase_tree::VaseTree;
use chico_sbs_trees::{SkippedLeafMeshMaterial, SkippedStickMeshMaterial};
use chico_tree_components::{HighBushFoliageStyle, HighBushShoots, HighBushShootsShape};
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
use crate::skipped_mesh_material::{
	SkippedLeafMeshMaterial as GroveSkippedLeafMeshMaterial,
	SkippedStickMeshMaterial as GroveSkippedStickMeshMaterial,
};
use crate::wandering_acacia::{
	definition, WanderingAcaciaBanyan, WanderingAcaciaCell, WanderingAcaciaHighBush,
	WanderingAcaciaItem, WanderingAcaciaTorch, WanderingAcaciaVaseTree,
};

/// Sope template (material slots match playground [`RenderSopesBanyan`]).
pub type WaSope = SopesBanyan<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	ChicoLeafMaterial,
	SkippedLeafMeshMaterial<ChicoLeafMaterial>,
>;

/// Typical [`ChicoStickMaterial`] / [`StandardMaterial`] Wandering Acacia instance.
pub type WanderingAcaciaStd = WanderingAcacia<
	ChicoStickMaterial,
	GroveSkippedStickMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	GroveSkippedLeafMeshMaterial<StandardMaterial>,
	FlatTerrainSample,
>;

/// Wandering Acacia grove preview (sparse High Bush, dry Sope's Banyan, and rare vase/torch accents).
#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct WanderingAcacia<StickM, StickS, LeafM, LeafS, Terrain = FlatTerrainSample>
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
	pub sope_template: WaSope,

	#[arg(
		long,
		default_value = "0,1.0,1.0,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
		help_heading = "The noise applied to the chains of sticks in bushes and banyans",
	)]
	pub bush_chain_noise: NoiseParams,

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
	resolved_placements: Option<Vec<GrovePlacedCell<WanderingAcaciaCell>>>,

	#[arg(skip)]
	__marker: PhantomData<fn() -> (StickM, LeafM)>,
}

impl<StickM, StickS, LeafM, LeafS, Terrain> Default
	for WanderingAcacia<StickM, StickS, LeafM, LeafS, Terrain>
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
			sope_template: WaSope::default(),
			bush_chain_noise: NoiseParams::from_scalar(0.0, 1.0, 1.0, 1),
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

impl<StickM, StickS, LeafM, LeafS, Terrain> WanderingAcacia<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: TerrainSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	pub fn with_resolved_placements(
		resolved_placements: Vec<GrovePlacedCell<WanderingAcaciaCell>>,
		terrain: Terrain,
		bush_chain_noise: NoiseParams,
		stick_surface_noise: NoiseParams,
		leaf_surface_noise: NoiseParams,
		stick_material: StickS,
		leaf_material: LeafS,
	) -> Self {
		Self {
			grove: GroveFrontend::default(),
			stick_material,
			leaf_material,
			sope_template: WaSope::default(),
			bush_chain_noise,
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

	pub fn placements(&self) -> Vec<GrovePlacedCell<WanderingAcaciaCell>> {
		if let Some(ref resolved) = self.resolved_placements {
			return resolved.clone();
		}
		self.grove.assemble(definition()).populate(&self.extent, &self.terrain)
	}
}

impl BuildWithNoise<HighBushShootsShape> for WanderingAcaciaHighBush {
	fn build_with_noise(&self, noise: NoiseParams) -> HighBushShootsShape {
		let config = NoiseConfig::new(noise);
		let sample_f32 = |range: UnitRange, salt| {
			let lo = range.start.min(range.end);
			let hi = range.start.max(range.end);
			config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
		};

		let sample_u32 = |range: &std::ops::RangeInclusive<u32>, salt| {
			let lo = *range.start() as usize;
			let hi = (*range.end() as usize).saturating_add(1);
			config.sample_range_usize_4d(lo, hi, 0.0, 0.0, 0.0, salt) as u32
		};

		let height = sample_f32(self.height, 1.0).max(self.height.start.min(self.height.end));
		let leaf_radius = sample_f32(self.leaf_radius, 2.0).max(0.01);

		HighBushShootsShape {
			height,
			anchor_lift_fraction: DEFAULT_ANCHOR_LIFT_FRACTION,
			shoot_count: sample_u32(&self.shoot_count, 3.0),
			radial_strength: sample_f32(self.radial_strength, 5.0),
			vertical_bias: sample_f32(self.vertical_bias, 6.0),
			branch_depth: sample_u32(&self.branch_depth, 4.0) as usize,
			segment_length_fraction_lo: DEFAULT_SEGMENT_LENGTH_FRACTION_LO,
			segment_length_fraction_hi: DEFAULT_SEGMENT_LENGTH_FRACTION_HI,
			segment_radius_fraction_lo: DEFAULT_SEGMENT_RADIUS_FRACTION_LO,
			segment_radius_fraction_hi: DEFAULT_SEGMENT_RADIUS_FRACTION_HI,
			leaf_radius_fraction: leaf_radius / height,
			foliage_style: HighBushFoliageStyle::PlaneSplay,
			chain_noise: noise,
		}
	}
}

fn span_fraction(canopy_spread: f32, height: f32) -> f32 {
	(canopy_spread / height.max(0.5)).clamp(0.35, 1.20)
}

struct SopeBanyanSamples {
	geometry: SopesBanyanSbs,
}

impl BuildWithNoise<SopeBanyanSamples> for WanderingAcaciaBanyan {
	fn build_with_noise(&self, noise: NoiseParams) -> SopeBanyanSamples {
		let config = NoiseConfig::new(noise);
		let sample_f32 = |range: UnitRange, salt: f32| {
			let lo = range.start.min(range.end);
			let hi = range.start.max(range.end);
			config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
		};

		let height =
			sample_f32(self.height, 1.0).max(self.height.start.min(self.height.end));
		let stalk_radius = sample_f32(self.stalk_radius, 1.5);
		let canopy_spread = sample_f32(self.canopy_spread, 2.0);
		let descender_threshold = sample_f32(self.descender_density, 3.0);
		let canopy_density = sample_f32(self.canopy_density, 4.0);
		let span = span_fraction(canopy_spread, height);

		let mut geometry = SopesBanyanSbs::default();
		geometry.scale.stalk_height = height;
		geometry.scale.canopy_height = height * 2.0;
		geometry.scale.stalk_base_radius = stalk_radius;
		geometry.projection.length_fraction_of_height =
			UnitRange::new(span * 0.05, span * 0.18);
		geometry.growth.descender_threshold = descender_threshold;
		geometry.leaf_ball_factor = 0.15 + canopy_density * 0.25;
		geometry.canopy_noise = noise;

		SopeBanyanSamples { geometry }
	}
}

impl BuildWithNoise<VaseTreeSbs> for WanderingAcaciaVaseTree {
	fn build_with_noise(&self, noise: NoiseParams) -> VaseTreeSbs {
		let config = NoiseConfig::new(noise);
		let sample_f32 = |range: UnitRange, salt: f32| {
			let lo = range.start.min(range.end);
			let hi = range.start.max(range.end);
			config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
		};

		let height =
			sample_f32(self.height, 1.0).max(self.height.start.min(self.height.end));
		let stalk_radius = sample_f32(self.stalk_radius, 1.5);
		let canopy_spread = sample_f32(self.canopy_spread, 2.0);
		let span = span_fraction(canopy_spread, height);

		let mut geometry = VaseTreeSbs::default();
		geometry.scale.tree_height = height;
		geometry.scale.stalk_base_radius = Some(stalk_radius);
		geometry.projection.span_fraction_of_height = UnitRange::new(span * 0.88, span * 1.08);
		geometry.canopy_noise = noise;
		geometry
	}
}

struct TorchSamples {
	height: f32,
	stalk_radius: f32,
	span: f32,
}

fn sample_torch(torch: &WanderingAcaciaTorch, noise: NoiseParams) -> TorchSamples {
	let config = NoiseConfig::new(noise);
	let sample_f32 = |range: UnitRange, salt: f32| {
		let lo = range.start.min(range.end);
		let hi = range.start.max(range.end);
		config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
	};

	let height =
		sample_f32(torch.height, 1.0).max(torch.height.start.min(torch.height.end));
	let stalk_radius = sample_f32(torch.stalk_radius, 1.5);
	let canopy_spread = sample_f32(torch.canopy_spread, 2.0);
	let span = span_fraction(canopy_spread, height);
	TorchSamples { height, stalk_radius, span }
}

impl BuildWithNoise<PenmarchTorchSbs> for WanderingAcaciaTorch {
	fn build_with_noise(&self, noise: NoiseParams) -> PenmarchTorchSbs {
		let s = sample_torch(self, noise);
		let mut geometry = PenmarchTorchSbs::default();
		geometry.scale.tree_height = s.height;
		geometry.scale.stalk_base_radius = Some(s.stalk_radius);
		geometry.projection.span_fraction_of_height = UnitRange::new(s.span * 0.88, s.span * 1.08);
		geometry.canopy_noise = noise;
		geometry
	}
}

impl BuildWithNoise<KamakuraTorchSbs> for WanderingAcaciaTorch {
	fn build_with_noise(&self, noise: NoiseParams) -> KamakuraTorchSbs {
		let s = sample_torch(self, noise);
		let mut geometry = KamakuraTorchSbs::default();
		geometry.scale.tree_height = s.height;
		geometry.scale.stalk_base_radius = Some(s.stalk_radius);
		geometry.projection.span_fraction_of_height = UnitRange::new(s.span * 0.88, s.span * 1.08);
		geometry.canopy_noise = noise;
		geometry
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
	for WanderingAcacia<StickM, StickS, LeafM, LeafS, Terrain>
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
			let chain_noise = placement_noise(self.bush_chain_noise, placed.position);
			let build_noise = placement_noise(self.grove.noise, placed.position);
			let foliage_noise = placement_noise(self.leaf_surface_noise, placed.position);
			let stick_seed = chain_noise.seed as i32;
			let canopy_seed = build_noise.seed as i32 + 31;

			let entities = match placed.variant.item() {
				WanderingAcaciaItem::HighBush(bush) => {
					let mut shape = bush.build_with_noise(build_noise);
					shape.chain_noise = chain_noise;
					let entities = HighBushShoots::<StickM, StickS, LeafM, LeafS>::spawn_from_shape(
						shape,
						self.stick_surface_noise,
						self.leaf_surface_noise,
						self.stick_material.clone(),
						self.leaf_material.clone(),
						commands,
						cascade_chunk,
						local,
					);
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
				WanderingAcaciaItem::Sope(banyan) => {
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
				WanderingAcaciaItem::VaseTree(vase) => {
					let geometry = vase.build_with_noise(build_noise);
					let mut tree =
						VaseTree::<StickM, StickS, LeafM, LeafS, LeafM, LeafS>::default();
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
				WanderingAcaciaItem::PenmarchTorch(torch) => {
					let geometry =
						BuildWithNoise::<PenmarchTorchSbs>::build_with_noise(torch, build_noise);
					let mut tree = PenmarchTorch::<StickM, StickS, LeafM, LeafS>::default();
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
				WanderingAcaciaItem::KamakuraTorch(torch) => {
					let geometry =
						BuildWithNoise::<KamakuraTorchSbs>::build_with_noise(torch, build_noise);
					let mut tree = KamakuraTorch::<StickM, StickS, LeafM, LeafS>::default();
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
	fn geometry_builds_within_authored_ranges() -> Result<()> {
		let noise = placement_noise(NoiseParams::default(), Vec3::new(5.0, 0.0, 5.0));

		let WanderingAcaciaItem::HighBush(bush) = WanderingAcaciaCell::WanderingHighBush.item()
		else {
			anyhow::bail!("expected wandering high bush item");
		};
		let shape = bush.build_with_noise(noise);
		assert!(shape.height >= bush.height.start.min(bush.height.end));
		assert!(shape.height <= bush.height.start.max(bush.height.end));
		assert!(bush.shoot_count.contains(&shape.shoot_count));
		assert_eq!(shape.foliage_style, HighBushFoliageStyle::PlaneSplay);

		let WanderingAcaciaItem::Sope(sope) =
			WanderingAcaciaCell::DryWanderingSopesBanyan.item()
		else {
			anyhow::bail!("expected dry wandering sope item");
		};
		let sope_samples: SopeBanyanSamples = sope.build_with_noise(noise);
		assert!(sope_samples.geometry.scale.stalk_height >= sope.height.start.min(sope.height.end));
		assert!(sope_samples.geometry.scale.stalk_height <= sope.height.start.max(sope.height.end));
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			WanderingAcaciaCell::WanderingHighBush,
			WanderingAcaciaCell::DryWanderingSopesBanyan,
			WanderingAcaciaCell::WanderingVaseTree,
			WanderingAcaciaCell::WanderingPenmarchTorch,
			WanderingAcaciaCell::WanderingKamakuraTorch,
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
		let placement = GrovePlacedCell::new(
			WanderingAcaciaCell::WanderingHighBush,
			Vec3::new(1.0, 0.0, 2.0),
			1.0,
		);
		let item = WanderingAcaciaStd::with_resolved_placements(
			vec![placement.clone()],
			FlatTerrainSample::default(),
			NoiseParams::default(),
			NoiseParams::default(),
			NoiseParams::default(),
			GroveSkippedStickMeshMaterial::<ChicoStickMaterial>::default(),
			GroveSkippedLeafMeshMaterial::<StandardMaterial>::default(),
		);
		assert_eq!(item.placements(), vec![placement]);
		Ok(())
	}

	#[test]
	fn default_weights_yield_very_low_density_placements_in_preview_grid() -> Result<()> {
		let span = 300.0;
		let grove = WanderingAcaciaStd::default()
			.with_terrain(FlatTerrainSample { elevation: 0.35, steepness: 0.15 })
			.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span)));
		let cells = grove.placement_cells().len();
		let placements = grove.placements();
		let placed_share = placements.len() as f32 / cells as f32;
		assert!(
			(0.03..=0.12).contains(&placed_share),
			"expected wandering-acacia fill, got {placed_share} ({}/{cells})",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn resolved_placements_cover_all_varietal_kinds() -> Result<()> {
		let placements = vec![
			GrovePlacedCell::new(
				WanderingAcaciaCell::WanderingHighBush,
				Vec3::new(0.0, 0.0, 0.0),
				1.0,
			),
			GrovePlacedCell::new(
				WanderingAcaciaCell::DryWanderingSopesBanyan,
				Vec3::new(4.0, 0.0, 0.0),
				1.0,
			),
			GrovePlacedCell::new(
				WanderingAcaciaCell::WanderingVaseTree,
				Vec3::new(8.0, 0.0, 0.0),
				1.0,
			),
			GrovePlacedCell::new(
				WanderingAcaciaCell::WanderingPenmarchTorch,
				Vec3::new(12.0, 0.0, 0.0),
				1.0,
			),
			GrovePlacedCell::new(
				WanderingAcaciaCell::WanderingKamakuraTorch,
				Vec3::new(16.0, 0.0, 0.0),
				1.0,
			),
		];
		let item = WanderingAcaciaStd::with_resolved_placements(
			placements.clone(),
			FlatTerrainSample::default(),
			NoiseParams::default(),
			NoiseParams::default(),
			NoiseParams::default(),
			GroveSkippedStickMeshMaterial::<ChicoStickMaterial>::default(),
			GroveSkippedLeafMeshMaterial::<StandardMaterial>::default(),
		);
		assert_eq!(item.placements().len(), 5);
		Ok(())
	}
}
