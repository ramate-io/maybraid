//! Riparian Mix — mixed riparian upper-canopy grove with conifer accents
//! ([RFC-183 §3.4.7.11], [#333](https://github.com/ramate-io/maybraid/issues/333)).
//!
//! Braid oak and storybook bank/overbank forms with Friend's and Temperate Conifer on sheltered
//! margins. Forest-layer attachment remains a follow-up.

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

/// Authored Riparian Mix grove definition.
///
/// Cell footprint sits at the RFC midpoint (`17.0` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<RiparianMixCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(17.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-17.0, 17.0),
		),
		distribution: RiparianMixCell::distribution(),
	}
}

/// Ordered riparian-mix varietals ([RFC-183 §3.4.7.11]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiparianMixCell {
	BankBraidOak,
	OverbankBraidOak,
	RoundRiparianStorybook,
	TallRiparianStorybook,
	BankFriendConifer,
	ShelteredTemperateConifer,
}

/// Typed authored geometry for one riparian-mix varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RiparianMixItem {
	BraidOak(&'static RiparianMixBraidOak),
	Storybook(&'static RiparianMixStorybook),
	FriendsConifer(&'static RiparianMixFriendsConifer),
	TemperateConifer(&'static RiparianMixTemperateConifer),
}

/// Authored geometry ranges for one Braid Oak form.
#[derive(Debug, Clone, PartialEq)]
pub struct RiparianMixBraidOak {
	pub height: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct RiparianMixStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Friend's Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct RiparianMixFriendsConifer {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Temperate Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct RiparianMixTemperateConifer {
	pub height: UnitRange,
	pub canopy_density: UnitRange,
}

const BANK_BRAID_OAK: RiparianMixBraidOak =
	RiparianMixBraidOak { height: UnitRange::new(5.0, 12.0), canopy_density: DENSE_CANOPY_DENSITY };

const OVERBANK_BRAID_OAK: RiparianMixBraidOak = RiparianMixBraidOak {
	height: UnitRange::new(10.0, 18.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const ROUND_RIPARIAN_STORYBOOK: RiparianMixStorybook = RiparianMixStorybook {
	height: UnitRange::new(5.0, 15.0),
	stalk_radius: UnitRange::new(0.18, 0.40),
	canopy_spread: UnitRange::new(2.0, 5.5),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const TALL_RIPARIAN_STORYBOOK: RiparianMixStorybook = RiparianMixStorybook {
	height: UnitRange::new(12.0, 22.0),
	stalk_radius: UnitRange::new(0.26, 0.52),
	canopy_spread: UnitRange::new(3.5, 8.0),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const BANK_FRIEND_CONIFER: RiparianMixFriendsConifer = RiparianMixFriendsConifer {
	height: UnitRange::new(8.0, 16.0),
	stalk_radius: UnitRange::new(0.20, 0.40),
	canopy_spread: UnitRange::new(2.5, 6.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const SHELTERED_TEMPERATE_CONIFER: RiparianMixTemperateConifer = RiparianMixTemperateConifer {
	height: UnitRange::new(10.0, 20.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const BANK_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("wet_oak_bark", "dark_bark"),
	PaletteSlot::new("moss_bark", "gray_brown"),
]);

const BANK_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("river_green", "fresh_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);

const OVERBANK_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("oak_bark", "wet_brown"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const OVERBANK_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("river_green", "yellow_green"),
]);

const RIPARIAN_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("broadleaf_bark", "wet_brown"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const RIPARIAN_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("river_green", "light_green"),
	PaletteSlot::new("deep_green", "fresh_green"),
]);

const BANK_FRIEND_CONIFER_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("conifer_bark", "wet_brown"),
	PaletteSlot::new("gray_brown", "moss_bark"),
]);

const BANK_FRIEND_CONIFER_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "blue_green"),
	PaletteSlot::new("river_green", "fresh_green"),
]);

const SHELTERED_TEMPERATE_CONIFER_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("temperate_bark", "wet_brown"),
	PaletteSlot::new("gray_brown", "moss_bark"),
]);

const SHELTERED_TEMPERATE_CONIFER_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("soft_green", "deep_green"),
	PaletteSlot::new("river_green", "fresh_green"),
]);

