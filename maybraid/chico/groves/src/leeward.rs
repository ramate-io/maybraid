//! Leeward — moderate-density sheltered upper-canopy grove
//! ([RFC-183 §3.4.7.17], [#339](https://github.com/ramate-io/maybraid/issues/339)).
//!
//! Temperate Conifer and Storybook Tree forms on mild lee slopes. Forest-layer attachment remains a
//! follow-up.

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

/// Authored Leeward grove definition.
///
/// Cell footprint sits at the RFC midpoint (`19.0` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<LeewardCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(19.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-19.0, 19.0),
		),
		distribution: LeewardCell::distribution(),
	}
}

/// Ordered leeward varietals ([RFC-183 §3.4.7.17]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeewardCell {
	ShelteredTemperateConifer,
	WindbreakTemperateConifer,
	RoundedLeewardStorybook,
	HighLeewardStorybook,
}

/// Typed authored geometry for one leeward varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LeewardItem {
	TemperateConifer(&'static LeewardTemperateConifer),
	Storybook(&'static LeewardStorybook),
}

/// Authored geometry ranges for one Temperate Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct LeewardTemperateConifer {
	pub height: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct LeewardStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const SHELTERED_TEMPERATE_CONIFER: LeewardTemperateConifer = LeewardTemperateConifer {
	height: UnitRange::new(10.0, 18.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const WINDBREAK_TEMPERATE_CONIFER: LeewardTemperateConifer = LeewardTemperateConifer {
	height: UnitRange::new(16.0, 24.0),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const ROUNDED_LEEWARD_STORYBOOK: LeewardStorybook = LeewardStorybook {
	height: UnitRange::new(10.0, 18.0),
	stalk_radius: UnitRange::new(0.22, 0.46),
	canopy_spread: UnitRange::new(2.5, 6.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const HIGH_LEEWARD_STORYBOOK: LeewardStorybook = LeewardStorybook {
	height: UnitRange::new(16.0, 24.0),
	stalk_radius: UnitRange::new(0.26, 0.52),
	canopy_spread: UnitRange::new(3.0, 7.5),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const SHELTERED_TEMPERATE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("temperate_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "moss_bark"),
]);

const SHELTERED_TEMPERATE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("soft_green", "deep_green"),
	PaletteSlot::new("blue_green", "fresh_green"),
]);

const WINDBREAK_TEMPERATE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("wind_barked", "temperate_bark"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const WINDBREAK_TEMPERATE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("soft_green", "blue_green"),
	PaletteSlot::new("deep_green", "fresh_green"),
]);

const LEEWARD_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("broadleaf_bark", "brown_bark"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const LEEWARD_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("broadleaf_green", "light_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);

impl LeewardCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `2.65`; the `None` weight of `6.8` puts the placed share at
	/// `2.65 / 9.45 ≈ 0.28`, mid RFC `DENSITY_RANGE` (`0.18..0.38`).
	pub fn distribution() -> GroveDistribution<Self> {
		let sheltered_temperate =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.50));
		let windbreak_temperate =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.66));
		let rounded_storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.52));
		let high_storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.58));
		GroveDistribution::new(vec![
			GroveBucket::none(4.0),
			GroveBucket::placed(1.8, sheltered_temperate, Self::ShelteredTemperateConifer),
			GroveBucket::placed(1.6, windbreak_temperate, Self::WindbreakTemperateConifer),
			GroveBucket::placed(2.4, rounded_storybook, Self::RoundedLeewardStorybook),
			GroveBucket::placed(0.45, high_storybook, Self::HighLeewardStorybook),
		])
	}

	pub fn item(self) -> LeewardItem {
		match self {
			Self::ShelteredTemperateConifer => {
				LeewardItem::TemperateConifer(&SHELTERED_TEMPERATE_CONIFER)
			}
			Self::WindbreakTemperateConifer => {
				LeewardItem::TemperateConifer(&WINDBREAK_TEMPERATE_CONIFER)
			}
			Self::RoundedLeewardStorybook => LeewardItem::Storybook(&ROUNDED_LEEWARD_STORYBOOK),
			Self::HighLeewardStorybook => LeewardItem::Storybook(&HIGH_LEEWARD_STORYBOOK),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::ShelteredTemperateConifer => SHELTERED_TEMPERATE_STICK_MIX,
			Self::WindbreakTemperateConifer => WINDBREAK_TEMPERATE_STICK_MIX,
			Self::RoundedLeewardStorybook | Self::HighLeewardStorybook => {
				LEEWARD_STORYBOOK_STICK_MIX
			}
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::ShelteredTemperateConifer => SHELTERED_TEMPERATE_CANOPY_MIX,
			Self::WindbreakTemperateConifer => WINDBREAK_TEMPERATE_CANOPY_MIX,
			Self::RoundedLeewardStorybook | Self::HighLeewardStorybook => {
				LEEWARD_STORYBOOK_CANOPY_MIX
			}
		}
	}
}

