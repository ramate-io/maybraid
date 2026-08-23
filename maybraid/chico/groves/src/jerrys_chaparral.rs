//! Jerry's Chaparral — well-known moderately dense dry scrub understory grove
//! ([RFC-183 §3.4.5.7], [#318](https://github.com/ramate-io/maybraid/issues/318)).
//!
//! Mixes Rory's Head-trained forms, Common High Bush chaparral mass, and rare small Friend's
//! Conifer accents. Forest-layer attachment remains a follow-up.

use std::ops::RangeInclusive;

use bevy_math::{Vec2, Vec3};
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, GroveWorldSample,
	PaletteMix, PaletteSlot, PlacementConstraints,
};

/// Uniform terrain tuned for chaparral placement constraints (RFC min elevation > 0).
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "render", derive(clap::Args))]
#[cfg_attr(feature = "render", command(next_help_heading = "Terrain"))]
pub struct ChaparralFlatTerrain {
	#[cfg_attr(feature = "render", arg(long, default_value_t = 0.35))]
	pub elevation: f32,
	#[cfg_attr(feature = "render", arg(long, default_value_t = 0.15))]
	pub steepness: f32,
}

impl Default for ChaparralFlatTerrain {
	fn default() -> Self {
		Self { elevation: 0.35, steepness: 0.15 }
	}
}

impl GroveWorldSample for ChaparralFlatTerrain {
	fn height_at(&self, _position: Vec3) -> f32 {
		self.elevation
	}

	fn steepness_at(&self, _position: Vec3) -> f32 {
		self.steepness
	}
}

/// RFC `projection_count: Moderate` — chaparral high-bush varietal.
const MODERATE_PROJECTION_RADIAL: UnitRange = UnitRange::new(0.32, 0.48);
const MODERATE_PROJECTION_VERTICAL: UnitRange = UnitRange::new(0.58, 0.78);

/// Authored Jerry's Chaparral grove definition.
///
/// Cell footprint sits at the RFC midpoint (`6.5` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<JerrysChaparralCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(6.5),
		placement: GrovePlacementRanges::new(UnitRange::new(0.85, 1.15), UnitRange::new(-6.5, 6.5)),
		distribution: JerrysChaparralCell::distribution(),
	}
}

/// Ordered chaparral varietals ([RFC-183 §3.4.5.7]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JerrysChaparralCell {
	DryRoryHeadTrained,
	ChaparralHighBush,
	SmallFriendsConifer,
	ManzanitaRory,
}

/// Typed authored geometry for one chaparral varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JerrysChaparralItem {
	RoryHead(&'static JerrysChaparralRoryHead),
	Bush(&'static JerrysChaparralBush),
	FriendsConifer(&'static JerrysChaparralFriendsConifer),
}

/// Authored geometry ranges for one Rory's Head-trained form.
#[derive(Debug, Clone, PartialEq)]
pub struct JerrysChaparralRoryHead {
	pub height: UnitRange,
	/// World-space stalk base radius (RFC `0.030 × H`).
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse..moderate canopy density at render time.
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Common High Bush shrub.
#[derive(Debug, Clone, PartialEq)]
pub struct JerrysChaparralBush {
	pub height: UnitRange,
	pub shoot_count: RangeInclusive<u32>,
	pub branch_depth: RangeInclusive<u32>,
	pub radial_strength: UnitRange,
	pub vertical_bias: UnitRange,
	/// Terminal foliage radius in **world meters** (render converts to a height fraction).
	pub leaf_radius: UnitRange,
}

/// Authored geometry ranges for one small Friend's Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct JerrysChaparralFriendsConifer {
	pub height: UnitRange,
	/// World-space stalk base radius (RFC `0.025 × H`).
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse..moderate canopy density at render time.
	pub canopy_density: UnitRange,
}

const DRY_RORY_HEAD: JerrysChaparralRoryHead = JerrysChaparralRoryHead {
	height: UnitRange::new(1.20, 3.20),
	stalk_radius: UnitRange::new(0.036, 0.096),
	canopy_spread: UnitRange::new(0.80, 2.00),
	canopy_density: UnitRange::new(0.35, 0.65),
};

const MANZANITA_RORY: JerrysChaparralRoryHead = JerrysChaparralRoryHead {
	height: UnitRange::new(1.40, 3.00),
	stalk_radius: UnitRange::new(0.042, 0.090),
	canopy_spread: UnitRange::new(0.90, 2.10),
	canopy_density: UnitRange::new(0.0, 0.35),
};

const CHAPARRAL_HIGH_BUSH: JerrysChaparralBush = JerrysChaparralBush {
	height: UnitRange::new(1.00, 2.40),
	shoot_count: 7..=11,
	branch_depth: 2..=4,
	radial_strength: MODERATE_PROJECTION_RADIAL,
	vertical_bias: MODERATE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.05, 0.11),
};

const SMALL_FRIENDS_CONIFER: JerrysChaparralFriendsConifer = JerrysChaparralFriendsConifer {
	height: UnitRange::new(2.00, 6.00),
	stalk_radius: UnitRange::new(0.05, 0.15),
	canopy_spread: UnitRange::new(0.50, 1.40),
	canopy_density: UnitRange::new(0.35, 0.65),
};

const DRY_RORY_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_bark", "gray_brown"),
	PaletteSlot::new("vine_bark", "tan_brown"),
]);

