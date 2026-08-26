//! Tropical Undergrowth — well-known moderate-to-dense hybrid understory grove
//! ([RFC-183 §3.4.5.5], [#315](https://github.com/ramate-io/maybraid/issues/315)).
//!
//! Mixes bright/deep tufts (mostly as patches), small palm bushes, and rare mini SBS-tree forms.
//! Forest-layer attachment remains a follow-up.

use std::ops::RangeInclusive;

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, GroveTuftPatch,
	PaletteMix, PaletteSlot, PlacementConstraints,
};

/// Authored Tropical Undergrowth grove definition.
///
/// Cell footprint sits at the RFC midpoint (`5.0` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<TropicalUndergrowthCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(5.0),
		placement: GrovePlacementRanges::new(UnitRange::new(0.85, 1.15), UnitRange::new(-5.0, 5.0)),
		distribution: TropicalUndergrowthCell::distribution(),
	}
}

/// Ordered tropical-undergrowth varietals ([RFC-183 §3.4.5.5]); the explicit `None` bucket lives
/// only in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TropicalUndergrowthCell {
	BrightTuft,
	DeepTuft,
	SmallPalmBush,
	MiniRoryHeadTrained,
	MiniVaseTree,
	MiniSparseStorybook,
	MiniPenmarchTorch,
	MiniKamakuraTorch,
	MiniTorchTree,
	BrightTuftPatch,
	DeepTuftPatch,
}

/// Typed authored geometry for one tropical-undergrowth varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TropicalUndergrowthItem {
	Tuft(&'static TropicalUndergrowthTuft),
	Patch(&'static GroveTuftPatch<TropicalUndergrowthTuft>),
	PalmBush(&'static TropicalUndergrowthPalm),
	RoryHead(&'static TropicalUndergrowthRoryHead),
	VaseTree(&'static TropicalUndergrowthVaseTree),
	Storybook(&'static TropicalUndergrowthStorybook),
	PenmarchTorch(&'static TropicalUndergrowthTorch),
	KamakuraTorch(&'static TropicalUndergrowthTorch),
	TorchTree(&'static TropicalUndergrowthTorch),
}

/// Authored geometry ranges for one tropical-undergrowth tuft clump.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalUndergrowthTuft {
	pub height: UnitRange,
	/// Blade width as a **fraction of blade length**.
	pub width_factor: UnitRange,
	pub blade_count: RangeInclusive<u32>,
	pub bend_segments: RangeInclusive<u32>,
	pub max_tilt_radians: UnitRange,
}

/// Authored geometry ranges for one ground-anchored palm bush companion.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalUndergrowthPalm {
	pub height: UnitRange,
	pub frond_count: RangeInclusive<u32>,
	pub frond_length: UnitRange,
	pub crown_spread: UnitRange,
}

/// Authored geometry ranges for one mini Rory's Head-trained form.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalUndergrowthRoryHead {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse..moderate canopy density at render time.
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one mini Vase Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalUndergrowthVaseTree {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
}

/// Authored geometry ranges for one mini torch form (Penmarch, Kamakura, or generic torch tree).
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalUndergrowthTorch {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
}

/// Authored geometry ranges for one mini Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalUndergrowthStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
}

/// Shared blade thickness band: ~2–4 % of blade length keeps blades grass-thin at any height.
const BLADE_WIDTH_FACTOR: UnitRange = UnitRange::new(0.02, 0.04);

const BLADE_COUNT: RangeInclusive<u32> = 6..=12;
const BEND_SEGMENTS: RangeInclusive<u32> = 1..=6;
const MAX_TILT_RADIANS: UnitRange = UnitRange::new(0.15, 0.35);

const BRIGHT_TUFT: TropicalUndergrowthTuft = TropicalUndergrowthTuft {
	height: UnitRange::new(0.30, 1.50),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

const DEEP_TUFT: TropicalUndergrowthTuft = TropicalUndergrowthTuft {
	height: UnitRange::new(0.40, 0.90),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

const BRIGHT_TUFT_PATCH: GroveTuftPatch<TropicalUndergrowthTuft> = GroveTuftPatch {
	clump: BRIGHT_TUFT,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(1.0, 2.4),
	base_spread: UnitRange::new(0.15, 0.35),
};

const DEEP_TUFT_PATCH: GroveTuftPatch<TropicalUndergrowthTuft> = GroveTuftPatch {
	clump: DEEP_TUFT,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(1.0, 2.4),
	base_spread: UnitRange::new(0.15, 0.35),
};

const SMALL_PALM_BUSH: TropicalUndergrowthPalm = TropicalUndergrowthPalm {
	height: UnitRange::new(1.00, 2.80),
	frond_count: 5..=9,
	frond_length: UnitRange::new(0.50, 1.40),
	crown_spread: UnitRange::new(0.70, 1.80),
};

const MINI_RORY_HEAD: TropicalUndergrowthRoryHead = TropicalUndergrowthRoryHead {
	height: UnitRange::new(0.80, 1.80),
	stalk_radius: UnitRange::new(0.037, 0.055),
	canopy_spread: UnitRange::new(0.70, 1.68),
	canopy_density: UnitRange::new(0.0, 1.0),
};

const MINI_VASE_TREE: TropicalUndergrowthVaseTree = TropicalUndergrowthVaseTree {
	height: UnitRange::new(1.00, 2.30),
	stalk_radius: UnitRange::new(0.046, 0.063),
	canopy_spread: UnitRange::new(0.98, 2.10),
};

const MINI_STORYBOOK: TropicalUndergrowthStorybook = TropicalUndergrowthStorybook {
	height: UnitRange::new(1.20, 2.50),
	stalk_radius: UnitRange::new(0.046, 0.063),
	canopy_spread: UnitRange::new(0.84, 1.96),
};

const MINI_TORCH_TREE: TropicalUndergrowthTorch = TropicalUndergrowthTorch {
	height: UnitRange::new(1.00, 2.20),
	stalk_radius: UnitRange::new(0.046, 0.063),
	canopy_spread: UnitRange::new(0.77, 1.68),
};

const MINI_PENMARCH_TORCH: TropicalUndergrowthTorch = TropicalUndergrowthTorch {
	height: UnitRange::new(1.20, 2.50),
	stalk_radius: UnitRange::new(0.046, 0.063),
	canopy_spread: UnitRange::new(0.91, 1.96),
};

const MINI_KAMAKURA_TORCH: TropicalUndergrowthTorch = TropicalUndergrowthTorch {
	height: UnitRange::new(1.00, 2.30),
	stalk_radius: UnitRange::new(0.046, 0.063),
	canopy_spread: UnitRange::new(0.84, 1.82),
};

const BRIGHT_TUFT_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("bright_green", "lime_green"),
	PaletteSlot::new("lush_green", "fresh_green"),
	PaletteSlot::new("yellow_green", "light_green"),
]);

const DEEP_TUFT_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "emerald_green"),
	PaletteSlot::new("dark_green", "wet_green"),
	PaletteSlot::new("blue_green", "bright_green"),
]);

const PALM_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_bark", "tan_bark"),
	PaletteSlot::new("green_stem", "wet_brown"),
]);