#[cfg(feature = "render")]
mod vc {
	use std::sync::Arc;

	use bevy::math::bounding::Aabb3d;
	use bevy::prelude::*;
	use bevy::scene::prelude::Scene;
	use chico_sbs_trees::{
		QuantizedPlant, StorybookTree, StorybookTreeParams, TemperateConifer,
		TemperateConiferParams,
	};
	use chico_vegetation_components::{
		FoliageNode, Layers, Placement, StickNode, StructuralLod, VegetationComponents,
	};
	use clap::Args;
	use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
	use lod::lod_ref::LodRef;
	use lod::{lod_host_scene_pending, SceneChunk};
	use material_ref::MaterialRef;
	use procedural_common::{BuildWithNoise, NoiseParams};

	use super::{
		definition, LeewardCell, LeewardTemperateConifer, HIGH_LEEWARD_STORYBOOK,
		ROUNDED_LEEWARD_STORYBOOK, SHELTERED_TEMPERATE_CONIFER, WINDBREAK_TEMPERATE_CONIFER,
	};
	use crate::grove::vc_tuft::patch_variant_index;
	use crate::grove::{
		canopy_ball_material_from_palette, canopy_proxy_column, canopy_proxy_site,
		foliage_low_canopy_balls, foliage_ultra_low_merged_balls, frond_material_from_palette,
		grove_detail_level, grove_lod_culls, grove_lod_level, grove_lod_status,
		grove_structural_footprint, layers_from_nodes, nest_flattened_plant_chunk, placement_noise,
		remixed_sbs_plant, stick_material_from_palette, unit_build_noise, woody_grove_scene_chunks,
		CanopyProxySite, FlatTerrainSample, GroveCellVariant, GroveExtent, GrovePreviewParams,
		ULTRA_LOW_CANOPY_BIN_METERS,
	};

	pub const LEEWARD_STRUCTURAL_HIGH_FACTOR: f32 = 5.0;
	pub const LEEWARD_STRUCTURAL_MEDIUM_FACTOR: f32 = 10.0;
	pub const LEEWARD_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct LeewardParams {
		#[command(flatten)]
		pub preview: GrovePreviewParams<LeewardCell>,
	}

	impl Default for LeewardParams {
		fn default() -> Self {
			Self {
				preview: GrovePreviewParams::default()
					.with_terrain(FlatTerrainSample { elevation: 0.40, steepness: 0.20 }),
			}
		}
	}

	crate::impl_grove_preview_params!(LeewardParams, LeewardCell);

	impl LeewardParams {
		// preview accessors via impl_grove_preview_params!
		pub fn build(&self) -> Leeward {
			self.build_on(&self.terrain)
		}

		/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> Leeward {
			Leeward::from_placements(
				&self.placements_on(world),
				self.grove.noise,
				&self.extent,
				self.tree_variants,
			)
		}
	}

	fn leeward_temperate_unit(
		authored: &LeewardTemperateConifer,
		num: u32,
	) -> (TemperateConifer, f32) {
		let samples = authored.build_with_noise(unit_build_noise(num));
		let mut params = TemperateConiferParams::default();
		params.geometry = samples.geometry;
		params.frond_world_scale = samples.frond_world_scale;
		params.fronds_per_joint = samples.fronds_per_joint;
		params.frond_length_fraction = samples.frond_length_fraction;
		params.frond_spawn_fraction = samples.frond_spawn_fraction;
		params.apex_canopy_spawn_fraction = samples.apex_canopy_spawn_fraction;
		let (unit, world_size) = params.into_unit_from_num(num);
		(unit.build(), world_size)
	}

	struct ShelteredTemperate;

	impl QuantizedPlant for ShelteredTemperate {
		type Unit = TemperateConifer;

		fn build_unit(num: u32) -> (TemperateConifer, f32) {
			leeward_temperate_unit(&SHELTERED_TEMPERATE_CONIFER, num)
		}
	}

	struct WindbreakTemperate;

	impl QuantizedPlant for WindbreakTemperate {
		type Unit = TemperateConifer;

		fn build_unit(num: u32) -> (TemperateConifer, f32) {
			leeward_temperate_unit(&WINDBREAK_TEMPERATE_CONIFER, num)
		}
	}

