//! Storyteller's — colorful whimsical Storybook and Braid Oak upper-canopy grove
//! ([RFC-183 §3.4.7.14], [#336](https://github.com/ramate-io/maybraid/issues/336)).
//!
//! Moderate-density color-pop canopy with common storybook, braid-oak, and torch forms. Forest-layer
//! attachment remains a follow-up.

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

/// Sparse sampled canopy-density band ([`0.0`, `0.35`]).
const SPARSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.0, 0.35);
/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);
/// Dense sampled canopy-density band ([`0.50`, `0.85`]).
const DENSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.50, 0.85);

/// Authored Storyteller's grove definition.
///
/// Cell footprint sits at the RFC midpoint (`22.0` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<StorytellersCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(22.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-22.0, 22.0),
		),
		distribution: StorytellersCell::distribution(),
	}
}

/// Ordered storyteller varietals ([RFC-183 §3.4.7.14]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorytellersCell {
	ColorfulStorybook,
	ColorfulBraidOak,
	BrightCanopyStorybook,
	PinkLanternStorybook,
	RedFestivalBraidOak,
	PurpleCrownStorybook,
	BlueMoonStorybook,
	GoldenLanternPenmarch,
	BlueFlameKamakura,
	FestivalTorchTree,
	VioletCanopyBraidOak,
	GoldLeafBraidOak,
	CopperFlameBraidOak,
}

