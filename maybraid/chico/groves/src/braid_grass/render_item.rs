//! [`RenderItem`] for populated Braid Grass groves ([#306](https://github.com/ramate-io/maybraid/issues/306)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::tuft::{BladeTuft, BladeTuftShape};
use clap::Args;
use gimme_gen::Cell;
use procedural_common::{noise_params_from_scalar_str, NoiseConfig, NoiseParams};
use render_item::{CascadeChunk, RenderItem};

use crate::braid_grass::{
	BraidGrassCell, BraidGrassClump, BraidGrassDefinition, BraidGrassGroveFrontend,
};
use crate::grove::{
	FlatTerrainSample, GrovePlacedCell, GroveRenderHelper, GroveRenderRule, TerrainSample,
	WithPalette,
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
	pub cells: Vec<Cell>,

	#[command(flatten, next_help_heading = "Terrain")]
	pub terrain: Terrain,

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
			cells: Vec::new(),
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
	/// Carry forward resolved placements when the grove is already in a stateful condition.
	///
	/// Use for replay, persisted forest snapshots, and isolation tests that pin bucket throw
	/// and constraint outcomes without re-running live cell selection.
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
			cells: Vec::new(),
			terrain,
			resolved_placements: Some(resolved_placements),
			__marker: PhantomData,
		}
	}

	pub fn with_cells(mut self, cells: Vec<Cell>) -> Self {
		self.cells = cells;
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
		self.assemble_grove().select_placements(&self.cells, &self.terrain)
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
		self.grove.clone().assemble(definition)
	}

	fn render_rule(&self) -> BraidGrassRenderRule<LeafM, LeafS> {
		BraidGrassRenderRule {
			foliage_noise: self.foliage_noise,
			leaf_material: self.leaf_material.clone(),
			__marker: PhantomData,
		}
	}

	fn render_helper(
		&self,
	) -> GroveRenderHelper<
		PaletteBladeTuft<LeafM>,
		BraidGrassCell,
		BraidGrassRenderRule<LeafM, LeafS>,
	> {
		GroveRenderHelper::new(self.placements(), self.render_rule())
	}
}

/// Blade tuft with palette-resolved owned material (spawn adds assets explicitly).
#[derive(Component, Clone)]
pub struct PaletteBladeTuft<M: Material + Clone> {
	pub shape: BladeTuftShape,
	pub material: M,
}

impl<M: Material + Clone> PaletteBladeTuft<M> {
	pub fn from_shape(shape: BladeTuftShape, material: M) -> Self {
		Self { shape, material }
	}

	pub fn build_mesh(&self, world_uniform_scale: f32) -> Mesh {
		BladeTuft::<M, SkippedLeafMeshMaterial<M>>::from_shape(
			self.shape.clone(),
			SkippedLeafMeshMaterial::default(),
		)
		.build_mesh(world_uniform_scale)
	}
}

impl<M: Material + Clone + Send + Sync + 'static> RenderItem for PaletteBladeTuft<M> {
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		spawn_blade_tuft_with_owned_material(self, commands, cascade_chunk, transform)
	}
}

impl<StickM, StickS, LeafM, LeafS, Terrain> RenderItem
	for BraidGrass<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material + Send + Sync + 'static,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material + WithPalette + Default + Clone + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: TerrainSample + Clone + Send + Sync + 'static + Default + clap::Args,
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

impl<LeafM, LeafS> GroveRenderRule<PaletteBladeTuft<LeafM>, BraidGrassCell>
	for BraidGrassRenderRule<LeafM, LeafS>
where
	LeafM: Material + WithPalette + Default + Clone + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Send + Sync + 'static,
{
	fn render_item_for(
		&self,
		placed: &GrovePlacedCell<BraidGrassCell>,
	) -> Option<(PaletteBladeTuft<LeafM>, Transform)> {
		let grass = braid_grass_clump(&placed.variant)?;
		let foliage_seed = self.foliage_noise.seed ^ placed.position.x.to_bits() as i32;
		let palette_seed =
			foliage_seed ^ placed.position.z.to_bits() as i32 ^ placed.position.y.to_bits() as i32;
		let mut shape = blade_tuft_shape_from(placed.position, grass, foliage_seed);
		shape.noise_amplitude = self.foliage_noise.amplitude;
		shape.noise_frequency = self.foliage_noise.frequency;

		let material =
			LeafM::with_palette(LeafM::default(), placed.variant.palette_mix(), palette_seed);
		let tuft = PaletteBladeTuft::from_shape(shape, material);
		let transform = Transform {
			translation: placed.position,
			rotation: Quat::IDENTITY,
			scale: Vec3::splat(placed.scale.max(1e-4)),
		};
		Some((tuft, transform))
	}
}

