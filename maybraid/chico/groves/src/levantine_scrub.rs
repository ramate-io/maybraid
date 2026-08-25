//! Levantine Scrub — well-known dry Mediterranean scrub understory grove
//! ([RFC-183 §3.4.5.8], [#320](https://github.com/ramate-io/maybraid/issues/320)).
//!
//! Mixes Rory's Head-trained forms, small Vase Trees, Common High Bush scrub mass, Penmarch Torch
//! accents, occasional small Braid Oak forms, and Simpleman's Hedge bands. Forest-layer attachment
//! remains a follow-up.

use std::ops::RangeInclusive;

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

/// RFC `projection_count: Moderate` — dry high-bush varietal.
const MODERATE_PROJECTION_RADIAL: UnitRange = UnitRange::new(0.32, 0.48);
const MODERATE_PROJECTION_VERTICAL: UnitRange = UnitRange::new(0.58, 0.78);

/// RFC `density: Moderate` for hedge bands.
const MODERATE_HEDGE_DENSITY: UnitRange = UnitRange::new(0.40, 0.60);

/// Authored Levantine Scrub grove definition.
///
/// Cell footprint sits at the RFC midpoint (`5.75` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<LevantineScrubCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(5.75),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-5.75, 5.75),
		),
		distribution: LevantineScrubCell::distribution(),
	}
}

/// Ordered scrub varietals ([RFC-183 §3.4.5.8]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevantineScrubCell {
	DryRoryHeadTrained,
	SmallVaseTree,
	DryHighBush,
	SmallPenmarchTorch,
	RedOliveTorch,
	SmallBraidOak,
	ScrubHedge,
}

/// Typed authored geometry for one scrub varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LevantineScrubItem {
	RoryHead(&'static LevantineScrubRoryHead),
	VaseTree(&'static LevantineScrubVaseTree),
	Bush(&'static LevantineScrubBush),
	PenmarchTorch(&'static LevantineScrubTorch),
	BraidOak(&'static LevantineScrubBraidOak),
	Hedge(&'static LevantineScrubHedge),
}

/// Authored geometry ranges for one Rory's Head-trained form.
#[derive(Debug, Clone, PartialEq)]
pub struct LevantineScrubRoryHead {
	pub height: UnitRange,
	/// World-space stalk base radius (RFC `0.030 × H`).
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse canopy density at render time.
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one small Vase Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct LevantineScrubVaseTree {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Common High Bush shrub.
#[derive(Debug, Clone, PartialEq)]
pub struct LevantineScrubBush {
	pub height: UnitRange,
	pub shoot_count: RangeInclusive<u32>,
	pub branch_depth: RangeInclusive<u32>,
	pub radial_strength: UnitRange,
	pub vertical_bias: UnitRange,
	pub leaf_radius: UnitRange,
}

/// Authored geometry ranges for one Penmarch Torch form.
#[derive(Debug, Clone, PartialEq)]
pub struct LevantineScrubTorch {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one small Braid Oak form.
#[derive(Debug, Clone, PartialEq)]
pub struct LevantineScrubBraidOak {
	pub height: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse..moderate canopy density at render time.
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Simpleman's Hedge band.
#[derive(Debug, Clone, PartialEq)]
pub struct LevantineScrubHedge {
	pub height: UnitRange,
	pub width: UnitRange,
	pub density: UnitRange,
}

const DRY_RORY_HEAD: LevantineScrubRoryHead = LevantineScrubRoryHead {
	height: UnitRange::new(1.20, 3.00),
	stalk_radius: UnitRange::new(0.036, 0.090),
	canopy_spread: UnitRange::new(0.80, 2.20),
	canopy_density: UnitRange::new(0.0, 0.35),
};

const SMALL_VASE_TREE: LevantineScrubVaseTree = LevantineScrubVaseTree {
	height: UnitRange::new(1.20, 3.00),
	stalk_radius: UnitRange::new(0.036, 0.090),
	canopy_spread: UnitRange::new(0.70, 1.80),
	canopy_density: UnitRange::new(0.35, 0.65),
};

const DRY_HIGH_BUSH: LevantineScrubBush = LevantineScrubBush {
	height: UnitRange::new(1.00, 2.50),
	shoot_count: 7..=11,
	branch_depth: 2..=4,
	radial_strength: MODERATE_PROJECTION_RADIAL,
	vertical_bias: MODERATE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.05, 0.11),
};

const SMALL_PENMARCH_TORCH: LevantineScrubTorch = LevantineScrubTorch {
	height: UnitRange::new(1.40, 3.20),
	stalk_radius: UnitRange::new(0.042, 0.096),
	canopy_spread: UnitRange::new(0.50, 1.30),
	canopy_density: UnitRange::new(0.35, 0.65),
};

const RED_OLIVE_TORCH: LevantineScrubTorch = LevantineScrubTorch {
	height: UnitRange::new(1.60, 3.40),
	stalk_radius: UnitRange::new(0.048, 0.102),
	canopy_spread: UnitRange::new(0.55, 1.35),
	canopy_density: UnitRange::new(0.0, 0.35),
};

const SMALL_BRAID_OAK: LevantineScrubBraidOak = LevantineScrubBraidOak {
	height: UnitRange::new(2.00, 5.50),
	canopy_spread: UnitRange::new(1.20, 3.00),
	canopy_density: UnitRange::new(0.35, 0.65),
};

const SCRUB_HEDGE: LevantineScrubHedge = LevantineScrubHedge {
	height: UnitRange::new(0.80, 1.60),
	width: UnitRange::new(0.70, 1.80),
	density: MODERATE_HEDGE_DENSITY,
};

const DRY_RORY_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_bark", "gray_brown"),
	PaletteSlot::new("vine_bark", "olive_brown"),
]);

const DRY_RORY_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "dry_green"),
	PaletteSlot::new("silver_green", "pale_green"),
	PaletteSlot::new("dark_green", "yellow_green"),
]);

const VASE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("ornamental_bark", "gray_brown"),
	PaletteSlot::new("dry_bark", "tan_brown"),
]);

const VASE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "light_green"),
	PaletteSlot::new("dry_green", "flower_white"),
	PaletteSlot::new("dark_green", "silver_green"),
]);

