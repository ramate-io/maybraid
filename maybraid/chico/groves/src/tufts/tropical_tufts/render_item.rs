//! [`RenderItem`] for populated Tropical Tufts groves ([#305](https://github.com/ramate-io/maybraid/issues/305)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::tuft::BladeTuft;
use chico_sbs_trees::palm_bush::PalmBush;
use clap::Args;
use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};
use render_item::{CascadeChunk, RenderItem};

use super::{TropicalTuftsCell, TropicalTuftsDefinition, TropicalTuftsGroveFrontend};
use crate::grove::{
	patch_spawned_leaf_material, placement_noise, CellGrove, FlatTerrainSample, GroveExtent,
	GrovePlacedCell, TerrainSample, WithPalette, DEFAULT_GROVE_EXTENT_XZ,
};
use crate::skipped_mesh_material::SkippedLeafMeshMaterial;

/// Typical [`StandardMaterial`] Tropical Tufts instance.
pub type TropicalTuftsStd =
	TropicalTufts<StandardMaterial, SkippedLeafMeshMaterial<StandardMaterial>, FlatTerrainSample>;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct TropicalTufts<LeafM, LeafS, Terrain = FlatTerrainSample>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: TerrainSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	#[command(flatten, next_help_heading = "Grove")]
	pub grove: TropicalTuftsGroveFrontend,

	#[command(flatten, next_help_heading = "Leaf Material")]
	pub leaf_material: LeafS,

	#[arg(
		long,
		default_value = "0,1,0.06,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES",
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
			grove: TropicalTuftsGroveFrontend::default(),
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
	pub fn with_resolved_placements(
		resolved_placements: Vec<GrovePlacedCell<TropicalTuftsCell>>,
		terrain: Terrain,
		foliage_noise: NoiseParams,
		leaf_material: LeafS,
	) -> Self {
		Self {
			grove: TropicalTuftsGroveFrontend::default(),
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

	pub fn placements(&self) -> Vec<GrovePlacedCell<TropicalTuftsCell>> {
		if let Some(ref resolved) = self.resolved_placements {
			return resolved.clone();
		}
		let cells = self.placement_cells();
		self.assemble_grove().select_placements(&self.extent, &cells, &self.terrain)
	}

	pub fn placement_cells(&self) -> Vec<gimme_gen::Cell> {
		let cell_extent_xz = self
			.definition()
			.map(|definition| definition.cell_extent_xz())
			.unwrap_or_else(|err| {
				log::warn!("tropical tufts definition: {err}; using authored cell extent");
				TropicalTuftsDefinition::cell_extent_xz_default()
			});
		self.extent.subdivide_xz(cell_extent_xz)
	}

	pub fn definition(&self) -> Result<TropicalTuftsDefinition, String> {
		let mut definition =
			TropicalTuftsDefinition::new().with_cell_extent_xz(self.grove.cell_extent_xz);
		if let Some(ref overrides) = self.grove.variant_weights {
			definition = definition.with_variant_weights(overrides)?;
		}
		Ok(definition)
	}

	pub fn assemble_grove(&self) -> crate::grove::Grove<TropicalTuftsDefinition> {
		let definition = self.definition().unwrap_or_else(|err| {
			log::warn!("tropical tufts definition: {err}; using authored weights");
			TropicalTuftsDefinition::new()
		});
		self.grove.clone().assemble(definition)
	}
}

fn placement_transform(placed: &GrovePlacedCell<TropicalTuftsCell>) -> Transform {
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
			match &placed.variant {
				TropicalTuftsCell::None(_) => {}
				TropicalTuftsCell::BrightTuft(bucket)
				| TropicalTuftsCell::DeepTuft(bucket)
				| TropicalTuftsCell::YellowGreenTuft(bucket) => {
					let mut shape = bucket.item.build_with_noise(noise);
					shape.noise_amplitude = self.foliage_noise.amplitude;
					shape.noise_frequency = self.foliage_noise.frequency;
					let tuft = BladeTuft::from_shape(shape, self.leaf_material.clone());
					let entities = tuft.spawn_render_items(commands, cascade_chunk, local);
					patch_spawned_leaf_material::<LeafM>(
						&entities,
						placed.variant.palette_mix(),
						noise.seed,
						commands,
					);
					out.extend(entities);
				}
				TropicalTuftsCell::SmallPalmBush(bucket)
				| TropicalTuftsCell::JuvenilePalmBush(bucket) => {
					let geometry = bucket.item.build_with_noise(noise);
					let bush = PalmBush::new(geometry, self.leaf_material.clone(), noise);
					log::info!("spawning palm bush {:?} with transform {:?}", bush.geometry, local);
					let mut local = local;
					let entities = bush.spawn_render_items(commands, cascade_chunk, local);
					patch_spawned_leaf_material::<LeafM>(
						&entities,
						placed.variant.palette_mix(),
						noise.seed,
						commands,
					);
					out.extend(entities);
				}
			}
		}
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use procedural_common::BuildWithNoise;

	#[test]
	fn render_builds_tuft_shape_for_placed_cell() -> Result<()> {
		let dist = TropicalTuftsCell::grove_distribution();
		let Some(TropicalTuftsCell::BrightTuft(bucket)) = dist.buckets[1].item.clone() else {
			anyhow::bail!("expected BrightTuft variant");
		};
		let placement = GrovePlacedCell::new(
			TropicalTuftsCell::BrightTuft(bucket.clone()),
			Vec3::new(5.0, 0.0, 5.0),
			1.0,
		);
		let noise = placement_noise(NoiseParams::default(), placement.position);
		let shape = bucket.item.build_with_noise(noise);
		assert!(shape.blade_length > 0.0);
		Ok(())
	}

	#[test]
	fn render_builds_palm_geometry_for_placed_cell() -> Result<()> {
		let dist = TropicalTuftsCell::grove_distribution();
		let Some(TropicalTuftsCell::SmallPalmBush(bucket)) = dist.buckets[4].item.clone() else {
			anyhow::bail!("expected SmallPalmBush variant");
		};
		let placement = GrovePlacedCell::new(
			TropicalTuftsCell::SmallPalmBush(bucket.clone()),
			Vec3::new(3.0, 0.0, 7.0),
			1.0,
		);
		let noise = placement_noise(NoiseParams::default(), placement.position);
		let geometry = bucket.item.build_with_noise(noise);
		assert!(geometry.crown.fronds_per_ring >= 4);
		assert!(geometry.scale.height > 0.0);
		Ok(())
	}

	#[test]
	fn with_resolved_placements_skips_live_selection() -> Result<()> {
		let dist = TropicalTuftsCell::grove_distribution();
		let Some(TropicalTuftsCell::BrightTuft(bucket)) = dist.buckets[1].item.clone() else {
			anyhow::bail!("expected BrightTuft variant");
		};
		let placement = GrovePlacedCell::new(
			TropicalTuftsCell::BrightTuft(bucket),
			Vec3::new(1.0, 0.0, 2.0),
			1.0,
		);
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
		let mut tufts = TropicalTuftsStd::default();
		let span = DEFAULT_GROVE_EXTENT_XZ;
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		tufts = tufts.with_extent(extent);
		let placements = tufts.placements();
		let palms = placements
			.iter()
			.filter(|p| {
				matches!(
					p.variant,
					TropicalTuftsCell::SmallPalmBush(_) | TropicalTuftsCell::JuvenilePalmBush(_)
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

	#[test]
	fn extent_subdivision_yields_sparse_placements() -> Result<()> {
		let mut tufts = TropicalTuftsStd::default();
		let cell_extent = TropicalTuftsDefinition::cell_extent_xz_default();
		let span = 5.0 * cell_extent;
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span.x, 1.0, span.y));
		tufts = tufts.with_extent(extent);
		let placements = tufts.placements();
		assert!(
			!placements.is_empty(),
			"expected placed tropical-tufts cells with default weights, got {}",
			placements.len()
		);
		Ok(())
	}
}