/// Typed authored geometry for one storyteller varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StorytellersItem {
	Storybook(&'static StorytellersStorybook),
	BraidOak(&'static StorytellersBraidOak),
	PenmarchTorch(&'static StorytellersTorch),
	KamakuraTorch(&'static StorytellersTorch),
	TorchTree(&'static StorytellersTorch),
}

/// Authored geometry ranges for one Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct StorytellersStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Braid Oak form.
#[derive(Debug, Clone, PartialEq)]
pub struct StorytellersBraidOak {
	pub height: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one upper-canopy torch form.
#[derive(Debug, Clone, PartialEq)]
pub struct StorytellersTorch {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
}

const COLORFUL_STORYBOOK: StorytellersStorybook = StorytellersStorybook {
	height: UnitRange::new(10.0, 30.0),
	stalk_radius: UnitRange::new(0.24, 0.55),
	canopy_spread: UnitRange::new(3.0, 8.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const COLORFUL_BRAID_OAK: StorytellersBraidOak = StorytellersBraidOak {
	height: UnitRange::new(10.0, 30.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const BRIGHT_CANOPY_STORYBOOK: StorytellersStorybook = StorytellersStorybook {
	height: UnitRange::new(10.0, 26.0),
	stalk_radius: UnitRange::new(0.22, 0.50),
	canopy_spread: UnitRange::new(2.5, 7.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const PINK_LANTERN_STORYBOOK: StorytellersStorybook = StorytellersStorybook {
	height: UnitRange::new(8.0, 18.0),
	stalk_radius: UnitRange::new(0.20, 0.44),
	canopy_spread: UnitRange::new(2.0, 5.5),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const RED_FESTIVAL_BRAID_OAK: StorytellersBraidOak = StorytellersBraidOak {
	height: UnitRange::new(12.0, 24.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const PURPLE_CROWN_STORYBOOK: StorytellersStorybook = StorytellersStorybook {
	height: UnitRange::new(14.0, 30.0),
	stalk_radius: UnitRange::new(0.28, 0.58),
	canopy_spread: UnitRange::new(3.5, 9.0),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const BLUE_MOON_STORYBOOK: StorytellersStorybook = StorytellersStorybook {
	height: UnitRange::new(12.0, 22.0),
	stalk_radius: UnitRange::new(0.22, 0.48),
	canopy_spread: UnitRange::new(2.5, 6.5),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const GOLDEN_LANTERN_PENMARCH: StorytellersTorch = StorytellersTorch {
	height: UnitRange::new(10.0, 26.0),
	stalk_radius: UnitRange::new(0.16, 0.38),
	canopy_spread: UnitRange::new(2.5, 7.0),
};

const BLUE_FLAME_KAMAKURA: StorytellersTorch = StorytellersTorch {
	height: UnitRange::new(12.0, 28.0),
	stalk_radius: UnitRange::new(0.14, 0.36),
	canopy_spread: UnitRange::new(2.0, 6.5),
};

const FESTIVAL_TORCH_TREE: StorytellersTorch = StorytellersTorch {
	height: UnitRange::new(10.0, 24.0),
	stalk_radius: UnitRange::new(0.16, 0.40),
	canopy_spread: UnitRange::new(2.5, 6.0),
};

const VIOLET_CANOPY_BRAID_OAK: StorytellersBraidOak = StorytellersBraidOak {
	height: UnitRange::new(14.0, 28.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const GOLD_LEAF_BRAID_OAK: StorytellersBraidOak = StorytellersBraidOak {
	height: UnitRange::new(10.0, 22.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const COPPER_FLAME_BRAID_OAK: StorytellersBraidOak = StorytellersBraidOak {
	height: UnitRange::new(12.0, 26.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const COLORFUL_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("warm_bark", "red_brown"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const COLORFUL_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("storybook_green", "gold_green"),
	PaletteSlot::new("rose_leaf", "fresh_green"),
]);

const COLORFUL_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("red_oak_bark", "copper_red"),
	PaletteSlot::new("oak_bark", "dark_bark"),
]);

const COLORFUL_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "gold_green"),
	PaletteSlot::new("copper_leaf", "fresh_green"),
]);

const BRIGHT_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("purple_brown", "red_brown"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const BRIGHT_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("gold_leaf", "fresh_green"),
	PaletteSlot::new("rose_leaf", "light_green"),
]);

const PINK_LANTERN_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("warm_bark", "purple_brown"),
	PaletteSlot::new("red_brown", "dark_bark"),
]);

const PINK_LANTERN_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("hot_pink", "rose_leaf"),
	PaletteSlot::new("fresh_green", "pink_bloom"),
]);

const RED_FESTIVAL_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("red_oak_bark", "bright_red_bark"),
	PaletteSlot::new("copper_red", "dark_bark"),
]);

const RED_FESTIVAL_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("red_leaf", "copper_leaf"),
	PaletteSlot::new("gold_leaf", "fresh_green"),
]);

const PURPLE_CROWN_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("purple_brown", "dark_bark"),
	PaletteSlot::new("red_brown", "gray_brown"),
]);

const PURPLE_CROWN_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("violet_leaf", "purple_leaf"),
	PaletteSlot::new("deep_green", "rose_leaf"),
]);

const BLUE_MOON_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("blue_gray_bark", "purple_brown"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const BLUE_MOON_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("blue_leaf", "cyan_leaf"),
	PaletteSlot::new("deep_blue_green", "fresh_green"),
]);

const GOLDEN_LANTERN_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("warm_bark", "gold"),
	PaletteSlot::new("ornamental_bark", "red_brown"),
]);

const GOLDEN_LANTERN_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("gold_leaf", "flower_yellow"),
	PaletteSlot::new("warm_yellow", "fresh_green"),
]);

const BLUE_FLAME_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("blue_gray_bark", "purple_brown"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const BLUE_FLAME_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("blue_leaf", "cyan_leaf"),
	PaletteSlot::new("deep_blue_green", "violet_leaf"),
]);

const FESTIVAL_TORCH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("ritual_red_bark", "copper_red"),
	PaletteSlot::new("bright_red_bark", "dark_bark"),
]);

const FESTIVAL_TORCH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("hot_pink", "gold_leaf"),
	PaletteSlot::new("flower_yellow", "rose_leaf"),
]);

