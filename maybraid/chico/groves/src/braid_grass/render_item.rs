//! [`RenderItem`] for populated Braid Grass groves ([#306](https://github.com/ramate-io/maybraid/issues/306)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::tuft::BladeTuft;
use clap::Args;
use gimme_gen::Cell;
use procedural_common::{noise_params_from_scalar_str, NoiseParams};
use render_item::{CascadeChunk, RenderItem};

use crate::braid_grass::sample::blade_tuft_shape_from;
use crate::braid_grass::{BraidGrassCell, BraidGrassClump, BraidGrassDefinition};
use crate::grove::{
	FlatTerrainSample, GroveFrontend, GrovePlacedCell, GroveRenderHelper, GroveRenderRule,
	TerrainSample,
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
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: TerrainSample + Clone + Send + Sync + 'static,
{
	#[command(flatten, next_help_heading = "Grove")]
	pub grove: GroveFrontend,

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
	pub cells: Vec<Cell>,

	#[arg(skip)]
	pub terrain: Option<Terrain>,

	/// Fixed outcomes when the grove is already resolved (replay, persistence, tests).
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
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static + Default,
	Terrain: TerrainSample + Clone + Send + Sync + 'static + Default,
{
	fn default() -> Self {
		Self {
			grove: GroveFrontend::default(),
			stick_material: StickS::default(),
			leaf_material: LeafS::default(),
			foliage_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
			cells: Vec::new(),
			terrain: Some(Terrain::default()),
			resolved_placements: None,
			__marker: PhantomData,
		}
	}
}

impl<StickM, StickS, LeafM, LeafS, Terrain> BraidGrass<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: TerrainSample + Clone + Send + Sync + 'static,
{
	/// Carry forward resolved placements when the grove is already in a stateful condition.
	///
	/// Use for replay, persisted forest snapshots, and isolation tests that pin bucket throw
	/// and constraint outcomes without re-running live cell selection.
	pub fn with_resolved_placements(
		resolved_placements: Vec<GrovePlacedCell<BraidGrassCell>>,
		foliage_noise: NoiseParams,
		stick_material: StickS,
		leaf_material: LeafS,
	) -> Self {
		Self {
			grove: GroveFrontend::default(),
			stick_material,
			leaf_material,
			foliage_noise,
			cells: Vec::new(),
			terrain: None,
			resolved_placements: Some(resolved_placements),
			__marker: PhantomData,
		}
	}

	pub fn with_cells(mut self, cells: Vec<Cell>) -> Self {
		self.cells = cells;
		self
	}

	pub fn with_terrain(mut self, terrain: Terrain) -> Self {
		self.terrain = Some(terrain);
		self
	}

	pub fn placements(&self) -> Vec<GrovePlacedCell<BraidGrassCell>> {
		if let Some(ref resolved) = self.resolved_placements {
			return resolved.clone();
		}
		let Some(ref terrain) = self.terrain else {
			log::warn!("braid grass live selection skipped: no terrain sample");
			return Vec::new();
		};
		self.assemble_grove().select_placements(&self.cells, terrain)
	}

	pub fn definition(&self) -> Result<BraidGrassDefinition, String> {
		let mut definition = BraidGrassDefinition::new();
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
		self.grove.assemble(definition)
	}

	fn render_rule(&self) -> BraidGrassRenderRule<LeafM, LeafS> {
		BraidGrassRenderRule {
			foliage_noise: self.foliage_noise,
			leaf_material: self.leaf_material.clone(),
			__marker: PhantomData,
		}
	}

	fn render_helper(&self) -> GroveRenderHelper<
		BladeTuft<LeafM, LeafS>,
		BraidGrassCell,
		BraidGrassRenderRule<LeafM, LeafS>,
	> {
		GroveRenderHelper::new(self.placements(), self.render_rule())
	}
}

impl<StickM, StickS, LeafM, LeafS, Terrain> RenderItem
	for BraidGrass<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material + Send + Sync + 'static,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: TerrainSample + Clone + Send + Sync + 'static,
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		self.render_helper().spawn_render_items(commands, cascade_chunk, transform)
	}
}

#[derive(Clone)]
pub struct BraidGrassRenderRule<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>>,
{
	pub foliage_noise: NoiseParams,
	pub leaf_material: LeafS,
	pub(crate) __marker: PhantomData<fn() -> LeafM>,
}

impl<LeafM, LeafS> GroveRenderRule<BladeTuft<LeafM, LeafS>, BraidGrassCell>
	for BraidGrassRenderRule<LeafM, LeafS>
where
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Send + Sync + 'static,
{
	fn render_item_for(
		&self,
		placed: &GrovePlacedCell<BraidGrassCell>,
	) -> Option<(BladeTuft<LeafM, LeafS>, Transform)> {
		let grass = braid_grass_clump(&placed.variant)?;
		let seed = self.foliage_noise.seed ^ placed.position.x.to_bits() as i32;
		let mut shape = blade_tuft_shape_from(placed.position, grass, seed);
		shape.noise_amplitude = self.foliage_noise.amplitude;
		shape.noise_frequency = self.foliage_noise.frequency;

		let tuft = BladeTuft::from_shape(shape, self.leaf_material.clone());
		let transform = Transform {
			translation: placed.position,
			rotation: Quat::IDENTITY,
			scale: Vec3::splat(placed.scale.max(1e-4)),
		};
		Some((tuft, transform))
	}
}

fn braid_grass_clump(variant: &BraidGrassCell) -> Option<&BraidGrassClump> {
	match variant {
		BraidGrassCell::None(_) => None,
		BraidGrassCell::DeepGreenBlade(bucket) => Some(&bucket.item),
		BraidGrassCell::PaleReedBlade(bucket) => Some(&bucket.item),
		BraidGrassCell::JungleBlade(bucket) => Some(&bucket.item),
		BraidGrassCell::RedEdgeBlade(bucket) => Some(&bucket.item),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::grove::CellGrove;
	use anyhow::Result;

	#[test]
	fn render_rule_builds_blade_tuft_for_placed_cell() -> Result<()> {
		let dist = BraidGrassCell::grove_distribution();
		let Some(BraidGrassCell::DeepGreenBlade(bucket)) = dist.buckets[1].item.clone() else {
			anyhow::bail!("expected DeepGreenBlade variant");
		};
		let placement = GrovePlacedCell::new(
			BraidGrassCell::DeepGreenBlade(bucket),
			Vec3::new(5.0, 0.0, 5.0),
			1.0,
		);

		let rule = BraidGrassRenderRule {
			foliage_noise: NoiseParams::default(),
			leaf_material: SkippedLeafMeshMaterial::<StandardMaterial>::default(),
			__marker: PhantomData,
		};
		let Some((tuft, transform)) = rule.render_item_for(&placement) else {
			anyhow::bail!("expected blade tuft render item");
		};
		assert!(tuft.shape.blade_count >= 10);
		assert!(transform.scale.x > 0.0);
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
			grove: GroveFrontend {
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
	fn sample_respects_authored_blade_count_range() -> Result<()> {
		use procedural_common::UnitRange;

		let grass = BraidGrassClump {
			height: UnitRange::new(1.0, 2.0),
			width: UnitRange::new(0.3, 0.8),
			blade_count: 12..=28,
			braid_twist: UnitRange::new(0.1, 0.3),
		};
		let shape = blade_tuft_shape_from(Vec3::new(3.0, 0.0, 7.0), &grass, 0);
		assert!((12..=28).contains(&shape.blade_count));
		Ok(())
	}
}
