//! [`RenderItem`] for populated Braid Grass groves ([#306](https://github.com/ramate-io/maybraid/issues/306)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::tuft::{BladeTuft, BladeTuftShape, SpearTuft, SpearTuftShape};
use clap::Args;
use procedural_common::{
	noise_params_from_scalar_str, BuildWithNoise, NoiseConfig, NoiseParams, UnitRange,
};
use render_item::{CascadeChunk, RenderItem};

use crate::braid_grass::{
	definition, BraidGrassCell, BraidGrassClump, BraidGrassItem, BraidSpearClump,
};
use crate::grove::{
	patch_spawned_leaf_material, placement_noise, FlatTerrainSample, GroveExtent, GroveFrontend,
	GrovePlacedCell, TerrainSample, WithPalette, DEFAULT_GROVE_EXTENT_XZ,
};
use crate::skipped_mesh_material::SkippedLeafMeshMaterial;

/// Typical [`StandardMaterial`] Braid Grass instance.
pub type BraidGrassStd =
	BraidGrass<StandardMaterial, SkippedLeafMeshMaterial<StandardMaterial>, FlatTerrainSample>;

/// Braid Grass grove preview (leaf material → blade tufts).
#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct BraidGrass<LeafM, LeafS, Terrain = FlatTerrainSample>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: TerrainSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	#[command(flatten, next_help_heading = "Grove")]
	pub grove: GroveFrontend,

	#[command(flatten, next_help_heading = "Leaf Material")]
	pub leaf_material: LeafS,

	#[arg(
		long,
		default_value = "0,1,0.06,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
		help_heading = "Foliage Surface Noise",
	)]
	pub foliage_noise: NoiseParams,

	#[arg(skip)]
	pub extent: GroveExtent,

	#[command(flatten, next_help_heading = "Terrain")]
	pub terrain: Terrain,

	#[arg(skip)]
	resolved_placements: Option<Vec<GrovePlacedCell<BraidGrassCell>>>,

	#[arg(skip)]
	__marker: PhantomData<fn() -> LeafM>,
}

