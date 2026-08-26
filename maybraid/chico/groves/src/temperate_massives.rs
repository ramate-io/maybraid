//! Temperate Massives — low-density giant broadleaf upper-canopy grove
//! ([RFC-183 §3.4.7.3], [#345](https://github.com/ramate-io/maybraid/issues/345)).
//!
//! Enormous Braid Oak, Storybook Tree, and rare Rory's Head-trained skyline forms above temperate
//! lower massives. Forest-layer attachment remains a follow-up.

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);
/// Dense sampled canopy-density band ([`0.50`, `0.85`]).
const DENSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.50, 0.85);

/// Authored Temperate Massives grove definition.
///
/// Cell footprint sits at the RFC midpoint (`49` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<TemperateMassivesCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(49.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-49.0, 49.0),
		),
		distribution: TemperateMassivesCell::distribution(),
	}
}

/// Ordered temperate-massive varietals ([RFC-183 §3.4.7.3]); the explicit `None` bucket lives only
/// in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemperateMassivesCell {
	MassiveBraidOak,
	MassiveStorybook,
	RareMassiveRory,
}

/// Typed authored geometry for one temperate-massive varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TemperateMassivesItem {
	BraidOak(&'static TemperateMassivesBraidOak),
	Storybook(&'static TemperateMassivesStorybook),
	Rory(&'static TemperateMassivesRory),
}

/// Authored geometry ranges for one Braid Oak form.
#[derive(Debug, Clone, PartialEq)]
pub struct TemperateMassivesBraidOak {
	pub height: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct TemperateMassivesStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one rare Rory's Head-trained form.
#[derive(Debug, Clone, PartialEq)]
pub struct TemperateMassivesRory {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const MASSIVE_BRAID_OAK: TemperateMassivesBraidOak = TemperateMassivesBraidOak {
	height: UnitRange::new(28.0, 80.0),
	canopy_spread: UnitRange::new(8.0, 20.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const MASSIVE_STORYBOOK: TemperateMassivesStorybook = TemperateMassivesStorybook {
	height: UnitRange::new(35.0, 170.0),
	stalk_radius: UnitRange::new(3.0, 9.0),
	canopy_spread: UnitRange::new(12.0, 35.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const RARE_MASSIVE_RORY: TemperateMassivesRory = TemperateMassivesRory {
	height: UnitRange::new(50.0, 200.0),
	stalk_radius: UnitRange::new(0.45, 1.80),
	canopy_spread: UnitRange::new(6.0, 14.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("oak_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "moss_bark"),
]);

const BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("dark_green", "light_green"),
]);

const STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("broadleaf_bark", "brown_bark"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("broadleaf_green", "light_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);

const RORY_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("weathered_bark", "dark_bark"),
	PaletteSlot::new("red_brown", "gray_brown"),
]);

const RORY_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("yellow_green", "light_green"),
]);

impl TemperateMassivesCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `4.35`; the `None` weight of `24.6` puts the placed share at
	/// `4.35 / 28.95 ≈ 0.15`, mid RFC `DENSITY_RANGE` (`0.08..0.22`).
	pub fn distribution() -> GroveDistribution<Self> {
		let braid_oak =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.44));
		let storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.50));
		let rory = PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.60));
		GroveDistribution::new(vec![
			GroveBucket::none(24.6),
			GroveBucket::placed(2.0, braid_oak, Self::MassiveBraidOak),
			GroveBucket::placed(2.0, storybook, Self::MassiveStorybook),
			GroveBucket::placed(0.35, rory, Self::RareMassiveRory),
		])
	}

	pub fn item(self) -> TemperateMassivesItem {
		match self {
			Self::MassiveBraidOak => TemperateMassivesItem::BraidOak(&MASSIVE_BRAID_OAK),
			Self::MassiveStorybook => TemperateMassivesItem::Storybook(&MASSIVE_STORYBOOK),
			Self::RareMassiveRory => TemperateMassivesItem::Rory(&RARE_MASSIVE_RORY),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::MassiveBraidOak => BRAID_OAK_STICK_MIX,
			Self::MassiveStorybook => STORYBOOK_STICK_MIX,
			Self::RareMassiveRory => RORY_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::MassiveBraidOak => BRAID_OAK_CANOPY_MIX,
			Self::MassiveStorybook => STORYBOOK_CANOPY_MIX,
			Self::RareMassiveRory => RORY_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
mod vc {
	use std::sync::Arc;

	
	use bevy::prelude::*;
	use chico_sbs_trees::{
		BraidOakTree, QuantizedPlant, RorysHeadTrained, RorysHeadTrainedParams, StorybookTree,
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
	use procedural_common::{BuildWithNoise, NoiseParams};

	use super::{
		definition, TemperateMassivesCell, TemperateMassivesItem, MASSIVE_STORYBOOK,
		RARE_MASSIVE_RORY,
	};
	use crate::grove::vc_tuft::{patch_variant_index, variant_noise};
	use crate::grove::{
		canopy_ball_material_from_palette, canopy_proxy_rory, canopy_proxy_site, frond_material_from_palette,
		grove_structural_footprint, nest_flattened_plant_chunk, placement_noise,
		remixed_sbs_plant, stick_material_from_palette, CanopyProxySite, FlatTerrainSample, GroveCellVariant,
		GroveExtent, GrovePreviewParams,
		WoodyGroveLod,
	};

	/// Typical large types ~170 m (storybook / rory). `grove_bands_for_typical_height(170)`.
	pub const TEMPERATE_MASSIVES_STRUCTURAL_HIGH_FACTOR: f32 = 10.0;
	pub const TEMPERATE_MASSIVES_STRUCTURAL_MEDIUM_FACTOR: f32 = 55.0;
	pub const TEMPERATE_MASSIVES_STRUCTURAL_LOW_FACTOR: f32 = 85.0;

	const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::rory_trunk(
		TEMPERATE_MASSIVES_STRUCTURAL_HIGH_FACTOR,
		TEMPERATE_MASSIVES_STRUCTURAL_MEDIUM_FACTOR,
		TEMPERATE_MASSIVES_STRUCTURAL_LOW_FACTOR,
	);

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct TemperateMassivesParams {
		#[command(flatten)]
		pub preview: GrovePreviewParams<TemperateMassivesCell>,
	}

	impl Default for TemperateMassivesParams {
		fn default() -> Self {
			Self {
				preview: GrovePreviewParams::default()
					.with_terrain(FlatTerrainSample { elevation: 0.35, steepness: 0.15 }),
			}
		}
	}

	crate::impl_grove_preview_params!(TemperateMassivesParams, TemperateMassivesCell);

	impl TemperateMassivesParams {
		// preview accessors via impl_grove_preview_params!
		pub fn build(&self) -> TemperateMassives {
			self.build_on(&self.terrain)
		}

		/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> TemperateMassives {
			TemperateMassives::from_placements(
				&self.placements_on(world),
				self.grove.noise,
				&self.extent,
				self.tree_variants,
			)
		}
	}

	remixed_sbs_plant!(MassiveStorybook, StorybookTree, StorybookTreeParams, MASSIVE_STORYBOOK);
	remixed_sbs_plant!(
		RareMassiveRory,
		RorysHeadTrained,
		RorysHeadTrainedParams,
		RARE_MASSIVE_RORY
	);

	#[derive(Clone)]
	enum TemperateMassivesKind {
		Oak(Arc<BraidOakTree>),
		Storybook(Arc<StorybookTree>),
		Rory(Arc<RorysHeadTrained>),
	}

	#[derive(Clone)]
	pub struct TemperateMassivesPlant {
		pub placement: Placement,
		kind: TemperateMassivesKind,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	}

	#[derive(Clone, Component)]
	pub struct TemperateMassives {
		pub plants: Arc<[TemperateMassivesPlant]>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl TemperateMassives {
		pub fn from_placements(
			placements: &[GroveCellVariant<TemperateMassivesCell>],
			grove_noise: NoiseParams,
			extent: &GroveExtent,
			tree_variants: u32,
		) -> Self {
			let plants: Arc<[TemperateMassivesPlant]> = placements
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
					TemperateMassivesKind::Oak(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					TemperateMassivesKind::Storybook(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					TemperateMassivesKind::Rory(t) => nest_flattened_plant_chunk(
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
						TemperateMassivesKind::Oak(t) => {
							canopy_proxy_site(t, plant.placement, material).into_iter().collect()
						}
						TemperateMassivesKind::Storybook(t) => {
							canopy_proxy_site(t, plant.placement, material).into_iter().collect()
						}
						TemperateMassivesKind::Rory(t) => vec![
							canopy_proxy_rory(t, plant.placement, &plant.stick_material, material)
								.crown,
						],
					}
				})
				.collect()
		}

		fn proxy_trunks(&self) -> Vec<StickNode> {
			self.plants
				.iter()
				.filter_map(|plant| match &plant.kind {
					TemperateMassivesKind::Rory(t) => {
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
		placed: &GroveCellVariant<TemperateMassivesCell>,
		grove_noise: NoiseParams,
		tree_variants: u32,
	) -> TemperateMassivesPlant {
		let variant = patch_variant_index(placed.position, tree_variants);
		let build_noise = variant_noise(grove_noise, variant);
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

		match placed.variant.item() {
			TemperateMassivesItem::BraidOak(oak) => {
				let world_size = oak.build_with_noise(build_noise).height();
				TemperateMassivesPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: TemperateMassivesKind::Oak(BraidOakTree::grow_num(variant).0),
					stick_material,
					ball_material,
					frond_material,
				}
			}
			TemperateMassivesItem::Storybook(_) => {
				let (tree, world_size) = MassiveStorybook::grow_num(variant);
				TemperateMassivesPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: TemperateMassivesKind::Storybook(tree),
					stick_material,
					ball_material,
					frond_material,
				}
			}
			TemperateMassivesItem::Rory(_) => {
				let (tree, world_size) = RareMassiveRory::grow_num(variant);
				TemperateMassivesPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: TemperateMassivesKind::Rory(tree),
					stick_material,
					ball_material,
					frond_material,
				}
			}
		}
	}

	crate::impl_woody_grove_lod!(TemperateMassives, WOODY_LOD, trunks);

	#[cfg(test)]
	mod tests {
		use super::*;
		use anyhow::Result;

		fn small_grove() -> TemperateMassives {
			TemperateMassivesParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(250.0, 1.0, 250.0)))
				.build()
		}

		fn plant_height(plant: &TemperateMassivesPlant) -> f32 {
			match &plant.kind {
				TemperateMassivesKind::Oak(t) => t.geometry.height(),
				TemperateMassivesKind::Storybook(t) => t.geometry.height(),
				TemperateMassivesKind::Rory(t) => t.geometry.height(),
			}
		}

		fn plant_seed(plant: &TemperateMassivesPlant) -> i32 {
			match &plant.kind {
				TemperateMassivesKind::Oak(t) => t.geometry.canopy_noise.seed,
				TemperateMassivesKind::Storybook(t) => t.geometry.canopy_noise.seed,
				TemperateMassivesKind::Rory(t) => t.geometry.canopy_noise.seed,
			}
		}

		#[test]
		fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
			let grove = small_grove();
			assert!(!grove.plants.is_empty(), "expected placed temperate massives");

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
				anyhow::bail!("High temperate massives should wrap plant chunks");
			};
			assert_eq!(parts.len(), 1, "expected one lazy plant producer");
			let lod::SceneChunk::Lazy { remaining_primitives, remaining_weight, .. } = &parts[0]
			else {
				anyhow::bail!("High temperate massives plants should be SceneChunk::Lazy");
			};
			assert_eq!(*remaining_primitives, grove.plants.len());
			assert_eq!(*remaining_weight as usize, grove.plants.len());

			assert!(grove.stick_nodes_for_level(LodSceneLevel::Low).len() <= 1);
			let low_foliage = grove.foliage_nodes_for_level(LodSceneLevel::Low).len();
			assert_eq!(low_foliage, grove.canopy_sites().len());
			assert!(low_foliage >= grove.plants.len());
			assert!(grove.foliage_nodes_for_level(LodSceneLevel::UltraLow).len() <= low_foliage);
			match grove.scene_chunks_with_level(&lod_ref, LodSceneLevel::Low) {
				lod::SceneChunk::Primitive { weight, .. } => {
					assert_eq!(weight, chico_vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT);
				}
				lod::SceneChunk::SubChunks(parts) => assert!(!parts.is_empty()),
				_ => anyhow::bail!("Low temperate massives should emit flattened canopy kits"),
			}
			Ok(())
		}

		#[test]
		fn tree_variants_quantize_archetypes() -> Result<()> {
			use std::collections::HashSet;

			let mut params = TemperateMassivesParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(250.0, 1.0, 250.0)));
			params.tree_variants = 4;
			let grove = params.build();
			assert!(!grove.plants.is_empty(), "expected placed temperate massives");
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
	TemperateMassives, TemperateMassivesParams, TemperateMassivesPlant,
	TEMPERATE_MASSIVES_STRUCTURAL_HIGH_FACTOR, TEMPERATE_MASSIVES_STRUCTURAL_LOW_FACTOR,
	TEMPERATE_MASSIVES_STRUCTURAL_MEDIUM_FACTOR,
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
		let dist = TemperateMassivesCell::distribution();
		assert_eq!(dist.len(), 4);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 24.6);
		assert_eq!(dist.buckets[1].item, Some(TemperateMassivesCell::MassiveBraidOak));
		assert_eq!(dist.buckets[1].weight, 2.0);
		assert_eq!(dist.buckets[2].item, Some(TemperateMassivesCell::MassiveStorybook));
		assert_eq!(dist.buckets[2].weight, 2.0);
		assert_eq!(dist.buckets[3].item, Some(TemperateMassivesCell::RareMassiveRory));
		assert_eq!(dist.buckets[3].weight, 0.35);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = TemperateMassivesCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.08..=0.22).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let TemperateMassivesItem::BraidOak(oak) = TemperateMassivesCell::MassiveBraidOak.item()
		else {
			anyhow::bail!("expected braid oak item");
		};
		assert_eq!(oak.height, UnitRange::new(28.0, 80.0));
		assert_eq!(oak.canopy_spread, UnitRange::new(8.0, 20.0));
		assert_eq!(oak.canopy_density, DENSE_CANOPY_DENSITY);

		let TemperateMassivesItem::Storybook(story) =
			TemperateMassivesCell::MassiveStorybook.item()
		else {
			anyhow::bail!("expected storybook item");
		};
		assert_eq!(story.height, UnitRange::new(35.0, 170.0));
		assert_eq!(story.stalk_radius, UnitRange::new(3.0, 9.0));
		assert_eq!(story.canopy_spread, UnitRange::new(12.0, 35.0));
		assert_eq!(story.canopy_density, DENSE_CANOPY_DENSITY);

		let TemperateMassivesItem::Rory(rory) = TemperateMassivesCell::RareMassiveRory.item()
		else {
			anyhow::bail!("expected rory item");
		};
		assert_eq!(rory.height, UnitRange::new(50.0, 200.0));
		assert_eq!(rory.canopy_spread, UnitRange::new(6.0, 14.0));
		assert_eq!(rory.canopy_density, MODERATE_CANOPY_DENSITY);
		Ok(())
	}

	#[test]
	fn placement_constraints_use_full_elevation_and_rfc_steepness() -> Result<()> {
		let dist = TemperateMassivesCell::distribution();
		for bucket in dist.buckets.iter().filter(|b| b.item.is_some()) {
			assert_eq!(bucket.constraints.elevation.start, 0.0);
			assert_eq!(bucket.constraints.elevation.end, 1.0);
		}
		let braid_oak = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(TemperateMassivesCell::MassiveBraidOak))
			.ok_or_else(|| anyhow::anyhow!("missing braid oak bucket"))?;
		assert_eq!(braid_oak.constraints.steepness.end, 0.44);

		let storybook = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(TemperateMassivesCell::MassiveStorybook))
			.ok_or_else(|| anyhow::anyhow!("missing storybook bucket"))?;
		assert_eq!(storybook.constraints.steepness.end, 0.50);

		let rory = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(TemperateMassivesCell::RareMassiveRory))
			.ok_or_else(|| anyhow::anyhow!("missing rory bucket"))?;
		assert_eq!(rory.constraints.steepness.end, 0.60);
		Ok(())
	}

	#[test]
	fn steep_slope_rejects_braid_oak_but_allows_rory() -> Result<()> {
		let prepared = TemperateMassivesCell::distribution().prepare(
			0.0,
			0.0,
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let terrain = FlatTerrainSample { elevation: 0.30, steepness: 0.55 };
		let outcome = prepared.select_from(
			8,
			Vec3::new(5.0, 0.30, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_ne!(variant, TemperateMassivesCell::MassiveBraidOak);
				assert_ne!(variant, TemperateMassivesCell::MassiveStorybook);
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			TemperateMassivesCell::MassiveBraidOak,
			TemperateMassivesCell::MassiveStorybook,
			TemperateMassivesCell::RareMassiveRory,
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
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(400.0, 1.0, 400.0));
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
