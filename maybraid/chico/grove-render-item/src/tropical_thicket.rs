//! [`RenderItem`] for populated Tropical Thicket groves ([#317](https://github.com/ramate-io/maybraid/issues/317)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sbs_trees::honu_banyan::HonuBanyanParams;
use chico_sbs_trees::palm_bush::PalmBushParams;
use chico_vegetation_components::{spawn_vegetation_components, vegetation_bounds};
use chico_tree_components::HighBushShoots;
use chico_vegetation_shaders::ChicoStickMaterial;
use clap::Args;
use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};
use render_item::{CascadeChunk, RenderItem};

use crate::skipped_mesh_material::{
	SkippedLeafMeshMaterial, SkippedStickMeshMaterial as GroveSkippedStickMeshMaterial,
};
use chico_groves::tropical_thicket::{definition, TropicalThicketCell, TropicalThicketItem};
use chico_groves::{
	patch_spawned_leaf_material, placement_noise, FlatTerrainSample, GroveCellVariant, GroveExtent,
	GroveFrontend, GroveWorldSample, WithPalette, DEFAULT_GROVE_EXTENT_XZ,
};

/// Honu template for mini-banyan placements (LodScene / VegetationComponents).
pub type ThicketHonu = HonuBanyanParams;

/// Typical [`ChicoStickMaterial`] / [`StandardMaterial`] Tropical Thicket instance.
pub type TropicalThicketStd = TropicalThicket<
	ChicoStickMaterial,
	GroveSkippedStickMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
	FlatTerrainSample,
>;

/// Tropical Thicket grove preview (palms, high bushes, and mini Honu Banyan forms).
#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct TropicalThicket<StickM, StickS, LeafM, LeafS, Terrain = FlatTerrainSample>
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

	#[arg(skip)]
	pub honu_template: ThicketHonu,

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
	resolved_placements: Option<Vec<GroveCellVariant<TropicalThicketCell>>>,

	#[arg(skip)]
	__marker: PhantomData<fn() -> (StickM, LeafM)>,
}

impl<StickM, StickS, LeafM, LeafS, Terrain> Default
	for TropicalThicket<StickM, StickS, LeafM, LeafS, Terrain>
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
			honu_template: ThicketHonu::default(),
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

