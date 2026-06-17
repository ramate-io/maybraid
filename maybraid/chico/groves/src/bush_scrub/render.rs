//! [`RenderItem`] for populated Bush Scrub groves ([#303](https://github.com/ramate-io/maybraid/issues/303)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::tuft::{BladeTuft, BladeTuftShape};
use chico_sbs_geometry::anchors::high_bush::{
	DEFAULT_ANCHOR_LIFT_FRACTION, DEFAULT_SEGMENT_LENGTH_FRACTION_HI,
	DEFAULT_SEGMENT_LENGTH_FRACTION_LO, DEFAULT_SEGMENT_RADIUS_FRACTION_HI,
	DEFAULT_SEGMENT_RADIUS_FRACTION_LO,
};
use chico_tree_components::{HighBushFoliageStyle, HighBushShoots, HighBushShootsShape};
use chico_vegetation_shaders::ChicoStickMaterial;
use clap::Args;
use procedural_common::{
	noise_params_from_scalar_str, BuildWithNoise, NoiseConfig, NoiseParams, UnitRange,
};
use render_item::{CascadeChunk, RenderItem};

use crate::bush_scrub::{definition, BushScrubBush, BushScrubCell, BushScrubItem, BushScrubTuft};
use crate::grove::{
	patch_spawned_leaf_material, placement_noise, FlatTerrainSample, GroveExtent, GroveFrontend,
	GrovePlacedCell, TerrainSample, WithPalette, DEFAULT_GROVE_EXTENT_XZ,
};
use crate::skipped_mesh_material::{SkippedLeafMeshMaterial, SkippedStickMeshMaterial};

/// Typical [`ChicoStickMaterial`] / [`StandardMaterial`] Bush Scrub instance.
pub type BushScrubStd = BushScrub<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
	FlatTerrainSample,
>;

/// Bush Scrub grove preview (stick + leaf materials → tufts and small Common High Bush shrubs).
#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct BushScrub<StickM, StickS, LeafM, LeafS, Terrain = FlatTerrainSample>
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

	#[arg(
		long,
		default_value = "0,1.0,1.0,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
		help_heading = "The noise applied to the chains of sticks in the bushes",
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
	resolved_placements: Option<Vec<GrovePlacedCell<BushScrubCell>>>,

	#[arg(skip)]
	__marker: PhantomData<fn() -> (StickM, LeafM)>,
}

impl<StickM, StickS, LeafM, LeafS, Terrain> Default
	for BushScrub<StickM, StickS, LeafM, LeafS, Terrain>
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

impl<StickM, StickS, LeafM, LeafS, Terrain> BushScrub<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: TerrainSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	pub fn with_resolved_placements(
		resolved_placements: Vec<GrovePlacedCell<BushScrubCell>>,
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

	pub fn placements(&self) -> Vec<GrovePlacedCell<BushScrubCell>> {
		if let Some(ref resolved) = self.resolved_placements {
			return resolved.clone();
		}
		self.grove.assemble(definition()).populate(&self.extent, &self.terrain)
	}
}

impl BuildWithNoise<BladeTuftShape> for BushScrubTuft {
	fn build_with_noise(&self, noise: NoiseParams) -> BladeTuftShape {
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

		let blade_length = sample_f32(self.height, 1.0).max(0.05);
		let blade_width = blade_length * sample_f32(self.width_factor, 2.0);

		BladeTuftShape {
			blade_count: sample_u32(&self.blade_count, 3.0),
			blade_length,
			blade_width,
			max_tilt_radians: sample_f32(self.max_tilt_radians, 4.0).max(0.01),
			bend_segments: sample_u32(&self.bend_segments, 5.0).max(1),
			seed: noise.seed,
			..BladeTuftShape::default()
		}
	}
}

/// Sample authored bush ranges into a [`HighBushShootsShape`] without applying the Common High
/// Bush playground preset.
impl BuildWithNoise<HighBushShootsShape> for BushScrubBush {
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

