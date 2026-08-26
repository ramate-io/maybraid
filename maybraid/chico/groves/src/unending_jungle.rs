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
	use std::sync::Arc;

	use super::variants::unending_jungle_banyan::{HonuBanyanSamples, SopeBanyanSamples};
	
	use bevy::prelude::*;
	use chico_sbs_trees::{
		HonuBanyan, JungleStorybookTree, PenmarchTorch, PenmarchTorchParams, QuantizedPlant,
		RorysHeadTrained, RorysHeadTrainedParams, SopesBanyan, StorybookTree, StorybookTreeParams,
		WaialeaPalm, WaialeaPalmParams,
	};
	use chico_vegetation_components::{
		FoliageNode, Placement, StickNode, VegetationComponents,
	};
	use clap::Args;
	#[cfg(test)]
	use bevy::math::bounding::Aabb3d;
	#[cfg(test)]
	use lod::gen::{LodScene, LodSceneLevel};
	use lod::lod_ref::LodRef;
	use lod::SceneChunk;
	use material_ref::MaterialRef;
	use procedural_common::{BuildWithNoise, NoiseParams};

	use super::{
		definition, UnendingJungleCell, LOWER_STORYBOOK, PENUMARCH_ACCENT, RED_JUNGLE_TORCH,
		RORY_ACCENT, SMALL_HONU_BANYAN, SMALL_JUNGLE_STORYBOOK, SMALL_SOPE_BANYAN,
		WAIALEA_PALM_ACCENT,
	};
	use crate::grove::vc_tuft::{patch_variant_index, variant_noise};
	use crate::grove::{
		canopy_ball_material_from_palette, canopy_proxy_rory, canopy_proxy_site,
		canopy_proxy_trunk, canopy_proxy_waialea, foliage_low_canopy_balls, frond_material_from_palette, grove_structural_footprint, nest_flattened_plant_chunk, placed_palm_low_fronds, placement_noise,
		remixed_sbs_plant, stick_material_from_palette, CanopyProxySite, FlatTerrainSample, GroveCellVariant,
		GroveExtent, GrovePreviewParams,
		WoodyGroveLod,
	};

	pub const UNENDING_JUNGLE_STRUCTURAL_HIGH_FACTOR: f32 = 2.0;
	pub const UNENDING_JUNGLE_STRUCTURAL_MEDIUM_FACTOR: f32 = 5.0;
	pub const UNENDING_JUNGLE_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

	const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::rory_trunk(
		UNENDING_JUNGLE_STRUCTURAL_HIGH_FACTOR,
		UNENDING_JUNGLE_STRUCTURAL_MEDIUM_FACTOR,
		UNENDING_JUNGLE_STRUCTURAL_LOW_FACTOR,
	);

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct UnendingJungleParams {
		#[command(flatten)]
		pub preview: GrovePreviewParams<UnendingJungleCell>,
	}

	impl Default for UnendingJungleParams {
		fn default() -> Self {
			Self {
				preview: GrovePreviewParams::default()
					.with_terrain(FlatTerrainSample { elevation: 0.35, steepness: 0.15 }),
			}
		}
	}

	crate::impl_grove_preview_params!(UnendingJungleParams, UnendingJungleCell);

	impl UnendingJungleParams {
		// preview accessors via impl_grove_preview_params!
		pub fn build(&self) -> UnendingJungle {
			self.build_on(&self.terrain)
		}

		/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> UnendingJungle {
			UnendingJungle::from_placements(
				&self.placements_on(world),
				self.grove.noise,
				&self.extent,
				self.tree_variants,
			)
		}
	}

	remixed_sbs_plant!(LowerStorybook, StorybookTree, StorybookTreeParams, LOWER_STORYBOOK);
	remixed_sbs_plant!(PenmarchAccent, PenmarchTorch, PenmarchTorchParams, PENUMARCH_ACCENT);
	remixed_sbs_plant!(RedJungleTorch, PenmarchTorch, PenmarchTorchParams, RED_JUNGLE_TORCH);
	remixed_sbs_plant!(RoryAccent, RorysHeadTrained, RorysHeadTrainedParams, RORY_ACCENT);
	remixed_sbs_plant!(WaialeaPalmAccent, WaialeaPalm, WaialeaPalmParams, WAIALEA_PALM_ACCENT);

	#[derive(Clone)]
	enum UnendingJungleKind {
		Honu(Arc<HonuBanyan>),
		Sope(Arc<SopesBanyan>),
		Storybook(Arc<StorybookTree>),
		JungleStorybook(Arc<JungleStorybookTree>),
		Torch(Arc<PenmarchTorch>),
		Rory(Arc<RorysHeadTrained>),
		Waialea(Arc<WaialeaPalm>),
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
		pub plants: Arc<[UnendingJunglePlant]>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl UnendingJungle {
		pub fn from_placements(
			placements: &[GroveCellVariant<UnendingJungleCell>],
			grove_noise: NoiseParams,
			extent: &GroveExtent,
			tree_variants: u32,
		) -> Self {
			let plants: Arc<[UnendingJunglePlant]> = placements
				.iter()
				.map(|placed| grow_plant(placed, grove_noise, tree_variants))
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
					UnendingJungleKind::Honu(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					UnendingJungleKind::Sope(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					UnendingJungleKind::Storybook(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					UnendingJungleKind::JungleStorybook(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					UnendingJungleKind::Torch(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					UnendingJungleKind::Rory(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					UnendingJungleKind::Waialea(t) => nest_flattened_plant_chunk(
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
				.flat_map(|plant| {
					let material = &plant.ball_material;
					match &plant.kind {
						UnendingJungleKind::Honu(t) => {
							canopy_proxy_site(t, plant.placement, material).into_iter().collect()
						}
						UnendingJungleKind::Sope(t) => {
							canopy_proxy_site(t, plant.placement, material).into_iter().collect()
						}
						UnendingJungleKind::Storybook(t) => {
							canopy_proxy_site(t, plant.placement, material).into_iter().collect()
						}
						UnendingJungleKind::JungleStorybook(t) => {
							canopy_proxy_site(t, plant.placement, material).into_iter().collect()
						}
						UnendingJungleKind::Torch(t) => {
							canopy_proxy_site(t, plant.placement, material).into_iter().collect()
						}
						UnendingJungleKind::Rory(t) => vec![
							canopy_proxy_rory(t, plant.placement, &plant.stick_material, material)
								.crown,
						],
						UnendingJungleKind::Waialea(t) => canopy_proxy_waialea(
							t,
							plant.placement,
							&plant.stick_material,
							material,
						),
					}
				})
				.collect()
		}

		fn foliage_low_nodes(&self) -> Vec<FoliageNode> {
			let mut nodes = Vec::new();
			let mut sites = Vec::new();
			for plant in self.plants.iter() {
				let material = &plant.ball_material;
				match &plant.kind {
					UnendingJungleKind::Honu(t) => {
						if let Some(site) = canopy_proxy_site(t, plant.placement, material) {
							sites.push(site);
						}
					}
					UnendingJungleKind::Sope(t) => {
						if let Some(site) = canopy_proxy_site(t, plant.placement, material) {
							sites.push(site);
						}
					}
					UnendingJungleKind::Storybook(t) => {
						if let Some(site) = canopy_proxy_site(t, plant.placement, material) {
							sites.push(site);
						}
					}
					UnendingJungleKind::JungleStorybook(t) => {
						if let Some(site) = canopy_proxy_site(t, plant.placement, material) {
							sites.push(site);
						}
					}
					UnendingJungleKind::Torch(t) => {
						if let Some(site) = canopy_proxy_site(t, plant.placement, material) {
							sites.push(site);
						}
					}
					UnendingJungleKind::Rory(t) => {
						sites.push(
							canopy_proxy_rory(t, plant.placement, &plant.stick_material, material)
								.crown,
						);
					}
					UnendingJungleKind::Waialea(t) => {
						nodes.extend(placed_palm_low_fronds(
							t.as_ref(),
							plant.placement,
							&plant.stick_material,
							material,
							&plant.frond_material,
						));
						if let Some(trunk) =
							canopy_proxy_trunk(t, plant.placement, &plant.stick_material)
						{
							sites.push(trunk);
						}
					}
				}
			}
			nodes.extend(foliage_low_canopy_balls(sites));
			nodes
		}

		fn proxy_trunks(&self) -> Vec<StickNode> {
			self.plants
				.iter()
				.filter_map(|plant| match &plant.kind {
					UnendingJungleKind::Rory(t) => {
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

	fn grow_plant(
		placed: &GroveCellVariant<UnendingJungleCell>,
		grove_noise: NoiseParams,
		tree_variants: u32,
	) -> UnendingJunglePlant {
		let variant = patch_variant_index(placed.position, tree_variants);
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

		let (kind, world_size) = match placed.variant {
			UnendingJungleCell::SmallHonuBanyan => {
				let build_noise = variant_noise(grove_noise, variant);
				let world_size = BuildWithNoise::<HonuBanyanSamples>::build_with_noise(
					&SMALL_HONU_BANYAN,
					build_noise,
				)
				.geometry
				.scale
				.tree_height;
				(UnendingJungleKind::Honu(HonuBanyan::grow_num(variant).0), world_size)
			}
			UnendingJungleCell::SmallSopeBanyan => {
				let build_noise = variant_noise(grove_noise, variant);
				let world_size = BuildWithNoise::<SopeBanyanSamples>::build_with_noise(
					&SMALL_SOPE_BANYAN,
					build_noise,
				)
				.geometry
				.scale
				.stalk_height;
				(UnendingJungleKind::Sope(SopesBanyan::grow_num(variant).0), world_size)
			}
			UnendingJungleCell::LowerStorybook => {
				let (tree, world_size) = LowerStorybook::grow_num(variant);
				(UnendingJungleKind::Storybook(tree), world_size)
			}
			UnendingJungleCell::SmallJungleStorybook => {
				let build_noise = variant_noise(grove_noise, variant);
				let world_size =
					SMALL_JUNGLE_STORYBOOK.build_with_noise(build_noise).geometry.height();
				(
					UnendingJungleKind::JungleStorybook(JungleStorybookTree::grow_num(variant).0),
					world_size,
				)
			}
			UnendingJungleCell::PenmarchAccent => {
				let (tree, world_size) = PenmarchAccent::grow_num(variant);
				(UnendingJungleKind::Torch(tree), world_size)
			}
			UnendingJungleCell::RedJungleTorch => {
				let (tree, world_size) = RedJungleTorch::grow_num(variant);
				(UnendingJungleKind::Torch(tree), world_size)
			}
			UnendingJungleCell::RoryAccent => {
				let (tree, world_size) = RoryAccent::grow_num(variant);
				(UnendingJungleKind::Rory(tree), world_size)
			}
			UnendingJungleCell::WaialeaPalmAccent => {
				let (tree, world_size) = WaialeaPalmAccent::grow_num(variant);
				(UnendingJungleKind::Waialea(tree), world_size)
			}
		};

		UnendingJunglePlant {
			placement: Placement::new(placed.position, 0.0)
				.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
			kind,
			stick_material,
			ball_material,
			frond_material,
		}
	}

	crate::impl_woody_grove_lod!(UnendingJungle, WOODY_LOD, trunks, low_nodes);

	#[cfg(test)]
	mod tests {
		use super::*;
		use anyhow::Result;

		fn small_grove() -> UnendingJungle {
			UnendingJungleParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0)))
				.build()
		}

		fn plant_height(plant: &UnendingJunglePlant) -> f32 {
			match &plant.kind {
				UnendingJungleKind::Honu(t) => t.geometry.scale.tree_height,
				UnendingJungleKind::Sope(t) => t.geometry.scale.stalk_height,
				UnendingJungleKind::Storybook(t) => t.geometry.height(),
				UnendingJungleKind::JungleStorybook(t) => t.geometry.height(),
				UnendingJungleKind::Torch(t) => t.geometry.height(),
				UnendingJungleKind::Rory(t) => t.geometry.height(),
				UnendingJungleKind::Waialea(t) => t.geometry.height(),
			}
		}

		fn plant_seed(plant: &UnendingJunglePlant) -> i32 {
			match &plant.kind {
				UnendingJungleKind::Honu(t) => t.geometry.canopy_noise.seed,
				UnendingJungleKind::Sope(t) => t.geometry.canopy_noise.seed,
				UnendingJungleKind::Storybook(t) => t.geometry.canopy_noise.seed,
				UnendingJungleKind::JungleStorybook(t) => t.geometry.canopy_noise.seed,
				UnendingJungleKind::Torch(t) => t.geometry.canopy_noise.seed,
				UnendingJungleKind::Rory(t) => t.geometry.canopy_noise.seed,
				UnendingJungleKind::Waialea(t) => t.geometry.trunk_noise.seed,
			}
		}

		#[test]
		fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
			let grove = small_grove();
			assert!(!grove.plants.is_empty(), "expected placed unending-jungle plants");

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
				anyhow::bail!("High unending-jungle should wrap plant chunks");
			};
			assert_eq!(parts.len(), 1, "expected one lazy plant producer");
			let lod::SceneChunk::Lazy { remaining_primitives, remaining_weight, .. } = &parts[0]
			else {
				anyhow::bail!("High unending-jungle plants should be SceneChunk::Lazy");
			};
			assert_eq!(*remaining_primitives, grove.plants.len());
			assert_eq!(*remaining_weight as usize, grove.plants.len());

			assert!(grove.stick_nodes_for_level(LodSceneLevel::Low).len() <= 1);
			let low_foliage = grove.foliage_nodes_for_level(LodSceneLevel::Low).flatten();
			let palms = grove
				.plants
				.iter()
				.filter(|p| matches!(p.kind, UnendingJungleKind::Waialea(_)))
				.count();
			let fronds = low_foliage.iter().filter(|n| n.geometry.is_frond_collection()).count();
			assert_eq!(fronds, palms * 5);
			assert!(!grove.foliage_nodes_for_level(LodSceneLevel::UltraLow).flatten().is_empty());
			match grove.scene_chunks_with_level(&lod_ref, LodSceneLevel::Low) {
				lod::SceneChunk::Primitive { weight, .. } => {
					assert_eq!(weight, chico_vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT);
				}
				lod::SceneChunk::SubChunks(parts) => assert!(!parts.is_empty()),
				_ => anyhow::bail!("Low unending-jungle should emit flattened kits"),
			}
			Ok(())
		}

		#[test]
		fn tree_variants_quantize_archetypes() -> Result<()> {
			use std::collections::HashSet;

			let mut params = UnendingJungleParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0)));
			params.tree_variants = 4;
			let grove = params.build();
			assert!(!grove.plants.is_empty(), "expected placed unending-jungle plants");
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
