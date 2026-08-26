//! Rolling Oaks — low-density open oak-country upper-canopy grove
//! ([RFC-183 §3.4.7.5], [#349](https://github.com/ramate-io/maybraid/issues/349)).
//!
//! Common dry Braid Oak forms with rare Storybook accents across rolling open woodland. Forest-layer
//! attachment remains a follow-up.

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);

/// Authored Rolling Oaks grove definition.
///
/// Cell footprint sits at the RFC midpoint (`22` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<RollingOaksCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(22.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-22.0, 22.0),
		),
		distribution: RollingOaksCell::distribution(),
	}
}

/// Ordered rolling-oaks varietals ([RFC-183 §3.4.7.5]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollingOaksCell {
	RollingBraidOak,
	RareTallRollingBraidOak,
	RareSentinelRollingBraidOak,
	RareRollingStorybook,
}

/// Typed authored geometry for one rolling-oaks varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RollingOaksItem {
	BraidOak(&'static RollingOaksBraidOak),
	Storybook(&'static RollingOaksStorybook),
}

/// Authored geometry ranges for one Braid Oak form.
#[derive(Debug, Clone, PartialEq)]
pub struct RollingOaksBraidOak {
	pub height: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct RollingOaksStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const ROLLING_BRAID_OAK: RollingOaksBraidOak = RollingOaksBraidOak {
	height: UnitRange::new(5.0, 20.0),
	canopy_spread: UnitRange::new(2.0, 7.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const RARE_TALL_ROLLING_BRAID_OAK: RollingOaksBraidOak = RollingOaksBraidOak {
	height: UnitRange::new(20.0, 32.0),
	canopy_spread: UnitRange::new(5.0, 11.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const RARE_SENTINEL_ROLLING_BRAID_OAK: RollingOaksBraidOak = RollingOaksBraidOak {
	height: UnitRange::new(28.0, 40.0),
	canopy_spread: UnitRange::new(7.0, 14.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const RARE_ROLLING_STORYBOOK: RollingOaksStorybook = RollingOaksStorybook {
	height: UnitRange::new(5.0, 20.0),
	stalk_radius: UnitRange::new(0.20, 0.48),
	canopy_spread: UnitRange::new(2.0, 6.5),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const ROLLING_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("oak_bark", "dry_brown"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const ROLLING_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "fresh_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);

const RARE_TALL_ROLLING_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("gnarled_brown", "oak_bark"),
	PaletteSlot::new("moss_bark", "dark_bark"),
]);

const RARE_TALL_ROLLING_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "deep_green"),
	PaletteSlot::new("olive_green", "light_green"),
]);

const RARE_SENTINEL_ROLLING_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("wet_bark", "gnarled_brown"),
	PaletteSlot::new("dark_bark", "moss_bark"),
]);

const RARE_SENTINEL_ROLLING_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("emerald_green", "deep_green"),
	PaletteSlot::new("moss_green", "olive_green"),
]);

const ROLLING_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("broadleaf_bark", "dry_brown"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const ROLLING_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "light_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);

impl RollingOaksCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `2.55`; the `None` weight of `12.4` puts the placed share at
	/// `2.55 / 14.95 ≈ 0.17`, mid RFC `DENSITY_RANGE` (`0.08..0.24`).
	pub fn distribution() -> GroveDistribution<Self> {
		let braid_oak =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.48));
		let tall_braid_oak =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.48));
		let sentinel_braid_oak =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.44));
		let storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.54));
		GroveDistribution::new(vec![
			GroveBucket::none(12.4),
			GroveBucket::placed(2.0, braid_oak, Self::RollingBraidOak),
			GroveBucket::placed(0.15, tall_braid_oak, Self::RareTallRollingBraidOak),
			GroveBucket::placed(0.05, sentinel_braid_oak, Self::RareSentinelRollingBraidOak),
			GroveBucket::placed(0.35, storybook, Self::RareRollingStorybook),
		])
	}

	pub fn item(self) -> RollingOaksItem {
		match self {
			Self::RollingBraidOak => RollingOaksItem::BraidOak(&ROLLING_BRAID_OAK),
			Self::RareTallRollingBraidOak => {
				RollingOaksItem::BraidOak(&RARE_TALL_ROLLING_BRAID_OAK)
			}
			Self::RareSentinelRollingBraidOak => {
				RollingOaksItem::BraidOak(&RARE_SENTINEL_ROLLING_BRAID_OAK)
			}
			Self::RareRollingStorybook => RollingOaksItem::Storybook(&RARE_ROLLING_STORYBOOK),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::RollingBraidOak => ROLLING_BRAID_OAK_STICK_MIX,
			Self::RareTallRollingBraidOak => RARE_TALL_ROLLING_BRAID_OAK_STICK_MIX,
			Self::RareSentinelRollingBraidOak => RARE_SENTINEL_ROLLING_BRAID_OAK_STICK_MIX,
			Self::RareRollingStorybook => ROLLING_STORYBOOK_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::RollingBraidOak => ROLLING_BRAID_OAK_CANOPY_MIX,
			Self::RareTallRollingBraidOak => RARE_TALL_ROLLING_BRAID_OAK_CANOPY_MIX,
			Self::RareSentinelRollingBraidOak => RARE_SENTINEL_ROLLING_BRAID_OAK_CANOPY_MIX,
			Self::RareRollingStorybook => ROLLING_STORYBOOK_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
mod vc {
	use std::sync::Arc;

	
	use bevy::prelude::*;
	use chico_sbs_trees::{BraidOakTree, QuantizedPlant, StorybookTree, StorybookTreeParams};
	use chico_vegetation_components::{
		Placement, VegetationComponents,
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

	use super::{definition, RollingOaksCell, RollingOaksItem, RARE_ROLLING_STORYBOOK};
	use crate::grove::vc_tuft::{patch_variant_index, variant_noise};
	use crate::grove::{
		canopy_ball_material_from_palette, canopy_proxy_site, frond_material_from_palette, grove_structural_footprint, nest_flattened_plant_chunk, placement_noise, remixed_sbs_plant,
		stick_material_from_palette, CanopyProxySite, FlatTerrainSample,
		GroveCellVariant, GroveExtent, GrovePreviewParams,
		WoodyGroveLod,
	};

	/// Typical sentinels ~36 m. `grove_bands_for_typical_height(36)`.
	pub const ROLLING_OAKS_STRUCTURAL_HIGH_FACTOR: f32 = 5.0;
	pub const ROLLING_OAKS_STRUCTURAL_MEDIUM_FACTOR: f32 = 15.0;
	pub const ROLLING_OAKS_STRUCTURAL_LOW_FACTOR: f32 = 25.0;

	const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::ordinary(
		ROLLING_OAKS_STRUCTURAL_HIGH_FACTOR,
		ROLLING_OAKS_STRUCTURAL_MEDIUM_FACTOR,
		ROLLING_OAKS_STRUCTURAL_LOW_FACTOR,
	);

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct RollingOaksParams {
		#[command(flatten)]
		pub preview: GrovePreviewParams<RollingOaksCell>,
	}

	impl Default for RollingOaksParams {
		fn default() -> Self {
			Self {
				preview: GrovePreviewParams::default()
					.with_terrain(FlatTerrainSample { elevation: 0.40, steepness: 0.15 }),
			}
		}
	}

	crate::impl_grove_preview_params!(RollingOaksParams, RollingOaksCell);

	impl RollingOaksParams {
		// preview accessors via impl_grove_preview_params!
		pub fn build(&self) -> RollingOaks {
			self.build_on(&self.terrain)
		}

		/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> RollingOaks {
			RollingOaks::from_placements(
				&self.placements_on(world),
				self.grove.noise,
				&self.extent,
				self.tree_variants,
			)
		}
	}

	remixed_sbs_plant!(
		RareRollingStorybook,
		StorybookTree,
		StorybookTreeParams,
		RARE_ROLLING_STORYBOOK
	);

	#[derive(Clone)]
	enum RollingOaksKind {
		Oak(Arc<BraidOakTree>),
		Storybook(Arc<StorybookTree>),
	}

	#[derive(Clone)]
	pub struct RollingOaksPlant {
		pub placement: Placement,
		kind: RollingOaksKind,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	}

	#[derive(Clone, Component)]
	pub struct RollingOaks {
		pub plants: Arc<[RollingOaksPlant]>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl RollingOaks {
		pub fn from_placements(
			placements: &[GroveCellVariant<RollingOaksCell>],
			grove_noise: NoiseParams,
			extent: &GroveExtent,
			tree_variants: u32,
		) -> Self {
			let plants: Arc<[RollingOaksPlant]> = placements
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
					RollingOaksKind::Oak(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					RollingOaksKind::Storybook(t) => nest_flattened_plant_chunk(
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
						RollingOaksKind::Oak(t) => canopy_proxy_site(t, plant.placement, material),
						RollingOaksKind::Storybook(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
					}
				})
				.collect()
		}
	}

	fn grow_plant(
		placed: &GroveCellVariant<RollingOaksCell>,
		grove_noise: NoiseParams,
		tree_variants: u32,
	) -> RollingOaksPlant {
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
			RollingOaksItem::BraidOak(oak) => {
				let world_size = oak.build_with_noise(build_noise).height();
				RollingOaksPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: RollingOaksKind::Oak(BraidOakTree::grow_num(variant).0),
					stick_material,
					ball_material,
					frond_material,
				}
			}
			RollingOaksItem::Storybook(_) => {
				let (tree, world_size) = RareRollingStorybook::grow_num(variant);
				RollingOaksPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: RollingOaksKind::Storybook(tree),
					stick_material,
					ball_material,
					frond_material,
				}
			}
		}
	}

	crate::impl_woody_grove_lod!(RollingOaks, WOODY_LOD);

	#[cfg(test)]
	mod tests {
		use super::*;
		use anyhow::Result;

		fn small_grove() -> RollingOaks {
			RollingOaksParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(220.0, 1.0, 220.0)))
				.build()
		}

		fn plant_height(plant: &RollingOaksPlant) -> f32 {
			match &plant.kind {
				RollingOaksKind::Oak(t) => t.geometry.height(),
				RollingOaksKind::Storybook(t) => t.geometry.height(),
			}
		}

		fn plant_seed(plant: &RollingOaksPlant) -> i32 {
			match &plant.kind {
				RollingOaksKind::Oak(t) => t.geometry.canopy_noise.seed,
				RollingOaksKind::Storybook(t) => t.geometry.canopy_noise.seed,
			}
		}

		#[test]
		fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
			let grove = small_grove();
			assert!(!grove.plants.is_empty(), "expected placed rolling oaks");

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
				anyhow::bail!("High rolling oaks should wrap plant chunks");
			};
			assert_eq!(parts.len(), 1, "expected one lazy plant producer");
			let lod::SceneChunk::Lazy { remaining_primitives, remaining_weight, .. } = &parts[0]
			else {
				anyhow::bail!("High rolling oaks plants should be SceneChunk::Lazy");
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
				anyhow::bail!("Low rolling oaks should emit one flattened canopy collection");
			};
			assert_eq!(weight, chico_vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT);
			Ok(())
		}

		#[test]
		fn tree_variants_quantize_archetypes() -> Result<()> {
			use std::collections::HashSet;

			let mut params = RollingOaksParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(260.0, 1.0, 260.0)));
			params.tree_variants = 4;
			let grove = params.build();
			assert!(!grove.plants.is_empty(), "expected placed rolling oaks");
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
	RollingOaks, RollingOaksParams, RollingOaksPlant, ROLLING_OAKS_STRUCTURAL_HIGH_FACTOR,
	ROLLING_OAKS_STRUCTURAL_LOW_FACTOR, ROLLING_OAKS_STRUCTURAL_MEDIUM_FACTOR,
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
		let dist = RollingOaksCell::distribution();
		assert_eq!(dist.len(), 5);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 12.4);
		assert_eq!(dist.buckets[1].item, Some(RollingOaksCell::RollingBraidOak));
		assert_eq!(dist.buckets[1].weight, 2.0);
		assert_eq!(dist.buckets[2].item, Some(RollingOaksCell::RareTallRollingBraidOak));
		assert_eq!(dist.buckets[2].weight, 0.15);
		assert_eq!(dist.buckets[3].item, Some(RollingOaksCell::RareSentinelRollingBraidOak));
		assert_eq!(dist.buckets[3].weight, 0.05);
		assert_eq!(dist.buckets[4].item, Some(RollingOaksCell::RareRollingStorybook));
		assert_eq!(dist.buckets[4].weight, 0.35);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = RollingOaksCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.08..=0.24).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let RollingOaksItem::BraidOak(oak) = RollingOaksCell::RollingBraidOak.item() else {
			anyhow::bail!("expected braid oak item");
		};
		assert_eq!(oak.height, UnitRange::new(5.0, 20.0));
		assert_eq!(oak.canopy_density, MODERATE_CANOPY_DENSITY);

		let RollingOaksItem::BraidOak(tall) = RollingOaksCell::RareTallRollingBraidOak.item()
		else {
			anyhow::bail!("expected rare tall braid oak item");
		};
		assert_eq!(tall.height, UnitRange::new(20.0, 32.0));

		let RollingOaksItem::BraidOak(sentinel) =
			RollingOaksCell::RareSentinelRollingBraidOak.item()
		else {
			anyhow::bail!("expected rare sentinel braid oak item");
		};
		assert_eq!(sentinel.height, UnitRange::new(28.0, 40.0));

		let RollingOaksItem::Storybook(story) = RollingOaksCell::RareRollingStorybook.item() else {
			anyhow::bail!("expected storybook item");
		};
		assert_eq!(story.height, UnitRange::new(5.0, 20.0));
		assert_eq!(story.canopy_density, MODERATE_CANOPY_DENSITY);
		Ok(())
	}

	#[test]
	fn placement_constraints_match_rfc() -> Result<()> {
		let dist = RollingOaksCell::distribution();
		let braid_oak = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(RollingOaksCell::RollingBraidOak))
			.ok_or_else(|| anyhow::anyhow!("missing braid oak bucket"))?;
		assert_eq!(braid_oak.constraints.elevation.start, 0.0);
		assert_eq!(braid_oak.constraints.elevation.end, 1.0);
		assert_eq!(braid_oak.constraints.steepness.end, 0.48);

		let tall_braid_oak = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(RollingOaksCell::RareTallRollingBraidOak))
			.ok_or_else(|| anyhow::anyhow!("missing tall braid oak bucket"))?;
		assert_eq!(tall_braid_oak.constraints.elevation.end, 1.0);
		assert_eq!(tall_braid_oak.constraints.steepness.end, 0.48);

		let sentinel_braid_oak = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(RollingOaksCell::RareSentinelRollingBraidOak))
			.ok_or_else(|| anyhow::anyhow!("missing sentinel braid oak bucket"))?;
		assert_eq!(sentinel_braid_oak.constraints.elevation.end, 1.0);
		assert_eq!(sentinel_braid_oak.constraints.steepness.end, 0.44);

		let storybook = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(RollingOaksCell::RareRollingStorybook))
			.ok_or_else(|| anyhow::anyhow!("missing storybook bucket"))?;
		assert_eq!(storybook.constraints.elevation.start, 0.0);
		assert_eq!(storybook.constraints.elevation.end, 1.0);
		assert_eq!(storybook.constraints.steepness.end, 0.54);
		Ok(())
	}

	#[test]
	#[ignore = "placement constraints deferred to forest-layer normalization"]
	fn steep_slope_rejects_braid_oak_but_allows_storybook() -> Result<()> {
		let prepared =
			RollingOaksCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let terrain = FlatTerrainSample { elevation: 0.40, steepness: 0.50 };
		let story_outcome = prepared.select_from(
			4,
			Vec3::new(5.0, 0.40, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match story_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, RollingOaksCell::RareRollingStorybook);
			}
			other => {
				anyhow::bail!("expected RareRollingStorybook on moderate slope, got {other:?}")
			}
		}
		let braid_outcome = prepared.select_from(
			1,
			Vec3::new(5.0, 0.40, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match braid_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, RollingOaksCell::RareRollingStorybook);
			}
			other => anyhow::bail!(
				"expected storybook after braid-oak variants reject steep slope, got {other:?}"
			),
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			RollingOaksCell::RollingBraidOak,
			RollingOaksCell::RareTallRollingBraidOak,
			RollingOaksCell::RareSentinelRollingBraidOak,
			RollingOaksCell::RareRollingStorybook,
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
		let terrain = FlatTerrainSample { elevation: 0.40, steepness: 0.15 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