const DRY_RORY_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "dry_green"),
	PaletteSlot::new("scrub_green", "pale_green"),
	PaletteSlot::new("dark_green", "yellow_green"),
]);

const CHAPARRAL_BUSH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_bark", "tan_brown"),
	PaletteSlot::new("gray_brown", "straw_brown"),
]);

const CHAPARRAL_BUSH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_green", "olive_green"),
	PaletteSlot::new("scrub_green", "tan_green"),
	PaletteSlot::new("dark_green", "pale_green"),
]);

const FRIENDS_CONIFER_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("conifer_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "dry_bark"),
]);

const FRIENDS_CONIFER_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "blue_green"),
	PaletteSlot::new("dry_green", "deep_green"),
	PaletteSlot::new("olive_green", "needle_green"),
]);

const MANZANITA_RORY_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("manzanita_red", "copper_red"),
	PaletteSlot::new("smooth_burgundy", "orange_bark"),
]);

const MANZANITA_RORY_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "pale_green"),
	PaletteSlot::new("flower_white", "dry_green"),
	PaletteSlot::new("dark_green", "yellow_green"),
]);

impl JerrysChaparralCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `4.3` (RFC relative proportions); the `None` weight of `7.0` puts
	/// the placed share at `4.3 / 11.3 ≈ 0.38`, mid RFC `DENSITY_RANGE` (`0.24..0.52`).
	pub fn distribution() -> GroveDistribution<Self> {
		let dry_rory =
			PlacementConstraints::new(UnitRange::new(0.10, 0.65), UnitRange::new(0.0, 0.78));
		let bush = PlacementConstraints::new(UnitRange::new(0.05, 0.70), UnitRange::new(0.0, 0.55));
		let conifer =
			PlacementConstraints::new(UnitRange::new(0.15, 0.75), UnitRange::new(0.0, 0.65));
		let manzanita =
			PlacementConstraints::new(UnitRange::new(0.15, 0.70), UnitRange::new(0.0, 0.72));
		GroveDistribution::new(vec![
			GroveBucket::none(7.0),
			GroveBucket::placed(1.5, dry_rory, Self::DryRoryHeadTrained),
			GroveBucket::placed(2.0, bush, Self::ChaparralHighBush),
			GroveBucket::placed(0.45, conifer, Self::SmallFriendsConifer),
			GroveBucket::placed(0.35, manzanita, Self::ManzanitaRory),
		])
	}

	pub fn item(self) -> JerrysChaparralItem {
		match self {
			Self::DryRoryHeadTrained => JerrysChaparralItem::RoryHead(&DRY_RORY_HEAD),
			Self::ChaparralHighBush => JerrysChaparralItem::Bush(&CHAPARRAL_HIGH_BUSH),
			Self::SmallFriendsConifer => {
				JerrysChaparralItem::FriendsConifer(&SMALL_FRIENDS_CONIFER)
			}
			Self::ManzanitaRory => JerrysChaparralItem::RoryHead(&MANZANITA_RORY),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::DryRoryHeadTrained => DRY_RORY_STICK_MIX,
			Self::ChaparralHighBush => CHAPARRAL_BUSH_STICK_MIX,
			Self::SmallFriendsConifer => FRIENDS_CONIFER_STICK_MIX,
			Self::ManzanitaRory => MANZANITA_RORY_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::DryRoryHeadTrained => DRY_RORY_CANOPY_MIX,
			Self::ChaparralHighBush => CHAPARRAL_BUSH_CANOPY_MIX,
			Self::SmallFriendsConifer => FRIENDS_CONIFER_CANOPY_MIX,
			Self::ManzanitaRory => MANZANITA_RORY_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
mod vc {
	use super::variants::jerrys_chaparral_friends_conifer::ConiferSamples;
	use bevy::math::bounding::Aabb3d;
	use bevy::prelude::*;
	use bevy::scene::prelude::Scene;
	use chico_sbs_trees::{
		FriendsConifer, FriendsConiferParams, HighBushShoots, HighBushShootsParams,
		RorysHeadTrained, RorysHeadTrainedParams,
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

	use super::ChaparralFlatTerrain;
	use super::{definition, JerrysChaparralCell, JerrysChaparralItem};
	use crate::grove::{
		canopy_ball_material_from_palette, canopy_proxy_site, foliage_low_canopy_balls,
		foliage_ultra_low_merged_balls, frond_material_from_palette, grove_detail_level,
		grove_lod_culls, grove_lod_level, grove_lod_status, grove_structural_footprint,
		layers_from_nodes, nest_placed_plant_chunk, placement_noise, stick_material_from_palette,
		woody_grove_scene_chunks, CanopyProxySite, GroveCellVariant, GroveExtent, GroveFrontend,
		DEFAULT_GROVE_EXTENT_XZ, ULTRA_LOW_CANOPY_BIN_METERS,
	};

	pub const JERRYS_CHAPARRAL_STRUCTURAL_HIGH_FACTOR: f32 = 2.0;
	pub const JERRYS_CHAPARRAL_STRUCTURAL_MEDIUM_FACTOR: f32 = 5.0;
	pub const JERRYS_CHAPARRAL_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct JerrysChaparralParams {
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
		pub terrain: ChaparralFlatTerrain,

		#[arg(skip)]
		resolved_placements: Option<Vec<GroveCellVariant<JerrysChaparralCell>>>,
	}

	impl Default for JerrysChaparralParams {
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
				terrain: ChaparralFlatTerrain::default(),
				resolved_placements: None,
			}
		}
	}

	impl JerrysChaparralParams {
		pub fn with_extent(mut self, extent: GroveExtent) -> Self {
			self.extent = extent;
			self
		}

		pub fn with_terrain(mut self, terrain: ChaparralFlatTerrain) -> Self {
			self.terrain = terrain;
			self
		}

		pub fn cell_extent_xz(&self) -> Vec2 {
			self.grove.definition(definition()).cell_extent_xz
		}

		pub fn placement_cells(&self) -> Vec<gimme_gen::Cell> {
			self.extent.subdivide_xz(self.cell_extent_xz())
		}

		pub fn placements(&self) -> Vec<GroveCellVariant<JerrysChaparralCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.placements_on(&self.terrain)
		}

		/// Select placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn placements_on(
			&self,
			world: &impl crate::GroveWorldSample,
		) -> Vec<GroveCellVariant<JerrysChaparralCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.grove.assemble(definition()).populate(&self.extent, world)
		}

		pub fn build(&self) -> JerrysChaparral {
			self.build_on(&self.terrain)
		}

		/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> JerrysChaparral {
			JerrysChaparral::from_placements(
				&self.placements_on(world),
				self.grove.noise,
				self.tree_chain_noise,
				&self.extent,
			)
		}
	}

	#[derive(Clone)]
	enum JerrysChaparralKind {
		Rory(RorysHeadTrained),
		Bush(HighBushShoots),
		Friends(FriendsConifer),
	}

	#[derive(Clone)]
	pub struct JerrysChaparralPlant {
		pub placement: Placement,
		kind: JerrysChaparralKind,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	}

	#[derive(Clone, Component)]
	pub struct JerrysChaparral {
		pub plants: Vec<JerrysChaparralPlant>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl JerrysChaparral {
		pub fn from_placements(
			placements: &[GroveCellVariant<JerrysChaparralCell>],
			grove_noise: NoiseParams,
			tree_chain_noise: NoiseParams,
			extent: &GroveExtent,
		) -> Self {
			let plants = placements
				.iter()
				.map(|placed| grow_plant(placed, grove_noise, tree_chain_noise))
				.collect();
			let (structural_center, footprint_radius) = grove_structural_footprint(extent);
			Self { plants, structural_center, footprint_radius, extent: *extent }
		}

		fn nest_plant_chunks(&self, lod_ref: &LodRef) -> Vec<SceneChunk> {
			self.plants
				.iter()
				.map(|plant| match &plant.kind {
					JerrysChaparralKind::Rory(t) => nest_placed_plant_chunk(
						t.clone(),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						lod_ref,
					),
					JerrysChaparralKind::Bush(t) => nest_placed_plant_chunk(
						t.clone(),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						lod_ref,
					),
					JerrysChaparralKind::Friends(t) => nest_placed_plant_chunk(
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
						JerrysChaparralKind::Rory(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						JerrysChaparralKind::Bush(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						JerrysChaparralKind::Friends(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
					}
				})
				.collect()
		}
	}

	fn grow_plant(
		placed: &GroveCellVariant<JerrysChaparralCell>,
		grove_noise: NoiseParams,
		tree_chain_noise: NoiseParams,
	) -> JerrysChaparralPlant {
		let build_noise = placement_noise(grove_noise, placed.position);
		let chain_noise = placement_noise(tree_chain_noise, placed.position);
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
		let placement =
			Placement::new(placed.position, 0.0).with_scale(Vec3::splat(placed.scale.max(1e-4)));

		let kind = match placed.variant.item() {
			JerrysChaparralItem::RoryHead(rory) => {
				let geometry = rory.build_with_noise(build_noise);
				let mut params = RorysHeadTrainedParams::default();
				params.geometry = geometry;
				JerrysChaparralKind::Rory(params.build())
			}
			JerrysChaparralItem::Bush(bush) => {
				let mut shape = bush.build_with_noise(build_noise);
				shape.chain_noise = chain_noise;
				JerrysChaparralKind::Bush(HighBushShootsParams::new(shape).build())
			}
			JerrysChaparralItem::FriendsConifer(conifer) => {
				let samples =
					BuildWithNoise::<ConiferSamples>::build_with_noise(conifer, build_noise);
				let mut params = FriendsConiferParams::default();
				params.geometry = samples.geometry;
				params.splay_radius_fraction_of_height = samples.splay_radius_fraction_of_height;
				params.apex_canopy_spawn_fraction = samples.apex_canopy_spawn_fraction;
				JerrysChaparralKind::Friends(params.build())
			}
		};

		JerrysChaparralPlant { placement, kind, stick_material, ball_material, frond_material }
	}

	impl VegetationComponents for JerrysChaparral {
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
				JERRYS_CHAPARRAL_STRUCTURAL_HIGH_FACTOR,
				JERRYS_CHAPARRAL_STRUCTURAL_MEDIUM_FACTOR,
				JERRYS_CHAPARRAL_STRUCTURAL_LOW_FACTOR,
			))
		}
	}

	impl LodScene for JerrysChaparral {
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
	JerrysChaparral, JerrysChaparralParams, JerrysChaparralPlant,
	JERRYS_CHAPARRAL_STRUCTURAL_HIGH_FACTOR, JERRYS_CHAPARRAL_STRUCTURAL_LOW_FACTOR,
	JERRYS_CHAPARRAL_STRUCTURAL_MEDIUM_FACTOR,
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
		let dist = JerrysChaparralCell::distribution();
		assert_eq!(dist.len(), 5);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 7.0);
		assert_eq!(dist.buckets[1].item, Some(JerrysChaparralCell::DryRoryHeadTrained));
		assert_eq!(dist.buckets[1].weight, 1.5);
		assert_eq!(dist.buckets[2].item, Some(JerrysChaparralCell::ChaparralHighBush));
		assert_eq!(dist.buckets[2].weight, 2.0);
		assert_eq!(dist.buckets[3].item, Some(JerrysChaparralCell::SmallFriendsConifer));
		assert_eq!(dist.buckets[3].weight, 0.45);
		assert_eq!(dist.buckets[4].item, Some(JerrysChaparralCell::ManzanitaRory));
		assert_eq!(dist.buckets[4].weight, 0.35);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = JerrysChaparralCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.24..=0.52).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn rory_bush_and_conifer_placed_weights_match_rfc_ratio() -> Result<()> {
		let weight = |kind: &str| -> f32 {
			JerrysChaparralCell::distribution()
				.buckets
				.iter()
				.filter(|b| {
					b.item.is_some_and(|cell| match (kind, cell.item()) {
						("rory", JerrysChaparralItem::RoryHead(_)) => true,
						("bush", JerrysChaparralItem::Bush(_)) => true,
						("conifer", JerrysChaparralItem::FriendsConifer(_)) => true,
						_ => false,
					})
				})
				.map(|b| b.weight)
				.sum()
		};
		let rory = weight("rory");
		let bush = weight("bush");
		let conifer = weight("conifer");
		assert!((rory - 1.85).abs() < 1e-4, "expected rory weight 1.85, got {rory}");
		assert!((bush - 2.0).abs() < 1e-4, "expected bush weight 2.0, got {bush}");
		assert!((conifer - 0.45).abs() < 1e-4, "expected conifer weight 0.45, got {conifer}");
		Ok(())
	}

	#[test]
	fn rory_bush_and_conifer_geometry_follows_authored_bands() -> Result<()> {
		let JerrysChaparralItem::RoryHead(dry) = JerrysChaparralCell::DryRoryHeadTrained.item()
		else {
			anyhow::bail!("expected dry rory item");
		};
		assert!(dry.height.start >= 1.20);
		assert!(dry.height.end <= 3.20);

		let JerrysChaparralItem::Bush(bush) = JerrysChaparralCell::ChaparralHighBush.item() else {
			anyhow::bail!("expected bush item");
		};
		assert_eq!(bush.shoot_count, 7..=11);
		assert!(bush.leaf_radius.end <= 0.11);

		let JerrysChaparralItem::FriendsConifer(conifer) =
			JerrysChaparralCell::SmallFriendsConifer.item()
		else {
			anyhow::bail!("expected conifer item");
		};
		assert!(conifer.height.end <= 6.00);

		let JerrysChaparralItem::RoryHead(manzanita) = JerrysChaparralCell::ManzanitaRory.item()
		else {
			anyhow::bail!("expected manzanita rory item");
		};
		assert!(manzanita.canopy_density.end <= 0.35);
		Ok(())
	}

	#[test]
	#[ignore = "placement constraints deferred to forest-layer normalization"]
	fn constraint_first_fit_fallback() -> Result<()> {
		// ChaparralHighBush (index 2) rejects steepness 0.60; first-fit falls to SmallFriendsConifer
		// (index 3), which allows steepness up to 0.65.
		let prepared = JerrysChaparralCell::distribution().prepare(
			0.0,
			0.0,
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.60 };
		let outcome = prepared.select_from(
			2,
			Vec3::new(5.0, 0.35, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, JerrysChaparralCell::SmallFriendsConifer);
			}
			other => anyhow::bail!("expected SmallFriendsConifer fallback, got {other:?}"),
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