const DRY_BUSH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_bark", "tan_brown"),
	PaletteSlot::new("gray_brown", "straw_brown"),
]);

const DRY_BUSH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "dry_green"),
	PaletteSlot::new("scrub_green", "tan_green"),
	PaletteSlot::new("pale_green", "yellow_green"),
]);

const PENMARCH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_bark", "dark_bark"),
	PaletteSlot::new("ornamental_bark", "gray_brown"),
]);

const PENMARCH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "olive_green"),
	PaletteSlot::new("dry_green", "light_green"),
	PaletteSlot::new("flower_yellow", "pale_green"),
]);

const RED_OLIVE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("copper_red", "orange_bark"),
	PaletteSlot::new("red_brown", "dark_bark"),
]);

const RED_OLIVE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "silver_green"),
	PaletteSlot::new("flower_yellow", "light_green"),
	PaletteSlot::new("dark_green", "pale_green"),
]);

const BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_bark", "gray_brown"),
	PaletteSlot::new("olive_brown", "tan_brown"),
]);

const BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "dry_green"),
	PaletteSlot::new("silver_green", "pale_green"),
	PaletteSlot::new("dark_green", "yellow_green"),
]);

const SCRUB_HEDGE_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("hedge_green", "olive_green"),
	PaletteSlot::new("dry_green", "pale_green"),
	PaletteSlot::new("flower_white", "leaf_green"),
]);

