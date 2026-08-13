//! Conifer Sapling — well-known moderate-density young conifer lower-canopy grove
//! ([RFC-183 §3.4.6.5], [#326](https://github.com/ramate-io/maybraid/issues/326)).
//!
//! Mixed Friend's and Northern Conifer saplings beneath taller evergreen canopy. Forest-layer
//! attachment remains a follow-up.

use bevy_math::{Vec2, Vec3};
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, GroveWorldSample,
	PaletteMix, PaletteSlot, PlacementConstraints,
};

/// Uniform terrain tuned for conifer sapling placement constraints (RFC elevation bands overlap).
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "render", derive(clap::Args))]
#[cfg_attr(feature = "render", command(next_help_heading = "Terrain"))]
pub struct SaplingFlatTerrain {
	#[cfg_attr(feature = "render", arg(long, default_value_t = 0.50))]
	pub elevation: f32,
	#[cfg_attr(feature = "render", arg(long, default_value_t = 0.30))]
	pub steepness: f32,
}

impl Default for SaplingFlatTerrain {
	fn default() -> Self {
		Self { elevation: 0.50, steepness: 0.30 }
	}
}

impl GroveWorldSample for SaplingFlatTerrain {
	fn elevation_at(&self, _position: Vec3) -> f32 {
		self.elevation
	}

	fn steepness_at(&self, _position: Vec3) -> f32 {
		self.steepness
	}
}

/// Standard sapling height band ([`1.0`, `4.0`] m).
const SAPLING_HEIGHT: UnitRange = UnitRange::new(1.0, 4.0);
/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);
/// Sparse..moderate band for windswept northern accents.
const SPARSE_TO_MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.20, 0.55);

/// Authored Conifer Sapling grove definition.
///
/// Cell footprint at the RFC midpoint (`10.5` m). The offset range is signed and ± one cell so
/// placements break the underlying grid.
pub fn definition() -> GroveDefinition<ConiferSaplingCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(10.5),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-10.5, 10.5),
		),
		distribution: ConiferSaplingCell::distribution(),
	}
}

/// Ordered conifer-sapling varietals ([RFC-183 §3.4.6.5]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConiferSaplingCell {
	FriendSapling,
	NorthernSapling,
	MossyFriendSapling,
	ColdNorthernSapling,
	BrightFriendSapling,
	WindsweptNorthernSapling,
}

/// Typed authored geometry for one conifer-sapling varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConiferSaplingItem {
	FriendsConifer(&'static ConiferSaplingFriendsConifer),
	NorthernConifer(&'static ConiferSaplingNorthernConifer),
}

/// Authored geometry ranges for one Friend's Conifer sapling form.
#[derive(Debug, Clone, PartialEq)]
pub struct ConiferSaplingFriendsConifer {
	pub height: UnitRange,
	/// World-space stalk base radius (RFC `0.025 × H`).
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse..moderate canopy density at render time.
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Northern Conifer sapling form.
#[derive(Debug, Clone, PartialEq)]
pub struct ConiferSaplingNorthernConifer {
	pub height: UnitRange,
	/// World-space stalk base radius (Northern `0.032 × H`).
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse..moderate canopy density at render time.
	pub canopy_density: UnitRange,
}

const FRIEND_SAPLING: ConiferSaplingFriendsConifer = ConiferSaplingFriendsConifer {
	height: SAPLING_HEIGHT,
	stalk_radius: UnitRange::new(0.025, 0.10),
	canopy_spread: UnitRange::new(0.20, 0.70),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const MOSSY_FRIEND_SAPLING: ConiferSaplingFriendsConifer = ConiferSaplingFriendsConifer {
	height: SAPLING_HEIGHT,
	stalk_radius: UnitRange::new(0.025, 0.10),
	canopy_spread: UnitRange::new(0.15, 0.55),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const BRIGHT_FRIEND_SAPLING: ConiferSaplingFriendsConifer = ConiferSaplingFriendsConifer {
	height: SAPLING_HEIGHT,
	stalk_radius: UnitRange::new(0.025, 0.10),
	canopy_spread: UnitRange::new(0.22, 0.75),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const NORTHERN_SAPLING: ConiferSaplingNorthernConifer = ConiferSaplingNorthernConifer {
	height: SAPLING_HEIGHT,
	stalk_radius: UnitRange::new(0.032, 0.128),
	canopy_spread: UnitRange::new(0.20, 0.70),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const COLD_NORTHERN_SAPLING: ConiferSaplingNorthernConifer = ConiferSaplingNorthernConifer {
	height: SAPLING_HEIGHT,
	stalk_radius: UnitRange::new(0.032, 0.128),
	canopy_spread: UnitRange::new(0.18, 0.60),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const WINDSWEPT_NORTHERN_SAPLING: ConiferSaplingNorthernConifer = ConiferSaplingNorthernConifer {
	height: SAPLING_HEIGHT,
	stalk_radius: UnitRange::new(0.032, 0.128),
	canopy_spread: UnitRange::new(0.12, 0.50),
	canopy_density: SPARSE_TO_MODERATE_CANOPY_DENSITY,
};

const FRIEND_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("conifer_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "moss_bark"),
]);

const FRIEND_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "blue_green"),
	PaletteSlot::new("dark_green", "fresh_green"),
]);

const MOSSY_FRIEND_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("moss_bark", "conifer_bark"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const MOSSY_FRIEND_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("moss_green", "deep_green"),
	PaletteSlot::new("olive_green", "needle_green"),
]);

const BRIGHT_FRIEND_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("young_bark", "conifer_bark"),
	PaletteSlot::new("gray_brown", "moss_bark"),
]);

