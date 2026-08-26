//! Forlorn Savanna — low-density sparse dry upper-canopy grove
//! ([RFC-183 §3.4.7.6], [#351](https://github.com/ramate-io/maybraid/issues/351)).
//!
//! Wind-shaped Rory's Head-trained forms, acacia-impression High Bush, and rare dry Storybook
//! accents across open savanna. Low / UltraLow keep one canopy proxy per plant — the grove
//! is too sparse for UltraLow 8 m bins. Forest-layer attachment remains a follow-up.

use std::ops::RangeInclusive;

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

/// Sparse sampled canopy-density band ([`0.0`, `0.35`]).
const SPARSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.0, 0.35);
/// Flat sparse crown projection for acacia-like High Bush forms.
const SPARSE_PROJECTION_RADIAL: UnitRange = UnitRange::new(0.42, 0.62);
const SPARSE_PROJECTION_VERTICAL: UnitRange = UnitRange::new(0.32, 0.52);

/// Authored Forlorn Savanna grove definition.
///
/// Cell footprint sits at the RFC midpoint (`30` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<ForlornSavannaCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(30.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-30.0, 30.0),
		),
		distribution: ForlornSavannaCell::distribution(),
	}
}

/// Ordered forlorn-savanna varietals ([RFC-183 §3.4.7.6]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForlornSavannaCell {
	SavannaRory,
	AcaciaHighBush,
	RareSavannaStorybook,
}

/// Typed authored geometry for one forlorn-savanna varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ForlornSavannaItem {
	Rory(&'static ForlornSavannaRory),
	HighBush(&'static ForlornSavannaHighBush),
	Storybook(&'static ForlornSavannaStorybook),
}

/// Authored geometry ranges for one Rory's Head-trained form.
#[derive(Debug, Clone, PartialEq)]
pub struct ForlornSavannaRory {
	pub height: UnitRange,
	/// Stalk base radius as a **fraction of sampled height**. Large savanna
	/// umbrellas stay thick; leftover metres would stay spindly on a 30 m tree.
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one acacia-impression Common High Bush form.
#[derive(Debug, Clone, PartialEq)]
pub struct ForlornSavannaHighBush {
	pub height: UnitRange,
	pub shoot_count: RangeInclusive<u32>,
	pub branch_depth: RangeInclusive<u32>,
	pub radial_strength: UnitRange,
	pub vertical_bias: UnitRange,
	/// Terminal foliage radius in **world meters** (render converts to a height fraction).
	pub leaf_radius: UnitRange,
}

/// Authored geometry ranges for one dry Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct ForlornSavannaStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const SAVANNA_RORY: ForlornSavannaRory = ForlornSavannaRory {
	height: UnitRange::new(5.0, 30.0),
	stalk_radius: UnitRange::new(0.15, 0.20),
	canopy_spread: UnitRange::new(3.0, 12.0),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const ACACIA_HIGH_BUSH: ForlornSavannaHighBush = ForlornSavannaHighBush {
	height: UnitRange::new(5.0, 10.0),
	shoot_count: 4..=12,
	branch_depth: 2..=3,
	radial_strength: SPARSE_PROJECTION_RADIAL,
	vertical_bias: SPARSE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.35, 0.55),
};

const RARE_SAVANNA_STORYBOOK: ForlornSavannaStorybook = ForlornSavannaStorybook {
	height: UnitRange::new(10.0, 20.0),
	stalk_radius: UnitRange::new(0.24, 0.52),
	canopy_spread: UnitRange::new(2.5, 6.5),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const SAVANNA_RORY_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("weathered_bark", "dark_bark"),
	PaletteSlot::new("red_brown", "gray_brown"),
]);

const SAVANNA_RORY_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "dry_green"),
	PaletteSlot::new("yellow_green", "dusty_green"),
]);

const ACACIA_HIGH_BUSH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("acacia_bark", "red_brown"),
	PaletteSlot::new("tan_bark", "gray_brown"),
]);

