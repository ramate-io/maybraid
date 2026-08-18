//! Forlorn Savanna — low-density sparse dry upper-canopy grove
//! ([RFC-183 §3.4.7.6], [#351](https://github.com/ramate-io/maybraid/issues/351)).
//!
//! Wind-shaped Rory's Head-trained forms, acacia-impression High Bush, and rare dry Storybook
//! accents across open savanna. Forest-layer attachment remains a follow-up.

use std::ops::RangeInclusive;

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

/// Sparse sampled canopy-density band ([`0.0`, `0.35`]).
const SPARSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.0, 0.35);
/// Flat sparse crown projection for acacia-like High Bush forms.
const SPARSE_PROJECTION_RADIAL: UnitRange = UnitRange::new(0.42, 0.62);
const SPARSE_PROJECTION_VERTICAL: UnitRange = UnitRange::new(0.32, 0.52);

/// Authored Forlorn Savanna grove definition.
///
/// Cell footprint sits at the RFC midpoint (`30` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<ForlornSavannaCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(30.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-30.0, 30.0),
		),
		distribution: ForlornSavannaCell::distribution(),
	}
}

/// Ordered forlorn-savanna varietals ([RFC-183 §3.4.7.6]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForlornSavannaCell {
	SavannaRory,
	AcaciaHighBush,
	RareSavannaStorybook,
}

/// Typed authored geometry for one forlorn-savanna varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ForlornSavannaItem {
	Rory(&'static ForlornSavannaRory),
	HighBush(&'static ForlornSavannaHighBush),
	Storybook(&'static ForlornSavannaStorybook),
}

/// Authored geometry ranges for one Rory's Head-trained form.
#[derive(Debug, Clone, PartialEq)]
pub struct ForlornSavannaRory {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one acacia-impression Common High Bush form.
#[derive(Debug, Clone, PartialEq)]
pub struct ForlornSavannaHighBush {
	pub height: UnitRange,
	pub shoot_count: RangeInclusive<u32>,
	pub branch_depth: RangeInclusive<u32>,
	pub radial_strength: UnitRange,
	pub vertical_bias: UnitRange,
	/// Terminal foliage radius in **world meters** (render converts to a height fraction).
	pub leaf_radius: UnitRange,
}

/// Authored geometry ranges for one dry Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct ForlornSavannaStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const SAVANNA_RORY: ForlornSavannaRory = ForlornSavannaRory {
	height: UnitRange::new(5.0, 30.0),
	stalk_radius: UnitRange::new(0.12, 0.45),
	canopy_spread: UnitRange::new(3.0, 12.0),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const ACACIA_HIGH_BUSH: ForlornSavannaHighBush = ForlornSavannaHighBush {
	height: UnitRange::new(5.0, 10.0),
	shoot_count: 4..=12,
	branch_depth: 2..=3,
	radial_strength: SPARSE_PROJECTION_RADIAL,
	vertical_bias: SPARSE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.35, 0.55),
};

const RARE_SAVANNA_STORYBOOK: ForlornSavannaStorybook = ForlornSavannaStorybook {
	height: UnitRange::new(10.0, 20.0),
	stalk_radius: UnitRange::new(0.24, 0.52),
	canopy_spread: UnitRange::new(2.5, 6.5),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const SAVANNA_RORY_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("weathered_bark", "dark_bark"),
	PaletteSlot::new("red_brown", "gray_brown"),
]);

const SAVANNA_RORY_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "dry_green"),
	PaletteSlot::new("yellow_green", "dusty_green"),
]);

const ACACIA_HIGH_BUSH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("acacia_bark", "red_brown"),
	PaletteSlot::new("tan_bark", "gray_brown"),
]);

const ACACIA_HIGH_BUSH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dusty_green", "olive_green"),
	PaletteSlot::new("yellow_green", "dry_green"),
]);

const SAVANNA_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_brown", "dark_bark"),
	PaletteSlot::new("gray_brown", "tan_bark"),
]);

const SAVANNA_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "yellow_green"),
	PaletteSlot::new("dusty_green", "light_green"),
]);