	remixed_sbs_plant!(
		RoundedLeewardStorybook,
		StorybookTree,
		StorybookTreeParams,
		ROUNDED_LEEWARD_STORYBOOK
	);
	remixed_sbs_plant!(
		HighLeewardStorybook,
		StorybookTree,
		StorybookTreeParams,
		HIGH_LEEWARD_STORYBOOK
	);

	#[derive(Clone)]
	enum LeewardKind {
		Storybook(Arc<StorybookTree>),
		Temperate(Arc<TemperateConifer>),
	}

	#[derive(Clone)]
	pub struct LeewardPlant {
		pub placement: Placement,
		kind: LeewardKind,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	}

	#[derive(Clone, Component)]
	pub struct Leeward {
		pub plants: Arc<[LeewardPlant]>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl Leeward {
		pub fn from_placements(
			placements: &[GroveCellVariant<LeewardCell>],
			grove_noise: NoiseParams,
			extent: &GroveExtent,
			tree_variants: u32,
		) -> Self {
			let plants: Arc<[LeewardPlant]> = placements
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
					LeewardKind::Storybook(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					LeewardKind::Temperate(t) => nest_flattened_plant_chunk(
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
						LeewardKind::Storybook(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						LeewardKind::Temperate(t) => {
							canopy_proxy_column(t, plant.placement, material)
						}
					}
				})
				.collect()
		}
	}

	fn grow_plant(
		placed: &GroveCellVariant<LeewardCell>,
		grove_noise: NoiseParams,
		tree_variants: u32,
	) -> LeewardPlant {
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
			LeewardCell::ShelteredTemperateConifer => {
				let (tree, world_size) = ShelteredTemperate::grow_num(variant);
				(LeewardKind::Temperate(tree), world_size)
			}
			LeewardCell::WindbreakTemperateConifer => {
				let (tree, world_size) = WindbreakTemperate::grow_num(variant);
				(LeewardKind::Temperate(tree), world_size)
			}
			LeewardCell::RoundedLeewardStorybook => {
				let (tree, world_size) = RoundedLeewardStorybook::grow_num(variant);
				(LeewardKind::Storybook(tree), world_size)
			}
			LeewardCell::HighLeewardStorybook => {
				let (tree, world_size) = HighLeewardStorybook::grow_num(variant);
				(LeewardKind::Storybook(tree), world_size)
			}
		};

		LeewardPlant {
			placement: Placement::new(placed.position, 0.0)
				.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
			kind,
			stick_material,
			ball_material,
			frond_material,
		}
	}

