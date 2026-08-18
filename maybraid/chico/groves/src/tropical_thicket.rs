//! Tropical Thicket — well-known dense tropical understory grove
//! ([RFC-183 §3.4.5.6], [#317](https://github.com/ramate-io/maybraid/issues/317)).
//!
//! Mixes larger palm bushes, moderate Common High Bush forms, and rare mini Honu Banyan accents.
//! Forest-layer attachment remains a follow-up.

use std::ops::RangeInclusive;

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

/// RFC `projection_count: Moderate` with extended upper tails for occasional wide-span shrubs.
const MODERATE_PROJECTION_RADIAL: UnitRange = UnitRange::new(0.32, 0.56);
const MODERATE_PROJECTION_VERTICAL: UnitRange = UnitRange::new(0.50, 0.82);
/// Stick segment reach as a fraction of shoot height; upper tail exceeds the generic bush default.
const MODERATE_SEGMENT_LENGTH: UnitRange = UnitRange::new(0.08, 0.24);
const FLOWERING_SEGMENT_LENGTH: UnitRange = UnitRange::new(0.08, 0.22);

/// Authored Tropical Thicket grove definition.
///
/// Cell footprint sits at the RFC midpoint (`6.5` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<TropicalThicketCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(6.5),
		placement: GrovePlacementRanges::new(UnitRange::new(0.85, 1.15), UnitRange::new(-6.5, 6.5)),
		distribution: TropicalThicketCell::distribution(),
	}
}

/// Ordered tropical-thicket varietals ([RFC-183 §3.4.5.6]); the explicit `None` bucket lives only
/// in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TropicalThicketCell {
	LargePalmBush,
	BroadWetPalmBush,
	MiniHonuBanyan,
	ModerateHighBush,
	FloweringHighBush,
	RedStemPalmBush,
}

/// Typed authored geometry for one tropical-thicket varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TropicalThicketItem {
	Palm(&'static TropicalThicketPalm),
	Banyan(&'static TropicalThicketBanyan),
	Bush(&'static TropicalThicketBush),
}

/// Authored geometry ranges for one ground-anchored palm bush.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalThicketPalm {
	pub height: UnitRange,
	pub frond_count: RangeInclusive<u32>,
	pub frond_length: UnitRange,
	pub crown_spread: UnitRange,
}

/// Authored geometry ranges for one mini Honu Banyan form.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalThicketBanyan {
	pub height: UnitRange,
	/// World-space stalk base radius (RFC mini form `0.2` m at mid height).
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled descender probability band; lower values keep descenders sparse.
	pub descender_density: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse..moderate canopy density at render time.
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Common High Bush shrub.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalThicketBush {
	pub height: UnitRange,
	pub shoot_count: RangeInclusive<u32>,
	pub branch_depth: RangeInclusive<u32>,
	pub radial_strength: UnitRange,
	pub vertical_bias: UnitRange,
	/// Per-segment stick length sampled as a fraction of shoot height.
	pub segment_length_fraction: UnitRange,
	/// Terminal foliage radius in **world meters** (render converts to a height fraction).
	pub leaf_radius: UnitRange,
}

const LARGE_PALM_BUSH: TropicalThicketPalm = TropicalThicketPalm {
	height: UnitRange::new(3.00, 6.60),
	frond_count: 7..=12,
	frond_length: UnitRange::new(1.65, 4.50),
	crown_spread: UnitRange::new(2.40, 6.30),
};

const BROAD_WET_PALM_BUSH: TropicalThicketPalm = TropicalThicketPalm {
	height: UnitRange::new(3.60, 7.80),
	frond_count: 8..=14,
	frond_length: UnitRange::new(2.10, 5.25),
	crown_spread: UnitRange::new(3.00, 7.80),
};

const RED_STEM_PALM_BUSH: TropicalThicketPalm = TropicalThicketPalm {
	height: UnitRange::new(3.00, 6.90),
	frond_count: 6..=11,
	frond_length: UnitRange::new(1.65, 4.35),
	crown_spread: UnitRange::new(2.40, 6.30),
};

const MINI_HONU_BANYAN: TropicalThicketBanyan = TropicalThicketBanyan {
	height: UnitRange::new(1.80, 3.80),
	stalk_radius: UnitRange::new(0.14, 0.30),
	canopy_spread: UnitRange::new(1.20, 3.40),
	descender_density: UnitRange::new(0.02, 0.04),
	canopy_density: UnitRange::new(0.35, 0.65),
};

const MODERATE_HIGH_BUSH: TropicalThicketBush = TropicalThicketBush {
	height: UnitRange::new(1.20, 2.40),
	shoot_count: 7..=11,
	branch_depth: 2..=5,
	radial_strength: MODERATE_PROJECTION_RADIAL,
	vertical_bias: MODERATE_PROJECTION_VERTICAL,
	segment_length_fraction: MODERATE_SEGMENT_LENGTH,
	leaf_radius: UnitRange::new(0.06, 0.15),
};

const FLOWERING_HIGH_BUSH: TropicalThicketBush = TropicalThicketBush {
	height: UnitRange::new(1.00, 2.20),
	shoot_count: 7..=10,
	branch_depth: 2..=5,
	radial_strength: MODERATE_PROJECTION_RADIAL,
	vertical_bias: MODERATE_PROJECTION_VERTICAL,
	segment_length_fraction: FLOWERING_SEGMENT_LENGTH,
	leaf_radius: UnitRange::new(0.06, 0.14),
};

const LARGE_PALM_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_bark", "tan_bark"),
	PaletteSlot::new("green_stem", "wet_brown"),
]);