const PALM_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("wet_green", "lime_green"),
]);

const RORY_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("tropical_bark", "dark_bark"),
	PaletteSlot::new("green_brown", "wet_brown"),
]);

const RORY_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("blue_green", "wet_green"),
	PaletteSlot::new("yellow_green", "lime_green"),
]);

const VASE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_bark", "tropical_bark"),
	PaletteSlot::new("green_brown", "wet_brown"),
]);

const VASE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("dark_green", "emerald_green"),
	PaletteSlot::new("flower_white", "fresh_green"),
]);

const STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("tropical_bark", "dark_bark"),
	PaletteSlot::new("green_brown", "wet_brown"),
]);

const STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "light_green"),
	PaletteSlot::new("wet_green", "fresh_green"),
	PaletteSlot::new("blue_green", "yellow_green"),
]);

const TORCH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("tropical_bark", "dark_bark"),
	PaletteSlot::new("green_brown", "wet_brown"),
]);

const TORCH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("yellow_green", "warm_yellow"),
	PaletteSlot::new("lime_green", "fresh_green"),
]);

impl TropicalUndergrowthCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `8.44` (RFC relative proportions plus torch companions); the
	/// `None` weight of `12.0` puts the placed share at `8.44 / 20.44 ≈ 0.41`, mid RFC
	/// `DENSITY_RANGE` (`0.22..0.58`).
	pub fn distribution() -> GroveDistribution<Self> {
		let lowland =
			PlacementConstraints::new(UnitRange::new(0.0, 0.40), UnitRange::new(0.0, 0.70));
		let palm = PlacementConstraints::new(UnitRange::new(0.0, 0.40), UnitRange::new(0.0, 0.60));
		let mini_tree =
			PlacementConstraints::new(UnitRange::new(0.0, 0.40), UnitRange::new(0.0, 0.50));
		GroveDistribution::new(vec![
			GroveBucket::none(12.0),
			GroveBucket::placed(0.4, lowland, Self::BrightTuft),
			GroveBucket::placed(0.3, lowland, Self::DeepTuft),
			GroveBucket::placed(1.0, palm, Self::SmallPalmBush),
			GroveBucket::placed(0.85, lowland, Self::MiniRoryHeadTrained),
			GroveBucket::placed(0.20, mini_tree, Self::MiniVaseTree),
			GroveBucket::placed(0.15, mini_tree, Self::MiniSparseStorybook),
			GroveBucket::placed(1.30, mini_tree, Self::MiniPenmarchTorch),
			GroveBucket::placed(1.22, mini_tree, Self::MiniKamakuraTorch),
			GroveBucket::placed(0.22, mini_tree, Self::MiniTorchTree),
			GroveBucket::placed(1.6, lowland, Self::BrightTuftPatch),
			GroveBucket::placed(1.2, lowland, Self::DeepTuftPatch),
		])
	}

	pub fn item(self) -> TropicalUndergrowthItem {
		match self {
			Self::BrightTuft => TropicalUndergrowthItem::Tuft(&BRIGHT_TUFT),
			Self::DeepTuft => TropicalUndergrowthItem::Tuft(&DEEP_TUFT),
			Self::SmallPalmBush => TropicalUndergrowthItem::PalmBush(&SMALL_PALM_BUSH),
			Self::MiniRoryHeadTrained => TropicalUndergrowthItem::RoryHead(&MINI_RORY_HEAD),
			Self::MiniVaseTree => TropicalUndergrowthItem::VaseTree(&MINI_VASE_TREE),
			Self::MiniSparseStorybook => TropicalUndergrowthItem::Storybook(&MINI_STORYBOOK),
			Self::MiniPenmarchTorch => TropicalUndergrowthItem::PenmarchTorch(&MINI_PENMARCH_TORCH),
			Self::MiniKamakuraTorch => TropicalUndergrowthItem::KamakuraTorch(&MINI_KAMAKURA_TORCH),
			Self::MiniTorchTree => TropicalUndergrowthItem::TorchTree(&MINI_TORCH_TREE),
			Self::BrightTuftPatch => TropicalUndergrowthItem::Patch(&BRIGHT_TUFT_PATCH),
			Self::DeepTuftPatch => TropicalUndergrowthItem::Patch(&DEEP_TUFT_PATCH),
		}
	}

	pub fn palette_mix(self) -> PaletteMix {
		match self {
			Self::BrightTuft | Self::BrightTuftPatch => BRIGHT_TUFT_MIX,
			Self::DeepTuft | Self::DeepTuftPatch => DEEP_TUFT_MIX,
			Self::SmallPalmBush => PALM_CANOPY_MIX,
			Self::MiniRoryHeadTrained => RORY_CANOPY_MIX,
			Self::MiniVaseTree => VASE_CANOPY_MIX,
			Self::MiniSparseStorybook => STORYBOOK_CANOPY_MIX,
			Self::MiniPenmarchTorch | Self::MiniKamakuraTorch | Self::MiniTorchTree => {
				TORCH_CANOPY_MIX
			}
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::SmallPalmBush => PALM_STICK_MIX,
			Self::MiniRoryHeadTrained => RORY_STICK_MIX,
			Self::MiniVaseTree => VASE_STICK_MIX,
			Self::MiniSparseStorybook => STORYBOOK_STICK_MIX,
			Self::MiniPenmarchTorch | Self::MiniKamakuraTorch | Self::MiniTorchTree => {
				TORCH_STICK_MIX
			}
			_ => PALM_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::SmallPalmBush => PALM_CANOPY_MIX,
			Self::MiniRoryHeadTrained => RORY_CANOPY_MIX,
			Self::MiniVaseTree => VASE_CANOPY_MIX,
			Self::MiniSparseStorybook => STORYBOOK_CANOPY_MIX,
			Self::MiniPenmarchTorch | Self::MiniKamakuraTorch | Self::MiniTorchTree => {
				TORCH_CANOPY_MIX
			}
			_ => BRIGHT_TUFT_MIX,
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
		KamakuraTorch, KamakuraTorchParams, PalmBush, PalmBushParams, PenmarchTorch,
		PenmarchTorchParams, QuantizedPlant, RorysHeadTrained, RorysHeadTrainedParams,
		StorybookTree, StorybookTreeParams, TuftPatch, VaseTree, VaseTreeParams,
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

	use super::{
		definition, TropicalUndergrowthCell, TropicalUndergrowthItem, TropicalUndergrowthTorch,
		MINI_KAMAKURA_TORCH, MINI_PENMARCH_TORCH, MINI_RORY_HEAD, MINI_STORYBOOK, MINI_TORCH_TREE,
		MINI_VASE_TREE, SMALL_PALM_BUSH,
	};
	use crate::grove::vc_tuft::{
		material_from_palette, patch_variant_index, single_blade_patch_params, stamp_foliage_noise,
		unit_plant_from_params, variant_noise, TUFT_GROVE_STRUCTURAL_HIGH_FACTOR,
		TUFT_GROVE_STRUCTURAL_LOW_FACTOR, TUFT_GROVE_STRUCTURAL_MEDIUM_FACTOR,
	};
	use crate::grove::{
		canopy_ball_material_from_palette, canopy_proxy_rory, canopy_proxy_site,
		foliage_low_canopy_balls, foliage_ultra_low_merged_balls, frond_material_from_palette,
		grove_detail_level, grove_lod_culls, grove_lod_level, grove_lod_status,
		grove_structural_footprint, layers_from_nodes, nest_flattened_plant_chunk, placement_noise,
		remixed_sbs_plant, stick_material_from_palette, trained_proxy_stick_nodes_for_level,
		unit_build_noise, woody_grove_scene_chunks, CanopyProxySite, FlatTerrainSample,
		GroveCellVariant, GroveExtent, GroveFrontend, DEFAULT_GROVE_EXTENT_XZ,
		ULTRA_LOW_CANOPY_BIN_METERS,
	};

	pub const TROPICAL_UNDERGROWTH_STRUCTURAL_HIGH_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_HIGH_FACTOR;
	pub const TROPICAL_UNDERGROWTH_STRUCTURAL_MEDIUM_FACTOR: f32 =
		TUFT_GROVE_STRUCTURAL_MEDIUM_FACTOR;
	pub const TROPICAL_UNDERGROWTH_STRUCTURAL_LOW_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_LOW_FACTOR;

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct TropicalUndergrowthParams {
		#[command(flatten, next_help_heading = "Grove")]
		pub grove: GroveFrontend,

		#[arg(
			long,
			default_value = "0,1.0,1.0,1",
			value_parser = noise_params_from_scalar_str,
			value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
			help_heading = "The noise applied to the chains of sticks in mini trees",
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

		#[arg(long, default_value_t = 100)]
		pub patch_variants: u32,

		/// Number of unit-height woody archetypes (`unit_from_num(0..n)`). Caps unique
		/// merged-mesh handles for High/Medium trees.
		#[arg(long, default_value_t = 100)]
		pub tree_variants: u32,

		#[arg(skip)]
		resolved_placements: Option<Vec<GroveCellVariant<TropicalUndergrowthCell>>>,
	}

	impl Default for TropicalUndergrowthParams {
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
				terrain: FlatTerrainSample::default(),
				patch_variants: 100,
				tree_variants: 100,
				resolved_placements: None,
			}
		}
	}

	impl TropicalUndergrowthParams {
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

		pub fn placements(&self) -> Vec<GroveCellVariant<TropicalUndergrowthCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.placements_on(&self.terrain)
		}

		/// Select placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn placements_on(
			&self,
			world: &impl crate::GroveWorldSample,
		) -> Vec<GroveCellVariant<TropicalUndergrowthCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.grove.assemble(definition()).populate(&self.extent, world)
		}

		pub fn build(&self) -> TropicalUndergrowth {
			self.build_on(&self.terrain)
		}

		/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> TropicalUndergrowth {
			TropicalUndergrowth::from_placements(
				&self.placements_on(world),
				self.grove.noise,
				self.leaf_surface_noise,
				self.patch_variants,
				self.tree_variants,
				&self.extent,
			)
		}
	}

	remixed_sbs_plant!(
		SmallPalmBush,
		PalmBush,
		PalmBushParams,
		SMALL_PALM_BUSH
	);
	remixed_sbs_plant!(
		MiniRoryHead,
		RorysHeadTrained,
		RorysHeadTrainedParams,
		MINI_RORY_HEAD
	);
	remixed_sbs_plant!(
		MiniVaseTree,
		VaseTree,
		VaseTreeParams,
		MINI_VASE_TREE
	);
	remixed_sbs_plant!(
		MiniSparseStorybook,
		StorybookTree,
		StorybookTreeParams,
		MINI_STORYBOOK
	);

	fn undergrowth_penmarch_unit(
		authored: &TropicalUndergrowthTorch,
		num: u32,
	) -> (PenmarchTorch, f32) {
		let mut params = PenmarchTorchParams::default();
		params.geometry =
			BuildWithNoise::<PenmarchTorchSbs>::build_with_noise(authored, unit_build_noise(num));
		let (unit, world_size) = params.into_unit_from_num(num);
		(unit.build(), world_size)
	}

	fn undergrowth_kamakura_unit(
		authored: &TropicalUndergrowthTorch,
		num: u32,
	) -> (KamakuraTorch, f32) {
		let mut params = KamakuraTorchParams::default();
		params.geometry =
			BuildWithNoise::<KamakuraTorchSbs>::build_with_noise(authored, unit_build_noise(num));
		let (unit, world_size) = params.into_unit_from_num(num);
		(unit.build(), world_size)
	}

	struct MiniPenmarchTorch;
	struct MiniKamakuraTorch;
	struct MiniTorchTree;

	impl QuantizedPlant for MiniPenmarchTorch {
		type Unit = PenmarchTorch;
		fn build_unit(num: u32) -> (PenmarchTorch, f32) {
			undergrowth_penmarch_unit(&MINI_PENMARCH_TORCH, num)
		}
	}

	impl QuantizedPlant for MiniKamakuraTorch {
		type Unit = KamakuraTorch;
		fn build_unit(num: u32) -> (KamakuraTorch, f32) {
			undergrowth_kamakura_unit(&MINI_KAMAKURA_TORCH, num)
		}
	}

	impl QuantizedPlant for MiniTorchTree {
		type Unit = PenmarchTorch;
		fn build_unit(num: u32) -> (PenmarchTorch, f32) {
			undergrowth_penmarch_unit(&MINI_TORCH_TREE, num)
		}
	}

	#[derive(Clone)]
	enum TropicalUndergrowthKind {
		Tuft(Arc<TuftPatch>),
		Palm(Arc<PalmBush>),
		Rory(Arc<RorysHeadTrained>),
		Vase(Arc<VaseTree>),
		Storybook(Arc<StorybookTree>),
		Penmarch(Arc<PenmarchTorch>),
		Kamakura(Arc<KamakuraTorch>),
	}

	#[derive(Clone)]
	struct TropicalUndergrowthPlant {
		placement: Placement,
		kind: TropicalUndergrowthKind,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	}

	#[derive(Clone, Component)]
	pub struct TropicalUndergrowth {
		plants: Arc<[TropicalUndergrowthPlant]>,
		structural_center: Vec3,
		footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl TropicalUndergrowth {
		pub fn from_placements(
			placements: &[GroveCellVariant<TropicalUndergrowthCell>],
			grove_noise: NoiseParams,
			leaf_surface_noise: NoiseParams,
			patch_variants: u32,
			tree_variants: u32,
			extent: &GroveExtent,
		) -> Self {
			let patch_variants = patch_variants.max(1);
			let tree_variants = tree_variants.max(1);
			let plants: Arc<[TropicalUndergrowthPlant]> = placements
				.iter()
				.map(|placed| {
					grow_plant(
						placed,
						grove_noise,
						leaf_surface_noise,
						patch_variants,
						tree_variants,
					)
				})
				.collect::<Vec<_>>()
				.into();
			let (structural_center, footprint_radius) = grove_structural_footprint(extent);
			Self { plants, structural_center, footprint_radius, extent: *extent }
		}

		pub fn is_empty(&self) -> bool {
			self.plants.is_empty()
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
					TropicalUndergrowthKind::Tuft(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					TropicalUndergrowthKind::Palm(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					TropicalUndergrowthKind::Rory(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					TropicalUndergrowthKind::Vase(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					TropicalUndergrowthKind::Storybook(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					TropicalUndergrowthKind::Penmarch(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					TropicalUndergrowthKind::Kamakura(t) => nest_flattened_plant_chunk(
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
				.flat_map(|plant| match &plant.kind {
					TropicalUndergrowthKind::Tuft(t) => {
						vec![tuft_proxy_site(t, plant.placement, &plant.ball_material)]
					}
					TropicalUndergrowthKind::Palm(t) => {
						canopy_proxy_site(t, plant.placement, &plant.ball_material)
							.into_iter()
							.collect()
					}
					TropicalUndergrowthKind::Rory(t) => vec![
						canopy_proxy_rory(
							t,
							plant.placement,
							&plant.stick_material,
							&plant.ball_material,
						)
						.crown,
					],
					TropicalUndergrowthKind::Vase(t) => {
						canopy_proxy_site(t, plant.placement, &plant.ball_material)
							.into_iter()
							.collect()
					}
					TropicalUndergrowthKind::Storybook(t) => {
						canopy_proxy_site(t, plant.placement, &plant.ball_material)
							.into_iter()
							.collect()
					}
					TropicalUndergrowthKind::Penmarch(t) => {
						canopy_proxy_site(t, plant.placement, &plant.ball_material)
							.into_iter()
							.collect()
					}
					TropicalUndergrowthKind::Kamakura(t) => {
						canopy_proxy_site(t, plant.placement, &plant.ball_material)
							.into_iter()
							.collect()
					}
				})
				.collect()
		}

		fn proxy_trunks(&self) -> Vec<StickNode> {
			self.plants
				.iter()
				.filter_map(|plant| match &plant.kind {
					TropicalUndergrowthKind::Rory(t) => {
						canopy_proxy_rory(
							t,
							plant.placement,
							&plant.stick_material,
							&plant.ball_material,
						)
						.trunk
					}
					_ => None,
				})
				.collect()
		}
	}

	fn tuft_proxy_site(
		patch: &TuftPatch,
		placement: Placement,
		material: &MaterialRef,
	) -> CanopyProxySite {
		let scale = placement.scale.abs().max_element().max(1e-4);
		let height = (patch.shape.blade_length * scale).max(0.15);
		let footprint = (patch.patch_extent_xz * 0.5 * scale).max(height * 0.35);
		CanopyProxySite::from_radius(
			placement.translation + Vec3::Y * (height * 0.4),
			footprint.max(0.25),
			material.clone(),
		)
	}

	fn woody_materials(
		placed: &GroveCellVariant<TropicalUndergrowthCell>,
		palette_noise: NoiseParams,
	) -> (MaterialRef, MaterialRef, MaterialRef) {
		let stick_seed = palette_noise.seed;
		let canopy_seed = palette_noise.seed.wrapping_add(31);
		(
			stick_material_from_palette(Some(placed.variant.stick_palette_mix()), stick_seed),
			canopy_ball_material_from_palette(
				Some(placed.variant.canopy_palette_mix()),
				canopy_seed,
			),
			frond_material_from_palette(Some(placed.variant.canopy_palette_mix()), canopy_seed),
		)
	}

	fn grow_plant(
		placed: &GroveCellVariant<TropicalUndergrowthCell>,
		grove_noise: NoiseParams,
		leaf_surface_noise: NoiseParams,
		patch_variants: u32,
		tree_variants: u32,
	) -> TropicalUndergrowthPlant {
		match placed.variant.item() {
			TropicalUndergrowthItem::Tuft(tuft) => {
				let variant = patch_variant_index(placed.position, patch_variants);
				let noise = variant_noise(leaf_surface_noise, variant);
				let params =
					single_blade_patch_params(tuft.build_with_noise(noise), leaf_surface_noise);
				let material = material_from_palette(
					placed.variant.palette_mix(),
					placed.position,
					leaf_surface_noise,
				);
				let (placement, patch, material) = unit_plant_from_params(
					params,
					variant,
					placed.position,
					placed.scale,
					material,
				);
				TropicalUndergrowthPlant {
					placement,
					kind: TropicalUndergrowthKind::Tuft(Arc::new(patch)),
					stick_material: MaterialRef::default(),
					ball_material: material.clone(),
					frond_material: material,
				}
			}
			TropicalUndergrowthItem::Patch(patch) => {
				let variant = patch_variant_index(placed.position, patch_variants);
				let noise = variant_noise(leaf_surface_noise, variant);
				let params = stamp_foliage_noise(patch.build_tuft_patch(noise), leaf_surface_noise);
				let material = material_from_palette(
					placed.variant.palette_mix(),
					placed.position,
					leaf_surface_noise,
				);
				let (placement, patch, material) = unit_plant_from_params(
					params,
					variant,
					placed.position,
					placed.scale,
					material,
				);
				TropicalUndergrowthPlant {
					placement,
					kind: TropicalUndergrowthKind::Tuft(Arc::new(patch)),
					stick_material: MaterialRef::default(),
					ball_material: material.clone(),
					frond_material: material,
				}
			}
			TropicalUndergrowthItem::PalmBush(_)
			| TropicalUndergrowthItem::RoryHead(_)
			| TropicalUndergrowthItem::VaseTree(_)
			| TropicalUndergrowthItem::Storybook(_)
			| TropicalUndergrowthItem::PenmarchTorch(_)
			| TropicalUndergrowthItem::KamakuraTorch(_)
			| TropicalUndergrowthItem::TorchTree(_) => {
				let variant = patch_variant_index(placed.position, tree_variants);
				let palette_noise = placement_noise(grove_noise, placed.position);
				let (stick, ball, frond) = woody_materials(placed, palette_noise);
				let (kind, world_size) = match placed.variant {
					TropicalUndergrowthCell::SmallPalmBush => {
						let (tree, world_size) = SmallPalmBush::grow_num(variant);
						(TropicalUndergrowthKind::Palm(tree), world_size)
					}
					TropicalUndergrowthCell::MiniRoryHeadTrained => {
						let (tree, world_size) = MiniRoryHead::grow_num(variant);
						(TropicalUndergrowthKind::Rory(tree), world_size)
					}
					TropicalUndergrowthCell::MiniVaseTree => {
						let (tree, world_size) = MiniVaseTree::grow_num(variant);
						(TropicalUndergrowthKind::Vase(tree), world_size)
					}
					TropicalUndergrowthCell::MiniSparseStorybook => {
						let (tree, world_size) = MiniSparseStorybook::grow_num(variant);
						(TropicalUndergrowthKind::Storybook(tree), world_size)
					}
					TropicalUndergrowthCell::MiniPenmarchTorch => {
						let (tree, world_size) = MiniPenmarchTorch::grow_num(variant);
						(TropicalUndergrowthKind::Penmarch(tree), world_size)
					}
					TropicalUndergrowthCell::MiniKamakuraTorch => {
						let (tree, world_size) = MiniKamakuraTorch::grow_num(variant);
						(TropicalUndergrowthKind::Kamakura(tree), world_size)
					}
					TropicalUndergrowthCell::MiniTorchTree => {
						let (tree, world_size) = MiniTorchTree::grow_num(variant);
						(TropicalUndergrowthKind::Penmarch(tree), world_size)
					}
					TropicalUndergrowthCell::BrightTuft
					| TropicalUndergrowthCell::DeepTuft
					| TropicalUndergrowthCell::BrightTuftPatch
					| TropicalUndergrowthCell::DeepTuftPatch => {
						unreachable!("tuft cells are handled above")
					}
				};
				TropicalUndergrowthPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind,
					stick_material: stick,
					ball_material: ball,
					frond_material: frond,
				}
			}
		}
	}

	impl VegetationComponents for TropicalUndergrowth {
		fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
			trained_proxy_stick_nodes_for_level(level, self.proxy_trunks())
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
				TROPICAL_UNDERGROWTH_STRUCTURAL_HIGH_FACTOR,
				TROPICAL_UNDERGROWTH_STRUCTURAL_MEDIUM_FACTOR,
				TROPICAL_UNDERGROWTH_STRUCTURAL_LOW_FACTOR,
			))
		}
	}

	impl LodScene for TropicalUndergrowth {
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

		fn small_grove() -> TropicalUndergrowth {
			TropicalUndergrowthParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0)))
				.build()
		}

		fn plant_unit_size(plant: &TropicalUndergrowthPlant) -> f32 {
			match &plant.kind {
				TropicalUndergrowthKind::Tuft(t) => t.patch_extent_xz.max(t.shape.blade_length),
				TropicalUndergrowthKind::Palm(t) => t.geometry.height(),
				TropicalUndergrowthKind::Rory(t) => t.geometry.height(),
				TropicalUndergrowthKind::Vase(t) => t.geometry.height(),
				TropicalUndergrowthKind::Storybook(t) => t.geometry.height(),
				TropicalUndergrowthKind::Penmarch(t) => t.geometry.height(),
				TropicalUndergrowthKind::Kamakura(t) => t.geometry.height(),
			}
		}

		fn plant_seed(plant: &TropicalUndergrowthPlant) -> i32 {
			match &plant.kind {
				TropicalUndergrowthKind::Tuft(t) => t.shape.seed,
				TropicalUndergrowthKind::Palm(t) => t.geometry.foliage_noise.seed,
				TropicalUndergrowthKind::Rory(t) => t.geometry.canopy_noise.seed,
				TropicalUndergrowthKind::Vase(t) => t.geometry.canopy_noise.seed,
				TropicalUndergrowthKind::Storybook(t) => t.geometry.canopy_noise.seed,
				TropicalUndergrowthKind::Penmarch(t) => t.geometry.canopy_noise.seed,
				TropicalUndergrowthKind::Kamakura(t) => t.geometry.canopy_noise.seed,
			}
		}

		#[test]
		fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
			let grove = small_grove();
			assert!(!grove.plants.is_empty(), "expected placed tropical undergrowth plants");

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
				anyhow::bail!("High tropical undergrowth should wrap plant chunks");
			};
			assert_eq!(parts.len(), 1, "expected one lazy plant producer");
			let lod::SceneChunk::Lazy { remaining_primitives, remaining_weight, .. } = &parts[0]
			else {
				anyhow::bail!("High tropical undergrowth plants should be SceneChunk::Lazy");
			};
			assert_eq!(*remaining_primitives, grove.plants.len());
			assert_eq!(*remaining_weight as usize, grove.plants.len());

			assert!(grove.stick_nodes_for_level(LodSceneLevel::Low).len() <= 1);
			let low_foliage = grove.foliage_nodes_for_level(LodSceneLevel::Low).len();
			assert_eq!(low_foliage, grove.canopy_sites().len());
			assert!(low_foliage >= grove.plants.len());
			assert!(grove.foliage_nodes_for_level(LodSceneLevel::UltraLow).len() <= low_foliage);
			match grove.scene_chunks_with_level(&lod_ref, LodSceneLevel::Low) {
				lod::SceneChunk::Primitive { weight, .. } => {
					assert_eq!(weight, chico_vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT);
				}
				lod::SceneChunk::SubChunks(parts) => assert!(!parts.is_empty()),
				_ => anyhow::bail!("Low tropical undergrowth should emit flattened canopy kits"),
			}
			Ok(())
		}

		#[test]
		fn tree_variants_quantize_archetypes() -> Result<()> {
			use std::collections::HashSet;

			let mut params = TropicalUndergrowthParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0)));
			params.patch_variants = 4;
			params.tree_variants = 4;
			let grove = params.build();
			assert!(!grove.plants.is_empty(), "expected placed tropical undergrowth plants");
			for plant in grove.plants.iter() {
				assert!(
					(plant_unit_size(plant) - 1.0).abs() < 1e-4,
					"expected unit size, got {}",
					plant_unit_size(plant)
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
	TropicalUndergrowth, TropicalUndergrowthParams, TROPICAL_UNDERGROWTH_STRUCTURAL_HIGH_FACTOR,
	TROPICAL_UNDERGROWTH_STRUCTURAL_LOW_FACTOR, TROPICAL_UNDERGROWTH_STRUCTURAL_MEDIUM_FACTOR,
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
		let dist = TropicalUndergrowthCell::distribution();
		assert_eq!(dist.len(), 12);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 12.0);
		assert_eq!(dist.buckets[1].item, Some(TropicalUndergrowthCell::BrightTuft));
		assert_eq!(dist.buckets[1].weight, 0.4);
		assert_eq!(dist.buckets[2].item, Some(TropicalUndergrowthCell::DeepTuft));
		assert_eq!(dist.buckets[2].weight, 0.3);
		assert_eq!(dist.buckets[3].item, Some(TropicalUndergrowthCell::SmallPalmBush));
		assert_eq!(dist.buckets[3].weight, 1.0);
		assert_eq!(dist.buckets[4].item, Some(TropicalUndergrowthCell::MiniRoryHeadTrained));
		assert_eq!(dist.buckets[4].weight, 0.85);
		assert_eq!(dist.buckets[5].item, Some(TropicalUndergrowthCell::MiniVaseTree));
		assert_eq!(dist.buckets[5].weight, 0.20);
		assert_eq!(dist.buckets[6].item, Some(TropicalUndergrowthCell::MiniSparseStorybook));
		assert_eq!(dist.buckets[6].weight, 0.15);
		assert_eq!(dist.buckets[7].item, Some(TropicalUndergrowthCell::MiniPenmarchTorch));
		assert_eq!(dist.buckets[7].weight, 1.30);
		assert_eq!(dist.buckets[8].item, Some(TropicalUndergrowthCell::MiniKamakuraTorch));
		assert_eq!(dist.buckets[8].weight, 1.22);
		assert_eq!(dist.buckets[9].item, Some(TropicalUndergrowthCell::MiniTorchTree));
		assert_eq!(dist.buckets[9].weight, 0.22);
		assert_eq!(dist.buckets[10].item, Some(TropicalUndergrowthCell::BrightTuftPatch));
		assert_eq!(dist.buckets[10].weight, 1.6);
		assert_eq!(dist.buckets[11].item, Some(TropicalUndergrowthCell::DeepTuftPatch));
		assert_eq!(dist.buckets[11].weight, 1.2);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = TropicalUndergrowthCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.22..=0.58).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn patches_outweigh_single_tufts() -> Result<()> {
		let tuft_weight = |patch: bool| -> f32 {
			TropicalUndergrowthCell::distribution()
				.buckets
				.iter()
				.filter(|b| {
					b.item.is_some_and(|cell| match cell.item() {
						TropicalUndergrowthItem::Tuft(_) => !patch,
						TropicalUndergrowthItem::Patch(_) => patch,
						_ => false,
					})
				})
				.map(|b| b.weight)
				.sum()
		};
		assert!(
			tuft_weight(true) > 2.0 * tuft_weight(false),
			"patches should dominate tuft weight"
		);
		Ok(())
	}

	#[test]
	fn tuft_palm_and_tree_placed_weights_match_rfc_ratio() -> Result<()> {
		let weight = |kind: &str| -> f32 {
			TropicalUndergrowthCell::distribution()
				.buckets
				.iter()
				.filter(|b| {
					b.item.is_some_and(|cell| match (kind, cell.item()) {
						(
							"tuft",
							TropicalUndergrowthItem::Tuft(_) | TropicalUndergrowthItem::Patch(_),
						) => true,
						("palm", TropicalUndergrowthItem::PalmBush(_)) => true,
						("rory", TropicalUndergrowthItem::RoryHead(_)) => true,
						("vase", TropicalUndergrowthItem::VaseTree(_)) => true,
						("story", TropicalUndergrowthItem::Storybook(_)) => true,
						(
							"torch",
							TropicalUndergrowthItem::PenmarchTorch(_)
							| TropicalUndergrowthItem::KamakuraTorch(_)
							| TropicalUndergrowthItem::TorchTree(_),
						) => true,
						_ => false,
					})
				})
				.map(|b| b.weight)
				.sum()
		};
		let tuft = weight("tuft");
		let palm = weight("palm");
		let rory = weight("rory");
		let vase = weight("vase");
		let story = weight("story");
		let torch = weight("torch");
		assert!((tuft - 3.5).abs() < 1e-4, "expected tuft weight 3.5, got {tuft}");
		assert!((palm - 1.0).abs() < 1e-4, "expected palm weight 1.0, got {palm}");
		assert!((rory - 0.85).abs() < 1e-4, "expected rory weight 0.85, got {rory}");
		assert!((vase - 0.20).abs() < 1e-4, "expected vase weight 0.20, got {vase}");
		assert!((story - 0.15).abs() < 1e-4, "expected story weight 0.15, got {story}");
		assert!((torch - 2.74).abs() < 1e-4, "expected torch weight 2.74, got {torch}");
		Ok(())
	}

	#[test]
	fn tuft_geometry_follows_authored_bands() -> Result<()> {
		let TropicalUndergrowthItem::Tuft(bright) = TropicalUndergrowthCell::BrightTuft.item()
		else {
			anyhow::bail!("expected bright tuft item");
		};
		assert!(bright.height.start >= 0.30);
		assert!(bright.height.end <= 1.50);

		let TropicalUndergrowthItem::Tuft(deep) = TropicalUndergrowthCell::DeepTuft.item() else {
			anyhow::bail!("expected deep tuft item");
		};
		assert!(deep.height.start >= 0.40);
		assert!(deep.height.end <= 0.90);
		Ok(())
	}

	#[test]
	fn palm_and_mini_tree_geometry_follows_authored_bands() -> Result<()> {
		let TropicalUndergrowthItem::PalmBush(palm) = TropicalUndergrowthCell::SmallPalmBush.item()
		else {
			anyhow::bail!("expected palm item");
		};
		assert!(palm.height.start >= 1.00);
		assert!(palm.height.end <= 2.80);
		assert_eq!(palm.frond_count, 5..=9);

		let TropicalUndergrowthItem::RoryHead(rory) =
			TropicalUndergrowthCell::MiniRoryHeadTrained.item()
		else {
			anyhow::bail!("expected rory item");
		};
		assert!(rory.height.start >= 0.80);
		assert!(rory.height.end <= 1.80);
		assert!(rory.stalk_radius.start >= 0.037);
		assert!(rory.stalk_radius.end <= 0.055);
		assert!(rory.canopy_spread.start >= 0.70);
		assert!(rory.canopy_density.end <= 1.0);

		let TropicalUndergrowthItem::VaseTree(vase) = TropicalUndergrowthCell::MiniVaseTree.item()
		else {
			anyhow::bail!("expected vase item");
		};
		assert!(vase.height.start >= 1.00);
		assert!(vase.height.end <= 2.30);
		assert!(vase.stalk_radius.start >= 0.046);
		assert!(vase.canopy_spread.end <= 2.10);

		let TropicalUndergrowthItem::Storybook(story) =
			TropicalUndergrowthCell::MiniSparseStorybook.item()
		else {
			anyhow::bail!("expected storybook item");
		};
		assert!(story.height.start >= 1.20);
		assert!(story.height.end <= 2.50);
		assert!(story.stalk_radius.end <= 0.063);
		assert!(story.canopy_spread.start >= 0.84);
		Ok(())
	}

	#[test]
	fn patch_wraps_bright_tuft_clump() -> Result<()> {
		let TropicalUndergrowthItem::Patch(patch) = TropicalUndergrowthCell::BrightTuftPatch.item()
		else {
			anyhow::bail!("expected patch item");
		};
		assert_eq!(patch.clump, BRIGHT_TUFT);
		assert!(*patch.clump_count.start() >= 3);
		assert!(patch.patch_extent_xz.start >= 1.0);
		Ok(())
	}

	#[test]
	#[ignore = "placement constraints deferred to forest-layer normalization"]
	fn constraint_first_fit_fallback() -> Result<()> {
		// SmallPalmBush (index 3) rejects steepness 0.65; first-fit falls to MiniRoryHeadTrained
		// (index 4), which allows steepness up to 0.70.
		let prepared = TropicalUndergrowthCell::distribution().prepare(
			0.0,
			0.0,
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.65 };
		let outcome = prepared.select_from(
			3,
			Vec3::new(5.0, 0.35, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, TropicalUndergrowthCell::MiniRoryHeadTrained);
			}
			other => anyhow::bail!("expected MiniRoryHeadTrained fallback, got {other:?}"),
		}
		Ok(())
	}

	#[test]
	fn placements_break_the_cell_grid() -> Result<()> {
		let noise = crate::grove::GroveFrontend::default().noise;
		let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0));
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
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
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