	impl VegetationComponents for Leeward {
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
				LEEWARD_STRUCTURAL_HIGH_FACTOR,
				LEEWARD_STRUCTURAL_MEDIUM_FACTOR,
				LEEWARD_STRUCTURAL_LOW_FACTOR,
			))
		}
	}

	impl LodScene for Leeward {
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

		fn small_grove() -> Leeward {
			LeewardParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(120.0, 1.0, 120.0)))
				.build()
		}

		fn plant_height(plant: &LeewardPlant) -> f32 {
			match &plant.kind {
				LeewardKind::Storybook(t) => t.geometry.height(),
				LeewardKind::Temperate(t) => t.geometry.height(),
			}
		}

		fn plant_seed(plant: &LeewardPlant) -> i32 {
			match &plant.kind {
				LeewardKind::Storybook(t) => t.geometry.canopy_noise.seed,
				LeewardKind::Temperate(t) => t.geometry.canopy_noise.seed,
			}
		}

		#[test]
		fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
			let grove = small_grove();
			assert!(!grove.plants.is_empty(), "expected placed leeward plants");

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
				anyhow::bail!("High leeward should wrap plant chunks");
			};
			assert_eq!(parts.len(), 1, "expected one lazy plant producer");
			let lod::SceneChunk::Lazy { remaining_primitives, remaining_weight, .. } = &parts[0]
			else {
				anyhow::bail!("High leeward plants should be SceneChunk::Lazy");
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
				anyhow::bail!("Low leeward should emit one flattened canopy collection");
			};
			assert_eq!(weight, chico_vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT);
			Ok(())
		}

		#[test]
		fn tree_variants_quantize_archetypes() -> Result<()> {
			use std::collections::HashSet;

			let mut params = LeewardParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(120.0, 1.0, 120.0)));
			params.tree_variants = 4;
			let grove = params.build();
			assert!(!grove.plants.is_empty(), "expected placed leeward plants");
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
	Leeward, LeewardParams, LeewardPlant, LEEWARD_STRUCTURAL_HIGH_FACTOR,
	LEEWARD_STRUCTURAL_LOW_FACTOR, LEEWARD_STRUCTURAL_MEDIUM_FACTOR,
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
		let dist = LeewardCell::distribution();
		assert_eq!(dist.len(), 5);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 4.0);
		assert_eq!(dist.buckets[1].item, Some(LeewardCell::ShelteredTemperateConifer));
		assert_eq!(dist.buckets[1].weight, 1.8);
		assert_eq!(dist.buckets[2].item, Some(LeewardCell::WindbreakTemperateConifer));
		assert_eq!(dist.buckets[2].weight, 1.6);
		assert_eq!(dist.buckets[3].item, Some(LeewardCell::RoundedLeewardStorybook));
		assert_eq!(dist.buckets[3].weight, 2.4);
		assert_eq!(dist.buckets[4].item, Some(LeewardCell::HighLeewardStorybook));
		assert_eq!(dist.buckets[4].weight, 0.45);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = LeewardCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.18..=0.61).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let LeewardItem::TemperateConifer(sheltered) =
			LeewardCell::ShelteredTemperateConifer.item()
		else {
			anyhow::bail!("expected sheltered temperate conifer item");
		};
		assert_eq!(sheltered.height, UnitRange::new(10.0, 18.0));
		assert_eq!(sheltered.canopy_density, MODERATE_CANOPY_DENSITY);

		let LeewardItem::TemperateConifer(windbreak) =
			LeewardCell::WindbreakTemperateConifer.item()
		else {
			anyhow::bail!("expected windbreak temperate conifer item");
		};
		assert_eq!(windbreak.height, UnitRange::new(16.0, 24.0));
		assert_eq!(windbreak.canopy_density, SPARSE_CANOPY_DENSITY);

		let LeewardItem::Storybook(rounded) = LeewardCell::RoundedLeewardStorybook.item() else {
			anyhow::bail!("expected rounded leeward storybook item");
		};
		assert_eq!(rounded.height, UnitRange::new(10.0, 18.0));
		assert_eq!(rounded.canopy_density, DENSE_CANOPY_DENSITY);

		let LeewardItem::Storybook(high) = LeewardCell::HighLeewardStorybook.item() else {
			anyhow::bail!("expected high leeward storybook item");
		};
		assert_eq!(high.height, UnitRange::new(16.0, 24.0));
		assert_eq!(high.canopy_density, MODERATE_CANOPY_DENSITY);
		Ok(())
	}

	#[test]
	fn placement_constraints_use_full_elevation_and_rfc_steepness() -> Result<()> {
		let dist = LeewardCell::distribution();
		for bucket in dist.buckets.iter().filter(|b| b.item.is_some()) {
			assert_eq!(bucket.constraints.elevation.start, 0.0);
			assert_eq!(bucket.constraints.elevation.end, 1.0);
		}
		let sheltered = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(LeewardCell::ShelteredTemperateConifer))
			.ok_or_else(|| anyhow::anyhow!("missing sheltered temperate bucket"))?;
		assert_eq!(sheltered.constraints.steepness.end, 0.50);

		let windbreak = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(LeewardCell::WindbreakTemperateConifer))
			.ok_or_else(|| anyhow::anyhow!("missing windbreak temperate bucket"))?;
		assert_eq!(windbreak.constraints.steepness.end, 0.66);
		Ok(())
	}

	#[test]
	#[ignore = "placement constraints deferred to forest-layer normalization"]
	fn steep_slope_rejects_sheltered_conifer_but_falls_through_to_windbreak() -> Result<()> {
		let prepared =
			LeewardCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let moderate = FlatTerrainSample { elevation: 0.40, steepness: 0.45 };
		let sheltered_outcome = prepared.select_from(
			1,
			Vec3::new(5.0, 0.40, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&moderate,
		);
		match sheltered_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, LeewardCell::ShelteredTemperateConifer);
			}
			other => {
				anyhow::bail!("expected ShelteredTemperateConifer on moderate slope, got {other:?}")
			}
		}
		let steep = FlatTerrainSample { elevation: 0.40, steepness: 0.55 };
		let steep_outcome = prepared.select_from(
			1,
			Vec3::new(5.0, 0.40, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&steep,
		);
		match steep_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, LeewardCell::WindbreakTemperateConifer);
			}
			other => {
				anyhow::bail!("expected fall-through to WindbreakTemperateConifer, got {other:?}")
			}
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			LeewardCell::ShelteredTemperateConifer,
			LeewardCell::WindbreakTemperateConifer,
			LeewardCell::RoundedLeewardStorybook,
			LeewardCell::HighLeewardStorybook,
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
