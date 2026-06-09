//! [`RenderItem`] for populated Braid Grass groves ([#306](https://github.com/ramate-io/maybraid/issues/306)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::tuft::BladeTuft;
use clap::Args;
use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};
use render_item::{CascadeChunk, RenderItem};

use super::{
	BraidGrassCell, BraidGrassDefinition, BraidGrassGroveFrontend,
};
use crate::grove::{
	owned_spawn, placement_noise, CellGrove, FlatTerrainSample, GroveExtent, GrovePlacedCell,
	TerrainSample, WithPalette, DEFAULT_GROVE_EXTENT_XZ,
};
use crate::skipped_mesh_material::{SkippedLeafMeshMaterial, SkippedStickMeshMaterial};

/// Typical [`StandardMaterial`] Braid Grass instance.
pub type BraidGrassStd = BraidGrass<
	StandardMaterial,
	SkippedStickMeshMaterial<StandardMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
>;

/// Braid Grass grove with hammer M/S material slots (leaf → blade tufts).
#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct BraidGrass<StickM, StickS, LeafM, LeafS, Terrain = FlatTerrainSample>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material + WithPalette + Default,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: TerrainSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	#[command(flatten, next_help_heading = "Grove")]
	pub grove: BraidGrassGroveFrontend,

	#[command(flatten, next_help_heading = "Stick Material")]
	pub stick_material: StickS,

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
	resolved_placements: Option<Vec<GrovePlacedCell<BraidGrassCell>>>,

	#[arg(skip)]
	__marker: PhantomData<(fn() -> StickM, fn() -> LeafM)>,
}