impl ForlornSavannaCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `5.2`; the `None` weight of `30.0` puts the placed share at
	/// `5.2 / 35.2 ≈ 0.15`, mid RFC `DENSITY_RANGE` (`0.06..0.20`).
	pub fn distribution() -> GroveDistribution<Self> {
		let rory = PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.58));
		let high_bush =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.64));
		let storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.50));
		GroveDistribution::new(vec![
			GroveBucket::none(30.0),
			GroveBucket::placed(3.0, rory, Self::SavannaRory),
			GroveBucket::placed(2.0, high_bush, Self::AcaciaHighBush),
			GroveBucket::placed(0.2, storybook, Self::RareSavannaStorybook),
		])
	}

	pub fn item(self) -> ForlornSavannaItem {
		match self {
			Self::SavannaRory => ForlornSavannaItem::Rory(&SAVANNA_RORY),
			Self::AcaciaHighBush => ForlornSavannaItem::HighBush(&ACACIA_HIGH_BUSH),
			Self::RareSavannaStorybook => ForlornSavannaItem::Storybook(&RARE_SAVANNA_STORYBOOK),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::SavannaRory => SAVANNA_RORY_STICK_MIX,
			Self::AcaciaHighBush => ACACIA_HIGH_BUSH_STICK_MIX,
			Self::RareSavannaStorybook => SAVANNA_STORYBOOK_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::SavannaRory => SAVANNA_RORY_CANOPY_MIX,
			Self::AcaciaHighBush => ACACIA_HIGH_BUSH_CANOPY_MIX,
			Self::RareSavannaStorybook => SAVANNA_STORYBOOK_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
mod vc {
	use bevy::math::bounding::Aabb3d;
	use bevy::prelude::*;
	use bevy::scene::prelude::Scene;
	use chico_sbs_trees::{
		HighBushShoots, HighBushShootsParams, RorysHeadTrained, RorysHeadTrainedParams,
		StorybookTree, StorybookTreeParams,
	};
	use chico_vegetation_components::{
		FoliageNode, Layers, Placement, StickNode, StructuralLod, VegetationComponents,
	};
	use clap::Args;
	use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
	use lod::lod_ref::LodRef;
	use lod::{lod_host_scene_pending, SceneChunk};
	use material_ref::MaterialRef;
	use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};

	use super::{definition, ForlornSavannaCell, ForlornSavannaItem};
	use crate::grove::{
		canopy_ball_material_from_palette, canopy_proxy_site, foliage_low_canopy_balls,
		foliage_ultra_low_merged_balls, frond_material_from_palette, grove_detail_level,
		grove_lod_culls, grove_lod_level, grove_lod_status, grove_structural_footprint,
		layers_from_nodes, nest_placed_plant_chunk, placement_noise, stick_material_from_palette,
		woody_grove_scene_chunks, CanopyProxySite, FlatTerrainSample, GroveCellVariant,
		GroveExtent, GroveFrontend, DEFAULT_GROVE_EXTENT_XZ, ULTRA_LOW_CANOPY_BIN_METERS,
	};

	pub const FORLORN_SAVANNA_STRUCTURAL_HIGH_FACTOR: f32 = 2.0;
	pub const FORLORN_SAVANNA_STRUCTURAL_MEDIUM_FACTOR: f32 = 5.0;
	pub const FORLORN_SAVANNA_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct ForlornSavannaParams {
		#[command(flatten, next_help_heading = "Grove")]
		pub grove: GroveFrontend,

		#[arg(
			long,
			default_value = "0,1.0,1.0,1",
			value_parser = noise_params_from_scalar_str,
			value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
			help_heading = "The noise applied to the chains of sticks in trees and bushes",
		)]
		pub tree_chain_noise: NoiseParams,

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
		pub terrain: FlatTerrainSample,

		#[arg(skip)]
		resolved_placements: Option<Vec<GroveCellVariant<ForlornSavannaCell>>>,
	}

	impl Default for ForlornSavannaParams {
		fn default() -> Self {
			Self {
				grove: GroveFrontend::default(),
				tree_chain_noise: NoiseParams::from_scalar(0.0, 1.0, 1.0, 1),
				stick_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
				leaf_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
				extent: GroveExtent::new(
					Vec3::ZERO,
					Vec3::new(DEFAULT_GROVE_EXTENT_XZ, 1.0, DEFAULT_GROVE_EXTENT_XZ),
				),
				terrain: FlatTerrainSample { elevation: 0.40, steepness: 0.20 },
				resolved_placements: None,
			}
		}
	}

	impl ForlornSavannaParams {
		pub fn with_extent(mut self, extent: GroveExtent) -> Self {
			self.extent = extent;
			self
		}

		pub fn with_terrain(mut self, terrain: FlatTerrainSample) -> Self {
			self.terrain = terrain;
			self
		}

		pub fn cell_extent_xz(&self) -> Vec2 {
			self.grove.definition(definition()).cell_extent_xz
		}

		pub fn placement_cells(&self) -> Vec<gimme_gen::Cell> {
			self.extent.subdivide_xz(self.cell_extent_xz())
		}

		pub fn placements(&self) -> Vec<GroveCellVariant<ForlornSavannaCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.placements_on(&self.terrain)
		}

		/// Select placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn placements_on(
			&self,
			world: &impl crate::GroveWorldSample,
		) -> Vec<GroveCellVariant<ForlornSavannaCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.grove.assemble(definition()).populate(&self.extent, world)
		}

		pub fn build(&self) -> ForlornSavanna {
			self.build_on(&self.terrain)
		}

		/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> ForlornSavanna {
			ForlornSavanna::from_placements(
				&self.placements_on(world),
				self.grove.noise,
				self.tree_chain_noise,
				&self.extent,
			)
		}
	}

	#[derive(Clone)]
	enum ForlornSavannaKind {
		Rory(RorysHeadTrained),
		Bush(HighBushShoots),
		Storybook(StorybookTree),
	}

	#[derive(Clone)]
	pub struct ForlornSavannaPlant {
		pub placement: Placement,
		kind: ForlornSavannaKind,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	}

	#[derive(Clone, Component)]
	pub struct ForlornSavanna {
		pub plants: Vec<ForlornSavannaPlant>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl ForlornSavanna {
		pub fn from_placements(
			placements: &[GroveCellVariant<ForlornSavannaCell>],
			grove_noise: NoiseParams,
			tree_chain_noise: NoiseParams,
			extent: &GroveExtent,
		) -> Self {
			let plants = placements
				.iter()
				.map(|placed| grow_plant(placed, grove_noise, tree_chain_noise))
				.collect();
			let (structural_center, footprint_radius) = grove_structural_footprint(extent);
			Self { plants, structural_center, footprint_radius, extent: *extent }
		}

		fn nest_plant_chunks(&self, lod_ref: &LodRef) -> Vec<SceneChunk> {
			self.plants
				.iter()
				.map(|plant| match &plant.kind {
					ForlornSavannaKind::Rory(t) => nest_placed_plant_chunk(
						t.clone(),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						lod_ref,
					),
					ForlornSavannaKind::Bush(t) => nest_placed_plant_chunk(
						t.clone(),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						lod_ref,
					),
					ForlornSavannaKind::Storybook(t) => nest_placed_plant_chunk(
						t.clone(),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						lod_ref,
					),
				})
				.collect()
		}

		fn canopy_sites(&self) -> Vec<CanopyProxySite> {
			self.plants
				.iter()
				.filter_map(|plant| {
					let material = &plant.ball_material;
					match &plant.kind {
						ForlornSavannaKind::Rory(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						ForlornSavannaKind::Bush(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						ForlornSavannaKind::Storybook(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
					}
				})
				.collect()
		}
	}

	fn grow_plant(
		placed: &GroveCellVariant<ForlornSavannaCell>,
		grove_noise: NoiseParams,
		tree_chain_noise: NoiseParams,
	) -> ForlornSavannaPlant {
		let build_noise = placement_noise(grove_noise, placed.position);
		let chain_noise = placement_noise(tree_chain_noise, placed.position);
		let stick_seed = chain_noise.seed;
		let canopy_seed = build_noise.seed.wrapping_add(31);
		let stick_material =
			stick_material_from_palette(Some(placed.variant.stick_palette_mix()), stick_seed);
		let ball_material = canopy_ball_material_from_palette(
			Some(placed.variant.canopy_palette_mix()),
			canopy_seed,
		);
		let frond_material =
			frond_material_from_palette(Some(placed.variant.canopy_palette_mix()), canopy_seed);
		let placement =
			Placement::new(placed.position, 0.0).with_scale(Vec3::splat(placed.scale.max(1e-4)));

		let kind = match placed.variant.item() {
			ForlornSavannaItem::Rory(rory) => {
				let geometry = rory.build_with_noise(build_noise);
				let mut params = RorysHeadTrainedParams::default();
				params.geometry = geometry;
				ForlornSavannaKind::Rory(params.build())
			}
			ForlornSavannaItem::HighBush(bush) => {
				let mut shape = bush.build_with_noise(build_noise);
				shape.chain_noise = chain_noise;
				ForlornSavannaKind::Bush(HighBushShootsParams::new(shape).build())
			}
			ForlornSavannaItem::Storybook(story) => {
				let geometry = story.build_with_noise(build_noise);
				let mut params = StorybookTreeParams::default();
				params.geometry = geometry;
				ForlornSavannaKind::Storybook(params.build())
			}
		};

		ForlornSavannaPlant { placement, kind, stick_material, ball_material, frond_material }
	}

	impl VegetationComponents for ForlornSavanna {
		fn stick_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StickNode> {
			Layers::new()
		}

		fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
			match level {
				LodSceneLevel::High | LodSceneLevel::Medium => Layers::new(),
				LodSceneLevel::Low => {
					layers_from_nodes(foliage_low_canopy_balls(self.canopy_sites()))
				}
				LodSceneLevel::UltraLow
				| LodSceneLevel::Distance(_)
				| LodSceneLevel::Resolution(_) => layers_from_nodes(foliage_ultra_low_merged_balls(
					&self.canopy_sites(),
					ULTRA_LOW_CANOPY_BIN_METERS,
				)),
			}
		}

		fn structural_lod(&self) -> Option<StructuralLod> {
			Some(StructuralLod::new(self.structural_center, self.footprint_radius).with_factors(
				FORLORN_SAVANNA_STRUCTURAL_HIGH_FACTOR,
				FORLORN_SAVANNA_STRUCTURAL_MEDIUM_FACTOR,
				FORLORN_SAVANNA_STRUCTURAL_LOW_FACTOR,
			))
		}
	}

	impl LodScene for ForlornSavanna {
		fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
			self.structural_lod()
				.map(|band| grove_lod_level(band, lod_ref))
				.unwrap_or(LodSceneLevel::High)
		}

		fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
			self.structural_lod()
				.map(|band| grove_lod_status(band, lod_ref))
				.unwrap_or(LodSceneStatus::Unchanged)
		}

		fn scene_lod_culls(&self, lod_ref: &LodRef, _current: LodSceneLevel) -> LodSceneCulls {
			self.structural_lod()
				.map(|band| grove_lod_culls(band, lod_ref))
				.unwrap_or(LodSceneCulls::None)
		}

		fn scene_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
			match grove_detail_level(level) {
				Some(_) => chico_vegetation_components::scene_children(Vec::new()),
				None => {
					let mut children: Vec<Box<dyn Scene>> = Vec::new();
					chico_vegetation_components::append_component_scenes(
						self,
						lod_ref,
						level,
						&mut children,
					);
					chico_vegetation_components::scene_children(children)
				}
			}
		}

		fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
			woody_grove_scene_chunks(level, lod_ref, self.nest_plant_chunks(lod_ref), self)
		}

		fn scene_bounds(&self) -> Aabb3d {
			self.structural_lod()
				.map(|p| p.footprint_aabb())
				.unwrap_or_else(|| chico_vegetation_components::vegetation_bounds(self))
		}

		fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
			lod_host_scene_pending(self.scene_lod_level(lod_ref), self.scene_bounds())
		}
	}
}

