//! [`RenderItem`] for populated Riverine Green groves ([#307](https://github.com/ramate-io/maybraid/issues/307)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_tree_components::HighBushShoots;
use chico_vegetation_shaders::ChicoStickMaterial;
use clap::Args;
use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};
use render_item::{CascadeChunk, RenderItem};

use chico_groves::{
	patch_spawned_leaf_material, placement_noise, FlatTerrainSample, GroveExtent, GroveFrontend,
	GroveCellVariant, GroveWorldSample, WithPalette, DEFAULT_GROVE_EXTENT_XZ,
};
use chico_groves::riverine_green::{definition, RiverineGreenCell, RiverineGreenItem};
use crate::skipped_mesh_material::{SkippedLeafMeshMaterial, SkippedStickMeshMaterial};

/// Typical [`ChicoStickMaterial`] / [`StandardMaterial`] Riverine Green instance.
pub type RiverineGreenStd = RiverineGreen<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
	FlatTerrainSample,
>;

/// Riverine Green grove preview (stick + leaf materials → Common High Bush shrubs).
#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct RiverineGreen<StickM, StickS, LeafM, LeafS, Terrain = FlatTerrainSample>
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
	resolved_placements: Option<Vec<GroveCellVariant<RiverineGreenCell>>>,

	#[arg(skip)]
	__marker: PhantomData<fn() -> (StickM, LeafM)>,
}

impl<StickM, StickS, LeafM, LeafS, Terrain> Default
	for RiverineGreen<StickM, StickS, LeafM, LeafS, Terrain>
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

impl<StickM, StickS, LeafM, LeafS, Terrain> RiverineGreen<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: GroveWorldSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	pub fn with_resolved_placements(
		resolved_placements: Vec<GroveCellVariant<RiverineGreenCell>>,
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

	pub fn placements(&self) -> Vec<GroveCellVariant<RiverineGreenCell>> {
		if let Some(ref resolved) = self.resolved_placements {
			return resolved.clone();
		}
		self.grove.assemble(definition()).populate(&self.extent, &self.terrain)
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
	for RiverineGreen<StickM, StickS, LeafM, LeafS, Terrain>
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
			let chain_noise = placement_noise(self.bush_chain_noise, placed.position);
			// Grove frontend noise samples authored geometry; chain noise drives stick growth.
			let build_noise = placement_noise(self.grove.noise, placed.position);
			let RiverineGreenItem::Bush(bush) = placed.variant.item();
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
			out.extend(entities);
		}
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use chico_tree_components::HighBushFoliageStyle;
	use anyhow::Result;

	#[test]
	fn bush_geometry_builds_within_authored_ranges() -> Result<()> {
		let noise = placement_noise(NoiseParams::default(), Vec3::new(5.0, 0.0, 5.0));
		for cell in [
			RiverineGreenCell::WetGreenBush,
			RiverineGreenCell::BrightBankBush,
			RiverineGreenCell::DeepShadeBush,
			RiverineGreenCell::PaleRiparianBush,
			RiverineGreenCell::RedTwigRiverBush,
		] {
			let RiverineGreenItem::Bush(bush) = cell.item();
			let shape = bush.build_with_noise(noise);
			assert!(shape.height >= bush.height.start.min(bush.height.end));
			assert!(shape.height <= bush.height.start.max(bush.height.end));
			assert!(bush.shoot_count.contains(&shape.shoot_count));
			assert!(bush.branch_depth.contains(&(shape.branch_depth as u32)));
			assert!(
				shape.radial_strength >= bush.radial_strength.start.min(bush.radial_strength.end)
			);
			assert!(
				shape.radial_strength <= bush.radial_strength.start.max(bush.radial_strength.end)
			);
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
	fn dual_palette_resolves_for_stick_and_canopy() -> Result<()> {
		for cell in [
			RiverineGreenCell::WetGreenBush,
			RiverineGreenCell::BrightBankBush,
			RiverineGreenCell::DeepShadeBush,
			RiverineGreenCell::PaleRiparianBush,
			RiverineGreenCell::RedTwigRiverBush,
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
			let stick = ChicoStickMaterial::with_palette(
				ChicoStickMaterial::default(),
				cell.stick_palette_mix(),
				7,
			);
			let canopy = StandardMaterial::with_palette(
				StandardMaterial::default(),
				cell.canopy_palette_mix(),
				7,
			);
			let picked_stick = cell.stick_palette_mix().pick_color(7);
			let picked_canopy = cell.canopy_palette_mix().pick_color(7);
			assert!(picked_stick.is_some());
			assert!(picked_canopy.is_some());
			let linear = bevy::color::LinearRgba::from(picked_stick.unwrap());
			assert!((stick.base_color.x - linear.red).abs() < 1e-5);
			assert!((stick.base_color.y - linear.green).abs() < 1e-5);
			assert!((stick.base_color.z - linear.blue).abs() < 1e-5);
			let canopy_allowed: Vec<_> = cell
				.canopy_palette_mix()
				.slots
				.iter()
				.flat_map(|s| [s.start.resolve(), s.end.resolve()])
				.flatten()
				.collect();
			assert!(canopy_allowed.contains(&canopy.base_color));
		}
		Ok(())
	}

	#[test]
	fn with_resolved_placements_skips_live_selection() -> Result<()> {
		let placement =
			GroveCellVariant::new(RiverineGreenCell::WetGreenBush, Vec3::new(1.0, 0.0, 2.0), 1.0);
		let item = RiverineGreenStd::with_resolved_placements(
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
	fn default_weights_yield_moderate_placements_in_preview_grid() -> Result<()> {
		let span = DEFAULT_GROVE_EXTENT_XZ;
		let grove = RiverineGreenStd::default()
			.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span)));
		let cells = grove.placement_cells().len();
		let placements = grove.placements();
		let placed_share = placements.len() as f32 / cells as f32;
		assert!(
			(0.15..=0.45).contains(&placed_share),
			"expected moderate riverine fill, got {placed_share} ({}/{cells})",
			placements.len()
		);
		assert!(!placements.is_empty());
		Ok(())
	}
}