impl<StickM, StickS, LeafM, LeafS, Terrain> TropicalThicket<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: GroveWorldSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	pub fn with_resolved_placements(
		resolved_placements: Vec<GroveCellVariant<TropicalThicketCell>>,
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
			honu_template: ThicketHonu::default(),
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

	pub fn placements(&self) -> Vec<GroveCellVariant<TropicalThicketCell>> {
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
	for TropicalThicket<StickM, StickS, LeafM, LeafS, Terrain>
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
			let foliage_noise = placement_noise(self.leaf_surface_noise, placed.position);
			let entities = match placed.variant.item() {
				TropicalThicketItem::Palm(palm) => {
					let geometry = palm.build_with_noise(foliage_noise);
					let mut params = PalmBushParams::default();
					params.geometry = geometry;
					let bush = params.build();
					let bounds = vegetation_bounds(&bush);
					spawn_vegetation_components(commands, &bush, local, bounds)
				}
				TropicalThicketItem::Bush(bush) => {
					let chain_noise = placement_noise(self.bush_chain_noise, placed.position);
					let build_noise = placement_noise(self.grove.noise, placed.position);
					let mut shape = bush.build_with_noise(build_noise);
					shape.chain_noise = chain_noise;
					let entities = HighBushShoots::<StickM, StickS, LeafM, LeafS>::spawn_from_shape(
						shape,
						placement_noise(self.stick_surface_noise, placed.position),
						foliage_noise,
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
				TropicalThicketItem::Banyan(banyan) => {
					let build_noise = placement_noise(self.grove.noise, placed.position);
					let samples = banyan.build_with_noise(build_noise);
					let mut params = self.honu_template.clone();
					params.geometry = samples.geometry;
					params.growth_spawn_fraction = samples.growth_spawn_fraction;
					let tree = params.build();
					let bounds = vegetation_bounds(&tree);
					spawn_vegetation_components(commands, &tree, local, bounds)
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
	use procedural_common::CountPair as RingLayout;

	#[test]
	fn palm_bush_and_banyan_geometry_builds_within_authored_ranges() -> Result<()> {
		let noise = placement_noise(NoiseParams::default(), Vec3::new(5.0, 0.0, 5.0));

		for cell in [
			TropicalThicketCell::LargePalmBush,
			TropicalThicketCell::BroadWetPalmBush,
			TropicalThicketCell::RedStemPalmBush,
		] {
			let TropicalThicketItem::Palm(palm) = cell.item() else {
				anyhow::bail!("expected palm item for {cell:?}");
			};
			let geom = palm.build_with_noise(noise);
			assert!(geom.height() >= palm.height.start.min(palm.height.end));
			assert!(geom.height() <= palm.height.start.max(palm.height.end));
			assert!(palm.frond_count.contains(&geom.crown.fronds_per_ring));
		}

		let TropicalThicketItem::Banyan(banyan) = TropicalThicketCell::MiniHonuBanyan.item() else {
			anyhow::bail!("expected banyan item");
		};
		let samples = banyan.build_with_noise(noise);
		assert!(samples.geometry.scale.tree_height >= banyan.height.start.min(banyan.height.end));
		assert!(samples.geometry.scale.tree_height <= banyan.height.start.max(banyan.height.end));
		assert_eq!(samples.geometry.rings.layout, RingLayout::new(2, 5));
		assert!(
			samples.geometry.growth.descender_threshold
				>= banyan.descender_density.start.min(banyan.descender_density.end)
		);
		assert!(
			samples.growth_spawn_fraction
				>= banyan.canopy_density.start.min(banyan.canopy_density.end)
		);

		for cell in [TropicalThicketCell::ModerateHighBush, TropicalThicketCell::FloweringHighBush]
		{
			let TropicalThicketItem::Bush(bush) = cell.item() else {
				anyhow::bail!("expected bush item for {cell:?}");
			};
			let shape = bush.build_with_noise(noise);
			assert!(shape.height >= bush.height.start.min(bush.height.end));
			assert!(shape.height <= bush.height.start.max(bush.height.end));
			assert!(bush.shoot_count.contains(&shape.shoot_count));
			assert!(bush.branch_depth.contains(&(shape.branch_depth as u32)));
			assert!(
				shape.segment_length_fraction_lo
					>= bush.segment_length_fraction.start.min(bush.segment_length_fraction.end)
			);
			assert!(
				shape.segment_length_fraction_hi
					<= bush.segment_length_fraction.start.max(bush.segment_length_fraction.end)
			);
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			TropicalThicketCell::LargePalmBush,
			TropicalThicketCell::BroadWetPalmBush,
			TropicalThicketCell::MiniHonuBanyan,
			TropicalThicketCell::ModerateHighBush,
			TropicalThicketCell::FloweringHighBush,
			TropicalThicketCell::RedStemPalmBush,
		] {
			match cell.item() {
				TropicalThicketItem::Palm(_) => {
					let palette = cell.canopy_palette_mix();
					let mut allowed = Vec::new();
					for slot in palette.slots {
						allowed.extend(slot.start.resolve());
						allowed.extend(slot.end.resolve());
					}
					assert!(!allowed.is_empty(), "unresolved canopy tokens for {cell:?}");
				}
				TropicalThicketItem::Bush(_) | TropicalThicketItem::Banyan(_) => {
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
	fn with_resolved_placements_skips_live_selection() -> Result<()> {
		let placement = GroveCellVariant::new(
			TropicalThicketCell::LargePalmBush,
			Vec3::new(1.0, 0.0, 2.0),
			1.0,
		);
		let item = TropicalThicketStd::with_resolved_placements(
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
		let grove = TropicalThicketStd::default()
			.with_terrain(FlatTerrainSample { elevation: 0.35, steepness: 0.15 })
			.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span)));
		let cells = grove.placement_cells().len();
		let placements = grove.placements();
		let placed_share = placements.len() as f32 / cells as f32;
		assert!(
			(0.24..=0.62).contains(&placed_share),
			"expected tropical-thicket fill, got {placed_share} ({}/{cells})",
			placements.len()
		);
		assert!(!placements.is_empty());
		Ok(())
	}

	#[test]
	fn default_extent_includes_palm_bush_and_banyan_placements() -> Result<()> {
		let span = DEFAULT_GROVE_EXTENT_XZ * 2.0;
		let grove = TropicalThicketStd::default()
			.with_terrain(FlatTerrainSample { elevation: 0.35, steepness: 0.15 })
			.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span)));
		let placements = grove.placements();
		let palms = placements
			.iter()
			.filter(|p| {
				matches!(
					p.variant,
					TropicalThicketCell::LargePalmBush
						| TropicalThicketCell::BroadWetPalmBush
						| TropicalThicketCell::RedStemPalmBush
				)
			})
			.count();
		let bushes = placements
			.iter()
			.filter(|p| {
				matches!(
					p.variant,
					TropicalThicketCell::ModerateHighBush | TropicalThicketCell::FloweringHighBush
				)
			})
			.count();
		let banyans = placements
			.iter()
			.filter(|p| matches!(p.variant, TropicalThicketCell::MiniHonuBanyan))
			.count();
		assert!(palms > 0, "expected palm placements among {} total", placements.len());
		assert!(bushes > 0, "expected bush placements among {} total", placements.len());
		assert!(banyans > 0, "expected banyan placements among {} total", placements.len());
		Ok(())
	}
}
