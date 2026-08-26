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
	use std::sync::Arc;

	use bevy::math::bounding::Aabb3d;
	use bevy::prelude::*;
	use bevy::scene::prelude::Scene;
	use chico_sbs_trees::{BraidOakTree, QuantizedPlant, StorybookTree, StorybookTreeParams};
	use chico_vegetation_components::{
		FoliageNode, Layers, Placement, StickNode, StructuralLod, VegetationComponents,
	};
	use clap::Args;
	use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
	use lod::lod_ref::LodRef;
	use lod::{lod_host_scene_pending, SceneChunk};
	use material_ref::MaterialRef;
	use procedural_common::{BuildWithNoise, NoiseParams};

	use super::{definition, GoettingenFollowCell, GoettingenFollowItem, FOLLOW_STORYBOOK};
	use crate::grove::vc_tuft::{patch_variant_index, variant_noise};
	use crate::grove::{
		canopy_ball_material_from_palette, canopy_proxy_site, foliage_low_canopy_balls,
		foliage_ultra_low_merged_balls, frond_material_from_palette, grove_detail_level,
		grove_lod_culls, grove_lod_level, grove_lod_status, grove_structural_footprint,
		layers_from_nodes, nest_flattened_plant_chunk, placement_noise, remixed_sbs_plant,
		stick_material_from_palette, woody_grove_scene_chunks, CanopyProxySite, FlatTerrainSample,
		GroveCellVariant, GroveExtent, GrovePreviewParams, ULTRA_LOW_CANOPY_BIN_METERS,
	};

	pub const GOETTINGEN_FOLLOW_STRUCTURAL_HIGH_FACTOR: f32 = 5.0;
	pub const GOETTINGEN_FOLLOW_STRUCTURAL_MEDIUM_FACTOR: f32 = 10.0;
	pub const GOETTINGEN_FOLLOW_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct GoettingenFollowParams {
		#[command(flatten)]
		pub preview: GrovePreviewParams<GoettingenFollowCell>,
	}

	impl Default for GoettingenFollowParams {
		fn default() -> Self {
			Self {
				preview: GrovePreviewParams::default()
					.with_terrain(FlatTerrainSample { elevation: 0.25, steepness: 0.12 }),
			}
		}
	}

	crate::impl_grove_preview_params!(GoettingenFollowParams, GoettingenFollowCell);

	impl GoettingenFollowParams {
		// preview accessors via impl_grove_preview_params!
		pub fn build(&self) -> GoettingenFollow {
			self.build_on(&self.terrain)
		}

		/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> GoettingenFollow {
			GoettingenFollow::from_placements(
				&self.placements_on(world),
				self.grove.noise,
				&self.extent,
				self.tree_variants,
			)
		}
	}

	remixed_sbs_plant!(FollowStorybook, StorybookTree, StorybookTreeParams, FOLLOW_STORYBOOK);

	#[derive(Clone)]
	enum GoettingenFollowKind {
		Oak(Arc<BraidOakTree>),
		Storybook(Arc<StorybookTree>),
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
		pub plants: Arc<[GoettingenFollowPlant]>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl GoettingenFollow {
		pub fn from_placements(
			placements: &[GroveCellVariant<GoettingenFollowCell>],
			grove_noise: NoiseParams,
			extent: &GroveExtent,
			tree_variants: u32,
		) -> Self {
			let plants: Arc<[GoettingenFollowPlant]> = placements
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
					GoettingenFollowKind::Oak(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					GoettingenFollowKind::Storybook(t) => nest_flattened_plant_chunk(
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
		tree_variants: u32,
	) -> GoettingenFollowPlant {
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
			GoettingenFollowItem::BraidOak(oak) => {
				let world_size = oak.build_with_noise(build_noise).height();
				GoettingenFollowPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: GoettingenFollowKind::Oak(BraidOakTree::grow_num(variant).0),
					stick_material,
					ball_material,
					frond_material,
				}
			}
			GoettingenFollowItem::Storybook(_) => {
				let (tree, world_size) = FollowStorybook::grow_num(variant);
				GoettingenFollowPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: GoettingenFollowKind::Storybook(tree),
					stick_material,
					ball_material,
					frond_material,
				}
			}
		}
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

	#[cfg(test)]
	mod tests {
		use super::*;
		use anyhow::Result;

		fn small_grove() -> GoettingenFollow {
			GoettingenFollowParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0)))
				.build()
		}

		fn plant_height(plant: &GoettingenFollowPlant) -> f32 {
			match &plant.kind {
				GoettingenFollowKind::Oak(t) => t.geometry.height(),
				GoettingenFollowKind::Storybook(t) => t.geometry.height(),
			}
		}

		fn plant_seed(plant: &GoettingenFollowPlant) -> i32 {
			match &plant.kind {
				GoettingenFollowKind::Oak(t) => t.geometry.canopy_noise.seed,
				GoettingenFollowKind::Storybook(t) => t.geometry.canopy_noise.seed,
			}
		}

		#[test]
		fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
			let grove = small_grove();
			assert!(!grove.plants.is_empty(), "expected placed goettingen trees");

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
				anyhow::bail!("High goettingen should wrap plant chunks");
			};
			assert_eq!(parts.len(), 1, "expected one lazy plant producer");
			let lod::SceneChunk::Lazy { remaining_primitives, remaining_weight, .. } = &parts[0]
			else {
				anyhow::bail!("High goettingen plants should be SceneChunk::Lazy");
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
				anyhow::bail!("Low goettingen should emit one flattened canopy collection");
			};
			assert_eq!(weight, chico_vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT);
			Ok(())
		}

		#[test]
		fn tree_variants_quantize_archetypes() -> Result<()> {
			use std::collections::HashSet;

			let mut params = GoettingenFollowParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(120.0, 1.0, 120.0)));
			params.tree_variants = 4;
			let grove = params.build();
			assert!(!grove.plants.is_empty(), "expected placed goettingen trees");
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