impl LevantineScrubCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `5.45`; the `None` weight of `11.0` puts the placed share at
	/// `5.45 / 16.45 ≈ 0.33`, mid RFC `DENSITY_RANGE` (`0.18..0.48`).
	pub fn distribution() -> GroveDistribution<Self> {
		let dry_rory =
			PlacementConstraints::new(UnitRange::new(0.05, 0.70), UnitRange::new(0.0, 0.70));
		let vase = PlacementConstraints::new(UnitRange::new(0.05, 0.65), UnitRange::new(0.0, 0.52));
		let bush = PlacementConstraints::new(UnitRange::new(0.00, 0.72), UnitRange::new(0.0, 0.65));
		let penmarch =
			PlacementConstraints::new(UnitRange::new(0.10, 0.70), UnitRange::new(0.0, 0.64));
		let red_olive =
			PlacementConstraints::new(UnitRange::new(0.10, 0.68), UnitRange::new(0.0, 0.60));
		let braid_oak =
			PlacementConstraints::new(UnitRange::new(0.08, 0.75), UnitRange::new(0.0, 0.68));
		let hedge =
			PlacementConstraints::new(UnitRange::new(0.00, 0.65), UnitRange::new(0.0, 0.35));
		GroveDistribution::new(vec![
			GroveBucket::none(11.0),
			GroveBucket::placed(1.2, dry_rory, Self::DryRoryHeadTrained),
			GroveBucket::placed(0.70, vase, Self::SmallVaseTree),
			GroveBucket::placed(2.0, bush, Self::DryHighBush),
			GroveBucket::placed(0.45, penmarch, Self::SmallPenmarchTorch),
			GroveBucket::placed(0.25, red_olive, Self::RedOliveTorch),
			GroveBucket::placed(0.35, braid_oak, Self::SmallBraidOak),
			GroveBucket::placed(0.50, hedge, Self::ScrubHedge),
		])
	}

	pub fn item(self) -> LevantineScrubItem {
		match self {
			Self::DryRoryHeadTrained => LevantineScrubItem::RoryHead(&DRY_RORY_HEAD),
			Self::SmallVaseTree => LevantineScrubItem::VaseTree(&SMALL_VASE_TREE),
			Self::DryHighBush => LevantineScrubItem::Bush(&DRY_HIGH_BUSH),
			Self::SmallPenmarchTorch => LevantineScrubItem::PenmarchTorch(&SMALL_PENMARCH_TORCH),
			Self::RedOliveTorch => LevantineScrubItem::PenmarchTorch(&RED_OLIVE_TORCH),
			Self::SmallBraidOak => LevantineScrubItem::BraidOak(&SMALL_BRAID_OAK),
			Self::ScrubHedge => LevantineScrubItem::Hedge(&SCRUB_HEDGE),
		}
	}

	pub fn stick_palette_mix(self) -> Option<PaletteMix> {
		match self {
			Self::DryRoryHeadTrained => Some(DRY_RORY_STICK_MIX),
			Self::SmallVaseTree => Some(VASE_STICK_MIX),
			Self::DryHighBush => Some(DRY_BUSH_STICK_MIX),
			Self::SmallPenmarchTorch => Some(PENMARCH_STICK_MIX),
			Self::RedOliveTorch => Some(RED_OLIVE_STICK_MIX),
			Self::SmallBraidOak => Some(BRAID_OAK_STICK_MIX),
			Self::ScrubHedge => None,
		}
	}

	pub fn canopy_palette_mix(self) -> Option<PaletteMix> {
		match self {
			Self::DryRoryHeadTrained => Some(DRY_RORY_CANOPY_MIX),
			Self::SmallVaseTree => Some(VASE_CANOPY_MIX),
			Self::DryHighBush => Some(DRY_BUSH_CANOPY_MIX),
			Self::SmallPenmarchTorch => Some(PENMARCH_CANOPY_MIX),
			Self::RedOliveTorch => Some(RED_OLIVE_CANOPY_MIX),
			Self::SmallBraidOak => Some(BRAID_OAK_CANOPY_MIX),
			Self::ScrubHedge => None,
		}
	}

	pub fn palette_mix(self) -> Option<PaletteMix> {
		match self {
			Self::ScrubHedge => Some(SCRUB_HEDGE_MIX),
			_ => None,
		}
	}
}

#[cfg(feature = "render")]
mod vc {
	use std::sync::Arc;

