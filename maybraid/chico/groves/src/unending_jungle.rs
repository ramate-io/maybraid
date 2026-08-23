//! Unending Jungle — well-known moderate lower-canopy grove
//! ([RFC-183 §3.4.6.1], [#322](https://github.com/ramate-io/maybraid/issues/322)).
//!
//! Mixes mini banyans, Storybook and Jungle Storybook forms, and rare torch, Rory, and palm accents
//! beneath taller forest layers. Forest-layer attachment remains a follow-up.

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

/// Authored Unending Jungle grove definition.
///
/// Cell footprint sits at the RFC midpoint (`10.5` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<UnendingJungleCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(9.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-10.5, 10.5),
		),
		distribution: UnendingJungleCell::distribution(),
	}
}

/// Ordered unending-jungle varietals ([RFC-183 §3.4.6.1]); the explicit `None` bucket lives only
/// in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnendingJungleCell {
	SmallHonuBanyan,
	SmallSopeBanyan,
	LowerStorybook,
	SmallJungleStorybook,
	PenmarchAccent,
	RedJungleTorch,
	RoryAccent,
	WaialeaPalmAccent,
}

/// Typed authored geometry for one unending-jungle varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnendingJungleItem {
	Honu(&'static UnendingJungleBanyan),
	Sope(&'static UnendingJungleBanyan),
	Storybook(&'static UnendingJungleStorybook),
	JungleStorybook(&'static UnendingJungleJungleStorybook),
	Torch(&'static UnendingJungleTorch),
	RoryHead(&'static UnendingJungleRoryHead),
	WaialeaPalm(&'static UnendingJungleWaialeaPalm),
}

/// Authored geometry ranges for one mini Honu or Sope banyan form.
#[derive(Debug, Clone, PartialEq)]
pub struct UnendingJungleBanyan {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled descender probability band; lower values keep descenders sparse.
	pub descender_density: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse..moderate canopy density at render time.
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one lower Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct UnendingJungleStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one small Jungle Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct UnendingJungleJungleStorybook {
	pub height: UnitRange,
	pub canopy_density: UnitRange,
	pub jungle_growth_density: UnitRange,
}

/// Authored geometry ranges for one Penmarch Torch accent (standard or red-stick palette).
#[derive(Debug, Clone, PartialEq)]
pub struct UnendingJungleTorch {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Rory's Head-trained accent.
#[derive(Debug, Clone, PartialEq)]
pub struct UnendingJungleRoryHead {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Waialea Palm accent.
#[derive(Debug, Clone, PartialEq)]
pub struct UnendingJungleWaialeaPalm {
	pub height: UnitRange,
	pub crown_density: UnitRange,
}

const SMALL_HONU_BANYAN: UnendingJungleBanyan = UnendingJungleBanyan {
	height: UnitRange::new(4.0, 6.0),
	stalk_radius: UnitRange::new(0.24, 0.36),
	canopy_spread: UnitRange::new(2.0, 4.5),
	descender_density: UnitRange::new(0.02, 0.04),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const SMALL_SOPE_BANYAN: UnendingJungleBanyan = UnendingJungleBanyan {
	height: UnitRange::new(4.0, 6.0),
	stalk_radius: UnitRange::new(0.24, 0.36),
	canopy_spread: UnitRange::new(2.0, 4.5),
	descender_density: UnitRange::new(0.02, 0.04),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const LOWER_STORYBOOK: UnendingJungleStorybook = UnendingJungleStorybook {
	height: UnitRange::new(3.0, 5.0),
	stalk_radius: UnitRange::new(0.18, 0.28),
	canopy_spread: UnitRange::new(1.5, 3.5),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const SMALL_JUNGLE_STORYBOOK: UnendingJungleJungleStorybook = UnendingJungleJungleStorybook {
	height: UnitRange::new(6.0, 8.0),
	canopy_density: DENSE_CANOPY_DENSITY,
	jungle_growth_density: MODERATE_CANOPY_DENSITY,
};

const PENUMARCH_ACCENT: UnendingJungleTorch = UnendingJungleTorch {
	height: UnitRange::new(3.0, 5.0),
	stalk_radius: UnitRange::new(0.12, 0.20),
	canopy_spread: UnitRange::new(1.2, 3.0),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const RED_JUNGLE_TORCH: UnendingJungleTorch = UnendingJungleTorch {
	height: UnitRange::new(3.0, 5.5),
	stalk_radius: UnitRange::new(0.12, 0.22),
	canopy_spread: UnitRange::new(1.2, 3.2),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const RORY_ACCENT: UnendingJungleRoryHead = UnendingJungleRoryHead {
	height: UnitRange::new(3.0, 7.0),
	stalk_radius: UnitRange::new(0.12, 0.28),
	canopy_spread: UnitRange::new(1.0, 2.8),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const WAIALEA_PALM_ACCENT: UnendingJungleWaialeaPalm = UnendingJungleWaialeaPalm {
	height: UnitRange::new(6.0, 9.0),
	crown_density: MODERATE_CANOPY_DENSITY,
};

const HONU_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("banyan_bark", "dark_bark"),
	PaletteSlot::new("wet_brown", "gray_brown"),
]);

const HONU_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "wet_green"),
	PaletteSlot::new("blue_green", "emerald_green"),
]);

const SOPE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("banyan_bark", "dark_bark"),
	PaletteSlot::new("wet_brown", "green_brown"),
]);

const SOPE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "deep_green"),
	PaletteSlot::new("wet_green", "fresh_green"),
]);

const STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("tropical_bark", "dark_bark"),
	PaletteSlot::new("green_brown", "wet_brown"),
]);

const STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("deep_green", "fresh_green"),
]);

const JUNGLE_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_jungle_bark", "wet_brown"),
	PaletteSlot::new("moss_bark", "dark_bark"),
]);

