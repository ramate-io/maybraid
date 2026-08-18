//! Alpine — cold upland conifer upper-canopy grove
//! ([RFC-183 §3.4.7.12], [#334](https://github.com/ramate-io/maybraid/issues/334)).
//!
//! Tall Friend's Conifer with less common Liam's Conifer on high, steep terrain. Forest-layer
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
/// Moderate sampled canopy-density band ([`0.25`, `0.45`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.25, 0.45);
/// Dense sampled canopy-density band ([`0.35`, `0.85`]).
const DENSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.85);

/// Authored Alpine grove definition.
///
/// Cell footprint sits at the RFC midpoint (`27.0` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<AlpineCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(27.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-27.0, 27.0),
		),
		distribution: AlpineCell::distribution(),
	}
}

/// Ordered alpine varietals ([RFC-183 §3.4.7.12]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlpineCell {
	TallAlpineFriendsConifer,
	WindlineFriendsConifer,
	AlpineLiamsConifer,
	NeedleSpireLiamsConifer,
}

/// Typed authored geometry for one alpine varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlpineItem {
	FriendsConifer(&'static AlpineFriendsConifer),
	LiamsConifer(&'static AlpineLiamsConifer),
}

/// Authored geometry ranges for one Friend's Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct AlpineFriendsConifer {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Liam's Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct AlpineLiamsConifer {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_density: UnitRange,
}

const TALL_ALPINE_FRIENDS: AlpineFriendsConifer = AlpineFriendsConifer {
	height: UnitRange::new(12.0, 40.0),
	stalk_radius: UnitRange::new(0.32, 0.72),
	canopy_spread: UnitRange::new(4.0, 7.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const WINDLINE_FRIENDS: AlpineFriendsConifer = AlpineFriendsConifer {
	height: UnitRange::new(6.0, 22.0),
	stalk_radius: UnitRange::new(0.18, 0.42),
	canopy_spread: UnitRange::new(1.5, 5.0),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const ALPINE_LIAMS: AlpineLiamsConifer = AlpineLiamsConifer {
	height: UnitRange::new(8.0, 40.0),
	stalk_radius: UnitRange::new(0.25, 0.85),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const NEEDLE_SPIRE_LIAMS: AlpineLiamsConifer = AlpineLiamsConifer {
	height: UnitRange::new(6.0, 32.0),
	stalk_radius: UnitRange::new(0.30, 0.55),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const TALL_FRIENDS_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "moss_bark"),
]);

const TALL_FRIENDS_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_green", "blue_green"),
	PaletteSlot::new("deep_green", "dark_green"),
]);

const WINDLINE_FRIENDS_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("wind_barked", "cold_bark"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const WINDLINE_FRIENDS_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_green", "blue_green"),
	PaletteSlot::new("dark_green", "deep_green"),
]);

const ALPINE_LIAMS_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "conifer_bark"),
]);

const ALPINE_LIAMS_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_green", "blue_green"),
	PaletteSlot::new("deep_green", "dark_green"),
]);

const NEEDLE_SPIRE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_bark", "dark_bark"),
	PaletteSlot::new("stone_gray", "conifer_bark"),
]);

const NEEDLE_SPIRE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("blue_green", "dark_green"),
	PaletteSlot::new("cold_green", "deep_green"),
]);

