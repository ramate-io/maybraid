//! Goettingen Follow — well-known low-density temperate lower-canopy follow grove
//! ([RFC-183 §3.4.6.4], [#325](https://github.com/ramate-io/maybraid/issues/325)).
//!
//! Sparse braid oaks and storybook forms beneath taller canopy. Forest-layer attachment remains
//! a follow-up.

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);

/// Authored Goettingen Follow grove definition.
///
/// Cell footprint at `9.0` m (below the RFC midpoint for tighter follow-layer spacing). The offset
/// range is signed and ± one cell so placements break the underlying grid.
pub fn definition() -> GroveDefinition<GoettingenFollowCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(9.0),
		placement: GrovePlacementRanges::new(UnitRange::new(0.85, 1.15), UnitRange::new(-9.0, 9.0)),
		distribution: GoettingenFollowCell::distribution(),
	}
}

/// Ordered goettingen-follow varietals ([RFC-183 §3.4.6.4]); the explicit `None` bucket lives only
/// in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoettingenFollowCell {
	FollowBraidOak,
	RedBranchBraidOak,
	MossyTrailBraidOak,
	ParkEdgeBraidOak,
	TallFollowBraidOak,
	OldGrowthFollowBraidOak,
	FollowStorybook,
}

/// Typed authored geometry for one goettingen-follow varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GoettingenFollowItem {
	BraidOak(&'static GoettingenFollowBraidOak),
	Storybook(&'static GoettingenFollowStorybook),
}

/// Authored geometry ranges for one Braid Oak form (shared geometry; palette differs per cell).
#[derive(Debug, Clone, PartialEq)]
pub struct GoettingenFollowBraidOak {
	pub height: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse..moderate canopy density at render time.
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one follow Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct GoettingenFollowStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const FOLLOW_BRAID_OAK: GoettingenFollowBraidOak = GoettingenFollowBraidOak {
	height: UnitRange::new(4.0, 9.0),
	canopy_spread: UnitRange::new(1.6, 4.2),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const TALL_FOLLOW_BRAID_OAK: GoettingenFollowBraidOak = GoettingenFollowBraidOak {
	height: UnitRange::new(7.0, 11.0),
	canopy_spread: UnitRange::new(2.0, 4.8),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const OLD_GROWTH_FOLLOW_BRAID_OAK: GoettingenFollowBraidOak = GoettingenFollowBraidOak {
	height: UnitRange::new(8.0, 12.0),
	canopy_spread: UnitRange::new(2.2, 5.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const FOLLOW_STORYBOOK: GoettingenFollowStorybook = GoettingenFollowStorybook {
	height: UnitRange::new(4.0, 9.0),
	stalk_radius: UnitRange::new(0.18, 0.40),
	canopy_spread: UnitRange::new(1.6, 4.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const FOLLOW_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("oak_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "moss_bark"),
]);

const FOLLOW_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("dark_green", "light_green"),
]);

const RED_BRANCH_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("red_oak_bark", "copper_red"),
	PaletteSlot::new("dark_bark", "gray_brown"),
]);

const RED_BRANCH_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("yellow_green", "light_green"),
]);

const MOSSY_TRAIL_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("moss_bark", "gnarled_brown"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const MOSSY_TRAIL_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("moss_green", "olive_green"),
	PaletteSlot::new("deep_green", "fresh_green"),
]);

const PARK_EDGE_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("ornamental_bark", "young_bark"),
	PaletteSlot::new("oak_bark", "gray_brown"),
]);

const PARK_EDGE_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("silver_green", "broadleaf_green"),
	PaletteSlot::new("light_green", "fresh_green"),
]);

const TALL_FOLLOW_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("oak_bark", "dark_bark"),
	PaletteSlot::new("moss_bark", "gray_brown"),
]);

const TALL_FOLLOW_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "deep_green"),
	PaletteSlot::new("olive_green", "light_green"),
]);

const OLD_GROWTH_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("gnarled_brown", "dark_bark"),
	PaletteSlot::new("moss_bark", "wet_bark"),
]);

const OLD_GROWTH_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "moss_green"),
	PaletteSlot::new("emerald_green", "fresh_green"),
]);