		let height = sample_f32(self.height, 1.0).max(0.25);
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

fn placement_transform<V>(placed: &GrovePlacedCell<V>) -> Transform {
	Transform {
		translation: placed.position,
		rotation: Quat::IDENTITY,
		scale: Vec3::splat(placed.scale.max(1e-4)),
	}
}

impl<StickM, StickS, LeafM, LeafS, Terrain> RenderItem
	for BushScrub<StickM, StickS, LeafM, LeafS, Terrain>
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
			let entities = match placed.variant.item() {
				BushScrubItem::Tuft(tuft) => {
					let mut shape = tuft.build_with_noise(foliage_noise);
					shape.noise_amplitude = self.leaf_surface_noise.amplitude;
					shape.noise_frequency = self.leaf_surface_noise.frequency;
					let tuft = BladeTuft::from_shape(shape, self.leaf_material.clone());
					let entities = tuft.spawn_render_items(commands, cascade_chunk, local);
					patch_spawned_leaf_material::<LeafM>(
						&entities,
						placed.variant.palette_mix(),
						foliage_noise.seed,
						commands,
					);
					entities
				}
				BushScrubItem::Patch(patch) => {
					let mut item = patch.build_tuft_patch(foliage_noise, self.leaf_material.clone());
					item.shape.noise_amplitude = self.leaf_surface_noise.amplitude;
					item.shape.noise_frequency = self.leaf_surface_noise.frequency;
					let entities = item.spawn_render_items(commands, cascade_chunk, local);
					patch_spawned_leaf_material::<LeafM>(
						&entities,
						placed.variant.palette_mix(),
						foliage_noise.seed,
						commands,
					);
					entities
				}
				BushScrubItem::Bush(bush) => {
					let chain_noise = placement_noise(self.bush_chain_noise, placed.position);
					let build_noise = placement_noise(self.grove.noise, placed.position);
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
					let stick_seed = chain_noise.seed as i32;
					let canopy_seed = build_noise.seed as i32 + 31;
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
	fn tuft_geometry_builds_within_authored_ranges() -> Result<()> {
		let noise = placement_noise(NoiseParams::default(), Vec3::new(5.0, 0.0, 5.0));
		for cell in [BushScrubCell::DryTuft, BushScrubCell::GreenTuft] {
			let BushScrubItem::Tuft(tuft) = cell.item() else {
				anyhow::bail!("expected tuft item for {cell:?}");
			};
			let shape = tuft.build_with_noise(noise);
			assert!(shape.blade_length >= tuft.height.start.min(tuft.height.end));
			assert!(shape.blade_length <= tuft.height.start.max(tuft.height.end));
			let factor = shape.blade_width / shape.blade_length;
			assert!(factor >= tuft.width_factor.start.min(tuft.width_factor.end));
			assert!(factor <= tuft.width_factor.start.max(tuft.width_factor.end));
			assert!(tuft.blade_count.contains(&shape.blade_count));
			assert!(tuft.bend_segments.contains(&shape.bend_segments));
			assert!(shape.max_tilt_radians >= tuft.max_tilt_radians.start);
			assert!(shape.max_tilt_radians <= tuft.max_tilt_radians.end);
		}
		Ok(())
	}

	#[test]
	fn bush_geometry_builds_within_authored_ranges() -> Result<()> {
		let noise = placement_noise(NoiseParams::default(), Vec3::new(5.0, 0.0, 5.0));
		for cell in [BushScrubCell::SmallBush, BushScrubCell::SaplingBush] {
			let BushScrubItem::Bush(bush) = cell.item() else {
				anyhow::bail!("expected bush item for {cell:?}");
			};
			let shape = bush.build_with_noise(noise);
			assert!(shape.height >= bush.height.start.min(bush.height.end));
			assert!(shape.height <= bush.height.start.max(bush.height.end));
			assert!(bush.shoot_count.contains(&shape.shoot_count));
			assert!(bush.branch_depth.contains(&(shape.branch_depth as u32)));
			assert!(shape.radial_strength >= bush.radial_strength.start.min(bush.radial_strength.end));
			assert!(shape.radial_strength <= bush.radial_strength.start.max(bush.radial_strength.end));
			assert!(shape.vertical_bias >= bush.vertical_bias.start.min(bush.vertical_bias.end));
			assert!(shape.vertical_bias <= bush.vertical_bias.start.max(bush.vertical_bias.end));
			let leaf_radius = shape.leaf_radius_world();
			assert!(leaf_radius >= bush.leaf_radius.start.min(bush.leaf_radius.end));
			assert!(leaf_radius <= bush.leaf_radius.start.max(bush.leaf_radius.end));
			assert_eq!(shape.foliage_style, HighBushFoliageStyle::PlaneSplay);
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			BushScrubCell::DryTuft,
			BushScrubCell::GreenTuft,
			BushScrubCell::SmallBush,
			BushScrubCell::SaplingBush,
			BushScrubCell::DryTuftPatch,
			BushScrubCell::GreenTuftPatch,
		] {
			match cell.item() {
				BushScrubItem::Tuft(_) | BushScrubItem::Patch(_) => {
					let palette = cell.palette_mix();
					let mut allowed = Vec::new();
					for slot in palette.slots {
						allowed.extend(slot.start.resolve());
						allowed.extend(slot.end.resolve());
					}
					assert!(!allowed.is_empty(), "unresolved palette tokens for {cell:?}");
					let material =
						StandardMaterial::with_palette(StandardMaterial::default(), palette, 7);
					assert!(allowed.contains(&material.base_color));
				}
				BushScrubItem::Bush(_) => {
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
			}
		}
		Ok(())
	}

	#[test]
	fn patch_build_samples_layout_within_authored_ranges() -> Result<()> {
		let noise = placement_noise(NoiseParams::default(), Vec3::new(5.0, 0.0, 5.0));
		let BushScrubItem::Patch(patch) = BushScrubCell::DryTuftPatch.item() else {
			anyhow::bail!("expected patch item");
		};
		let item = patch.build_tuft_patch::<StandardMaterial, _>(
			noise,
			SkippedLeafMeshMaterial::<StandardMaterial>::default(),
		);
		assert!(patch.clump_count.contains(&item.clump_count));
		assert!(item.patch_extent_xz >= patch.patch_extent_xz.start);
		assert!(item.patch_extent_xz <= patch.patch_extent_xz.end);
		assert_eq!(item.clump_anchors().len(), item.clump_count as usize);
		assert!(item.shape.base_spread >= patch.base_spread.start);
		assert!(item.shape.base_spread <= patch.base_spread.end);
		Ok(())
	}

	#[test]
	fn with_resolved_placements_skips_live_selection() -> Result<()> {
		let placement =
			GrovePlacedCell::new(BushScrubCell::DryTuft, Vec3::new(1.0, 0.0, 2.0), 1.0);
		let item = BushScrubStd::with_resolved_placements(
			vec![placement.clone()],
			FlatTerrainSample::default(),
			NoiseParams::default(),
			NoiseParams::default(),
			NoiseParams::default(),
			SkippedStickMeshMaterial::<ChicoStickMaterial>::default(),
			SkippedLeafMeshMaterial::<StandardMaterial>::default(),
		);
		assert_eq!(item.placements(), vec![placement]);
		Ok(())
	}

	#[test]
	fn default_weights_yield_sparse_placements_in_preview_grid() -> Result<()> {
		let span = DEFAULT_GROVE_EXTENT_XZ;
		let scrub = BushScrubStd::default()
			.with_terrain(FlatTerrainSample { elevation: 0.40, steepness: 0.15 })
			.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span)));
		let cells = scrub.placement_cells().len();
		let placements = scrub.placements();
		let placed_share = placements.len() as f32 / cells as f32;
		assert!(
			(0.10..=0.30).contains(&placed_share),
			"expected sparse bush-scrub fill, got {placed_share} ({}/{cells})",
			placements.len()
		);
		assert!(!placements.is_empty());
		Ok(())
	}
}