impl AlpineCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `3.7`; the `None` weight of `9.5` puts the placed share at
	/// `3.7 / 13.2 ≈ 0.28`, mid RFC `DENSITY_RANGE` (`0.18..0.38`).
	pub fn distribution() -> GroveDistribution<Self> {
		let tall_friends =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.68));
		let windline_friends =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.86));
		let alpine_liams =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.86));
		let needle_spire =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.92));
		GroveDistribution::new(vec![
			GroveBucket::none(9.5),
			GroveBucket::placed(1.5, tall_friends, Self::TallAlpineFriendsConifer),
			GroveBucket::placed(0.75, windline_friends, Self::WindlineFriendsConifer),
			GroveBucket::placed(1.0, alpine_liams, Self::AlpineLiamsConifer),
			GroveBucket::placed(0.45, needle_spire, Self::NeedleSpireLiamsConifer),
		])
	}

	pub fn item(self) -> AlpineItem {
		match self {
			Self::TallAlpineFriendsConifer | Self::WindlineFriendsConifer => match self {
				Self::TallAlpineFriendsConifer => AlpineItem::FriendsConifer(&TALL_ALPINE_FRIENDS),
				Self::WindlineFriendsConifer => AlpineItem::FriendsConifer(&WINDLINE_FRIENDS),
				_ => unreachable!(),
			},
			Self::AlpineLiamsConifer => AlpineItem::LiamsConifer(&ALPINE_LIAMS),
			Self::NeedleSpireLiamsConifer => AlpineItem::LiamsConifer(&NEEDLE_SPIRE_LIAMS),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::TallAlpineFriendsConifer => TALL_FRIENDS_STICK_MIX,
			Self::WindlineFriendsConifer => WINDLINE_FRIENDS_STICK_MIX,
			Self::AlpineLiamsConifer => ALPINE_LIAMS_STICK_MIX,
			Self::NeedleSpireLiamsConifer => NEEDLE_SPIRE_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::TallAlpineFriendsConifer => TALL_FRIENDS_CANOPY_MIX,
			Self::WindlineFriendsConifer => WINDLINE_FRIENDS_CANOPY_MIX,
			Self::AlpineLiamsConifer => ALPINE_LIAMS_CANOPY_MIX,
			Self::NeedleSpireLiamsConifer => NEEDLE_SPIRE_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
mod vc {
	use super::variants::alpine_friends_conifer::FriendsConiferSamples;
	use bevy::math::bounding::Aabb3d;
	use bevy::prelude::*;
	use bevy::scene::prelude::Scene;
	use chico_sbs_trees::{FriendsConifer, FriendsConiferParams, LiamsConifer, LiamsConiferParams};
	use chico_vegetation_components::{
		FoliageNode, Layers, Placement, StickNode, StructuralLod, VegetationComponents,
	};
	use clap::Args;
	use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
	use lod::lod_ref::LodRef;
	use lod::{lod_host_scene_pending, SceneChunk};
	use material_ref::MaterialRef;
	use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};

	use super::{definition, AlpineCell, AlpineItem};
	use crate::grove::{
		canopy_ball_material_from_palette, canopy_proxy_site, foliage_low_canopy_balls,
		foliage_ultra_low_merged_balls, frond_material_from_palette, grove_detail_level,
		grove_lod_culls, grove_lod_level, grove_lod_status, grove_structural_footprint,
		layers_from_nodes, nest_placed_plant_chunk, placement_noise, stick_material_from_palette,
		woody_grove_scene_chunks, CanopyProxySite, FlatTerrainSample, GroveCellVariant,
		GroveExtent, GroveFrontend, DEFAULT_GROVE_EXTENT_XZ, ULTRA_LOW_CANOPY_BIN_METERS,
	};

	pub const ALPINE_STRUCTURAL_HIGH_FACTOR: f32 = 2.0;
	pub const ALPINE_STRUCTURAL_MEDIUM_FACTOR: f32 = 5.0;
	pub const ALPINE_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct AlpineParams {
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
		resolved_placements: Option<Vec<GroveCellVariant<AlpineCell>>>,
	}

	impl Default for AlpineParams {
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

	impl AlpineParams {
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

		pub fn placements(&self) -> Vec<GroveCellVariant<AlpineCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.placements_on(&self.terrain)
		}

		/// Select placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn placements_on(
			&self,
			world: &impl crate::GroveWorldSample,
		) -> Vec<GroveCellVariant<AlpineCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.grove.assemble(definition()).populate(&self.extent, world)
		}

		pub fn build(&self) -> Alpine {
			self.build_on(&self.terrain)
		}

		/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> Alpine {
			Alpine::from_placements(&self.placements_on(world), self.grove.noise, &self.extent)
		}
	}

	#[derive(Clone)]
	enum AlpineKind {
		Friends(FriendsConifer),
		Liams(LiamsConifer),
	}

	#[derive(Clone)]
	pub struct AlpinePlant {
		pub placement: Placement,
		kind: AlpineKind,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	}

	#[derive(Clone, Component)]
	pub struct Alpine {
		pub plants: Vec<AlpinePlant>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl Alpine {
		pub fn from_placements(
			placements: &[GroveCellVariant<AlpineCell>],
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
					AlpineKind::Friends(t) => nest_placed_plant_chunk(
						t.clone(),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						lod_ref,
					),
					AlpineKind::Liams(t) => nest_placed_plant_chunk(
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
						AlpineKind::Friends(t) => canopy_proxy_site(t, plant.placement, material),
						AlpineKind::Liams(t) => canopy_proxy_site(t, plant.placement, material),
					}
				})
				.collect()
		}
	}

	fn grow_plant(placed: &GroveCellVariant<AlpineCell>, grove_noise: NoiseParams) -> AlpinePlant {
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
			AlpineItem::FriendsConifer(conifer) => {
				let samples =
					BuildWithNoise::<FriendsConiferSamples>::build_with_noise(conifer, build_noise);
				let mut params = FriendsConiferParams::default();
				params.geometry = samples.geometry;
				params.splay_radius_fraction_of_height = samples.splay_radius_fraction_of_height;
				params.apex_canopy_spawn_fraction = samples.apex_canopy_spawn_fraction;
				AlpineKind::Friends(params.build())
			}
			AlpineItem::LiamsConifer(conifer) => {
				let geometry = conifer.build_with_noise(build_noise);
				let mut params = LiamsConiferParams::default();
				params.geometry = geometry;
				AlpineKind::Liams(params.build())
			}
		};

		AlpinePlant { placement, kind, stick_material, ball_material, frond_material }
	}

	impl VegetationComponents for Alpine {
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
				ALPINE_STRUCTURAL_HIGH_FACTOR,
				ALPINE_STRUCTURAL_MEDIUM_FACTOR,
				ALPINE_STRUCTURAL_LOW_FACTOR,
			))
		}
	}

	impl LodScene for Alpine {
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
	Alpine, AlpineParams, AlpinePlant, ALPINE_STRUCTURAL_HIGH_FACTOR, ALPINE_STRUCTURAL_LOW_FACTOR,
	ALPINE_STRUCTURAL_MEDIUM_FACTOR,
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
		let dist = AlpineCell::distribution();
		assert_eq!(dist.len(), 5);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 9.5);
		assert_eq!(dist.buckets[1].item, Some(AlpineCell::TallAlpineFriendsConifer));
		assert_eq!(dist.buckets[1].weight, 1.5);
		assert_eq!(dist.buckets[2].item, Some(AlpineCell::WindlineFriendsConifer));
		assert_eq!(dist.buckets[2].weight, 0.75);
		assert_eq!(dist.buckets[3].item, Some(AlpineCell::AlpineLiamsConifer));
		assert_eq!(dist.buckets[3].weight, 1.0);
		assert_eq!(dist.buckets[4].item, Some(AlpineCell::NeedleSpireLiamsConifer));
		assert_eq!(dist.buckets[4].weight, 0.45);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = AlpineCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.18..=0.38).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let AlpineItem::FriendsConifer(tall) = AlpineCell::TallAlpineFriendsConifer.item() else {
			anyhow::bail!("expected tall friends item");
		};
		assert_eq!(tall.height, UnitRange::new(12.0, 40.0));
		assert_eq!(tall.canopy_density, DENSE_CANOPY_DENSITY);

		let AlpineItem::LiamsConifer(spire) = AlpineCell::NeedleSpireLiamsConifer.item() else {
			anyhow::bail!("expected needle spire item");
		};
		assert_eq!(spire.height, UnitRange::new(6.0, 32.0));
		assert_eq!(spire.canopy_density, SPARSE_CANOPY_DENSITY);
		Ok(())
	}

	#[test]
	fn placement_constraints_use_full_elevation_and_rfc_steepness() -> Result<()> {
		let dist = AlpineCell::distribution();
		for bucket in dist.buckets.iter().filter(|b| b.item.is_some()) {
			assert_eq!(bucket.constraints.elevation.start, 0.0);
			assert_eq!(bucket.constraints.elevation.end, 1.0);
		}
		let tall = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(AlpineCell::TallAlpineFriendsConifer))
			.ok_or_else(|| anyhow::anyhow!("missing tall friends bucket"))?;
		assert_eq!(tall.constraints.steepness.end, 0.68);

		let windline = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(AlpineCell::WindlineFriendsConifer))
			.ok_or_else(|| anyhow::anyhow!("missing windline friends bucket"))?;
		assert_eq!(windline.constraints.steepness.end, 0.86);
		Ok(())
	}

	#[test]
	#[ignore = "placement constraints deferred to forest-layer normalization"]
	fn steep_slope_rejects_tall_friends_but_allows_windline() -> Result<()> {
		let prepared =
			AlpineCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let moderate = FlatTerrainSample { elevation: 0.30, steepness: 0.40 };
		let moderate_outcome = prepared.select_from(
			1,
			Vec3::new(5.0, 0.30, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&moderate,
		);
		match moderate_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, AlpineCell::TallAlpineFriendsConifer);
			}
			other => {
				anyhow::bail!("expected TallAlpineFriendsConifer on moderate slope, got {other:?}")
			}
		}
		let steep = FlatTerrainSample { elevation: 0.30, steepness: 0.70 };
		let steep_outcome = prepared.select_from(
			1,
			Vec3::new(5.0, 0.30, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&steep,
		);
		match steep_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, AlpineCell::WindlineFriendsConifer);
			}
			other => anyhow::bail!("expected WindlineFriendsConifer on steep ridge, got {other:?}"),
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			AlpineCell::TallAlpineFriendsConifer,
			AlpineCell::WindlineFriendsConifer,
			AlpineCell::AlpineLiamsConifer,
			AlpineCell::NeedleSpireLiamsConifer,
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