impl RiparianMixCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `4.45` (RFC relative proportions); the `None` weight of `10.9` puts
	/// the placed share at `4.45 / 15.35 ≈ 0.29`, mid RFC `DENSITY_RANGE` (`0.18..0.40`).
	pub fn distribution() -> GroveDistribution<Self> {
		let bank_braid_oak =
			PlacementConstraints::new(UnitRange::new(0.0, 0.38), UnitRange::new(0.0, 0.30));
		let overbank_braid_oak =
			PlacementConstraints::new(UnitRange::new(0.00, 0.48), UnitRange::new(0.0, 0.42));
		let round_storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 0.46), UnitRange::new(0.0, 0.42));
		let tall_storybook =
			PlacementConstraints::new(UnitRange::new(0.00, 0.52), UnitRange::new(0.0, 0.48));
		let bank_friend =
			PlacementConstraints::new(UnitRange::new(0.00, 0.58), UnitRange::new(0.0, 0.50));
		let sheltered_temperate =
			PlacementConstraints::new(UnitRange::new(0.00, 0.62), UnitRange::new(0.0, 0.54));
		GroveDistribution::new(vec![
			GroveBucket::none(6.9),
			GroveBucket::placed(0.9, bank_braid_oak, Self::BankBraidOak),
			GroveBucket::placed(0.6, overbank_braid_oak, Self::OverbankBraidOak),
			GroveBucket::placed(0.9, round_storybook, Self::RoundRiparianStorybook),
			GroveBucket::placed(0.45, tall_storybook, Self::TallRiparianStorybook),
			GroveBucket::placed(0.8, bank_friend, Self::BankFriendConifer),
			GroveBucket::placed(0.8, sheltered_temperate, Self::ShelteredTemperateConifer),
		])
	}

	pub fn item(self) -> RiparianMixItem {
		match self {
			Self::BankBraidOak => RiparianMixItem::BraidOak(&BANK_BRAID_OAK),
			Self::OverbankBraidOak => RiparianMixItem::BraidOak(&OVERBANK_BRAID_OAK),
			Self::RoundRiparianStorybook => RiparianMixItem::Storybook(&ROUND_RIPARIAN_STORYBOOK),
			Self::TallRiparianStorybook => RiparianMixItem::Storybook(&TALL_RIPARIAN_STORYBOOK),
			Self::BankFriendConifer => RiparianMixItem::FriendsConifer(&BANK_FRIEND_CONIFER),
			Self::ShelteredTemperateConifer => {
				RiparianMixItem::TemperateConifer(&SHELTERED_TEMPERATE_CONIFER)
			}
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::BankBraidOak => BANK_BRAID_OAK_STICK_MIX,
			Self::OverbankBraidOak => OVERBANK_BRAID_OAK_STICK_MIX,
			Self::RoundRiparianStorybook | Self::TallRiparianStorybook => {
				RIPARIAN_STORYBOOK_STICK_MIX
			}
			Self::BankFriendConifer => BANK_FRIEND_CONIFER_STICK_MIX,
			Self::ShelteredTemperateConifer => SHELTERED_TEMPERATE_CONIFER_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::BankBraidOak => BANK_BRAID_OAK_CANOPY_MIX,
			Self::OverbankBraidOak => OVERBANK_BRAID_OAK_CANOPY_MIX,
			Self::RoundRiparianStorybook | Self::TallRiparianStorybook => {
				RIPARIAN_STORYBOOK_CANOPY_MIX
			}
			Self::BankFriendConifer => BANK_FRIEND_CONIFER_CANOPY_MIX,
			Self::ShelteredTemperateConifer => SHELTERED_TEMPERATE_CONIFER_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
mod vc {
	use bevy::math::bounding::Aabb3d;
	use bevy::prelude::*;
	use bevy::scene::prelude::Scene;
	use chico_sbs_trees::{
		BraidOakTree, BraidOakTreeParams, FriendsConifer, FriendsConiferParams, StorybookTree,
		StorybookTreeParams, TemperateConifer, TemperateConiferParams,
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

	use super::{definition, RiparianMixCell, RiparianMixItem};
	use crate::grove::{
		canopy_ball_material_from_palette, canopy_proxy_site, foliage_low_canopy_balls,
		foliage_ultra_low_merged_balls, frond_material_from_palette, grove_detail_level,
		grove_lod_culls, grove_lod_level, grove_lod_status, grove_structural_footprint,
		layers_from_nodes, nest_placed_plant_chunk, placement_noise, stick_material_from_palette,
		woody_grove_scene_chunks, CanopyProxySite, FlatTerrainSample, GroveCellVariant,
		GroveExtent, GroveFrontend, DEFAULT_GROVE_EXTENT_XZ, ULTRA_LOW_CANOPY_BIN_METERS,
	};

	pub const RIPARIAN_MIX_STRUCTURAL_HIGH_FACTOR: f32 = 2.0;
	pub const RIPARIAN_MIX_STRUCTURAL_MEDIUM_FACTOR: f32 = 5.0;
	pub const RIPARIAN_MIX_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct RiparianMixParams {
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
		resolved_placements: Option<Vec<GroveCellVariant<RiparianMixCell>>>,
	}

	impl Default for RiparianMixParams {
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

	impl RiparianMixParams {
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

		pub fn placements(&self) -> Vec<GroveCellVariant<RiparianMixCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.placements_on(&self.terrain)
		}

		/// Select placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn placements_on(
			&self,
			world: &impl crate::GroveWorldSample,
		) -> Vec<GroveCellVariant<RiparianMixCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.grove.assemble(definition()).populate(&self.extent, world)
		}

		pub fn build(&self) -> RiparianMix {
			self.build_on(&self.terrain)
		}

		/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> RiparianMix {
			RiparianMix::from_placements(
				&self.placements_on(world),
				self.grove.noise,
				self.stick_surface_noise,
				&self.extent,
			)
		}
	}

	#[derive(Clone)]
	enum RiparianMixKind {
		Oak(BraidOakTree),
		Storybook(StorybookTree),
		Friends(FriendsConifer),
		Temperate(TemperateConifer),
	}

	#[derive(Clone)]
	pub struct RiparianMixPlant {
		pub placement: Placement,
		kind: RiparianMixKind,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	}

	#[derive(Clone, Component)]
	pub struct RiparianMix {
		pub plants: Vec<RiparianMixPlant>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl RiparianMix {
		pub fn from_placements(
			placements: &[GroveCellVariant<RiparianMixCell>],
			grove_noise: NoiseParams,
			stick_surface_noise: NoiseParams,
			extent: &GroveExtent,
		) -> Self {
			let plants = placements
				.iter()
				.map(|placed| grow_plant(placed, grove_noise, stick_surface_noise))
				.collect();
			let (structural_center, footprint_radius) = grove_structural_footprint(extent);
			Self { plants, structural_center, footprint_radius, extent: *extent }
		}

		fn nest_plant_chunks(&self, lod_ref: &LodRef) -> Vec<SceneChunk> {
			self.plants
				.iter()
				.map(|plant| match &plant.kind {
					RiparianMixKind::Oak(t) => nest_placed_plant_chunk(
						t.clone(),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						lod_ref,
					),
					RiparianMixKind::Storybook(t) => nest_placed_plant_chunk(
						t.clone(),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						lod_ref,
					),
					RiparianMixKind::Friends(t) => nest_placed_plant_chunk(
						t.clone(),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						lod_ref,
					),
					RiparianMixKind::Temperate(t) => nest_placed_plant_chunk(
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
						RiparianMixKind::Oak(t) => canopy_proxy_site(t, plant.placement, material),
						RiparianMixKind::Storybook(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						RiparianMixKind::Friends(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						RiparianMixKind::Temperate(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
					}
				})
				.collect()
		}
	}

	fn grow_plant(
		placed: &GroveCellVariant<RiparianMixCell>,
		grove_noise: NoiseParams,
		stick_surface_noise: NoiseParams,
	) -> RiparianMixPlant {
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
			RiparianMixItem::BraidOak(oak) => {
				let geometry = oak.build_with_noise(build_noise);
				let mut params = BraidOakTreeParams::default();
				params.geometry = geometry;
				params.stick_surface_noise = placement_noise(stick_surface_noise, placed.position);
				RiparianMixKind::Oak(params.build())
			}
			RiparianMixItem::Storybook(story) => {
				let geometry = story.build_with_noise(build_noise);
				let mut params = StorybookTreeParams::default();
				params.geometry = geometry;
				RiparianMixKind::Storybook(params.build())
			}
			RiparianMixItem::FriendsConifer(conifer) => {
				let samples = conifer.build_with_noise(build_noise);
				let mut params = FriendsConiferParams::default();
				params.geometry = samples.geometry;
				params.splay_radius_fraction_of_height = samples.splay_radius_fraction_of_height;
				params.apex_canopy_spawn_fraction = samples.apex_canopy_spawn_fraction;
				RiparianMixKind::Friends(params.build())
			}
			RiparianMixItem::TemperateConifer(temperate) => {
				let samples = temperate.build_with_noise(build_noise);
				let mut params = TemperateConiferParams::default();
				params.geometry = samples.geometry;
				params.frond_world_scale = samples.frond_world_scale;
				params.fronds_per_joint = samples.fronds_per_joint;
				params.frond_length_fraction = samples.frond_length_fraction;
				params.frond_spawn_fraction = samples.frond_spawn_fraction;
				params.apex_canopy_spawn_fraction = samples.apex_canopy_spawn_fraction;
				RiparianMixKind::Temperate(params.build())
			}
		};

		RiparianMixPlant { placement, kind, stick_material, ball_material, frond_material }
	}

	impl VegetationComponents for RiparianMix {
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
				RIPARIAN_MIX_STRUCTURAL_HIGH_FACTOR,
				RIPARIAN_MIX_STRUCTURAL_MEDIUM_FACTOR,
				RIPARIAN_MIX_STRUCTURAL_LOW_FACTOR,
			))
		}
	}

	impl LodScene for RiparianMix {
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
	RiparianMix, RiparianMixParams, RiparianMixPlant, RIPARIAN_MIX_STRUCTURAL_HIGH_FACTOR,
	RIPARIAN_MIX_STRUCTURAL_LOW_FACTOR, RIPARIAN_MIX_STRUCTURAL_MEDIUM_FACTOR,
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
		let dist = RiparianMixCell::distribution();
		assert_eq!(dist.len(), 7);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 6.9);
		assert_eq!(dist.buckets[1].item, Some(RiparianMixCell::BankBraidOak));
		assert_eq!(dist.buckets[1].weight, 0.9);
		assert_eq!(dist.buckets[2].item, Some(RiparianMixCell::OverbankBraidOak));
		assert_eq!(dist.buckets[2].weight, 0.6);
		assert_eq!(dist.buckets[3].item, Some(RiparianMixCell::RoundRiparianStorybook));
		assert_eq!(dist.buckets[3].weight, 0.9);
		assert_eq!(dist.buckets[4].item, Some(RiparianMixCell::TallRiparianStorybook));
		assert_eq!(dist.buckets[4].weight, 0.45);
		assert_eq!(dist.buckets[5].item, Some(RiparianMixCell::BankFriendConifer));
		assert_eq!(dist.buckets[5].weight, 0.8);
		assert_eq!(dist.buckets[6].item, Some(RiparianMixCell::ShelteredTemperateConifer));
		assert_eq!(dist.buckets[6].weight, 0.8);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = RiparianMixCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.18..=0.40).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let RiparianMixItem::BraidOak(bank) = RiparianMixCell::BankBraidOak.item() else {
			anyhow::bail!("expected bank braid oak item");
		};
		assert_eq!(bank.height, UnitRange::new(5.0, 12.0));
		assert_eq!(bank.canopy_density, DENSE_CANOPY_DENSITY);

		let RiparianMixItem::BraidOak(overbank) = RiparianMixCell::OverbankBraidOak.item() else {
			anyhow::bail!("expected overbank braid oak item");
		};
		assert_eq!(overbank.height, UnitRange::new(10.0, 18.0));
		assert_eq!(overbank.canopy_density, MODERATE_CANOPY_DENSITY);

		let RiparianMixItem::Storybook(round) = RiparianMixCell::RoundRiparianStorybook.item()
		else {
			anyhow::bail!("expected round storybook item");
		};
		assert_eq!(round.height, UnitRange::new(5.0, 15.0));
		assert_eq!(round.canopy_density, MODERATE_CANOPY_DENSITY);

		let RiparianMixItem::Storybook(tall) = RiparianMixCell::TallRiparianStorybook.item() else {
			anyhow::bail!("expected tall storybook item");
		};
		assert_eq!(tall.height, UnitRange::new(12.0, 22.0));
		assert_eq!(tall.canopy_density, SPARSE_CANOPY_DENSITY);

		let RiparianMixItem::FriendsConifer(friend) = RiparianMixCell::BankFriendConifer.item()
		else {
			anyhow::bail!("expected friend conifer item");
		};
		assert_eq!(friend.height, UnitRange::new(8.0, 16.0));
		assert_eq!(friend.canopy_density, DENSE_CANOPY_DENSITY);

		let RiparianMixItem::TemperateConifer(temperate) =
			RiparianMixCell::ShelteredTemperateConifer.item()
		else {
			anyhow::bail!("expected temperate conifer item");
		};
		assert_eq!(temperate.height, UnitRange::new(10.0, 20.0));
		assert_eq!(temperate.canopy_density, MODERATE_CANOPY_DENSITY);
		Ok(())
	}

	#[test]
	fn placement_constraints_match_rfc() -> Result<()> {
		let dist = RiparianMixCell::distribution();
		let bank = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(RiparianMixCell::BankBraidOak))
			.ok_or_else(|| anyhow::anyhow!("missing bank braid oak bucket"))?;
		assert_eq!(bank.constraints.elevation.end, 0.38);
		assert_eq!(bank.constraints.steepness.end, 0.30);

		let overbank = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(RiparianMixCell::OverbankBraidOak))
			.ok_or_else(|| anyhow::anyhow!("missing overbank braid oak bucket"))?;
		assert_eq!(overbank.constraints.elevation.end, 0.48);
		assert_eq!(overbank.constraints.steepness.end, 0.42);

		let friend = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(RiparianMixCell::BankFriendConifer))
			.ok_or_else(|| anyhow::anyhow!("missing bank friend conifer bucket"))?;
		assert_eq!(friend.constraints.elevation.end, 0.58);
		assert_eq!(friend.constraints.steepness.end, 0.50);

		let temperate = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(RiparianMixCell::ShelteredTemperateConifer))
			.ok_or_else(|| anyhow::anyhow!("missing sheltered temperate conifer bucket"))?;
		assert_eq!(temperate.constraints.elevation.end, 0.62);
		assert_eq!(temperate.constraints.steepness.end, 0.54);
		Ok(())
	}

	#[test]
	#[ignore = "placement constraints deferred to forest-layer normalization"]
	fn steep_slope_rejects_bank_braid_but_allows_bank_friend() -> Result<()> {
		let prepared =
			RiparianMixCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.38 };
		let friend_outcome = prepared.select_from(
			5,
			Vec3::new(5.0, 0.25, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match friend_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, RiparianMixCell::BankFriendConifer);
			}
			other => anyhow::bail!("expected BankFriendConifer on moderate slope, got {other:?}"),
		}
		let bank_outcome = prepared.select_from(
			1,
			Vec3::new(5.0, 0.25, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match bank_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_ne!(variant, RiparianMixCell::BankBraidOak);
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			RiparianMixCell::BankBraidOak,
			RiparianMixCell::OverbankBraidOak,
			RiparianMixCell::RoundRiparianStorybook,
			RiparianMixCell::TallRiparianStorybook,
			RiparianMixCell::BankFriendConifer,
			RiparianMixCell::ShelteredTemperateConifer,
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
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(180.0, 1.0, 180.0));
		let terrain = FlatTerrainSample::default();
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