impl<LeafM, LeafS, Terrain> Default for BraidGrass<LeafM, LeafS, Terrain>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static + Default,
	Terrain: TerrainSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	fn default() -> Self {
		Self {
			grove: GroveFrontend::default(),
			leaf_material: LeafS::default(),
			foliage_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
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

impl<LeafM, LeafS, Terrain> BraidGrass<LeafM, LeafS, Terrain>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: TerrainSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	/// Render precomputed placements instead of selecting live from the grove frontend.
	pub fn with_resolved_placements(
		resolved_placements: Vec<GrovePlacedCell<BraidGrassCell>>,
		terrain: Terrain,
		foliage_noise: NoiseParams,
		leaf_material: LeafS,
	) -> Self {
		Self {
			grove: GroveFrontend::default(),
			leaf_material,
			foliage_noise,
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

	/// Effective vegetation cell footprint (frontend override or authored).
	pub fn cell_extent_xz(&self) -> Vec2 {
		self.grove.definition(definition()).cell_extent_xz
	}

	pub fn placement_cells(&self) -> Vec<gimme_gen::Cell> {
		self.extent.subdivide_xz(self.cell_extent_xz())
	}

	pub fn placements(&self) -> Vec<GrovePlacedCell<BraidGrassCell>> {
		if let Some(ref resolved) = self.resolved_placements {
			return resolved.clone();
		}
		self.grove.assemble(definition()).populate(&self.extent, &self.terrain)
	}
}

/// Sample a clump's authored geometry ranges into a blade tuft shape.
///
/// Blade width is **length-proportional** (`length * width_factor`), so short and tall
/// varietals stay equally grass-thin.
impl BuildWithNoise<BladeTuftShape> for BraidGrassClump {
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

		let blade_length = sample_f32(self.height, 1.0).max(0.1);
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

/// Sample a spear clump's authored geometry ranges into a spear tuft shape.
///
/// The belly half-width is **length-proportional** (`length * belly_factor`); the base tapers
/// to roughly a third of the belly, keeping the authored belly→tip ribbon profile.
impl BuildWithNoise<SpearTuftShape> for BraidSpearClump {
	fn build_with_noise(&self, noise: NoiseParams) -> SpearTuftShape {
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

		let spear_length = sample_f32(self.height, 1.0).max(0.1);
		let belly_half_width = spear_length * sample_f32(self.belly_factor, 2.0);

		SpearTuftShape {
			spear_count: sample_u32(&self.spear_count, 3.0),
			spear_length,
			base_half_width: belly_half_width * 0.35,
			belly_half_width,
			max_tilt_radians: sample_f32(self.max_tilt_radians, 4.0).max(0.01),
			bend_segments: sample_u32(&self.bend_segments, 5.0).max(1),
			seed: noise.seed,
			..SpearTuftShape::default()
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

impl<LeafM, LeafS, Terrain> RenderItem for BraidGrass<LeafM, LeafS, Terrain>
where
	LeafM: Material + WithPalette + Default + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
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
			let noise = placement_noise(self.foliage_noise, placed.position);
			let entities = match placed.variant.item() {
				BraidGrassItem::Blade(clump) => {
					let mut shape = clump.build_with_noise(noise);
					shape.noise_amplitude = self.foliage_noise.amplitude;
					shape.noise_frequency = self.foliage_noise.frequency;
					let tuft = BladeTuft::from_shape(shape, self.leaf_material.clone());
					tuft.spawn_render_items(commands, cascade_chunk, local)
				}
				BraidGrassItem::Spear(clump) => {
					let mut shape = clump.build_with_noise(noise);
					shape.noise_amplitude = self.foliage_noise.amplitude;
					shape.noise_frequency = self.foliage_noise.frequency;
					let tuft = SpearTuft::from_shape(shape, self.leaf_material.clone());
					tuft.spawn_render_items(commands, cascade_chunk, local)
				}
			};
			patch_spawned_leaf_material::<LeafM>(
				&entities,
				placed.variant.palette_mix(),
				noise.seed,
				commands,
			);
			out.extend(entities);
		}
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::grove::parse_variant_weights;
	use anyhow::Result;

	#[test]
	fn build_with_noise_respects_authored_shape_ranges() -> Result<()> {
		let BraidGrassItem::Blade(clump) = BraidGrassCell::DeepGreenBlade.item() else {
			anyhow::bail!("expected blade item");
		};
		let shape = clump.build_with_noise(NoiseParams::from_scalar(42.0, 1.0, 1.0, 1));
		assert!(clump.blade_count.contains(&shape.blade_count));
		assert!(clump.bend_segments.contains(&shape.bend_segments));
		assert!(shape.max_tilt_radians >= clump.max_tilt_radians.start);
		assert!(shape.max_tilt_radians <= clump.max_tilt_radians.end);
		Ok(())
	}

	#[test]
	fn spear_build_respects_authored_shape_ranges() -> Result<()> {
		let BraidGrassItem::Spear(clump) = BraidGrassCell::FountainSpear.item() else {
			anyhow::bail!("expected spear item");
		};
		let shape = clump.build_with_noise(NoiseParams::from_scalar(42.0, 1.0, 1.0, 1));
		assert!(clump.spear_count.contains(&shape.spear_count));
		assert!(clump.bend_segments.contains(&shape.bend_segments));
		assert!(shape.max_tilt_radians >= clump.max_tilt_radians.start);
		assert!(shape.max_tilt_radians <= clump.max_tilt_radians.end);
		let factor = shape.belly_half_width / shape.spear_length;
		assert!(factor >= clump.belly_factor.start);
		assert!(factor <= clump.belly_factor.end);
		assert!(shape.base_half_width < shape.belly_half_width);
		Ok(())
	}

	#[test]
	fn palette_resolves_to_authored_color() -> Result<()> {
		let palette = BraidGrassCell::DeepGreenBlade.palette_mix();
		let mut allowed = Vec::new();
		for slot in palette.slots {
			allowed.extend(slot.start.resolve());
			allowed.extend(slot.end.resolve());
		}
		let material = StandardMaterial::with_palette(StandardMaterial::default(), palette, 7);
		assert!(allowed.contains(&material.base_color));
		Ok(())
	}

	#[test]
	fn with_resolved_placements_skips_live_selection() -> Result<()> {
		let placement =
			GrovePlacedCell::new(BraidGrassCell::DeepGreenBlade, Vec3::new(1.0, 0.0, 2.0), 1.0);
		let item = BraidGrassStd::with_resolved_placements(
			vec![placement.clone()],
			FlatTerrainSample::default(),
			NoiseParams::default(),
			SkippedLeafMeshMaterial::<StandardMaterial>::default(),
		);
		assert_eq!(item.placements(), vec![placement]);
		Ok(())
	}

	#[test]
	fn zero_none_weight_still_places_blades() -> Result<()> {
		let mut grass = BraidGrassStd {
			terrain: FlatTerrainSample { elevation: 0.4, steepness: 0.1 },
			..Default::default()
		};
		grass.grove.variant_weights =
			Some(parse_variant_weights("0.0,9.0,x,x,x,x,x").map_err(|e| anyhow::anyhow!("{e}"))?);
		let span = 3.0 * grass.cell_extent_xz();
		grass = grass.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(span.x, 1.0, span.y)));
		assert!(!grass.placements().is_empty());
		Ok(())
	}

	#[test]
	fn default_weights_yield_placements_in_preview_grid() -> Result<()> {
		let mut grass = BraidGrassStd::default();
		let span = 5.0 * grass.cell_extent_xz();
		grass = grass.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(span.x, 1.0, span.y)));
		assert!(!grass.placements().is_empty());
		Ok(())
	}
}