#[cfg(feature = "render")]
pub use vc::{
	ForlornSavanna, ForlornSavannaParams, ForlornSavannaPlant,
	FORLORN_SAVANNA_STRUCTURAL_HIGH_FACTOR, FORLORN_SAVANNA_STRUCTURAL_LOW_FACTOR,
	FORLORN_SAVANNA_STRUCTURAL_MEDIUM_FACTOR,
};

#[cfg(test)]
mod tests {
	use super::*;
	use crate::grove::{
		FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent,
	};
	use anyhow::Result;
	use bevy_math::Vec3;
	use gimme_gen::Cell;
	use procedural_common::NoiseParams;

	#[test]
	fn distribution_matches_rfc_order_and_weights() -> Result<()> {
		let dist = ForlornSavannaCell::distribution();
		assert_eq!(dist.len(), 4);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 30.0);
		assert_eq!(dist.buckets[1].item, Some(ForlornSavannaCell::SavannaRory));
		assert_eq!(dist.buckets[1].weight, 3.0);
		assert_eq!(dist.buckets[2].item, Some(ForlornSavannaCell::AcaciaHighBush));
		assert_eq!(dist.buckets[2].weight, 2.0);
		assert_eq!(dist.buckets[3].item, Some(ForlornSavannaCell::RareSavannaStorybook));
		assert_eq!(dist.buckets[3].weight, 0.2);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = ForlornSavannaCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.06..=0.20).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let ForlornSavannaItem::Rory(rory) = ForlornSavannaCell::SavannaRory.item() else {
			anyhow::bail!("expected rory item");
		};
		assert_eq!(rory.height, UnitRange::new(5.0, 30.0));
		assert_eq!(rory.canopy_spread, UnitRange::new(3.0, 12.0));
		assert_eq!(rory.canopy_density, SPARSE_CANOPY_DENSITY);

		let ForlornSavannaItem::HighBush(bush) = ForlornSavannaCell::AcaciaHighBush.item() else {
			anyhow::bail!("expected high bush item");
		};
		assert_eq!(bush.height, UnitRange::new(5.0, 10.0));

		let ForlornSavannaItem::Storybook(story) = ForlornSavannaCell::RareSavannaStorybook.item()
		else {
			anyhow::bail!("expected storybook item");
		};
		assert_eq!(story.height, UnitRange::new(10.0, 20.0));
		assert_eq!(story.canopy_density, SPARSE_CANOPY_DENSITY);
		Ok(())
	}

	#[test]
	fn placement_constraints_match_rfc() -> Result<()> {
		let dist = ForlornSavannaCell::distribution();
		let rory = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(ForlornSavannaCell::SavannaRory))
			.ok_or_else(|| anyhow::anyhow!("missing rory bucket"))?;
		assert_eq!(rory.constraints.elevation.start, 0.0);
		assert_eq!(rory.constraints.elevation.end, 1.0);
		assert_eq!(rory.constraints.steepness.end, 0.58);

		let high_bush = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(ForlornSavannaCell::AcaciaHighBush))
			.ok_or_else(|| anyhow::anyhow!("missing high bush bucket"))?;
		assert_eq!(high_bush.constraints.elevation.end, 1.0);
		assert_eq!(high_bush.constraints.steepness.end, 0.64);

		let storybook = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(ForlornSavannaCell::RareSavannaStorybook))
			.ok_or_else(|| anyhow::anyhow!("missing storybook bucket"))?;
		assert_eq!(storybook.constraints.elevation.start, 0.0);
		assert_eq!(storybook.constraints.steepness.end, 0.50);
		Ok(())
	}

	#[test]
	#[ignore = "placement constraints deferred to forest-layer normalization"]
	fn steep_slope_rejects_rory_but_allows_high_bush() -> Result<()> {
		let prepared = ForlornSavannaCell::distribution().prepare(
			0.0,
			0.0,
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let terrain = FlatTerrainSample { elevation: 0.40, steepness: 0.60 };
		let bush_outcome = prepared.select_from(
			5,
			Vec3::new(5.0, 0.40, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match bush_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, ForlornSavannaCell::AcaciaHighBush);
			}
			other => anyhow::bail!("expected AcaciaHighBush on moderate slope, got {other:?}"),
		}
		let rory_outcome = prepared.select_from(
			1,
			Vec3::new(5.0, 0.40, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match rory_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_ne!(variant, ForlornSavannaCell::SavannaRory);
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			ForlornSavannaCell::SavannaRory,
			ForlornSavannaCell::AcaciaHighBush,
			ForlornSavannaCell::RareSavannaStorybook,
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
	fn populated_grove_is_deterministic_and_non_empty() -> Result<()> {
		let noise = crate::grove::GroveFrontend::default().noise;
		let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(300.0, 1.0, 300.0));
		let terrain = FlatTerrainSample { elevation: 0.40, steepness: 0.20 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