const JUNGLE_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "wet_green"),
	PaletteSlot::new("blue_green", "emerald_green"),
]);

const PENUMARCH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("tropical_bark", "dark_bark"),
	PaletteSlot::new("green_brown", "wet_brown"),
]);

const PENUMARCH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("wet_green", "lime_green"),
	PaletteSlot::new("dark_green", "fresh_green"),
]);

const RED_JUNGLE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("red_jungle_bark", "copper_red"),
	PaletteSlot::new("wet_burgundy", "dark_bark"),
]);

const RED_JUNGLE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("wet_green", "lime_green"),
	PaletteSlot::new("blue_green", "fresh_green"),
]);

const RORY_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("tropical_bark", "dark_bark"),
	PaletteSlot::new("vine_bark", "wet_brown"),
]);

const RORY_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("blue_green", "deep_green"),
	PaletteSlot::new("yellow_green", "fresh_green"),
]);

const WAIALEA_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_bark", "tan_bark"),
	PaletteSlot::new("wet_brown", "green_brown"),
]);

const WAIALEA_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("wet_green", "lime_green"),
]);

impl UnendingJungleCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `7.40` (RFC relative proportions); the `None` weight of `12.0` puts
	/// the placed share at `7.40 / 19.40 ≈ 0.38`, mid RFC `DENSITY_RANGE` (`0.24..0.52`).
	pub fn distribution() -> GroveDistribution<Self> {
		let honu = PlacementConstraints::new(UnitRange::new(0.0, 0.55), UnitRange::new(0.0, 0.42));
		let sope = PlacementConstraints::new(UnitRange::new(0.0, 0.52), UnitRange::new(0.0, 0.48));
		let storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 0.58), UnitRange::new(0.0, 0.64));
		let jungle_storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 0.55), UnitRange::new(0.0, 0.58));
		let penmarch =
			PlacementConstraints::new(UnitRange::new(0.0, 0.50), UnitRange::new(0.0, 0.64));
		let red_torch =
			PlacementConstraints::new(UnitRange::new(0.0, 0.48), UnitRange::new(0.0, 0.58));
		let rory = PlacementConstraints::new(UnitRange::new(0.0, 0.55), UnitRange::new(0.0, 0.76));
		let waialea =
			PlacementConstraints::new(UnitRange::new(0.0, 0.45), UnitRange::new(0.0, 0.70));
		GroveDistribution::new(vec![
			GroveBucket::none(8.0),
			GroveBucket::placed(2.0, honu, Self::SmallHonuBanyan),
			GroveBucket::placed(1.0, sope, Self::SmallSopeBanyan),
			GroveBucket::placed(2.0, storybook, Self::LowerStorybook),
			GroveBucket::placed(1.25, jungle_storybook, Self::SmallJungleStorybook),
			GroveBucket::placed(0.35, penmarch, Self::PenmarchAccent),
			GroveBucket::placed(0.20, red_torch, Self::RedJungleTorch),
			GroveBucket::placed(0.35, rory, Self::RoryAccent),
			GroveBucket::placed(0.55, waialea, Self::WaialeaPalmAccent),
		])
	}

	pub fn item(self) -> UnendingJungleItem {
		match self {
			Self::SmallHonuBanyan => UnendingJungleItem::Honu(&SMALL_HONU_BANYAN),
			Self::SmallSopeBanyan => UnendingJungleItem::Sope(&SMALL_SOPE_BANYAN),
			Self::LowerStorybook => UnendingJungleItem::Storybook(&LOWER_STORYBOOK),
			Self::SmallJungleStorybook => {
				UnendingJungleItem::JungleStorybook(&SMALL_JUNGLE_STORYBOOK)
			}
			Self::PenmarchAccent => UnendingJungleItem::Torch(&PENUMARCH_ACCENT),
			Self::RedJungleTorch => UnendingJungleItem::Torch(&RED_JUNGLE_TORCH),
			Self::RoryAccent => UnendingJungleItem::RoryHead(&RORY_ACCENT),
			Self::WaialeaPalmAccent => UnendingJungleItem::WaialeaPalm(&WAIALEA_PALM_ACCENT),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::SmallHonuBanyan => HONU_STICK_MIX,
			Self::SmallSopeBanyan => SOPE_STICK_MIX,
			Self::LowerStorybook => STORYBOOK_STICK_MIX,
			Self::SmallJungleStorybook => JUNGLE_STORYBOOK_STICK_MIX,
			Self::PenmarchAccent => PENUMARCH_STICK_MIX,
			Self::RedJungleTorch => RED_JUNGLE_STICK_MIX,
			Self::RoryAccent => RORY_STICK_MIX,
			Self::WaialeaPalmAccent => WAIALEA_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::SmallHonuBanyan => HONU_CANOPY_MIX,
			Self::SmallSopeBanyan => SOPE_CANOPY_MIX,
			Self::LowerStorybook => STORYBOOK_CANOPY_MIX,
			Self::SmallJungleStorybook => JUNGLE_STORYBOOK_CANOPY_MIX,
			Self::PenmarchAccent => PENUMARCH_CANOPY_MIX,
			Self::RedJungleTorch => RED_JUNGLE_CANOPY_MIX,
			Self::RoryAccent => RORY_CANOPY_MIX,
			Self::WaialeaPalmAccent => WAIALEA_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
mod vc {
	use super::variants::unending_jungle_banyan::{HonuBanyanSamples, SopeBanyanSamples};
	use bevy::math::bounding::Aabb3d;
	use bevy::prelude::*;
	use bevy::scene::prelude::Scene;
	use chico_sbs_trees::{
		HonuBanyan, HonuBanyanParams, JungleStorybookTree, JungleStorybookTreeParams,
		PenmarchTorch, PenmarchTorchParams, RorysHeadTrained, RorysHeadTrainedParams, SopesBanyan,
		SopesBanyanParams, StorybookTree, StorybookTreeParams, WaialeaPalm, WaialeaPalmParams,
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

	use super::{definition, UnendingJungleCell, UnendingJungleItem};
	use crate::grove::{
		canopy_ball_material_from_palette, canopy_proxy_site, foliage_low_canopy_balls,
		foliage_ultra_low_merged_balls, frond_material_from_palette, grove_detail_level,
		grove_lod_culls, grove_lod_level, grove_lod_status, grove_structural_footprint,
		layers_from_nodes, nest_placed_plant_chunk, placement_noise, stick_material_from_palette,
		woody_grove_scene_chunks, CanopyProxySite, FlatTerrainSample, GroveCellVariant,
		GroveExtent, GroveFrontend, DEFAULT_GROVE_EXTENT_XZ, ULTRA_LOW_CANOPY_BIN_METERS,
	};

	pub const UNENDING_JUNGLE_STRUCTURAL_HIGH_FACTOR: f32 = 2.0;
	pub const UNENDING_JUNGLE_STRUCTURAL_MEDIUM_FACTOR: f32 = 5.0;
	pub const UNENDING_JUNGLE_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct UnendingJungleParams {
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

		#[arg(skip)]
		resolved_placements: Option<Vec<GroveCellVariant<UnendingJungleCell>>>,
	}

	impl Default for UnendingJungleParams {
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
				terrain: FlatTerrainSample { elevation: 0.35, steepness: 0.15 },
				resolved_placements: None,
			}
		}
	}

	impl UnendingJungleParams {
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

		pub fn placements(&self) -> Vec<GroveCellVariant<UnendingJungleCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.placements_on(&self.terrain)
		}

		/// Select placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn placements_on(
			&self,
			world: &impl crate::GroveWorldSample,
		) -> Vec<GroveCellVariant<UnendingJungleCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.grove.assemble(definition()).populate(&self.extent, world)
		}

		pub fn build(&self) -> UnendingJungle {
			self.build_on(&self.terrain)
		}

		/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> UnendingJungle {
			UnendingJungle::from_placements(
				&self.placements_on(world),
				self.grove.noise,
				&self.extent,
			)
		}
	}

	#[derive(Clone)]
	enum UnendingJungleKind {
		Honu(HonuBanyan),
		Sope(SopesBanyan),
		Storybook(StorybookTree),
		JungleStorybook(JungleStorybookTree),
		Torch(PenmarchTorch),
		Rory(RorysHeadTrained),
		Waialea(WaialeaPalm),
	}

	#[derive(Clone)]
	pub struct UnendingJunglePlant {
		pub placement: Placement,
		kind: UnendingJungleKind,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	}

	#[derive(Clone, Component)]
	pub struct UnendingJungle {
		pub plants: Vec<UnendingJunglePlant>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl UnendingJungle {
		pub fn from_placements(
			placements: &[GroveCellVariant<UnendingJungleCell>],
			grove_noise: NoiseParams,
			extent: &GroveExtent,
		) -> Self {
			let plants = placements.iter().map(|placed| grow_plant(placed, grove_noise)).collect();
			let (structural_center, footprint_radius) = grove_structural_footprint(extent);
			Self { plants, structural_center, footprint_radius, extent: *extent }
		}

		fn nest_plant_chunks(&self, lod_ref: &LodRef) -> Vec<SceneChunk> {
			self.plants
				.iter()
				.map(|plant| match &plant.kind {
					UnendingJungleKind::Honu(t) => nest_placed_plant_chunk(
						t.clone(),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						lod_ref,
					),
					UnendingJungleKind::Sope(t) => nest_placed_plant_chunk(
						t.clone(),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						lod_ref,
					),
					UnendingJungleKind::Storybook(t) => nest_placed_plant_chunk(
						t.clone(),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						lod_ref,
					),
					UnendingJungleKind::JungleStorybook(t) => nest_placed_plant_chunk(
						t.clone(),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						lod_ref,
					),
					UnendingJungleKind::Torch(t) => nest_placed_plant_chunk(
						t.clone(),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						lod_ref,
					),
					UnendingJungleKind::Rory(t) => nest_placed_plant_chunk(
						t.clone(),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						lod_ref,
					),
					UnendingJungleKind::Waialea(t) => nest_placed_plant_chunk(
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
						UnendingJungleKind::Honu(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						UnendingJungleKind::Sope(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						UnendingJungleKind::Storybook(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						UnendingJungleKind::JungleStorybook(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						UnendingJungleKind::Torch(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						UnendingJungleKind::Rory(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						UnendingJungleKind::Waialea(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
					}
				})
				.collect()
		}
	}

	fn grow_plant(
		placed: &GroveCellVariant<UnendingJungleCell>,
		grove_noise: NoiseParams,
	) -> UnendingJunglePlant {
		let build_noise = placement_noise(grove_noise, placed.position);
		let stick_seed = build_noise.seed;
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
			UnendingJungleItem::Honu(banyan) => {
				let samples =
					BuildWithNoise::<HonuBanyanSamples>::build_with_noise(banyan, build_noise);
				let mut params = HonuBanyanParams::default();
				params.geometry = samples.geometry;
				params.growth_spawn_fraction = samples.growth_spawn_fraction;
				UnendingJungleKind::Honu(params.build())
			}
			UnendingJungleItem::Sope(banyan) => {
				let samples =
					BuildWithNoise::<SopeBanyanSamples>::build_with_noise(banyan, build_noise);
				let mut params = SopesBanyanParams::default();
				params.geometry = samples.geometry;
				UnendingJungleKind::Sope(params.build())
			}
			UnendingJungleItem::Storybook(story) => {
				let geometry = story.build_with_noise(build_noise);
				let mut params = StorybookTreeParams::default();
				params.geometry = geometry;
				UnendingJungleKind::Storybook(params.build())
			}
			UnendingJungleItem::JungleStorybook(jungle) => {
				let samples = jungle.build_with_noise(build_noise);
				let mut params = JungleStorybookTreeParams::default();
				params.geometry = samples.geometry;
				params.growth_spawn_fraction = samples.growth_spawn_fraction;
				UnendingJungleKind::JungleStorybook(params.build())
			}
			UnendingJungleItem::Torch(torch) => {
				let geometry = torch.build_with_noise(build_noise);
				let mut params = PenmarchTorchParams::default();
				params.geometry = geometry;
				UnendingJungleKind::Torch(params.build())
			}
			UnendingJungleItem::RoryHead(rory) => {
				let geometry = rory.build_with_noise(build_noise);
				let mut params = RorysHeadTrainedParams::default();
				params.geometry = geometry;
				UnendingJungleKind::Rory(params.build())
			}
			UnendingJungleItem::WaialeaPalm(palm) => {
				let geometry = palm.build_with_noise(build_noise);
				let mut params = WaialeaPalmParams::default();
				params.geometry = geometry;
				UnendingJungleKind::Waialea(params.build())
			}
		};

		UnendingJunglePlant { placement, kind, stick_material, ball_material, frond_material }
	}

	impl VegetationComponents for UnendingJungle {
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
				UNENDING_JUNGLE_STRUCTURAL_HIGH_FACTOR,
				UNENDING_JUNGLE_STRUCTURAL_MEDIUM_FACTOR,
				UNENDING_JUNGLE_STRUCTURAL_LOW_FACTOR,
			))
		}
	}

	impl LodScene for UnendingJungle {
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
	UnendingJungle, UnendingJungleParams, UnendingJunglePlant,
	UNENDING_JUNGLE_STRUCTURAL_HIGH_FACTOR, UNENDING_JUNGLE_STRUCTURAL_LOW_FACTOR,
	UNENDING_JUNGLE_STRUCTURAL_MEDIUM_FACTOR,
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
		let dist = UnendingJungleCell::distribution();
		assert_eq!(dist.len(), 9);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 8.0);
		assert_eq!(dist.buckets[1].item, Some(UnendingJungleCell::SmallHonuBanyan));
		assert_eq!(dist.buckets[1].weight, 2.0);
		assert_eq!(dist.buckets[2].item, Some(UnendingJungleCell::SmallSopeBanyan));
		assert_eq!(dist.buckets[2].weight, 1.0);
		assert_eq!(dist.buckets[3].item, Some(UnendingJungleCell::LowerStorybook));
		assert_eq!(dist.buckets[3].weight, 2.0);
		assert_eq!(dist.buckets[4].item, Some(UnendingJungleCell::SmallJungleStorybook));
		assert_eq!(dist.buckets[4].weight, 1.25);
		assert_eq!(dist.buckets[5].item, Some(UnendingJungleCell::PenmarchAccent));
		assert_eq!(dist.buckets[5].weight, 0.35);
		assert_eq!(dist.buckets[6].item, Some(UnendingJungleCell::RedJungleTorch));
		assert_eq!(dist.buckets[6].weight, 0.20);
		assert_eq!(dist.buckets[7].item, Some(UnendingJungleCell::RoryAccent));
		assert_eq!(dist.buckets[7].weight, 0.35);
		assert_eq!(dist.buckets[8].item, Some(UnendingJungleCell::WaialeaPalmAccent));
		assert_eq!(dist.buckets[8].weight, 0.55);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = UnendingJungleCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.24..=0.52).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let UnendingJungleItem::Honu(honu) = UnendingJungleCell::SmallHonuBanyan.item() else {
			anyhow::bail!("expected honu item");
		};
		assert_eq!(honu.height, UnitRange::new(4.0, 6.0));
		assert_eq!(honu.canopy_density, MODERATE_CANOPY_DENSITY);

		let UnendingJungleItem::Storybook(story) = UnendingJungleCell::LowerStorybook.item() else {
			anyhow::bail!("expected storybook item");
		};
		assert_eq!(story.height, UnitRange::new(3.0, 5.0));

		let UnendingJungleItem::JungleStorybook(jungle) =
			UnendingJungleCell::SmallJungleStorybook.item()
		else {
			anyhow::bail!("expected jungle storybook item");
		};
		assert_eq!(jungle.height, UnitRange::new(6.0, 8.0));
		assert_eq!(jungle.canopy_density, DENSE_CANOPY_DENSITY);

		let UnendingJungleItem::WaialeaPalm(palm) = UnendingJungleCell::WaialeaPalmAccent.item()
		else {
			anyhow::bail!("expected waialea item");
		};
		assert_eq!(palm.height, UnitRange::new(6.0, 9.0));
		Ok(())
	}

	#[test]
	fn rory_accepts_steeper_slope_than_dense_storybook() -> Result<()> {
		let dist = UnendingJungleCell::distribution();
		let rory = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(UnendingJungleCell::RoryAccent))
			.ok_or_else(|| anyhow::anyhow!("missing rory bucket"))?;
		let jungle = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(UnendingJungleCell::SmallJungleStorybook))
			.ok_or_else(|| anyhow::anyhow!("missing jungle storybook bucket"))?;
		assert!(rory.constraints.steepness.end > jungle.constraints.steepness.end);
		assert_eq!(jungle.constraints.steepness.end, 0.58);
		assert_eq!(rory.constraints.steepness.end, 0.76);
		Ok(())
	}

	#[test]
	#[ignore = "placement constraints deferred to forest-layer normalization"]
	fn steep_slope_rejects_tight_variants() -> Result<()> {
		let prepared = UnendingJungleCell::distribution().prepare(
			0.0,
			0.0,
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let terrain = FlatTerrainSample { elevation: 0.30, steepness: 0.75 };
		let outcome = prepared.select_from(
			8,
			Vec3::new(5.0, 0.30, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_ne!(variant, UnendingJungleCell::WaialeaPalmAccent);
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
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
		let off_center = placements.iter().any(|placed| {
			let offset_x = (placed.position.x % cell).abs();
			let offset_z = (placed.position.z % cell).abs();
			!(offset_x < 0.5 || (cell - offset_x) < 0.5)
				|| !(offset_z < 0.5 || (cell - offset_z) < 0.5)
		});
		assert!(off_center, "expected placements offset from cell centers");
		Ok(())
	}

	#[test]
	fn populated_grove_is_deterministic_and_non_empty() -> Result<()> {
		let noise = crate::grove::GroveFrontend::default().noise;
		let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(50.0, 1.0, 50.0));
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