const VIOLET_CANOPY_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("purple_brown", "blue_gray_bark"),
	PaletteSlot::new("red_brown", "dark_bark"),
]);

const VIOLET_CANOPY_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("violet_leaf", "purple_leaf"),
	PaletteSlot::new("deep_green", "rose_leaf"),
]);

const GOLD_LEAF_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("oak_bark", "warm_bark"),
	PaletteSlot::new("ornamental_bark", "gray_brown"),
]);

const GOLD_LEAF_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("gold_leaf", "gold_green"),
	PaletteSlot::new("warm_yellow", "fresh_green"),
]);

const COPPER_FLAME_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("copper_red", "bright_red_bark"),
	PaletteSlot::new("red_oak_bark", "dark_bark"),
]);

const COPPER_FLAME_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("copper_leaf", "red_leaf"),
	PaletteSlot::new("gold_leaf", "orange_brown"),
]);

impl StorytellersCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `6.28`; the `None` weight of `16.2` puts the placed share at
	/// `6.28 / 22.48 ≈ 0.28`, mid RFC `DENSITY_RANGE` (`0.18..0.38`).
	pub fn distribution() -> GroveDistribution<Self> {
		let colorful_storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.52));
		let colorful_braid =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.48));
		let bright_storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.56));
		let pink_lantern =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.50));
		let red_festival =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.46));
		let purple_crown =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.54));
		let blue_moon =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.58));
		let golden_lantern =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.52));
		let blue_flame =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.54));
		let festival_torch =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.50));
		let violet_canopy =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.50));
		let gold_leaf =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.52));
		let copper_flame =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.48));
		GroveDistribution::new(vec![
			GroveBucket::none(16.2),
			GroveBucket::placed(1.5, colorful_storybook, Self::ColorfulStorybook),
			GroveBucket::placed(1.5, colorful_braid, Self::ColorfulBraidOak),
			GroveBucket::placed(0.75, bright_storybook, Self::BrightCanopyStorybook),
			GroveBucket::placed(0.35, pink_lantern, Self::PinkLanternStorybook),
			GroveBucket::placed(0.30, red_festival, Self::RedFestivalBraidOak),
			GroveBucket::placed(0.25, purple_crown, Self::PurpleCrownStorybook),
			GroveBucket::placed(0.25, blue_moon, Self::BlueMoonStorybook),
			GroveBucket::placed(0.22, golden_lantern, Self::GoldenLanternPenmarch),
			GroveBucket::placed(0.20, blue_flame, Self::BlueFlameKamakura),
			GroveBucket::placed(0.18, festival_torch, Self::FestivalTorchTree),
			GroveBucket::placed(0.28, violet_canopy, Self::VioletCanopyBraidOak),
			GroveBucket::placed(0.26, gold_leaf, Self::GoldLeafBraidOak),
			GroveBucket::placed(0.24, copper_flame, Self::CopperFlameBraidOak),
		])
	}

	pub fn item(self) -> StorytellersItem {
		match self {
			Self::ColorfulStorybook => StorytellersItem::Storybook(&COLORFUL_STORYBOOK),
			Self::ColorfulBraidOak => StorytellersItem::BraidOak(&COLORFUL_BRAID_OAK),
			Self::BrightCanopyStorybook => StorytellersItem::Storybook(&BRIGHT_CANOPY_STORYBOOK),
			Self::PinkLanternStorybook => StorytellersItem::Storybook(&PINK_LANTERN_STORYBOOK),
			Self::RedFestivalBraidOak => StorytellersItem::BraidOak(&RED_FESTIVAL_BRAID_OAK),
			Self::PurpleCrownStorybook => StorytellersItem::Storybook(&PURPLE_CROWN_STORYBOOK),
			Self::BlueMoonStorybook => StorytellersItem::Storybook(&BLUE_MOON_STORYBOOK),
			Self::GoldenLanternPenmarch => {
				StorytellersItem::PenmarchTorch(&GOLDEN_LANTERN_PENMARCH)
			}
			Self::BlueFlameKamakura => StorytellersItem::KamakuraTorch(&BLUE_FLAME_KAMAKURA),
			Self::FestivalTorchTree => StorytellersItem::TorchTree(&FESTIVAL_TORCH_TREE),
			Self::VioletCanopyBraidOak => StorytellersItem::BraidOak(&VIOLET_CANOPY_BRAID_OAK),
			Self::GoldLeafBraidOak => StorytellersItem::BraidOak(&GOLD_LEAF_BRAID_OAK),
			Self::CopperFlameBraidOak => StorytellersItem::BraidOak(&COPPER_FLAME_BRAID_OAK),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::ColorfulStorybook => COLORFUL_STORYBOOK_STICK_MIX,
			Self::ColorfulBraidOak => COLORFUL_BRAID_OAK_STICK_MIX,
			Self::BrightCanopyStorybook => BRIGHT_STORYBOOK_STICK_MIX,
			Self::PinkLanternStorybook => PINK_LANTERN_STICK_MIX,
			Self::RedFestivalBraidOak => RED_FESTIVAL_STICK_MIX,
			Self::PurpleCrownStorybook => PURPLE_CROWN_STICK_MIX,
			Self::BlueMoonStorybook => BLUE_MOON_STICK_MIX,
			Self::GoldenLanternPenmarch => GOLDEN_LANTERN_STICK_MIX,
			Self::BlueFlameKamakura => BLUE_FLAME_STICK_MIX,
			Self::FestivalTorchTree => FESTIVAL_TORCH_STICK_MIX,
			Self::VioletCanopyBraidOak => VIOLET_CANOPY_STICK_MIX,
			Self::GoldLeafBraidOak => GOLD_LEAF_STICK_MIX,
			Self::CopperFlameBraidOak => COPPER_FLAME_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::ColorfulStorybook => COLORFUL_STORYBOOK_CANOPY_MIX,
			Self::ColorfulBraidOak => COLORFUL_BRAID_OAK_CANOPY_MIX,
			Self::BrightCanopyStorybook => BRIGHT_STORYBOOK_CANOPY_MIX,
			Self::PinkLanternStorybook => PINK_LANTERN_CANOPY_MIX,
			Self::RedFestivalBraidOak => RED_FESTIVAL_CANOPY_MIX,
			Self::PurpleCrownStorybook => PURPLE_CROWN_CANOPY_MIX,
			Self::BlueMoonStorybook => BLUE_MOON_CANOPY_MIX,
			Self::GoldenLanternPenmarch => GOLDEN_LANTERN_CANOPY_MIX,
			Self::BlueFlameKamakura => BLUE_FLAME_CANOPY_MIX,
			Self::FestivalTorchTree => FESTIVAL_TORCH_CANOPY_MIX,
			Self::VioletCanopyBraidOak => VIOLET_CANOPY_CANOPY_MIX,
			Self::GoldLeafBraidOak => GOLD_LEAF_CANOPY_MIX,
			Self::CopperFlameBraidOak => COPPER_FLAME_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
mod vc {
	use std::sync::Arc;

	use bevy::math::bounding::Aabb3d;
	use bevy::prelude::*;
	use bevy::scene::prelude::Scene;
	use chico_sbs_geometry::{KamakuraTorchSbs, PenmarchTorchSbs};
	use chico_sbs_trees::{
		BraidOakTree, KamakuraTorch, KamakuraTorchParams, PenmarchTorch, PenmarchTorchParams,
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

	use super::{definition, StorytellersCell, StorytellersItem};
	use crate::grove::vc_tuft::{patch_variant_index, variant_noise};
	use crate::grove::{
		canopy_ball_material_from_palette, canopy_proxy_site, foliage_low_canopy_balls,
		foliage_ultra_low_merged_balls, frond_material_from_palette, grove_detail_level,
		grove_lod_culls, grove_lod_level, grove_lod_status, grove_structural_footprint,
		layers_from_nodes, nest_flattened_plant_chunk, placement_noise,
		stick_material_from_palette, woody_grove_scene_chunks, CanopyProxySite, FlatTerrainSample,
		GroveCellVariant, GroveExtent, GroveFrontend, DEFAULT_GROVE_EXTENT_XZ,
		ULTRA_LOW_CANOPY_BIN_METERS,
	};

	pub const STORYTELLERS_STRUCTURAL_HIGH_FACTOR: f32 = 5.0;
	pub const STORYTELLERS_STRUCTURAL_MEDIUM_FACTOR: f32 = 20.0;
	pub const STORYTELLERS_STRUCTURAL_LOW_FACTOR: f32 = 30.0;

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct StorytellersParams {
		#[command(flatten, next_help_heading = "Grove")]
		pub grove: GroveFrontend,

		#[arg(
			long,
			default_value = "0,1.0,1.0,1",
			value_parser = noise_params_from_scalar_str,
			value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
			help_heading = "The noise applied to the chains of sticks in trees",
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

		/// Number of unit-height tree archetypes (`unit_from_num(0..n)`). Caps unique
		/// merged-mesh handles for High/Medium.
		#[arg(long, default_value_t = 100)]
		pub tree_variants: u32,

		#[arg(skip)]
		resolved_placements: Option<Vec<GroveCellVariant<StorytellersCell>>>,
	}

	impl Default for StorytellersParams {
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
				tree_variants: 100,
				resolved_placements: None,
			}
		}
	}

	impl StorytellersParams {
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

		pub fn placements(&self) -> Vec<GroveCellVariant<StorytellersCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.placements_on(&self.terrain)
		}

		/// Select placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn placements_on(
			&self,
			world: &impl crate::GroveWorldSample,
		) -> Vec<GroveCellVariant<StorytellersCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.grove.assemble(definition()).populate(&self.extent, world)
		}

		pub fn build(&self) -> Storytellers {
			self.build_on(&self.terrain)
		}

		/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> Storytellers {
			Storytellers::from_placements(
				&self.placements_on(world),
				self.grove.noise,
				self.stick_surface_noise,
				&self.extent,
				self.tree_variants,
			)
		}
	}

	#[derive(Clone)]
	enum StorytellersKind {
		Oak(Arc<BraidOakTree>),
		Storybook(Arc<StorybookTree>),
		Penmarch(Arc<PenmarchTorch>),
		Kamakura(Arc<KamakuraTorch>),
	}

	#[derive(Clone)]
	pub struct StorytellersPlant {
		pub placement: Placement,
		kind: StorytellersKind,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	}

	#[derive(Clone, Component)]
	pub struct Storytellers {
		pub plants: Arc<[StorytellersPlant]>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl Storytellers {
		pub fn from_placements(
			placements: &[GroveCellVariant<StorytellersCell>],
			grove_noise: NoiseParams,
			stick_surface_noise: NoiseParams,
			extent: &GroveExtent,
			tree_variants: u32,
		) -> Self {
			let plants: Arc<[StorytellersPlant]> = placements
				.iter()
				.map(|placed| grow_plant(placed, grove_noise, stick_surface_noise, tree_variants))
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
					StorytellersKind::Oak(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					StorytellersKind::Storybook(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					StorytellersKind::Penmarch(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					StorytellersKind::Kamakura(t) => nest_flattened_plant_chunk(
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
						StorytellersKind::Oak(t) => canopy_proxy_site(t, plant.placement, material),
						StorytellersKind::Storybook(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						StorytellersKind::Penmarch(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						StorytellersKind::Kamakura(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
					}
				})
				.collect()
		}
	}

	fn grow_plant(
		placed: &GroveCellVariant<StorytellersCell>,
		grove_noise: NoiseParams,
		_stick_surface_noise: NoiseParams,
		tree_variants: u32,
	) -> StorytellersPlant {
		let variant = patch_variant_index(placed.position, tree_variants);
		let build_noise = variant_noise(grove_noise, variant);
		let palette_noise = placement_noise(grove_noise, placed.position);
		let stick_seed = palette_noise.seed;
		let canopy_seed = palette_noise.seed.wrapping_add(31);
		let stick_material =
			stick_material_from_palette(Some(placed.variant.stick_palette_mix()), stick_seed);
		let ball_material = canopy_ball_material_from_palette(
			Some(placed.variant.canopy_palette_mix()),
			canopy_seed,
		);
		let frond_material =
			frond_material_from_palette(Some(placed.variant.canopy_palette_mix()), canopy_seed);

		match placed.variant.item() {
			StorytellersItem::BraidOak(oak) => {
				let world_size = oak.build_with_noise(build_noise).height();
				StorytellersPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: StorytellersKind::Oak(Arc::new(BraidOakTree::unit_from_num(variant))),
					stick_material,
					ball_material,
					frond_material,
				}
			}
			StorytellersItem::Storybook(story) => {
				let geometry = story.build_with_noise(build_noise);
				let mut params = StorybookTreeParams::default();
				params.geometry = geometry;
				let (unit_params, world_size) = params.into_unit_from_num(variant);
				StorytellersPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: StorytellersKind::Storybook(Arc::new(unit_params.build())),
					stick_material,
					ball_material,
					frond_material,
				}
			}
			StorytellersItem::PenmarchTorch(torch) | StorytellersItem::TorchTree(torch) => {
				let geometry =
					BuildWithNoise::<PenmarchTorchSbs>::build_with_noise(torch, build_noise);
				let mut params = PenmarchTorchParams::default();
				params.geometry = geometry;
				let (unit_params, world_size) = params.into_unit_from_num(variant);
				StorytellersPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: StorytellersKind::Penmarch(Arc::new(unit_params.build())),
					stick_material,
					ball_material,
					frond_material,
				}
			}
			StorytellersItem::KamakuraTorch(torch) => {
				let geometry =
					BuildWithNoise::<KamakuraTorchSbs>::build_with_noise(torch, build_noise);
				let mut params = KamakuraTorchParams::default();
				params.geometry = geometry;
				let (unit_params, world_size) = params.into_unit_from_num(variant);
				StorytellersPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: StorytellersKind::Kamakura(Arc::new(unit_params.build())),
					stick_material,
					ball_material,
					frond_material,
				}
			}
		}
	}

	impl VegetationComponents for Storytellers {
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
				STORYTELLERS_STRUCTURAL_HIGH_FACTOR,
				STORYTELLERS_STRUCTURAL_MEDIUM_FACTOR,
				STORYTELLERS_STRUCTURAL_LOW_FACTOR,
			))
		}
	}

	impl LodScene for Storytellers {
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

	#[cfg(test)]
	mod tests {
		use super::*;
		use anyhow::Result;

		fn small_grove() -> Storytellers {
			StorytellersParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(220.0, 1.0, 220.0)))
				.build()
		}

		fn plant_height(plant: &StorytellersPlant) -> f32 {
			match &plant.kind {
				StorytellersKind::Oak(t) => t.geometry.height(),
				StorytellersKind::Storybook(t) => t.geometry.height(),
				StorytellersKind::Penmarch(t) => t.geometry.height(),
				StorytellersKind::Kamakura(t) => t.geometry.height(),
			}
		}

		fn plant_seed(plant: &StorytellersPlant) -> i32 {
			match &plant.kind {
				StorytellersKind::Oak(t) => t.geometry.canopy_noise.seed,
				StorytellersKind::Storybook(t) => t.geometry.canopy_noise.seed,
				StorytellersKind::Penmarch(t) => t.geometry.canopy_noise.seed,
				StorytellersKind::Kamakura(t) => t.geometry.canopy_noise.seed,
			}
		}

		#[test]
		fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
			let grove = small_grove();
			assert!(!grove.plants.is_empty(), "expected placed storytellers trees");

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
				anyhow::bail!("High storytellers should wrap plant chunks");
			};
			assert_eq!(parts.len(), 1, "expected one lazy plant producer");
			let lod::SceneChunk::Lazy { remaining_primitives, remaining_weight, .. } = &parts[0]
			else {
				anyhow::bail!("High storytellers plants should be SceneChunk::Lazy");
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
				anyhow::bail!("Low storytellers should emit one flattened canopy collection");
			};
			assert_eq!(weight, chico_vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT);
			Ok(())
		}

		#[test]
		fn tree_variants_quantize_archetypes() -> Result<()> {
			use std::collections::HashSet;

			let mut params = StorytellersParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(260.0, 1.0, 260.0)));
			params.tree_variants = 4;
			let grove = params.build();
			assert!(!grove.plants.is_empty(), "expected placed storytellers trees");
			for plant in grove.plants.iter() {
				assert!(
					(plant_height(plant) - 1.0).abs() < 1e-4,
					"expected unit height, got {}",
					plant_height(plant)
				);
			}
			let seeds: HashSet<i32> = grove.plants.iter().map(plant_seed).collect();
			assert!(seeds.len() <= 4, "expected ≤4 unique unit seeds, got {}", seeds.len());
			Ok(())
		}
	}
}

