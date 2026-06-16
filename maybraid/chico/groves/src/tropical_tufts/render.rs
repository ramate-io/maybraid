//! [`RenderItem`] for populated Tropical Tufts groves ([#305](https://github.com/ramate-io/maybraid/issues/305)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::tuft::{BladeTuft, BladeTuftShape};
use chico_sbs_geometry::PalmBushSbs;
use chico_sbs_trees::palm_bush::PalmBush;
use clap::Args;
use procedural_common::{
	noise_params_from_scalar_str, BuildWithNoise, NoiseConfig, NoiseParams, UnitRange,
};
use render_item::{CascadeChunk, RenderItem};

use crate::grove::{
	patch_spawned_leaf_material, placement_noise, FlatTerrainSample, GroveExtent, GroveFrontend,
	GrovePlacedCell, TerrainSample, WithPalette, DEFAULT_GROVE_EXTENT_XZ,
};
use crate::skipped_mesh_material::SkippedLeafMeshMaterial;
use crate::tropical_tufts::{
	definition, TropicalPalmBush, TropicalTuftClump, TropicalTuftsCell, TropicalTuftsItem,
};

/// Typical [`StandardMaterial`] Tropical Tufts instance.
pub type TropicalTuftsStd =
	TropicalTufts<StandardMaterial, SkippedLeafMeshMaterial<StandardMaterial>, FlatTerrainSample>;

/// Tropical Tufts grove preview (leaf material → blade tufts and palm bush companions).
#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct TropicalTufts<LeafM, LeafS, Terrain = FlatTerrainSample>
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
	resolved_placements: Option<Vec<GrovePlacedCell<TropicalTuftsCell>>>,

	#[arg(skip)]
	__marker: PhantomData<fn() -> LeafM>,
}

impl<LeafM, LeafS, Terrain> Default for TropicalTufts<LeafM, LeafS, Terrain>
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

impl<LeafM, LeafS, Terrain> TropicalTufts<LeafM, LeafS, Terrain>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: TerrainSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	/// Render precomputed placements instead of selecting live from the grove frontend.
	pub fn with_resolved_placements(
		resolved_placements: Vec<GrovePlacedCell<TropicalTuftsCell>>,
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

	pub fn placements(&self) -> Vec<GrovePlacedCell<TropicalTuftsCell>> {
		if let Some(ref resolved) = self.resolved_placements {
			return resolved.clone();
		}
		self.grove.assemble(definition()).populate(&self.extent, &self.terrain)
	}
}

/// Sample a tuft clump's authored geometry ranges into a blade tuft shape.
///
/// Blade width is **length-proportional** (`length * width_factor`), so short and tall
/// varietals stay equally grass-thin.
impl BuildWithNoise<BladeTuftShape> for TropicalTuftClump {
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

impl BuildWithNoise<PalmBushSbs> for TropicalPalmBush {
	fn build_with_noise(&self, noise: NoiseParams) -> PalmBushSbs {
		// TODO: sample the authored `height` / `frond_count` / `frond_length` /
		// `crown_spread` ranges from `noise` instead of fixed companion values.
		PalmBushSbs::default()
			.with_height(2.4)
			.with_frond_world_scale(0.6)
			.with_noise_params(noise)
	}
}

fn placement_transform<V>(placed: &GrovePlacedCell<V>) -> Transform {
	Transform {
		translation: placed.position,
		rotation: Quat::IDENTITY,
		scale: Vec3::splat(placed.scale.max(1e-4)),
	}
}

impl<LeafM, LeafS, Terrain> RenderItem for TropicalTufts<LeafM, LeafS, Terrain>
where
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
			let noise = placement_noise(self.foliage_noise, placed.position);
			let entities = match placed.variant.item() {
				TropicalTuftsItem::Tuft(clump) => {
					let mut shape = clump.build_with_noise(noise);
					shape.noise_amplitude = self.foliage_noise.amplitude;
					shape.noise_frequency = self.foliage_noise.frequency;
					let tuft = BladeTuft::from_shape(shape, self.leaf_material.clone());
					tuft.spawn_render_items(commands, cascade_chunk, local)
				}
				TropicalTuftsItem::PalmBush(palm) => {
					let geometry = palm.build_with_noise(noise);
					let bush = PalmBush::new(geometry, self.leaf_material.clone());
					bush.spawn_render_items(commands, cascade_chunk, local)
				}
				TropicalTuftsItem::Patch(patch) => {
					let mut item =
						patch.build_tuft_patch(noise, self.leaf_material.clone());
					item.shape.noise_amplitude = self.foliage_noise.amplitude;
					item.shape.noise_frequency = self.foliage_noise.frequency;
					item.spawn_render_items(commands, cascade_chunk, local)
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
	use anyhow::Result;

	#[test]
	fn tuft_and_palm_geometry_build_from_noise() -> Result<()> {
		let noise = placement_noise(NoiseParams::default(), Vec3::new(5.0, 0.0, 5.0));
		let TropicalTuftsItem::Tuft(clump) = TropicalTuftsCell::BrightTuft.item() else {
			anyhow::bail!("expected tuft item");
		};
		assert!(clump.build_with_noise(noise).blade_length > 0.0);

		let TropicalTuftsItem::PalmBush(palm) = TropicalTuftsCell::SmallPalmBush.item() else {
			anyhow::bail!("expected palm bush item");
		};
		let geometry = palm.build_with_noise(noise);
		assert!(geometry.crown.fronds_per_ring >= 4);
		assert!(geometry.scale.height > 0.0);
		Ok(())
	}

	#[test]
	fn with_resolved_placements_skips_live_selection() -> Result<()> {
		let placement =
			GrovePlacedCell::new(TropicalTuftsCell::BrightTuft, Vec3::new(1.0, 0.0, 2.0), 1.0);
		let item = TropicalTuftsStd::with_resolved_placements(
			vec![placement.clone()],
			FlatTerrainSample::default(),
			NoiseParams::default(),
			SkippedLeafMeshMaterial::<StandardMaterial>::default(),
		);
		assert_eq!(item.placements(), vec![placement]);
		Ok(())
	}

	#[test]
	fn default_extent_includes_palm_placements() -> Result<()> {
		let span = DEFAULT_GROVE_EXTENT_XZ;
		let tufts = TropicalTuftsStd::default()
			.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span)));
		let placements = tufts.placements();
		let palms = placements
			.iter()
			.filter(|p| {
				matches!(
					p.variant,
					TropicalTuftsCell::SmallPalmBush | TropicalTuftsCell::JuvenilePalmBush
				)
			})
			.count();
		assert!(
			palms > 0,
			"expected palm buckets in default tropical-tufts grove, got {palms} palms among {} placements",
			placements.len()
		);
		Ok(())
	}
}