const BRIGHT_FRIEND_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("fresh_green", "yellow_green"),
	PaletteSlot::new("light_green", "spring_green"),
]);

const NORTHERN_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "conifer_bark"),
]);

const NORTHERN_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_green", "blue_green"),
	PaletteSlot::new("deep_green", "dark_green"),
]);

const COLD_NORTHERN_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_bark", "gray_brown"),
	PaletteSlot::new("conifer_bark", "dark_bark"),
]);

const COLD_NORTHERN_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_green", "deep_green"),
	PaletteSlot::new("blue_green", "dark_green"),
]);

const WINDSWEPT_NORTHERN_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("gray_brown", "cold_bark"),
	PaletteSlot::new("conifer_bark", "dry_bark"),
]);

const WINDSWEPT_NORTHERN_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("blue_green", "cold_green"),
	PaletteSlot::new("needle_green", "olive_green"),
]);

impl ConiferSaplingCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `3.4` (RFC pair plus sapling accents); the `None` weight of `5.2` puts
	/// the placed share at `3.4 / 8.6 ≈ 0.40`, mid RFC `DENSITY_RANGE` (`0.28..0.48`).
	pub fn distribution() -> GroveDistribution<Self> {
		let friend =
			PlacementConstraints::new(UnitRange::new(0.18, 0.82), UnitRange::new(0.0, 0.64));
		let northern =
			PlacementConstraints::new(UnitRange::new(0.22, 0.88), UnitRange::new(0.0, 0.72));
		GroveDistribution::new(vec![
			GroveBucket::none(5.2),
			GroveBucket::placed(1.0, friend, Self::FriendSapling),
			GroveBucket::placed(1.0, northern, Self::NorthernSapling),
			GroveBucket::placed(0.35, friend, Self::MossyFriendSapling),
			GroveBucket::placed(0.35, northern, Self::ColdNorthernSapling),
			GroveBucket::placed(0.30, friend, Self::BrightFriendSapling),
			GroveBucket::placed(0.40, northern, Self::WindsweptNorthernSapling),
		])
	}

	pub fn item(self) -> ConiferSaplingItem {
		match self {
			Self::FriendSapling => ConiferSaplingItem::FriendsConifer(&FRIEND_SAPLING),
			Self::BrightFriendSapling => ConiferSaplingItem::FriendsConifer(&BRIGHT_FRIEND_SAPLING),
			Self::MossyFriendSapling => ConiferSaplingItem::FriendsConifer(&MOSSY_FRIEND_SAPLING),
			Self::NorthernSapling => ConiferSaplingItem::NorthernConifer(&NORTHERN_SAPLING),
			Self::ColdNorthernSapling => {
				ConiferSaplingItem::NorthernConifer(&COLD_NORTHERN_SAPLING)
			}
			Self::WindsweptNorthernSapling => {
				ConiferSaplingItem::NorthernConifer(&WINDSWEPT_NORTHERN_SAPLING)
			}
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::FriendSapling => FRIEND_SAPLING_STICK_MIX,
			Self::MossyFriendSapling => MOSSY_FRIEND_SAPLING_STICK_MIX,
			Self::BrightFriendSapling => BRIGHT_FRIEND_SAPLING_STICK_MIX,
			Self::NorthernSapling => NORTHERN_SAPLING_STICK_MIX,
			Self::ColdNorthernSapling => COLD_NORTHERN_SAPLING_STICK_MIX,
			Self::WindsweptNorthernSapling => WINDSWEPT_NORTHERN_SAPLING_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::FriendSapling => FRIEND_SAPLING_CANOPY_MIX,
			Self::MossyFriendSapling => MOSSY_FRIEND_SAPLING_CANOPY_MIX,
			Self::BrightFriendSapling => BRIGHT_FRIEND_SAPLING_CANOPY_MIX,
			Self::NorthernSapling => NORTHERN_SAPLING_CANOPY_MIX,
			Self::ColdNorthernSapling => COLD_NORTHERN_SAPLING_CANOPY_MIX,
			Self::WindsweptNorthernSapling => WINDSWEPT_NORTHERN_SAPLING_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
mod vc {
	use bevy::math::bounding::Aabb3d;
	use bevy::prelude::*;
	use bevy::scene::prelude::Scene;
	use super::SaplingFlatTerrain;
	use super::variants::conifer_sapling_friends_conifer::FriendConiferSamples;
	use chico_sbs_trees::{FriendsConifer, FriendsConiferParams, NorthernConifer, NorthernConiferParams};
	use chico_vegetation_components::{
		FoliageNode, Layers, Placement, StickNode, StructuralLod, VegetationComponents,
	};
	use clap::Args;
	use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
	use lod::lod_ref::LodRef;
	use lod::{lod_host_scene_pending, SceneChunk};
	use material_ref::MaterialRef;
	use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};

	use super::{definition, ConiferSaplingCell, ConiferSaplingItem};
	use crate::grove::{
		canopy_ball_material_from_palette, canopy_proxy_site, foliage_low_canopy_balls,
		foliage_ultra_low_merged_balls, frond_material_from_palette, grove_detail_level,
		grove_lod_culls, grove_lod_level, grove_lod_status, grove_structural_footprint,
		layers_from_nodes, nest_placed_plant_chunk, placement_noise, stick_material_from_palette,
		woody_grove_scene_chunks, CanopyProxySite, GroveCellVariant, GroveExtent, GroveFrontend,
		DEFAULT_GROVE_EXTENT_XZ, ULTRA_LOW_CANOPY_BIN_METERS,
	};

	pub const CONIFER_SAPLING_STRUCTURAL_HIGH_FACTOR: f32 = 2.0;
	pub const CONIFER_SAPLING_STRUCTURAL_MEDIUM_FACTOR: f32 = 5.0;
	pub const CONIFER_SAPLING_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct ConiferSaplingParams {
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
		pub terrain: SaplingFlatTerrain,

		#[arg(skip)]
		resolved_placements: Option<Vec<GroveCellVariant<ConiferSaplingCell>>>,
	}

	impl Default for ConiferSaplingParams {
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
				terrain: SaplingFlatTerrain::default(),
				resolved_placements: None,
			}
		}
	}

	impl ConiferSaplingParams {
		pub fn with_extent(mut self, extent: GroveExtent) -> Self {
			self.extent = extent;
			self
		}

		pub fn with_terrain(mut self, terrain: SaplingFlatTerrain) -> Self {
			self.terrain = terrain;
			self
		}

		pub fn cell_extent_xz(&self) -> Vec2 {
			self.grove.definition(definition()).cell_extent_xz
		}

		pub fn placement_cells(&self) -> Vec<gimme_gen::Cell> {
			self.extent.subdivide_xz(self.cell_extent_xz())
		}

		pub fn placements(&self) -> Vec<GroveCellVariant<ConiferSaplingCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.grove.assemble(definition()).populate(&self.extent, &self.terrain)
		}

		pub fn build(&self) -> ConiferSapling {
			ConiferSapling::from_placements(&self.placements(), self.grove.noise, &self.extent)
		}
	}

	#[derive(Clone)]
	enum ConiferSaplingKind {
		Friends(FriendsConifer),
		Northern(NorthernConifer),
	}

	#[derive(Clone)]
	pub struct ConiferSaplingPlant {
		pub placement: Placement,
		kind: ConiferSaplingKind,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	}

	#[derive(Clone, Component)]
	pub struct ConiferSapling {
		pub plants: Vec<ConiferSaplingPlant>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl ConiferSapling {
		pub fn from_placements(
			placements: &[GroveCellVariant<ConiferSaplingCell>],
			grove_noise: NoiseParams,
			extent: &GroveExtent,
		) -> Self {
			let plants = placements.iter().map(|placed| grow_plant(placed, grove_noise)).collect();
			let (structural_center, footprint_radius) = grove_structural_footprint(extent);
			Self {
				plants,
				structural_center,
				footprint_radius,
				extent: *extent,
			}
		}

		fn nest_plant_chunks(&self, lod_ref: &LodRef) -> Vec<SceneChunk> {
			self.plants
				.iter()
				.map(|plant| match &plant.kind {
					ConiferSaplingKind::Friends(t) => nest_placed_plant_chunk(
						t.clone(),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						lod_ref,
					),
					ConiferSaplingKind::Northern(t) => nest_placed_plant_chunk(
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
						ConiferSaplingKind::Friends(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						ConiferSaplingKind::Northern(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
					}
				})
				.collect()
		}
	}

	fn grow_plant(
		placed: &GroveCellVariant<ConiferSaplingCell>,
		grove_noise: NoiseParams,
	) -> ConiferSaplingPlant {
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
			ConiferSaplingItem::FriendsConifer(conifer) => {
				let samples =
					BuildWithNoise::<FriendConiferSamples>::build_with_noise(conifer, build_noise);
				let mut params = FriendsConiferParams::default();
				params.geometry = samples.geometry;
				params.splay_radius_fraction_of_height = samples.splay_radius_fraction_of_height;
				params.apex_canopy_spawn_fraction = samples.apex_canopy_spawn_fraction;
				ConiferSaplingKind::Friends(params.build())
			}
			ConiferSaplingItem::NorthernConifer(conifer) => {
				let samples = conifer.build_with_noise(build_noise);
				let mut params = NorthernConiferParams::default();
				params.geometry = samples.geometry;
				params.splay_radius_fraction_of_height = samples.splay_radius_fraction_of_height;
				params.splay_spawn_fraction = samples.splay_spawn_fraction;
				params.apex_canopy_spawn_fraction = samples.apex_canopy_spawn_fraction;
				ConiferSaplingKind::Northern(params.build())
			}
		};

		ConiferSaplingPlant {
			placement,
			kind,
			stick_material,
			ball_material,
			frond_material,
		}
	}

	impl VegetationComponents for ConiferSapling {
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
				| LodSceneLevel::Resolution(_) => layers_from_nodes(
					foliage_ultra_low_merged_balls(&self.canopy_sites(), ULTRA_LOW_CANOPY_BIN_METERS),
				),
			}
		}

		fn structural_lod(&self) -> Option<StructuralLod> {
			Some(
				StructuralLod::new(self.structural_center, self.footprint_radius).with_factors(
					CONIFER_SAPLING_STRUCTURAL_HIGH_FACTOR,
					CONIFER_SAPLING_STRUCTURAL_MEDIUM_FACTOR,
					CONIFER_SAPLING_STRUCTURAL_LOW_FACTOR,
				),
			)
		}
	}

	impl LodScene for ConiferSapling {
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
						self, lod_ref, level, &mut children,
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
	ConiferSapling, ConiferSaplingParams, ConiferSaplingPlant, CONIFER_SAPLING_STRUCTURAL_HIGH_FACTOR,
	CONIFER_SAPLING_STRUCTURAL_LOW_FACTOR, CONIFER_SAPLING_STRUCTURAL_MEDIUM_FACTOR,
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
		let dist = ConiferSaplingCell::distribution();
		assert_eq!(dist.len(), 7);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 5.2);
		assert_eq!(dist.buckets[1].item, Some(ConiferSaplingCell::FriendSapling));
		assert_eq!(dist.buckets[1].weight, 1.0);
		assert_eq!(dist.buckets[2].item, Some(ConiferSaplingCell::NorthernSapling));
		assert_eq!(dist.buckets[2].weight, 1.0);
		assert_eq!(dist.buckets[3].item, Some(ConiferSaplingCell::MossyFriendSapling));
		assert_eq!(dist.buckets[3].weight, 0.35);
		assert_eq!(dist.buckets[4].item, Some(ConiferSaplingCell::ColdNorthernSapling));
		assert_eq!(dist.buckets[4].weight, 0.35);
		assert_eq!(dist.buckets[5].item, Some(ConiferSaplingCell::BrightFriendSapling));
		assert_eq!(dist.buckets[5].weight, 0.30);
		assert_eq!(dist.buckets[6].item, Some(ConiferSaplingCell::WindsweptNorthernSapling));
		assert_eq!(dist.buckets[6].weight, 0.40);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = ConiferSaplingCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.28..=0.48).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let ConiferSaplingItem::FriendsConifer(friend) = ConiferSaplingCell::FriendSapling.item()
		else {
			anyhow::bail!("expected friend sapling item");
		};
		assert_eq!(friend.height, SAPLING_HEIGHT);
		assert_eq!(friend.canopy_density, MODERATE_CANOPY_DENSITY);

		let ConiferSaplingItem::NorthernConifer(northern) =
			ConiferSaplingCell::NorthernSapling.item()
		else {
			anyhow::bail!("expected northern sapling item");
		};
		assert_eq!(northern.height, SAPLING_HEIGHT);

		let ConiferSaplingItem::NorthernConifer(windswept) =
			ConiferSaplingCell::WindsweptNorthernSapling.item()
		else {
			anyhow::bail!("expected windswept northern item");
		};
		assert_eq!(windswept.canopy_density, SPARSE_TO_MODERATE_CANOPY_DENSITY);
		Ok(())
	}

	#[test]
	fn constraint_first_fit_selects_per_bucket() -> Result<()> {
		let prepared = ConiferSaplingCell::distribution().prepare(
			0.0,
			0.0,
			NoiseParams::default(),
			Vec3::ZERO,
		);

		let terrain = FlatTerrainSample { elevation: 0.50, steepness: 0.30 };
		let outcome = prepared.select_from(
			1,
			Vec3::new(5.0, 0.50, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, ConiferSaplingCell::FriendSapling);
			}
			other => anyhow::bail!("expected FriendSapling at mid elevation, got {other:?}"),
		}

		// Friend max elevation is 0.82; Northern accepts up to 0.88.
		let high_terrain = FlatTerrainSample { elevation: 0.85, steepness: 0.30 };
		let outcome = prepared.select_from(
			2,
			Vec3::new(6.0, 0.85, 6.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&high_terrain,
		);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, ConiferSaplingCell::NorthernSapling);
			}
			other => anyhow::bail!("expected NorthernSapling at high elevation, got {other:?}"),
		}

		// Friend max steepness is 0.64; Northern accepts up to 0.72.
		let steep_terrain = FlatTerrainSample { elevation: 0.50, steepness: 0.70 };
		let outcome = prepared.select_from(
			1,
			Vec3::new(7.0, 0.50, 7.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&steep_terrain,
		);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, ConiferSaplingCell::NorthernSapling);
			}
			other => anyhow::bail!("expected NorthernSapling on steep slope, got {other:?}"),
		}
		Ok(())
	}

	#[test]
	fn placements_break_the_cell_grid() -> Result<()> {
		let noise = crate::grove::GroveFrontend::default().noise;
		let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0));
		let terrain = FlatTerrainSample { elevation: 0.50, steepness: 0.30 };
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
		let terrain = FlatTerrainSample { elevation: 0.50, steepness: 0.30 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