#[cfg(feature = "render")]
pub use vc::{
	Storytellers, StorytellersParams, StorytellersPlant, STORYTELLERS_STRUCTURAL_HIGH_FACTOR,
	STORYTELLERS_STRUCTURAL_LOW_FACTOR, STORYTELLERS_STRUCTURAL_MEDIUM_FACTOR,
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
	fn distribution_matches_authored_order_and_weights() -> Result<()> {
		let dist = StorytellersCell::distribution();
		assert_eq!(dist.len(), 14);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 16.2);
		assert_eq!(dist.buckets[1].item, Some(StorytellersCell::ColorfulStorybook));
		assert_eq!(dist.buckets[1].weight, 1.5);
		assert_eq!(dist.buckets[2].item, Some(StorytellersCell::ColorfulBraidOak));
		assert_eq!(dist.buckets[2].weight, 1.5);
		assert_eq!(dist.buckets[3].item, Some(StorytellersCell::BrightCanopyStorybook));
		assert_eq!(dist.buckets[3].weight, 0.75);
		assert_eq!(dist.buckets[4].item, Some(StorytellersCell::PinkLanternStorybook));
		assert_eq!(dist.buckets[4].weight, 0.35);
		assert_eq!(dist.buckets[5].item, Some(StorytellersCell::RedFestivalBraidOak));
		assert_eq!(dist.buckets[5].weight, 0.30);
		assert_eq!(dist.buckets[6].item, Some(StorytellersCell::PurpleCrownStorybook));
		assert_eq!(dist.buckets[6].weight, 0.25);
		assert_eq!(dist.buckets[7].item, Some(StorytellersCell::BlueMoonStorybook));
		assert_eq!(dist.buckets[7].weight, 0.25);
		assert_eq!(dist.buckets[8].item, Some(StorytellersCell::GoldenLanternPenmarch));
		assert_eq!(dist.buckets[8].weight, 0.22);
		assert_eq!(dist.buckets[9].item, Some(StorytellersCell::BlueFlameKamakura));
		assert_eq!(dist.buckets[9].weight, 0.20);
		assert_eq!(dist.buckets[10].item, Some(StorytellersCell::FestivalTorchTree));
		assert_eq!(dist.buckets[10].weight, 0.18);
		assert_eq!(dist.buckets[11].item, Some(StorytellersCell::VioletCanopyBraidOak));
		assert_eq!(dist.buckets[11].weight, 0.28);
		assert_eq!(dist.buckets[12].item, Some(StorytellersCell::GoldLeafBraidOak));
		assert_eq!(dist.buckets[12].weight, 0.26);
		assert_eq!(dist.buckets[13].item, Some(StorytellersCell::CopperFlameBraidOak));
		assert_eq!(dist.buckets[13].weight, 0.24);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = StorytellersCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.18..=0.38).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let StorytellersItem::Storybook(colorful) = StorytellersCell::ColorfulStorybook.item()
		else {
			anyhow::bail!("expected colorful storybook item");
		};
		assert_eq!(colorful.height, UnitRange::new(10.0, 30.0));
		assert_eq!(colorful.canopy_density, DENSE_CANOPY_DENSITY);

		let StorytellersItem::BraidOak(festival) = StorytellersCell::RedFestivalBraidOak.item()
		else {
			anyhow::bail!("expected red festival braid oak item");
		};
		assert_eq!(festival.height, UnitRange::new(12.0, 24.0));
		assert_eq!(festival.canopy_density, MODERATE_CANOPY_DENSITY);
		Ok(())
	}

	#[test]
	fn placement_constraints_use_full_elevation_and_rfc_steepness() -> Result<()> {
		let dist = StorytellersCell::distribution();
		for bucket in dist.buckets.iter().filter(|b| b.item.is_some()) {
			assert_eq!(bucket.constraints.elevation.start, 0.0);
			assert_eq!(bucket.constraints.elevation.end, 1.0);
		}
		let braid = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(StorytellersCell::ColorfulBraidOak))
			.ok_or_else(|| anyhow::anyhow!("missing colorful braid oak bucket"))?;
		assert_eq!(braid.constraints.steepness.end, 0.48);

		let bright = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(StorytellersCell::BrightCanopyStorybook))
			.ok_or_else(|| anyhow::anyhow!("missing bright storybook bucket"))?;
		assert_eq!(bright.constraints.steepness.end, 0.56);
		Ok(())
	}

	#[test]
	#[ignore = "placement constraints deferred to forest-layer normalization"]
	fn steep_slope_rejects_colorful_braid_oak_but_allows_bright_storybook() -> Result<()> {
		let prepared =
			StorytellersCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let moderate = FlatTerrainSample { elevation: 0.40, steepness: 0.40 };
		let braid_outcome = prepared.select_from(
			2,
			Vec3::new(5.0, 0.40, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&moderate,
		);
		match braid_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, StorytellersCell::ColorfulBraidOak);
			}
			other => anyhow::bail!("expected ColorfulBraidOak on moderate slope, got {other:?}"),
		}
		let steep = FlatTerrainSample { elevation: 0.40, steepness: 0.50 };
		let steep_outcome = prepared.select_from(
			2,
			Vec3::new(5.0, 0.40, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&steep,
		);
		match steep_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, StorytellersCell::BrightCanopyStorybook);
			}
			other => {
				anyhow::bail!("expected BrightCanopyStorybook on steep slope, got {other:?}")
			}
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			StorytellersCell::ColorfulStorybook,
			StorytellersCell::ColorfulBraidOak,
			StorytellersCell::BrightCanopyStorybook,
			StorytellersCell::PinkLanternStorybook,
			StorytellersCell::RedFestivalBraidOak,
			StorytellersCell::PurpleCrownStorybook,
			StorytellersCell::BlueMoonStorybook,
			StorytellersCell::GoldenLanternPenmarch,
			StorytellersCell::BlueFlameKamakura,
			StorytellersCell::FestivalTorchTree,
			StorytellersCell::VioletCanopyBraidOak,
			StorytellersCell::GoldLeafBraidOak,
			StorytellersCell::CopperFlameBraidOak,
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
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(220.0, 1.0, 220.0));
		let terrain = FlatTerrainSample::default();
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