fn spawn_blade_tuft_with_owned_material<M>(
	tuft: &PaletteBladeTuft<M>,
	commands: &mut Commands,
	cascade_chunk: &CascadeChunk,
	transform: Transform,
) -> Vec<Entity>
where
	M: Material + Send + Sync + 'static + Clone,
{
	let mesh = tuft.build_mesh(1.0);
	let material = tuft.material.clone();
	let marker = tuft.clone();
	let root = commands
		.spawn((marker, cascade_chunk.clone(), transform, Visibility::default()))
		.id();
	commands.queue(move |world: &mut World| {
		let mesh_handle = world.resource_mut::<Assets<Mesh>>().add(mesh);
		let material_handle = world.resource_mut::<Assets<M>>().add(material);
		world
			.entity_mut(root)
			.insert((Mesh3d(mesh_handle), MeshMaterial3d(material_handle)));
	});
	vec![root]
}

fn blade_tuft_shape_from(
	position: Vec3,
	grass: &BraidGrassClump,
	foliage_seed: i32,
) -> BladeTuftShape {
	let noise = NoiseConfig::new(NoiseParams::from_scalar(foliage_seed as f32, 1.0, 1.0, 1));
	let sample_f32 = |range: procedural_common::UnitRange, salt| {
		let lo = range.start.min(range.end);
		let hi = range.start.max(range.end);
		noise.sample_range_f32_4d(lo, hi, position.x, position.y, position.z, salt)
	};
	let sample_u32 = |range: &std::ops::RangeInclusive<u32>, salt| {
		let lo = *range.start() as usize;
		let hi = (*range.end() as usize).saturating_add(1);
		noise.sample_range_usize_4d(lo, hi, position.x, position.y, position.z, salt) as u32
	};

	BladeTuftShape {
		blade_count: sample_u32(&grass.blade_count, 3.0),
		blade_length: sample_f32(grass.height, 1.0).max(0.1),
		blade_width: sample_f32(grass.width, 2.0).max(0.005),
		max_tilt_radians: sample_f32(grass.braid_twist, 4.0).max(0.01),
		seed: foliage_seed,
		..BladeTuftShape::default()
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
		assert!(
			allowed.contains(&tuft.material.base_color),
			"expected palette-resolved color, got {:?}",
			tuft.material.base_color
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
		use crate::braid_grass::BraidGrassDefinition;
		use crate::grove::{parse_variant_weights, preview_cell_grid};

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
		grass.cells = preview_cell_grid(3, BraidGrassDefinition::preview_cell_extent());
		assert!(!grass.placements().is_empty());
		Ok(())
	}

	#[test]
	fn preview_cell_grid_yields_placements_with_default_weights() -> Result<()> {
		use crate::braid_grass::BraidGrassDefinition;
		use crate::grove::preview_cell_grid;

		let mut grass = BraidGrassStd::default();
		grass.cells = preview_cell_grid(5, BraidGrassDefinition::preview_cell_extent());
		let placements = grass.placements();
		assert!(
			!placements.is_empty(),
			"expected placed braid-grass cells with default weights, got {}",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn preview_cell_grid_yields_placements_with_authored_terrain() -> Result<()> {
		use crate::braid_grass::BraidGrassDefinition;
		use crate::grove::{parse_variant_weights, preview_cell_grid};

		let mut grass = BraidGrassStd::default();
		grass.grove.variant_weights =
			Some(parse_variant_weights("0,3,2,2,1").map_err(|e| anyhow::anyhow!("{e}"))?);
		grass.cells = preview_cell_grid(5, BraidGrassDefinition::preview_cell_extent());
		let placements = grass.placements();
		assert!(
			!placements.is_empty(),
			"expected placed braid-grass cells in preview grid, got {}",
			placements.len()
		);
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