const ACACIA_HIGH_BUSH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dusty_green", "olive_green"),
	PaletteSlot::new("yellow_green", "dry_green"),
]);

const SAVANNA_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_brown", "dark_bark"),
	PaletteSlot::new("gray_brown", "tan_bark"),
]);

const SAVANNA_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "yellow_green"),
	PaletteSlot::new("dusty_green", "light_green"),
]);

impl ForlornSavannaCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `5.2`; the `None` weight of `30.0` puts the placed share at
	/// `5.2 / 35.2 ≈ 0.15`, mid RFC `DENSITY_RANGE` (`0.06..0.20`).
	pub fn distribution() -> GroveDistribution<Self> {
		let rory = PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.58));
		let high_bush =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.64));
		let storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.50));
		GroveDistribution::new(vec![
			GroveBucket::none(30.0),
			GroveBucket::placed(3.0, rory, Self::SavannaRory),
			GroveBucket::placed(2.0, high_bush, Self::AcaciaHighBush),
			GroveBucket::placed(0.2, storybook, Self::RareSavannaStorybook),
		])
	}

	pub fn item(self) -> ForlornSavannaItem {
		match self {
			Self::SavannaRory => ForlornSavannaItem::Rory(&SAVANNA_RORY),
			Self::AcaciaHighBush => ForlornSavannaItem::HighBush(&ACACIA_HIGH_BUSH),
			Self::RareSavannaStorybook => ForlornSavannaItem::Storybook(&RARE_SAVANNA_STORYBOOK),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::SavannaRory => SAVANNA_RORY_STICK_MIX,
			Self::AcaciaHighBush => ACACIA_HIGH_BUSH_STICK_MIX,
			Self::RareSavannaStorybook => SAVANNA_STORYBOOK_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::SavannaRory => SAVANNA_RORY_CANOPY_MIX,
			Self::AcaciaHighBush => ACACIA_HIGH_BUSH_CANOPY_MIX,
			Self::RareSavannaStorybook => SAVANNA_STORYBOOK_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
mod vc {
	use std::sync::Arc;

	
	use bevy::prelude::*;
	use chico_sbs_trees::{
		HighBushShoots, QuantizedPlant, RorysHeadTrained, RorysHeadTrainedParams, StorybookTree,
		StorybookTreeParams,
	};
	use chico_vegetation_components::{
		Placement, StickNode, VegetationComponents,
	};
	use clap::Args;
	#[cfg(test)]
	use bevy::math::bounding::Aabb3d;
	#[cfg(test)]
	use lod::gen::{LodScene, LodSceneLevel};
	use lod::lod_ref::LodRef;
	use lod::SceneChunk;
	use material_ref::MaterialRef;
	use procedural_common::NoiseParams;

	use super::{
		definition, ForlornSavannaCell, ForlornSavannaItem, ACACIA_HIGH_BUSH,
		RARE_SAVANNA_STORYBOOK, SAVANNA_RORY,
	};
	use crate::grove::vc_tuft::patch_variant_index;
	use crate::grove::{
		canopy_ball_material_from_palette, canopy_proxy_rory, canopy_proxy_site, frond_material_from_palette, grove_structural_footprint,
		nest_flattened_plant_chunk, placement_noise, remixed_bush_plant, remixed_sbs_plant,
		stick_material_from_palette,
		CanopyProxySite, FlatTerrainSample, GroveCellVariant, GroveExtent, GrovePreviewParams,
		WoodyGroveLod,
	};

	/// Typical large types ~25 m. `grove_bands_for_typical_height(25)`.
	pub const FORLORN_SAVANNA_STRUCTURAL_HIGH_FACTOR: f32 = 5.0;
	pub const FORLORN_SAVANNA_STRUCTURAL_MEDIUM_FACTOR: f32 = 20.0;
	pub const FORLORN_SAVANNA_STRUCTURAL_LOW_FACTOR: f32 = 25.0;

	const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::skip_ultralow_bins(
		FORLORN_SAVANNA_STRUCTURAL_HIGH_FACTOR,
		FORLORN_SAVANNA_STRUCTURAL_MEDIUM_FACTOR,
		FORLORN_SAVANNA_STRUCTURAL_LOW_FACTOR,
	).with_rory_trunks();

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct ForlornSavannaParams {
		#[command(flatten)]
		pub preview: GrovePreviewParams<ForlornSavannaCell>,
	}

	impl Default for ForlornSavannaParams {
		fn default() -> Self {
			Self {
				preview: GrovePreviewParams::default()
					.with_terrain(FlatTerrainSample { elevation: 0.40, steepness: 0.20 }),
			}
		}
	}

	crate::impl_grove_preview_params!(ForlornSavannaParams, ForlornSavannaCell);

	impl ForlornSavannaParams {
		// preview accessors via impl_grove_preview_params!
		pub fn build(&self) -> ForlornSavanna {
			self.build_on(&self.terrain)
		}

		/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> ForlornSavanna {
			ForlornSavanna::from_placements(
				&self.placements_on(world),
				self.grove.noise,
				&self.extent,
				self.tree_variants,
			)
		}
	}

	remixed_sbs_plant!(SavannaRory, RorysHeadTrained, RorysHeadTrainedParams, SAVANNA_RORY);
	remixed_bush_plant!(AcaciaHighBush, ACACIA_HIGH_BUSH);
	remixed_sbs_plant!(
		RareSavannaStorybook,
		StorybookTree,
		StorybookTreeParams,
		RARE_SAVANNA_STORYBOOK
	);

	#[derive(Clone)]
	enum ForlornSavannaKind {
		Rory(Arc<RorysHeadTrained>),
		Bush(Arc<HighBushShoots>),
		Storybook(Arc<StorybookTree>),
	}

	#[derive(Clone)]
	pub struct ForlornSavannaPlant {
		pub placement: Placement,
		kind: ForlornSavannaKind,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	}

	#[derive(Clone, Component)]
	pub struct ForlornSavanna {
		pub plants: Arc<[ForlornSavannaPlant]>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl ForlornSavanna {
		pub fn from_placements(
			placements: &[GroveCellVariant<ForlornSavannaCell>],
			grove_noise: NoiseParams,
			extent: &GroveExtent,
			tree_variants: u32,
		) -> Self {
			let plants: Arc<[ForlornSavannaPlant]> = placements
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
					ForlornSavannaKind::Rory(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					ForlornSavannaKind::Bush(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					ForlornSavannaKind::Storybook(t) => nest_flattened_plant_chunk(
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
						ForlornSavannaKind::Rory(t) => {
							vec![
								canopy_proxy_rory(
									t,
									plant.placement,
									&plant.stick_material,
									material,
								)
								.crown,
							]
						}
						ForlornSavannaKind::Bush(t) => {
							canopy_proxy_site(t, plant.placement, material).into_iter().collect()
						}
						ForlornSavannaKind::Storybook(t) => {
							canopy_proxy_site(t, plant.placement, material).into_iter().collect()
						}
					}
				})
				.collect()
		}

		fn proxy_trunks(&self) -> Vec<StickNode> {
			self.plants
				.iter()
				.filter_map(|plant| match &plant.kind {
					ForlornSavannaKind::Rory(t) => {
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
		placed: &GroveCellVariant<ForlornSavannaCell>,
		grove_noise: NoiseParams,
		tree_variants: u32,
	) -> ForlornSavannaPlant {
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

		let (kind, world_size) = match placed.variant.item() {
			ForlornSavannaItem::Rory(_) => {
				let (tree, world_size) = SavannaRory::grow_num(variant);
				(ForlornSavannaKind::Rory(tree), world_size)
			}
			ForlornSavannaItem::HighBush(_) => {
				let (tree, world_size) = AcaciaHighBush::grow_num(variant);
				(ForlornSavannaKind::Bush(tree), world_size)
			}
			ForlornSavannaItem::Storybook(_) => {
				let (tree, world_size) = RareSavannaStorybook::grow_num(variant);
				(ForlornSavannaKind::Storybook(tree), world_size)
			}
		};

		ForlornSavannaPlant {
			placement: Placement::new(placed.position, 0.0)
				.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
			kind,
			stick_material,
			ball_material,
			frond_material,
		}
	}

	crate::impl_woody_grove_lod!(ForlornSavanna, WOODY_LOD, trunks);

	#[cfg(test)]
	mod tests {
		use super::*;
		use anyhow::Result;

		fn small_grove() -> ForlornSavanna {
			ForlornSavannaParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(220.0, 1.0, 220.0)))
				.build()
		}

		fn plant_height(plant: &ForlornSavannaPlant) -> f32 {
			match &plant.kind {
				ForlornSavannaKind::Rory(t) => t.geometry.height(),
				ForlornSavannaKind::Bush(t) => t.shape.height,
				ForlornSavannaKind::Storybook(t) => t.geometry.height(),
			}
		}

		fn plant_seed(plant: &ForlornSavannaPlant) -> i32 {
			match &plant.kind {
				ForlornSavannaKind::Rory(t) => t.geometry.canopy_noise.seed,
				ForlornSavannaKind::Bush(t) => t.shape.chain_noise.seed,
				ForlornSavannaKind::Storybook(t) => t.geometry.canopy_noise.seed,
			}
		}

		#[test]
		fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
			let grove = small_grove();
			assert!(!grove.plants.is_empty(), "expected placed forlorn-savanna plants");

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
				anyhow::bail!("High forlorn-savanna should wrap plant chunks");
			};
			assert_eq!(parts.len(), 1, "expected one lazy plant producer");
			let lod::SceneChunk::Lazy { remaining_primitives, remaining_weight, .. } = &parts[0]
			else {
				anyhow::bail!("High forlorn-savanna plants should be SceneChunk::Lazy");
			};
			assert_eq!(*remaining_primitives, grove.plants.len());
			assert_eq!(*remaining_weight as usize, grove.plants.len());

			assert!(grove.stick_nodes_for_level(LodSceneLevel::Low).len() <= 1);
			let rory_n = grove
				.plants
				.iter()
				.filter(|plant| matches!(plant.kind, ForlornSavannaKind::Rory(_)))
				.count();
			assert_eq!(grove.proxy_trunks().len(), rory_n, "each Rory trunk has a proxy stick");
			let low_foliage = grove.foliage_nodes_for_level(LodSceneLevel::Low).len();
			assert_eq!(low_foliage, grove.canopy_sites().len());
			assert!(low_foliage >= grove.plants.len());
			assert_eq!(
				grove.foliage_nodes_for_level(LodSceneLevel::UltraLow).len(),
				low_foliage,
				"sparse savanna keeps one crown per plant through UltraLow"
			);
			match grove.scene_chunks_with_level(&lod_ref, LodSceneLevel::Low) {
				lod::SceneChunk::Primitive { weight, .. } => {
					assert_eq!(weight, chico_vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT);
				}
				lod::SceneChunk::SubChunks(parts) => assert!(!parts.is_empty()),
				_ => anyhow::bail!("Low forlorn-savanna should emit flattened canopy kits"),
			}
			Ok(())
		}

		#[test]
		fn tree_variants_quantize_archetypes() -> Result<()> {
			use std::collections::HashSet;

			let mut params = ForlornSavannaParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(260.0, 1.0, 260.0)));
			params.tree_variants = 4;
			let grove = params.build();
			assert!(!grove.plants.is_empty(), "expected placed forlorn-savanna plants");
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
	ForlornSavanna, ForlornSavannaParams, ForlornSavannaPlant,
	FORLORN_SAVANNA_STRUCTURAL_HIGH_FACTOR, FORLORN_SAVANNA_STRUCTURAL_LOW_FACTOR,
	FORLORN_SAVANNA_STRUCTURAL_MEDIUM_FACTOR,
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
		let dist = ForlornSavannaCell::distribution();
		assert_eq!(dist.len(), 4);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 30.0);
		assert_eq!(dist.buckets[1].item, Some(ForlornSavannaCell::SavannaRory));
		assert_eq!(dist.buckets[1].weight, 3.0);
		assert_eq!(dist.buckets[2].item, Some(ForlornSavannaCell::AcaciaHighBush));
		assert_eq!(dist.buckets[2].weight, 2.0);
		assert_eq!(dist.buckets[3].item, Some(ForlornSavannaCell::RareSavannaStorybook));
		assert_eq!(dist.buckets[3].weight, 0.2);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = ForlornSavannaCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.06..=0.20).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let ForlornSavannaItem::Rory(rory) = ForlornSavannaCell::SavannaRory.item() else {
			anyhow::bail!("expected rory item");
		};
		assert_eq!(rory.height, UnitRange::new(5.0, 30.0));
		assert_eq!(rory.stalk_radius, UnitRange::new(0.15, 0.20));
		assert_eq!(rory.canopy_spread, UnitRange::new(3.0, 12.0));
		assert_eq!(rory.canopy_density, SPARSE_CANOPY_DENSITY);

		let ForlornSavannaItem::HighBush(bush) = ForlornSavannaCell::AcaciaHighBush.item() else {
			anyhow::bail!("expected high bush item");
		};
		assert_eq!(bush.height, UnitRange::new(5.0, 10.0));

		let ForlornSavannaItem::Storybook(story) = ForlornSavannaCell::RareSavannaStorybook.item()
		else {
			anyhow::bail!("expected storybook item");
		};
		assert_eq!(story.height, UnitRange::new(10.0, 20.0));
		assert_eq!(story.canopy_density, SPARSE_CANOPY_DENSITY);
		Ok(())
	}

	#[test]
	fn placement_constraints_match_rfc() -> Result<()> {
		let dist = ForlornSavannaCell::distribution();
		let rory = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(ForlornSavannaCell::SavannaRory))
			.ok_or_else(|| anyhow::anyhow!("missing rory bucket"))?;
		assert_eq!(rory.constraints.elevation.start, 0.0);
		assert_eq!(rory.constraints.elevation.end, 1.0);
		assert_eq!(rory.constraints.steepness.end, 0.58);

		let high_bush = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(ForlornSavannaCell::AcaciaHighBush))
			.ok_or_else(|| anyhow::anyhow!("missing high bush bucket"))?;
		assert_eq!(high_bush.constraints.elevation.end, 1.0);
		assert_eq!(high_bush.constraints.steepness.end, 0.64);

		let storybook = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(ForlornSavannaCell::RareSavannaStorybook))
			.ok_or_else(|| anyhow::anyhow!("missing storybook bucket"))?;
		assert_eq!(storybook.constraints.elevation.start, 0.0);
		assert_eq!(storybook.constraints.steepness.end, 0.50);
		Ok(())
	}

	#[test]
	#[ignore = "placement constraints deferred to forest-layer normalization"]
	fn steep_slope_rejects_rory_but_allows_high_bush() -> Result<()> {
		let prepared = ForlornSavannaCell::distribution().prepare(
			0.0,
			0.0,
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let terrain = FlatTerrainSample { elevation: 0.40, steepness: 0.60 };
		let bush_outcome = prepared.select_from(
			5,
			Vec3::new(5.0, 0.40, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match bush_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, ForlornSavannaCell::AcaciaHighBush);
			}
			other => anyhow::bail!("expected AcaciaHighBush on moderate slope, got {other:?}"),
		}
		let rory_outcome = prepared.select_from(
			1,
			Vec3::new(5.0, 0.40, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match rory_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_ne!(variant, ForlornSavannaCell::SavannaRory);
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			ForlornSavannaCell::SavannaRory,
			ForlornSavannaCell::AcaciaHighBush,
			ForlornSavannaCell::RareSavannaStorybook,
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
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(300.0, 1.0, 300.0));
		let terrain = FlatTerrainSample { elevation: 0.40, steepness: 0.20 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