const LARGE_PALM_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("wet_green", "lime_green"),
]);

const BROAD_WET_PALM_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_bark", "dark_bark"),
	PaletteSlot::new("wet_brown", "green_brown"),
]);

const BROAD_WET_PALM_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("blue_green", "deep_green"),
	PaletteSlot::new("emerald_green", "wet_green"),
	PaletteSlot::new("yellow_green", "fresh_green"),
]);

const RED_STEM_PALM_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("red_palm_stem", "copper_red"),
	PaletteSlot::new("wet_burgundy", "dark_bark"),
]);

const RED_STEM_PALM_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "bright_green"),
	PaletteSlot::new("lime_green", "fresh_green"),
	PaletteSlot::new("blue_green", "wet_green"),
]);

const HONU_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("banyan_bark", "dark_bark"),
	PaletteSlot::new("wet_brown", "gray_brown"),
]);

const HONU_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "deep_green"),
	PaletteSlot::new("wet_green", "blue_green"),
	PaletteSlot::new("emerald_green", "fresh_green"),
]);

const MODERATE_BUSH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("shrub_bark", "green_brown"),
	PaletteSlot::new("dark_bark", "wet_brown"),
]);

const MODERATE_BUSH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("blue_green", "light_green"),
]);

const FLOWERING_BUSH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("shrub_bark", "dark_bark"),
	PaletteSlot::new("green_brown", "wet_brown"),
]);

const FLOWERING_BUSH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "leaf_green"),
	PaletteSlot::new("flower_white", "fresh_green"),
	PaletteSlot::new("flower_yellow", "lime_green"),
]);