impl<StickM, StickS, LeafM, LeafS, Terrain> Default
	for BraidGrass<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static + Default,
	LeafM: Material + WithPalette + Default,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static + Default,
	Terrain: TerrainSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	fn default() -> Self {
		Self {
			grove: BraidGrassGroveFrontend::default(),
			stick_material: StickS::default(),
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

impl<StickM, StickS, LeafM, LeafS, Terrain> BraidGrass<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material + WithPalette + Default,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: TerrainSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	pub fn with_resolved_placements(
		resolved_placements: Vec<GrovePlacedCell<BraidGrassCell>>,
		terrain: Terrain,
		foliage_noise: NoiseParams,
		stick_material: StickS,
		leaf_material: LeafS,
	) -> Self {
		Self {
			grove: BraidGrassGroveFrontend::default(),
			stick_material,
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

	pub fn placements(&self) -> Vec<GrovePlacedCell<BraidGrassCell>> {
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
				log::warn!("braid grass definition: {err}; using authored cell extent");
				BraidGrassDefinition::cell_extent_xz_default()
			});
		self.extent.subdivide_xz(cell_extent_xz)
	}

	pub fn definition(&self) -> Result<BraidGrassDefinition, String> {
		let mut definition =
			BraidGrassDefinition::new().with_cell_extent_xz(self.grove.cell_extent_xz);
		if let Some(ref overrides) = self.grove.variant_weights {
			definition = definition.with_variant_weights(overrides)?;
		}
		Ok(definition)
	}

	pub fn assemble_grove(&self) -> crate::grove::Grove<BraidGrassDefinition> {
		let definition = self.definition().unwrap_or_else(|err| {
			log::warn!("braid grass definition: {err}; using authored weights");
			BraidGrassDefinition::new()
		});
		self.grove.clone().assemble(definition)
	}
}

fn placement_transform(placed: &GrovePlacedCell<BraidGrassCell>) -> Transform {
	Transform {
		translation: placed.position,
		rotation: Quat::IDENTITY,
		scale: Vec3::splat(placed.scale.max(1e-4)),
	}
}

#[derive(Component, Clone, Copy)]
struct PlacedBladeTuft;

fn spawn_blade_tuft_for_bucket<LeafM>(
	clump: &super::BraidGrassClump,
	palette_mix: &crate::grove::PaletteMix,
	placed: &GrovePlacedCell<BraidGrassCell>,
	foliage_noise: NoiseParams,
	commands: &mut Commands,
	cascade_chunk: &CascadeChunk,
	transform: Transform,
) -> Vec<Entity>
where
	LeafM: Material + WithPalette + Default + Send + Sync + 'static,
{
	let noise = placement_noise(foliage_noise, placed.position);
	let mut shape = clump.build_with_noise(noise);
	shape.noise_amplitude = foliage_noise.amplitude;
	shape.noise_frequency = foliage_noise.frequency;
	let material = LeafM::with_palette(LeafM::default(), palette_mix, noise.seed);
	let mesh = BladeTuft::<LeafM, SkippedLeafMeshMaterial<LeafM>>::from_shape(
		shape.clone(),
		SkippedLeafMeshMaterial::default(),
	)
	.build_mesh(1.0);
	owned_spawn::spawn_owned_mesh(PlacedBladeTuft, mesh, material, commands, cascade_chunk, transform)
}

impl<StickM, StickS, LeafM, LeafS, Terrain> RenderItem
	for BraidGrass<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material + Send + Sync + 'static,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
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
			match &placed.variant {
				BraidGrassCell::None(_) => {}
				BraidGrassCell::DeepGreenBlade(bucket)
				| BraidGrassCell::PaleReedBlade(bucket)
				| BraidGrassCell::JungleBlade(bucket)
				| BraidGrassCell::RedEdgeBlade(bucket) => {
					out.extend(spawn_blade_tuft_for_bucket::<LeafM>(
						&bucket.item,
						placed.variant.palette_mix(),
						&placed,
						self.foliage_noise,
						commands,
						cascade_chunk,
						local,
					));
				}
			}
		}
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::grove::CellGrove;
	use anyhow::Result;

	#[test]
	fn build_with_noise_produces_palette_resolved_tuft() -> Result<()> {
		let dist = BraidGrassCell::grove_distribution();
		let Some(BraidGrassCell::DeepGreenBlade(bucket)) = dist.buckets[1].item.clone() else {
			anyhow::bail!("expected DeepGreenBlade variant");
		};
		let placement = GrovePlacedCell::new(
			BraidGrassCell::DeepGreenBlade(bucket),
			Vec3::new(5.0, 0.0, 5.0),
			1.0,
		);
		let BraidGrassCell::DeepGreenBlade(bucket) = &placement.variant else {
			anyhow::bail!("expected DeepGreenBlade bucket");
		};
		let noise = placement_noise(NoiseParams::default(), placement.position);
		let shape = bucket.item.build_with_noise(noise);
		assert!(shape.blade_count >= 10);
		let palette = placement.variant.palette_mix();
		let mut allowed = Vec::new();
		for slot in &palette.slots {
			if let Some(color) = slot.start.resolve() {
				allowed.push(color);
			}
			if let Some(color) = slot.end.resolve() {
				allowed.push(color);
			}
		}
		let material =
			StandardMaterial::with_palette(StandardMaterial::default(), palette, noise.seed);
		assert!(
			allowed.contains(&material.base_color),
			"expected palette-resolved color, got {:?}",
			material.base_color
		);
		Ok(())
	}

	#[test]
	fn with_resolved_placements_skips_live_selection() -> Result<()> {
		let dist = BraidGrassCell::grove_distribution();
		let Some(BraidGrassCell::DeepGreenBlade(bucket)) = dist.buckets[1].item.clone() else {
			anyhow::bail!("expected DeepGreenBlade variant");
		};
		let placement = GrovePlacedCell::new(
			BraidGrassCell::DeepGreenBlade(bucket),
			Vec3::new(1.0, 0.0, 2.0),
			1.0,
		);
		let item = BraidGrassStd::with_resolved_placements(
			vec![placement.clone()],
			FlatTerrainSample::default(),
			NoiseParams::default(),
			SkippedStickMeshMaterial::<StandardMaterial>::default(),
			SkippedLeafMeshMaterial::<StandardMaterial>::default(),
		);
		assert_eq!(item.placements(), vec![placement]);
		Ok(())
	}

	#[test]
	fn variant_weights_override_definition_before_assembly() -> Result<()> {
		use crate::grove::parse_variant_weights;

		let item = BraidGrassStd {
			grove: BraidGrassGroveFrontend {
				variant_weights: Some(
					parse_variant_weights("0.0,9.0,x,x,x").map_err(|e| anyhow::anyhow!("{e}"))?,
				),
				..Default::default()
			},
			..Default::default()
		};
		let definition = item.definition().map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(definition.distribution().buckets[0].weight, 0.0);
		assert_eq!(definition.distribution().buckets[1].weight, 9.0);
		Ok(())
	}

	#[test]
	fn zero_none_weight_still_places_blades() -> Result<()> {
		use crate::grove::{parse_variant_weights, GroveExtent};

		let mut grass = BraidGrassStd {
			grove: BraidGrassGroveFrontend {
				variant_weights: Some(
					parse_variant_weights("0.0,9.0,x,x,x").map_err(|e| anyhow::anyhow!("{e}"))?,
				),
				..Default::default()
			},
			terrain: FlatTerrainSample { elevation: 0.4, steepness: 0.1 },
			..Default::default()
		};
		let cell_extent = BraidGrassDefinition::cell_extent_xz_default();
		let span = 3.0 * cell_extent;
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span.x, 1.0, span.y));
		grass = grass.with_extent(extent);
		assert!(!grass.placements().is_empty());
		Ok(())
	}

	#[test]
	fn extent_subdivision_yields_placements_with_default_weights() -> Result<()> {
		use crate::grove::GroveExtent;

		let mut grass = BraidGrassStd::default();
		let cell_extent = BraidGrassDefinition::cell_extent_xz_default();
		let span = 5.0 * cell_extent;
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span.x, 1.0, span.y));
		grass = grass.with_extent(extent);
		let placements = grass.placements();
		assert!(
			!placements.is_empty(),
			"expected placed braid-grass cells with default weights, got {}",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn extent_subdivision_yields_placements_with_authored_terrain() -> Result<()> {
		use crate::grove::{parse_variant_weights, GroveExtent};

		let mut grass = BraidGrassStd::default();
		grass.grove.variant_weights =
			Some(parse_variant_weights("0,3,2,2,1").map_err(|e| anyhow::anyhow!("{e}"))?);
		let cell_extent = BraidGrassDefinition::cell_extent_xz_default();
		let span = 5.0 * cell_extent;
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span.x, 1.0, span.y));
		grass = grass.with_extent(extent);
		let placements = grass.placements();
		assert!(
			!placements.is_empty(),
			"expected placed braid-grass cells in preview grid, got {}",
			placements.len()
		);
		Ok(())
	}
}