const FOLLOW_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("broadleaf_bark", "brown_bark"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const FOLLOW_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("broadleaf_green", "light_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);

impl GoettingenFollowCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `3.75` (RFC braid-oak and storybook proportions plus follow accents);
	/// the `None` weight of `9.7` puts the placed share at `3.75 / 13.45 ≈ 0.28`, upper RFC
	/// `DENSITY_RANGE` (`0.10..0.28`).
	/// Placement constraints are unconstrained until RFC elevation bands land ([#325](https://github.com/ramate-io/maybraid/issues/325)).
	pub fn distribution() -> GroveDistribution<Self> {
		GroveDistribution::new(vec![
			GroveBucket::none(9.7),
			GroveBucket::placed(1.0, PlacementConstraints::UNCONSTRAINED, Self::FollowBraidOak),
			GroveBucket::placed(0.35, PlacementConstraints::UNCONSTRAINED, Self::RedBranchBraidOak),
			GroveBucket::placed(
				0.40,
				PlacementConstraints::UNCONSTRAINED,
				Self::MossyTrailBraidOak,
			),
			GroveBucket::placed(0.30, PlacementConstraints::UNCONSTRAINED, Self::ParkEdgeBraidOak),
			GroveBucket::placed(
				0.45,
				PlacementConstraints::UNCONSTRAINED,
				Self::TallFollowBraidOak,
			),
			GroveBucket::placed(
				0.25,
				PlacementConstraints::UNCONSTRAINED,
				Self::OldGrowthFollowBraidOak,
			),
			GroveBucket::placed(1.0, PlacementConstraints::UNCONSTRAINED, Self::FollowStorybook),
		])
	}

	pub fn item(self) -> GoettingenFollowItem {
		match self {
			Self::FollowBraidOak
			| Self::RedBranchBraidOak
			| Self::MossyTrailBraidOak
			| Self::ParkEdgeBraidOak => GoettingenFollowItem::BraidOak(&FOLLOW_BRAID_OAK),
			Self::TallFollowBraidOak => GoettingenFollowItem::BraidOak(&TALL_FOLLOW_BRAID_OAK),
			Self::OldGrowthFollowBraidOak => {
				GoettingenFollowItem::BraidOak(&OLD_GROWTH_FOLLOW_BRAID_OAK)
			}
			Self::FollowStorybook => GoettingenFollowItem::Storybook(&FOLLOW_STORYBOOK),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::FollowBraidOak => FOLLOW_BRAID_OAK_STICK_MIX,
			Self::RedBranchBraidOak => RED_BRANCH_BRAID_OAK_STICK_MIX,
			Self::MossyTrailBraidOak => MOSSY_TRAIL_BRAID_OAK_STICK_MIX,
			Self::ParkEdgeBraidOak => PARK_EDGE_BRAID_OAK_STICK_MIX,
			Self::TallFollowBraidOak => TALL_FOLLOW_BRAID_OAK_STICK_MIX,
			Self::OldGrowthFollowBraidOak => OLD_GROWTH_BRAID_OAK_STICK_MIX,
			Self::FollowStorybook => FOLLOW_STORYBOOK_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::FollowBraidOak => FOLLOW_BRAID_OAK_CANOPY_MIX,
			Self::RedBranchBraidOak => RED_BRANCH_BRAID_OAK_CANOPY_MIX,
			Self::MossyTrailBraidOak => MOSSY_TRAIL_BRAID_OAK_CANOPY_MIX,
			Self::ParkEdgeBraidOak => PARK_EDGE_BRAID_OAK_CANOPY_MIX,
			Self::TallFollowBraidOak => TALL_FOLLOW_BRAID_OAK_CANOPY_MIX,
			Self::OldGrowthFollowBraidOak => OLD_GROWTH_BRAID_OAK_CANOPY_MIX,
			Self::FollowStorybook => FOLLOW_STORYBOOK_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
mod vc {
	use bevy::math::bounding::Aabb3d;
	use bevy::prelude::*;
	use bevy::scene::prelude::Scene;
	use chico_sbs_trees::{BraidOakTree, BraidOakTreeParams, StorybookTree, StorybookTreeParams};
	use chico_vegetation_components::{
		FoliageNode, Layers, Placement, StickNode, StructuralLod, VegetationComponents,
	};
	use clap::Args;
	use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
	use lod::lod_ref::LodRef;
	use lod::{lod_host_scene_pending, SceneChunk};
	use material_ref::MaterialRef;
	use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};

	use super::{definition, GoettingenFollowCell, GoettingenFollowItem};
	use crate::grove::{
		canopy_ball_material_from_palette, canopy_proxy_site, foliage_low_canopy_balls,
		foliage_ultra_low_merged_balls, frond_material_from_palette, grove_detail_level,
		grove_lod_culls, grove_lod_level, grove_lod_status, grove_structural_footprint,
		layers_from_nodes, nest_placed_plant_chunk, placement_noise, stick_material_from_palette,
		woody_grove_scene_chunks, CanopyProxySite, FlatTerrainSample, GroveCellVariant,
		GroveExtent, GroveFrontend, DEFAULT_GROVE_EXTENT_XZ, ULTRA_LOW_CANOPY_BIN_METERS,
	};

	pub const GOETTINGEN_FOLLOW_STRUCTURAL_HIGH_FACTOR: f32 = 2.0;
	pub const GOETTINGEN_FOLLOW_STRUCTURAL_MEDIUM_FACTOR: f32 = 5.0;
	pub const GOETTINGEN_FOLLOW_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct GoettingenFollowParams {
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
		resolved_placements: Option<Vec<GroveCellVariant<GoettingenFollowCell>>>,
	}

	impl Default for GoettingenFollowParams {
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
				terrain: FlatTerrainSample { elevation: 0.25, steepness: 0.12 },
				resolved_placements: None,
			}
		}
	}

	impl GoettingenFollowParams {
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

		pub fn placements(&self) -> Vec<GroveCellVariant<GoettingenFollowCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.placements_on(&self.terrain)
		}

		/// Select placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn placements_on(
			&self,
			world: &impl crate::GroveWorldSample,
		) -> Vec<GroveCellVariant<GoettingenFollowCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.grove.assemble(definition()).populate(&self.extent, world)
		}

		pub fn build(&self) -> GoettingenFollow {
			self.build_on(&self.terrain)
		}

		/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> GoettingenFollow {
			GoettingenFollow::from_placements(
				&self.placements_on(world),
				self.grove.noise,
				self.stick_surface_noise,
				&self.extent,
			)
		}
	}

	#[derive(Clone)]
	enum GoettingenFollowKind {
		Oak(BraidOakTree),
		Storybook(StorybookTree),
	}

	#[derive(Clone)]
	pub struct GoettingenFollowPlant {
		pub placement: Placement,
		kind: GoettingenFollowKind,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	}

	#[derive(Clone, Component)]
	pub struct GoettingenFollow {
		pub plants: Vec<GoettingenFollowPlant>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl GoettingenFollow {
		pub fn from_placements(
			placements: &[GroveCellVariant<GoettingenFollowCell>],
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
					GoettingenFollowKind::Oak(t) => nest_placed_plant_chunk(
						t.clone(),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						lod_ref,
					),
					GoettingenFollowKind::Storybook(t) => nest_placed_plant_chunk(
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
						GoettingenFollowKind::Oak(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						GoettingenFollowKind::Storybook(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
					}
				})
				.collect()
		}
	}

	fn grow_plant(
		placed: &GroveCellVariant<GoettingenFollowCell>,
		grove_noise: NoiseParams,
		stick_surface_noise: NoiseParams,
	) -> GoettingenFollowPlant {
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
			GoettingenFollowItem::BraidOak(oak) => {
				let geometry = oak.build_with_noise(build_noise);
				let mut params = BraidOakTreeParams::default();
				params.geometry = geometry;
				params.stick_surface_noise = placement_noise(stick_surface_noise, placed.position);
				GoettingenFollowKind::Oak(params.build())
			}
			GoettingenFollowItem::Storybook(story) => {
				let geometry = story.build_with_noise(build_noise);
				let mut params = StorybookTreeParams::default();
				params.geometry = geometry;
				GoettingenFollowKind::Storybook(params.build())
			}
		};

		GoettingenFollowPlant { placement, kind, stick_material, ball_material, frond_material }
	}

	impl VegetationComponents for GoettingenFollow {
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
				GOETTINGEN_FOLLOW_STRUCTURAL_HIGH_FACTOR,
				GOETTINGEN_FOLLOW_STRUCTURAL_MEDIUM_FACTOR,
				GOETTINGEN_FOLLOW_STRUCTURAL_LOW_FACTOR,
			))
		}
	}

	impl LodScene for GoettingenFollow {
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
	GoettingenFollow, GoettingenFollowParams, GoettingenFollowPlant,
	GOETTINGEN_FOLLOW_STRUCTURAL_HIGH_FACTOR, GOETTINGEN_FOLLOW_STRUCTURAL_LOW_FACTOR,
	GOETTINGEN_FOLLOW_STRUCTURAL_MEDIUM_FACTOR,
};