	use bevy::math::bounding::Aabb3d;
	use bevy::prelude::*;
	use bevy::scene::prelude::Scene;
	use chico_sbs_trees::{
		BraidOakTree, HighBushShoots, HighBushShootsParams, PenmarchTorch,
		PenmarchTorchParams, RorysHeadTrained, RorysHeadTrainedParams, SimplemansHedge,
		SimplemansHedgeParams, VaseTree, VaseTreeParams,
	};
	use chico_vegetation_components::{
		flattened_canopy_proxy_chunks, FoliageNode, Layers, Placement, StickNode, StructuralLod,
		VegetationComponents,
	};
	use clap::Args;
	use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
	use lod::lod_ref::LodRef;
	use lod::{lod_host_scene_pending, SceneChunk};
	use material_ref::MaterialRef;
	use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};

	use super::{definition, LevantineScrubCell, LevantineScrubItem};
	use crate::grove::vc_tuft::{patch_variant_index, variant_noise};
	use crate::grove::{
		canopy_ball_material_from_palette, canopy_proxy_site, foliage_low_canopy_balls,
		foliage_ultra_low_merged_balls, frond_material_from_palette, grove_detail_level,
		grove_lod_culls, grove_lod_level, grove_lod_status, grove_structural_footprint,
		layers_from_nodes, nest_flattened_plant_chunk, placement_noise,
		stick_material_from_palette, CanopyProxySite, FlatTerrainSample, GroveCellVariant,
		GroveExtent, GroveFrontend, DEFAULT_GROVE_EXTENT_XZ, ULTRA_LOW_CANOPY_BIN_METERS,
	};

	/// Structural High band (× footprint).
	pub const LEVANTINE_SCRUB_STRUCTURAL_HIGH_FACTOR: f32 = 2.0;
	/// Structural Medium band (× footprint).
	pub const LEVANTINE_SCRUB_STRUCTURAL_MEDIUM_FACTOR: f32 = 5.0;
	/// Structural Low band (× footprint).
	pub const LEVANTINE_SCRUB_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

	/// Authoring / CLI parameters for Levantine Scrub.
	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct LevantineScrubParams {
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

		/// Number of unit-height plant archetypes (`unit_from_num(0..n)`). Caps unique
		/// merged-mesh handles for High/Medium.
		#[arg(long, default_value_t = 100)]
		pub tree_variants: u32,

		#[arg(skip)]
		resolved_placements: Option<Vec<GroveCellVariant<LevantineScrubCell>>>,
	}

	impl Default for LevantineScrubParams {
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
				terrain: FlatTerrainSample { elevation: 0.25, steepness: 0.15 },
				tree_variants: 100,
				resolved_placements: None,
			}
		}
	}

	impl LevantineScrubParams {
		pub fn with_resolved_placements(
			resolved_placements: Vec<GroveCellVariant<LevantineScrubCell>>,
			terrain: FlatTerrainSample,
			tree_chain_noise: NoiseParams,
			stick_surface_noise: NoiseParams,
			leaf_surface_noise: NoiseParams,
		) -> Self {
			Self {
				grove: GroveFrontend::default(),
				tree_chain_noise,
				stick_surface_noise,
				leaf_surface_noise,
				extent: GroveExtent::new(
					Vec3::ZERO,
					Vec3::new(DEFAULT_GROVE_EXTENT_XZ, 1.0, DEFAULT_GROVE_EXTENT_XZ),
				),
				terrain,
				tree_variants: 100,
				resolved_placements: Some(resolved_placements),
			}
		}

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

		pub fn placements(&self) -> Vec<GroveCellVariant<LevantineScrubCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.placements_on(&self.terrain)
		}

		/// Select placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn placements_on(
			&self,
			world: &impl crate::GroveWorldSample,
		) -> Vec<GroveCellVariant<LevantineScrubCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.grove.assemble(definition()).populate(&self.extent, world)
		}

		pub fn build(&self) -> LevantineScrub {
			self.build_on(&self.terrain)
		}

		/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> LevantineScrub {
			LevantineScrub::from_placements(
				&self.placements_on(world),
				self.grove.noise,
				self.tree_chain_noise,
				self.stick_surface_noise,
				&self.extent,
				self.tree_variants,
			)
		}
	}

	#[derive(Clone)]
	enum LevantineScrubKind {
		Rory(Arc<RorysHeadTrained>),
		Vase(Arc<VaseTree>),
		Bush(Arc<HighBushShoots>),
		Torch(Arc<PenmarchTorch>),
		Oak(Arc<BraidOakTree>),
		Hedge(Arc<SimplemansHedge>),
	}

	/// One scrub plant with placement and palette materials.
	#[derive(Clone)]
	pub struct LevantineScrubPlant {
		pub placement: Placement,
		kind: LevantineScrubKind,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	}

	/// Built Levantine Scrub grove (`LodScene` nests plant `ComponentsOnly` hosts).
	#[derive(Clone, Component)]
	pub struct LevantineScrub {
		pub plants: Arc<[LevantineScrubPlant]>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl LevantineScrub {
		pub fn from_placements(
			placements: &[GroveCellVariant<LevantineScrubCell>],
			grove_noise: NoiseParams,
			tree_chain_noise: NoiseParams,
			stick_surface_noise: NoiseParams,
			extent: &GroveExtent,
			tree_variants: u32,
		) -> Self {
			let plants: Arc<[LevantineScrubPlant]> = placements
				.iter()
				.map(|placed| {
					grow_plant(
						placed,
						grove_noise,
						tree_chain_noise,
						stick_surface_noise,
						tree_variants,
					)
				})
				.collect::<Vec<_>>()
				.into();
			let (structural_center, footprint_radius) = grove_structural_footprint(extent);
			Self { plants, structural_center, footprint_radius, extent: *extent }
		}

		fn nest_plant_chunks(&self, lod_ref: &LodRef) -> Vec<SceneChunk> {
			if self.plants.is_empty() {
				return Vec::new();
			}
			let n = self.plants.len();
			let plants = Arc::clone(&self.plants);
			let prev = *lod_ref.previous_transform;
			let curr = *lod_ref.current_transform;
			let bounds = *lod_ref.bounds;
			let entity = lod_ref.entity;
			let mut index = 0usize;
			vec![SceneChunk::lazy(n as u32, n, move || {
				if index >= plants.len() {
					return None;
				}
				let plant = &plants[index];
				index += 1;
				let plant_lod = LodRef {
					entity,
					previous_transform: &prev,
					current_transform: &curr,
					bounds: &bounds,
				};
				Some(match &plant.kind {
					LevantineScrubKind::Rory(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					LevantineScrubKind::Vase(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					LevantineScrubKind::Bush(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					LevantineScrubKind::Torch(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					LevantineScrubKind::Oak(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					LevantineScrubKind::Hedge(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
				})
			})]
		}

		fn canopy_sites(&self) -> Vec<CanopyProxySite> {
			self.plants
				.iter()
				.filter_map(|plant| {
					let material = &plant.ball_material;
					match &plant.kind {
						LevantineScrubKind::Rory(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						LevantineScrubKind::Vase(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						LevantineScrubKind::Bush(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						LevantineScrubKind::Torch(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						LevantineScrubKind::Oak(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						LevantineScrubKind::Hedge(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
					}
				})
				.collect()
		}
	}

	fn grow_plant(
		placed: &GroveCellVariant<LevantineScrubCell>,
		grove_noise: NoiseParams,
		tree_chain_noise: NoiseParams,
		_stick_surface_noise: NoiseParams,
		tree_variants: u32,
	) -> LevantineScrubPlant {
		let variant = patch_variant_index(placed.position, tree_variants);
		let build_noise = variant_noise(grove_noise, variant);
		let chain_noise = variant_noise(tree_chain_noise, variant);
		let palette_noise = placement_noise(grove_noise, placed.position);
		let stick_seed = palette_noise.seed;
		let canopy_seed = palette_noise.seed.wrapping_add(31);
		let stick_material =
			stick_material_from_palette(placed.variant.stick_palette_mix(), stick_seed);
		let canopy_palette =
			placed.variant.canopy_palette_mix().or_else(|| placed.variant.palette_mix());
		let ball_material = canopy_ball_material_from_palette(canopy_palette, canopy_seed);
		let frond_material = frond_material_from_palette(canopy_palette, canopy_seed);

		match placed.variant.item() {
			LevantineScrubItem::RoryHead(rory) => {
				let geometry = rory.build_with_noise(build_noise);
				let mut params = RorysHeadTrainedParams::default();
				params.geometry = geometry;
				let (unit_params, world_size) = params.into_unit_from_num(variant);
				LevantineScrubPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: LevantineScrubKind::Rory(Arc::new(unit_params.build())),
					stick_material,
					ball_material,
					frond_material,
				}
			}
			LevantineScrubItem::VaseTree(vase) => {
				let geometry = vase.build_with_noise(build_noise);
				let mut params = VaseTreeParams::default();
				params.geometry = geometry;
				let (unit_params, world_size) = params.into_unit_from_num(variant);
				LevantineScrubPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: LevantineScrubKind::Vase(Arc::new(unit_params.build())),
					stick_material,
					ball_material,
					frond_material,
				}
			}
			LevantineScrubItem::Bush(bush) => {
				let mut shape = bush.build_with_noise(build_noise);
				shape.chain_noise = chain_noise;
				let (unit_params, world_size) =
					HighBushShootsParams::new(shape).into_unit_from_num(variant);
				LevantineScrubPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: LevantineScrubKind::Bush(Arc::new(unit_params.build())),
					stick_material,
					ball_material,
					frond_material,
				}
			}
			LevantineScrubItem::PenmarchTorch(torch) => {
				let geometry = torch.build_with_noise(build_noise);
				let mut params = PenmarchTorchParams::default();
				params.geometry = geometry;
				let (unit_params, world_size) = params.into_unit_from_num(variant);
				LevantineScrubPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: LevantineScrubKind::Torch(Arc::new(unit_params.build())),
					stick_material,
					ball_material,
					frond_material,
				}
			}
			LevantineScrubItem::BraidOak(oak) => {
				let world_size = oak.build_with_noise(build_noise).height();
				LevantineScrubPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: LevantineScrubKind::Oak(Arc::new(BraidOakTree::unit_from_num(variant))),
					stick_material,
					ball_material,
					frond_material,
				}
			}
			LevantineScrubItem::Hedge(hedge) => {
				let samples = hedge.build_with_noise(build_noise);
				let (unit_params, world_size) = SimplemansHedgeParams::new(
					samples.height,
					samples.footprint_xz,
					samples.density,
					samples.seed,
				)
				.into_unit_from_num(variant);
				LevantineScrubPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: LevantineScrubKind::Hedge(Arc::new(unit_params.build())),
					stick_material,
					ball_material,
					frond_material,
				}
			}
		}
	}

	impl VegetationComponents for LevantineScrub {
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
				LEVANTINE_SCRUB_STRUCTURAL_HIGH_FACTOR,
				LEVANTINE_SCRUB_STRUCTURAL_MEDIUM_FACTOR,
				LEVANTINE_SCRUB_STRUCTURAL_LOW_FACTOR,
			))
		}
	}

	impl LodScene for LevantineScrub {
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
			match grove_detail_level(level) {
				Some(_) => {
					let chunks = self.nest_plant_chunks(lod_ref);
					if chunks.is_empty() {
						SceneChunk::primitive(chico_vegetation_components::scene_children(
							Vec::new(),
						))
					} else {
						SceneChunk::chunks(chunks)
					}
				}
				None => flattened_canopy_proxy_chunks(self, lod_ref, level),
			}
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

	#[cfg(test)]
	mod tests {
		use super::*;
		use anyhow::Result;

		fn small_grove() -> LevantineScrub {
			LevantineScrubParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0)))
				.build()
		}

		fn plant_height(plant: &LevantineScrubPlant) -> f32 {
			match &plant.kind {
				LevantineScrubKind::Rory(t) => t.geometry.height(),
				LevantineScrubKind::Vase(t) => t.geometry.height(),
				LevantineScrubKind::Bush(t) => t.shape.height,
				LevantineScrubKind::Torch(t) => t.geometry.height(),
				LevantineScrubKind::Oak(t) => t.geometry.height(),
				LevantineScrubKind::Hedge(t) => t.height,
			}
		}

		fn plant_seed(plant: &LevantineScrubPlant) -> i32 {
			match &plant.kind {
				LevantineScrubKind::Rory(t) => t.geometry.canopy_noise.seed,
				LevantineScrubKind::Vase(t) => t.geometry.canopy_noise.seed,
				LevantineScrubKind::Bush(t) => t.shape.chain_noise.seed,
				LevantineScrubKind::Torch(t) => t.geometry.canopy_noise.seed,
				LevantineScrubKind::Oak(t) => t.geometry.canopy_noise.seed,
				LevantineScrubKind::Hedge(t) => t.seed as i32,
			}
		}

		#[test]
		fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
			let grove = small_grove();
			assert!(!grove.plants.is_empty(), "expected placed levantine scrub plants");

			assert_eq!(grove.stick_nodes_for_level(LodSceneLevel::High).len(), 0);
			assert_eq!(grove.foliage_nodes_for_level(LodSceneLevel::High).len(), 0);
			assert_eq!(grove.stick_nodes_for_level(LodSceneLevel::Medium).len(), 0);
			assert_eq!(grove.foliage_nodes_for_level(LodSceneLevel::Medium).len(), 0);

			let camera = Transform::from_translation(Vec3::new(40.0, 2.0, 40.0));
			let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
			let lod_ref = LodRef {
				entity: Entity::PLACEHOLDER,
				previous_transform: &camera,
				current_transform: &camera,
				bounds: &bounds,
			};
			let high = grove.scene_chunks_with_level(&lod_ref, LodSceneLevel::High);
			let lod::SceneChunk::SubChunks(parts) = high else {
				anyhow::bail!("High levantine scrub should wrap plant chunks");
			};
			assert_eq!(parts.len(), 1, "expected one lazy plant producer");
			let lod::SceneChunk::Lazy { remaining_primitives, remaining_weight, .. } = &parts[0]
			else {
				anyhow::bail!("High levantine scrub plants should be SceneChunk::Lazy");
			};
			assert_eq!(*remaining_primitives, grove.plants.len());
			assert_eq!(*remaining_weight as usize, grove.plants.len());

			assert_eq!(grove.stick_nodes_for_level(LodSceneLevel::Low).len(), 0);
			let low_foliage = grove.foliage_nodes_for_level(LodSceneLevel::Low).len();
			assert_eq!(low_foliage, grove.plants.len());
			assert!(grove.foliage_nodes_for_level(LodSceneLevel::UltraLow).len() <= low_foliage);
			let lod::SceneChunk::Primitive { weight, .. } =
				grove.scene_chunks_with_level(&lod_ref, LodSceneLevel::Low)
			else {
				anyhow::bail!("Low levantine scrub should emit one flattened canopy collection");
			};
			assert_eq!(weight, chico_vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT);
			Ok(())
		}

		#[test]
		fn tree_variants_quantize_archetypes() -> Result<()> {
			use std::collections::HashSet;

			let mut params = LevantineScrubParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0)));
			params.tree_variants = 4;
			let grove = params.build();
			assert!(!grove.plants.is_empty(), "expected placed levantine scrub plants");
			for plant in grove.plants.iter() {
				assert!(
					(plant_height(plant) - 1.0).abs() < 1e-4,
					"expected unit height, got {}",
					plant_height(plant)
				);
			}
			let seeds: HashSet<i32> = grove.plants.iter().map(plant_seed).collect();
			assert!(seeds.len() <= 4, "expected <=4 unique unit seeds, got {}", seeds.len());
			Ok(())
		}
	}
}