impl TropicalThicketCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `5.25` (RFC relative proportions); the `None` weight of `7.0` puts
	/// the placed share at `5.25 / 12.25 ≈ 0.43`, mid RFC `DENSITY_RANGE` (`0.24..0.62`).
	pub fn distribution() -> GroveDistribution<Self> {
		let gentle =
			PlacementConstraints::new(UnitRange::new(0.0, 0.45), UnitRange::new(0.0, 0.28));
		let wet_palm =
			PlacementConstraints::new(UnitRange::new(0.0, 0.45), UnitRange::new(0.0, 0.68));
		let flowering =
			PlacementConstraints::new(UnitRange::new(0.0, 0.45), UnitRange::new(0.0, 0.78));
		let red_palm =
			PlacementConstraints::new(UnitRange::new(0.0, 0.42), UnitRange::new(0.0, 0.60));
		GroveDistribution::new(vec![
			GroveBucket::none(7.0),
			GroveBucket::placed(2.0, gentle, Self::LargePalmBush),
			GroveBucket::placed(1.25, wet_palm, Self::BroadWetPalmBush),
			GroveBucket::placed(0.45, gentle, Self::MiniHonuBanyan),
			GroveBucket::placed(1.0, gentle, Self::ModerateHighBush),
			GroveBucket::placed(0.30, flowering, Self::FloweringHighBush),
			GroveBucket::placed(0.25, red_palm, Self::RedStemPalmBush),
		])
	}

	pub fn item(self) -> TropicalThicketItem {
		match self {
			Self::LargePalmBush => TropicalThicketItem::Palm(&LARGE_PALM_BUSH),
			Self::BroadWetPalmBush => TropicalThicketItem::Palm(&BROAD_WET_PALM_BUSH),
			Self::MiniHonuBanyan => TropicalThicketItem::Banyan(&MINI_HONU_BANYAN),
			Self::ModerateHighBush => TropicalThicketItem::Bush(&MODERATE_HIGH_BUSH),
			Self::FloweringHighBush => TropicalThicketItem::Bush(&FLOWERING_HIGH_BUSH),
			Self::RedStemPalmBush => TropicalThicketItem::Palm(&RED_STEM_PALM_BUSH),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::LargePalmBush => LARGE_PALM_STICK_MIX,
			Self::BroadWetPalmBush => BROAD_WET_PALM_STICK_MIX,
			Self::RedStemPalmBush => RED_STEM_PALM_STICK_MIX,
			Self::MiniHonuBanyan => HONU_STICK_MIX,
			Self::ModerateHighBush => MODERATE_BUSH_STICK_MIX,
			Self::FloweringHighBush => FLOWERING_BUSH_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::LargePalmBush => LARGE_PALM_CANOPY_MIX,
			Self::BroadWetPalmBush => BROAD_WET_PALM_CANOPY_MIX,
			Self::RedStemPalmBush => RED_STEM_PALM_CANOPY_MIX,
			Self::MiniHonuBanyan => HONU_CANOPY_MIX,
			Self::ModerateHighBush => MODERATE_BUSH_CANOPY_MIX,
			Self::FloweringHighBush => FLOWERING_BUSH_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
mod vc {
	use bevy::math::bounding::Aabb3d;
	use bevy::prelude::*;
	use bevy::scene::prelude::Scene;
	use chico_sbs_trees::{
		HighBushShoots, HighBushShootsParams, HonuBanyan, HonuBanyanParams, PalmBush,
		PalmBushParams,
	};
	use chico_vegetation_components::{
		vegetation_scene_chunks, FoliageNode, Layers, Placement, StickNode, StructuralLod,
		VegetationComponents,
	};
	use clap::Args;
	use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
	use lod::lod_ref::LodRef;
	use lod::{lod_host_scene_pending, SceneChunk};
	use material_ref::MaterialRef;
	use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};

	use super::{definition, TropicalThicketCell, TropicalThicketItem};
	use crate::grove::{
		canopy_ball_material_from_palette, canopy_proxy_site, foliage_low_canopy_balls,
		foliage_ultra_low_merged_balls, frond_material_from_palette, grove_detail_level,
		grove_lod_culls, grove_lod_level, grove_lod_status, grove_structural_footprint,
		layers_from_nodes, nest_placed_plant_chunk, placement_noise, stick_material_from_palette,
		CanopyProxySite, FlatTerrainSample, GroveCellVariant, GroveExtent, GroveFrontend,
		DEFAULT_GROVE_EXTENT_XZ, ULTRA_LOW_CANOPY_BIN_METERS,
	};

	pub const TROPICAL_THICKET_STRUCTURAL_HIGH_FACTOR: f32 = 2.0;
	pub const TROPICAL_THICKET_STRUCTURAL_MEDIUM_FACTOR: f32 = 5.0;
	pub const TROPICAL_THICKET_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

	/// Authoring / CLI parameters for Tropical Thicket.
	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct TropicalThicketParams {
		#[command(flatten, next_help_heading = "Grove")]
		pub grove: GroveFrontend,

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
		pub terrain: FlatTerrainSample,

		#[arg(skip)]
		resolved_placements: Option<Vec<GroveCellVariant<TropicalThicketCell>>>,
	}

	impl Default for TropicalThicketParams {
		fn default() -> Self {
			Self {
				grove: GroveFrontend::default(),
				bush_chain_noise: NoiseParams::from_scalar(0.0, 1.0, 1.0, 1),
				stick_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
				leaf_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
				extent: GroveExtent::new(
					Vec3::ZERO,
					Vec3::new(DEFAULT_GROVE_EXTENT_XZ, 1.0, DEFAULT_GROVE_EXTENT_XZ),
				),
				terrain: FlatTerrainSample::default(),
				resolved_placements: None,
			}
		}
	}

	impl TropicalThicketParams {
		pub fn with_resolved_placements(
			resolved_placements: Vec<GroveCellVariant<TropicalThicketCell>>,
			terrain: FlatTerrainSample,
			bush_chain_noise: NoiseParams,
			stick_surface_noise: NoiseParams,
			leaf_surface_noise: NoiseParams,
		) -> Self {
			Self {
				grove: GroveFrontend::default(),
				bush_chain_noise,
				stick_surface_noise,
				leaf_surface_noise,
				extent: GroveExtent::new(
					Vec3::ZERO,
					Vec3::new(DEFAULT_GROVE_EXTENT_XZ, 1.0, DEFAULT_GROVE_EXTENT_XZ),
				),
				terrain,
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

		pub fn placements(&self) -> Vec<GroveCellVariant<TropicalThicketCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.placements_on(&self.terrain)
		}

		/// Select placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn placements_on(
			&self,
			world: &impl crate::GroveWorldSample,
		) -> Vec<GroveCellVariant<TropicalThicketCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.grove.assemble(definition()).populate(&self.extent, world)
		}

		pub fn build(&self) -> TropicalThicket {
			self.build_on(&self.terrain)
		}

		/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> TropicalThicket {
			TropicalThicket::from_placements(
				&self.placements_on(world),
				self.grove.noise,
				self.bush_chain_noise,
				&self.extent,
			)
		}
	}

	#[derive(Clone)]
	enum TropicalThicketKind {
		/// Ground palm bush; crown counts keyed by [`PalmBushParams::unit_detail_from_num`].
		Palm(PalmBush),
		Banyan(HonuBanyan),
		Bush(HighBushShoots),
	}

	#[derive(Clone)]
	pub struct TropicalThicketPlant {
		pub placement: Placement,
		kind: TropicalThicketKind,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	}

	#[derive(Clone, Component)]
	pub struct TropicalThicket {
		pub plants: Vec<TropicalThicketPlant>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl TropicalThicket {
		pub fn from_placements(
			placements: &[GroveCellVariant<TropicalThicketCell>],
			grove_noise: NoiseParams,
			bush_chain_noise: NoiseParams,
			extent: &GroveExtent,
		) -> Self {
			let plants = placements
				.iter()
				.map(|placed| grow_plant(placed, grove_noise, bush_chain_noise))
				.collect();
			let (structural_center, footprint_radius) = grove_structural_footprint(extent);
			Self { plants, structural_center, footprint_radius, extent: *extent }
		}

		pub(crate) fn nest_plant_chunks(&self, lod_ref: &LodRef) -> Vec<SceneChunk> {
			self.plants
				.iter()
				.map(|plant| match &plant.kind {
					TropicalThicketKind::Palm(t) => nest_placed_plant_chunk(
						t.clone(),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						lod_ref,
					),
					TropicalThicketKind::Banyan(t) => nest_placed_plant_chunk(
						t.clone(),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						lod_ref,
					),
					TropicalThicketKind::Bush(t) => nest_placed_plant_chunk(
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
						TropicalThicketKind::Palm(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						TropicalThicketKind::Banyan(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						TropicalThicketKind::Bush(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
					}
				})
				.collect()
		}
	}

	fn grow_plant(
		placed: &GroveCellVariant<TropicalThicketCell>,
		grove_noise: NoiseParams,
		bush_chain_noise: NoiseParams,
	) -> TropicalThicketPlant {
		let build_noise = placement_noise(grove_noise, placed.position);
		let chain_noise = placement_noise(bush_chain_noise, placed.position);
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

		let (kind, placement) = match placed.variant.item() {
			TropicalThicketItem::Palm(palm) => {
				let mut geometry = palm.build_with_noise(build_noise);
				let seed = build_noise.seed.unsigned_abs();
				// Quantize ring / frond counts + foliage seed; keep authored height / scale.
				let unit = PalmBushParams::unit_detail_from_num(seed);
				geometry.crown.ring_count = unit.geometry.crown.ring_count;
				geometry.crown.fronds_per_ring = unit.geometry.crown.fronds_per_ring;
				geometry.foliage_noise.seed = seed as i32;
				let bush = PalmBushParams::new(geometry).build();
				let placement = Placement::new(placed.position, 0.0)
					.with_scale(Vec3::splat(placed.scale.max(1e-4)));
				(TropicalThicketKind::Palm(bush), placement)
			}
			TropicalThicketItem::Bush(bush) => {
				let mut shape = bush.build_with_noise(build_noise);
				shape.chain_noise = chain_noise;
				let placement = Placement::new(placed.position, 0.0)
					.with_scale(Vec3::splat(placed.scale.max(1e-4)));
				(TropicalThicketKind::Bush(HighBushShootsParams::new(shape).build()), placement)
			}
			TropicalThicketItem::Banyan(banyan) => {
				let samples = banyan.build_with_noise(build_noise);
				let mut params = HonuBanyanParams::default();
				params.geometry = samples.geometry;
				params.growth_spawn_fraction = samples.growth_spawn_fraction;
				// Mini Honu (~2–4 m) must not keep full-canopy growth radius (4.0).
				params = params.with_growth_scale_for_height();
				let placement = Placement::new(placed.position, 0.0)
					.with_scale(Vec3::splat(placed.scale.max(1e-4)));
				(TropicalThicketKind::Banyan(params.build()), placement)
			}
		};

		TropicalThicketPlant { placement, kind, stick_material, ball_material, frond_material }
	}

	impl VegetationComponents for TropicalThicket {
		fn stick_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StickNode> {
			// High/Medium nest plant hosts via [`LodScene`]; sticks stay empty.
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
				TROPICAL_THICKET_STRUCTURAL_HIGH_FACTOR,
				TROPICAL_THICKET_STRUCTURAL_MEDIUM_FACTOR,
				TROPICAL_THICKET_STRUCTURAL_LOW_FACTOR,
			))
		}
	}

	impl LodScene for TropicalThicket {
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
			// High/Medium content is nested hosts in chunks; Low/UltraLow use canopy balls.
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
				None => vegetation_scene_chunks(self, lod_ref, level),
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
}

#[cfg(feature = "render")]
pub use vc::{
	TropicalThicket, TropicalThicketParams, TropicalThicketPlant,
	TROPICAL_THICKET_STRUCTURAL_HIGH_FACTOR, TROPICAL_THICKET_STRUCTURAL_LOW_FACTOR,
	TROPICAL_THICKET_STRUCTURAL_MEDIUM_FACTOR,
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
		let dist = TropicalThicketCell::distribution();
		assert_eq!(dist.len(), 7);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 7.0);
		assert_eq!(dist.buckets[1].item, Some(TropicalThicketCell::LargePalmBush));
		assert_eq!(dist.buckets[1].weight, 2.0);
		assert_eq!(dist.buckets[2].item, Some(TropicalThicketCell::BroadWetPalmBush));
		assert_eq!(dist.buckets[2].weight, 1.25);
		assert_eq!(dist.buckets[3].item, Some(TropicalThicketCell::MiniHonuBanyan));
		assert_eq!(dist.buckets[3].weight, 0.45);
		assert_eq!(dist.buckets[4].item, Some(TropicalThicketCell::ModerateHighBush));
		assert_eq!(dist.buckets[4].weight, 1.0);
		assert_eq!(dist.buckets[5].item, Some(TropicalThicketCell::FloweringHighBush));
		assert_eq!(dist.buckets[5].weight, 0.30);
		assert_eq!(dist.buckets[6].item, Some(TropicalThicketCell::RedStemPalmBush));
		assert_eq!(dist.buckets[6].weight, 0.25);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = TropicalThicketCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.24..=0.62).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn palm_banyan_and_bush_placed_weights_match_rfc_ratio() -> Result<()> {
		let weight = |kind: &str| -> f32 {
			TropicalThicketCell::distribution()
				.buckets
				.iter()
				.filter(|b| {
					b.item.is_some_and(|cell| match (kind, cell.item()) {
						("palm", TropicalThicketItem::Palm(_)) => true,
						("banyan", TropicalThicketItem::Banyan(_)) => true,
						("bush", TropicalThicketItem::Bush(_)) => true,
						_ => false,
					})
				})
				.map(|b| b.weight)
				.sum()
		};
		let palm = weight("palm");
		let banyan = weight("banyan");
		let bush = weight("bush");
		assert!((palm - 3.5).abs() < 1e-4, "expected palm weight 3.5, got {palm}");
		assert!((banyan - 0.45).abs() < 1e-4, "expected banyan weight 0.45, got {banyan}");
		assert!((bush - 1.30).abs() < 1e-4, "expected bush weight 1.30, got {bush}");
		Ok(())
	}

	#[test]
	fn palm_banyan_and_bush_geometry_follows_authored_bands() -> Result<()> {
		let TropicalThicketItem::Palm(large) = TropicalThicketCell::LargePalmBush.item() else {
			anyhow::bail!("expected large palm item");
		};
		assert!(large.height.start >= 3.00);
		assert!(large.height.end <= 6.60);
		assert_eq!(large.frond_count, 7..=12);

		let TropicalThicketItem::Palm(wet) = TropicalThicketCell::BroadWetPalmBush.item() else {
			anyhow::bail!("expected broad wet palm item");
		};
		assert!(wet.height.end <= 7.80);
		assert_eq!(wet.frond_count, 8..=14);

		let TropicalThicketItem::Banyan(banyan) = TropicalThicketCell::MiniHonuBanyan.item() else {
			anyhow::bail!("expected banyan item");
		};
		assert!(banyan.height.start >= 1.80);
		assert!(banyan.height.end <= 3.80);
		assert!(banyan.canopy_spread.start >= 1.20);

		let TropicalThicketItem::Bush(moderate) = TropicalThicketCell::ModerateHighBush.item()
		else {
			anyhow::bail!("expected moderate bush item");
		};
		assert!(moderate.height.start >= 1.20);
		assert!(moderate.leaf_radius.end <= 0.15);
		assert_eq!(moderate.branch_depth, 2..=5);

		let TropicalThicketItem::Bush(flowering) = TropicalThicketCell::FloweringHighBush.item()
		else {
			anyhow::bail!("expected flowering bush item");
		};
		assert!(flowering.height.end <= 2.20);
		assert_eq!(flowering.shoot_count, 7..=10);
		assert_eq!(flowering.branch_depth, 2..=5);

		let TropicalThicketItem::Palm(red) = TropicalThicketCell::RedStemPalmBush.item() else {
			anyhow::bail!("expected red stem palm item");
		};
		assert!(red.crown_spread.end <= 6.30);
		Ok(())
	}

	#[test]
	#[ignore = "placement constraints deferred to forest-layer normalization"]
	fn constraint_first_fit_fallback() -> Result<()> {
		// LargePalmBush (index 1) rejects steepness 0.30; first-fit falls to BroadWetPalmBush
		// (index 2), which allows steepness up to 0.68.
		let prepared = TropicalThicketCell::distribution().prepare(
			0.0,
			0.0,
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.30 };
		let outcome = prepared.select_from(
			1,
			Vec3::new(5.0, 0.35, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, TropicalThicketCell::BroadWetPalmBush);
			}
			other => anyhow::bail!("expected BroadWetPalmBush fallback, got {other:?}"),
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

	#[cfg(feature = "render")]
	#[test]
	fn low_and_ultra_low_emit_canopy_ball_proxies() -> Result<()> {
		use chico_vegetation_components::VegetationComponents;
		use lod::gen::LodSceneLevel;

		let mut params = TropicalThicketParams::default();
		params.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(40.0, 1.0, 40.0));
		params.terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
		let grove = params.build();
		assert!(!grove.plants.is_empty());

		let low = grove.foliage_nodes_for_level(LodSceneLevel::Low).flatten();
		assert_eq!(low.len(), grove.plants.len());
		assert!(low.iter().all(|n| matches!(
			n.geometry,
			chico_vegetation_components::FoliageGeometry::CheapBall
		)));
		assert!(grove.stick_nodes_for_level(LodSceneLevel::Low).flatten().is_empty());

		let ultra = grove.foliage_nodes_for_level(LodSceneLevel::UltraLow).flatten();
		assert!(!ultra.is_empty());
		assert!(ultra.len() <= grove.plants.len());
		assert!(ultra.iter().all(|n| matches!(
			n.geometry,
			chico_vegetation_components::FoliageGeometry::CheapBall
		)));
		Ok(())
	}

	#[cfg(feature = "render")]
	#[test]
	fn high_nests_one_plant_host_chunk_per_plant() -> Result<()> {
		use bevy::prelude::Transform;
		use chico_vegetation_components::VegetationComponents;
		use lod::gen::{LodScene, LodSceneLevel};
		use lod::lod_ref::LodRef;

		let mut params = TropicalThicketParams::default();
		params.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(40.0, 1.0, 40.0));
		params.terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
		let grove = params.build();
		assert!(!grove.plants.is_empty());

		let identity = Transform::IDENTITY;
		let bounds = grove.scene_bounds();
		let lod_ref = LodRef {
			entity: bevy::prelude::Entity::PLACEHOLDER,
			previous_transform: &identity,
			current_transform: &identity,
			bounds: &bounds,
		};
		let chunks = grove.nest_plant_chunks(&lod_ref);
		assert_eq!(chunks.len(), grove.plants.len());
		assert!(
			grove.foliage_nodes_for_level(LodSceneLevel::High).flatten().is_empty(),
			"High foliage stays on nested plant hosts, not the grove"
		);
		Ok(())
	}
}