#[cfg(test)]
mod tests {
	use super::*;
	use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveExtent};
	use anyhow::Result;
	use bevy_math::Vec3;
	use procedural_common::NoiseParams;

	#[test]
	fn distribution_matches_rfc_order_and_weights() -> Result<()> {
		let dist = GoettingenFollowCell::distribution();
		assert_eq!(dist.len(), 8);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 9.7);
		assert_eq!(dist.buckets[1].item, Some(GoettingenFollowCell::FollowBraidOak));
		assert_eq!(dist.buckets[1].weight, 1.0);
		assert_eq!(dist.buckets[2].item, Some(GoettingenFollowCell::RedBranchBraidOak));
		assert_eq!(dist.buckets[2].weight, 0.35);
		assert_eq!(dist.buckets[3].item, Some(GoettingenFollowCell::MossyTrailBraidOak));
		assert_eq!(dist.buckets[3].weight, 0.40);
		assert_eq!(dist.buckets[4].item, Some(GoettingenFollowCell::ParkEdgeBraidOak));
		assert_eq!(dist.buckets[4].weight, 0.30);
		assert_eq!(dist.buckets[5].item, Some(GoettingenFollowCell::TallFollowBraidOak));
		assert_eq!(dist.buckets[5].weight, 0.45);
		assert_eq!(dist.buckets[6].item, Some(GoettingenFollowCell::OldGrowthFollowBraidOak));
		assert_eq!(dist.buckets[6].weight, 0.25);
		assert_eq!(dist.buckets[7].item, Some(GoettingenFollowCell::FollowStorybook));
		assert_eq!(dist.buckets[7].weight, 1.0);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = GoettingenFollowCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.10..=0.28).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let GoettingenFollowItem::BraidOak(oak) = GoettingenFollowCell::FollowBraidOak.item()
		else {
			anyhow::bail!("expected braid oak item");
		};
		assert_eq!(oak.height, UnitRange::new(4.0, 9.0));
		assert_eq!(oak.canopy_density, MODERATE_CANOPY_DENSITY);

		let GoettingenFollowItem::BraidOak(tall) = GoettingenFollowCell::TallFollowBraidOak.item()
		else {
			anyhow::bail!("expected tall braid oak item");
		};
		assert_eq!(tall.height, UnitRange::new(7.0, 11.0));

		let GoettingenFollowItem::BraidOak(old) =
			GoettingenFollowCell::OldGrowthFollowBraidOak.item()
		else {
			anyhow::bail!("expected old-growth braid oak item");
		};
		assert_eq!(old.height, UnitRange::new(8.0, 12.0));

		let GoettingenFollowItem::Storybook(story) = GoettingenFollowCell::FollowStorybook.item()
		else {
			anyhow::bail!("expected storybook item");
		};
		assert_eq!(story.height, UnitRange::new(4.0, 9.0));
		assert_eq!(story.canopy_spread, UnitRange::new(1.6, 4.0));
		Ok(())
	}

	#[test]
	fn placements_break_the_cell_grid() -> Result<()> {
		let noise = crate::grove::GroveFrontend::default().noise;
		let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0));
		let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.12 };
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
		let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.12 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