#[cfg(feature = "render")]
pub use vc::{
	LevantineScrub, LevantineScrubParams, LevantineScrubPlant,
	LEVANTINE_SCRUB_STRUCTURAL_HIGH_FACTOR, LEVANTINE_SCRUB_STRUCTURAL_LOW_FACTOR,
	LEVANTINE_SCRUB_STRUCTURAL_MEDIUM_FACTOR,
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
		let dist = LevantineScrubCell::distribution();
		assert_eq!(dist.len(), 8);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 11.0);
		assert_eq!(dist.buckets[1].item, Some(LevantineScrubCell::DryRoryHeadTrained));
		assert_eq!(dist.buckets[1].weight, 1.2);
		assert_eq!(dist.buckets[2].item, Some(LevantineScrubCell::SmallVaseTree));
		assert_eq!(dist.buckets[2].weight, 0.70);
		assert_eq!(dist.buckets[3].item, Some(LevantineScrubCell::DryHighBush));
		assert_eq!(dist.buckets[3].weight, 2.0);
		assert_eq!(dist.buckets[4].item, Some(LevantineScrubCell::SmallPenmarchTorch));
		assert_eq!(dist.buckets[4].weight, 0.45);
		assert_eq!(dist.buckets[5].item, Some(LevantineScrubCell::RedOliveTorch));
		assert_eq!(dist.buckets[5].weight, 0.25);
		assert_eq!(dist.buckets[6].item, Some(LevantineScrubCell::SmallBraidOak));
		assert_eq!(dist.buckets[6].weight, 0.35);
		assert_eq!(dist.buckets[7].item, Some(LevantineScrubCell::ScrubHedge));
		assert_eq!(dist.buckets[7].weight, 0.50);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = LevantineScrubCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.18..=0.48).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn scrub_geometry_follows_authored_bands() -> Result<()> {
		let LevantineScrubItem::RoryHead(rory) = LevantineScrubCell::DryRoryHeadTrained.item()
		else {
			anyhow::bail!("expected dry rory item");
		};
		assert!(rory.canopy_density.end <= 0.35);

		let LevantineScrubItem::VaseTree(vase) = LevantineScrubCell::SmallVaseTree.item() else {
			anyhow::bail!("expected vase item");
		};
		assert!(vase.height.end <= 3.00);

		let LevantineScrubItem::Bush(bush) = LevantineScrubCell::DryHighBush.item() else {
			anyhow::bail!("expected bush item");
		};
		assert_eq!(bush.shoot_count, 7..=11);

		let LevantineScrubItem::Hedge(hedge) = LevantineScrubCell::ScrubHedge.item() else {
			anyhow::bail!("expected hedge item");
		};
		assert!(hedge.width.end <= 1.80);
		Ok(())
	}

	#[test]
	fn hedge_accepts_gentle_slopes_only() -> Result<()> {
		let prepared = LevantineScrubCell::distribution().prepare(
			0.0,
			0.0,
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.30 };
		let outcome = prepared.select_from(
			7,
			Vec3::new(5.0, 0.25, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, LevantineScrubCell::ScrubHedge);
			}
			other => anyhow::bail!("expected ScrubHedge on gentle slope, got {other:?}"),
		}
		Ok(())
	}

	#[test]
	#[ignore = "placement constraints deferred to forest-layer normalization"]
	fn steep_slope_rejects_all_placed_buckets() -> Result<()> {
		let prepared = LevantineScrubCell::distribution().prepare(
			0.0,
			0.0,
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.69 };
		let outcome = prepared.select_from(
			3,
			Vec3::new(5.0, 0.25, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match outcome {
			GroveCellOutcome::Empty { .. } => {}
			other => anyhow::bail!("expected Empty on steep slope, got {other:?}"),
		}
		Ok(())
	}

	#[test]
	fn bush_fits_moderate_slope_from_high_bush_bucket() -> Result<()> {
		let prepared = LevantineScrubCell::distribution().prepare(
			0.0,
			0.0,
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.62 };
		let outcome = prepared.select_from(
			3,
			Vec3::new(5.0, 0.25, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, LevantineScrubCell::DryHighBush);
			}
			other => anyhow::bail!("expected DryHighBush, got {other:?}"),
		}
		Ok(())
	}

	#[test]
	fn placements_break_the_cell_grid() -> Result<()> {
		let noise = crate::grove::GroveFrontend::default().noise;
		let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0));
		let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.15 };
		let placements = grove.populate(&extent, &terrain);
		assert!(!placements.is_empty());

		let cell = definition().cell_extent_xz.x;
		let off_center = placements
			.iter()
			.filter(|p| {
				let local_x = (p.position.x / cell).fract() - 0.5;
				let local_z = (p.position.z / cell).fract() - 0.5;
				local_x.abs() > 0.25 || local_z.abs() > 0.25
			})
			.count();
		assert!(
			off_center * 2 >= placements.len(),
			"expected at least half of {} placements off cell centers, got {off_center}",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn populated_grove_is_deterministic_and_non_empty() -> Result<()> {
		let grove = Grove::assemble(
			definition(),
			ForestGroveBiases::default(),
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(40.0, 1.0, 40.0));
		let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.15 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
