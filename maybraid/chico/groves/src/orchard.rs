//! Orchard — high-density cultivated Storybook Tree upper-canopy grove
//! ([RFC-183 §3.4.7.7], [#353](https://github.com/ramate-io/maybraid/issues/353)).
//!
//! Compact fruiting and pale-bloom storybook forms on low-slope terrain with tight cell offset.
//! Forest-layer attachment remains a follow-up.
//!
//! Under `render`, High/Medium nest one flattened Storybook tree host per plant
//! (posed kit content, no per-stick / per-ball LOD hosts). Plants unitize through
//! [`StorybookTree::unit_from_num`](chico_sbs_trees::StorybookTree::unit_from_num)
//! (`tree_variants`, default `100`) so merged stick/ball collections share archetypal
//! meshes. Low ≈ one canopy ball per tree; UltraLow bins those sites at
//! [`ULTRA_LOW_CANOPY_BIN_METERS`].

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);

/// Authored Orchard grove definition.
///
/// Cell footprint sits at the RFC midpoint (`11.0` m). Placements stay on cell centroids with only
/// ±`0.5` m horizontal jitter so the grove reads as regular tended rows.
pub fn definition() -> GroveDefinition<OrchardCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(11.0),
		placement: GrovePlacementRanges::new(UnitRange::new(1.0, 1.0), UnitRange::new(-0.5, 0.5)),
		distribution: OrchardCell::distribution(),
	}
}

/// Ordered orchard varietals ([RFC-183 §3.4.7.7]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrchardCell {
	FruitingStorybook,
	PaleBloomStorybook,
}

/// Typed authored geometry for one orchard varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OrchardItem {
	Storybook(&'static OrchardStorybook),
}

/// Authored geometry ranges for one cultivated Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct OrchardStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const FRUITING_STORYBOOK: OrchardStorybook = OrchardStorybook {
	height: UnitRange::new(5.0, 10.0),
	stalk_radius: UnitRange::new(0.22, 0.44),
	canopy_spread: UnitRange::new(1.8, 4.2),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const PALE_BLOOM_STORYBOOK: OrchardStorybook = OrchardStorybook {
	height: UnitRange::new(5.0, 9.0),
	stalk_radius: UnitRange::new(0.20, 0.38),
	canopy_spread: UnitRange::new(1.6, 3.8),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const FRUITING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("orchard_bark", "brown_bark"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const FRUITING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("fresh_green", "light_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);

const PALE_BLOOM_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("orchard_bark", "gray_brown"),
	PaletteSlot::new("tan_bark", "brown_bark"),
]);

const PALE_BLOOM_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("pale_blossom", "fresh_green"),
	PaletteSlot::new("light_green", "yellow_green"),
]);

/// Explicit `None` weight paired with placed weights so ~`95%` of cells receive a tree.
const CULTIVATED_EMPTY_WEIGHT: f32 = 2.25 / 19.0;

impl OrchardCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `2.25`; the `None` weight of `2.25 / 19` yields a `~0.95` placed share
	/// for regular tended-row planting.
	pub fn distribution() -> GroveDistribution<Self> {
		let fruiting =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.30));
		let pale_bloom =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.28));
		GroveDistribution::new(vec![
			GroveBucket::none(CULTIVATED_EMPTY_WEIGHT),
			GroveBucket::placed(1.5, fruiting, Self::FruitingStorybook),
			GroveBucket::placed(0.75, pale_bloom, Self::PaleBloomStorybook),
		])
	}

	pub fn item(self) -> OrchardItem {
		match self {
			Self::FruitingStorybook => OrchardItem::Storybook(&FRUITING_STORYBOOK),
			Self::PaleBloomStorybook => OrchardItem::Storybook(&PALE_BLOOM_STORYBOOK),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::FruitingStorybook => FRUITING_STICK_MIX,
			Self::PaleBloomStorybook => PALE_BLOOM_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::FruitingStorybook => FRUITING_CANOPY_MIX,
			Self::PaleBloomStorybook => PALE_BLOOM_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
mod vc {
	use std::sync::Arc;

	use bevy::math::bounding::Aabb3d;
	use bevy::prelude::*;
	use bevy::scene::prelude::Scene;
	use chico_sbs_trees::{StorybookTree, StorybookTreeParams};
	use chico_vegetation_components::{
		FoliageNode, Layers, Placement, StickNode, StructuralLod, VegetationComponents,
	};
	use clap::Args;
	use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
	use lod::lod_ref::LodRef;
	use lod::{lod_host_scene_pending, SceneChunk};
	use material_ref::MaterialRef;
	use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};

	use super::{definition, OrchardCell, OrchardItem};
	use crate::grove::vc_tuft::{patch_variant_index, variant_noise};
	use crate::grove::{
		canopy_ball_material_from_palette, canopy_proxy_site, foliage_low_canopy_balls,
		foliage_ultra_low_merged_balls, frond_material_from_palette, grove_detail_level,
		grove_lod_culls, grove_lod_level, grove_lod_status, grove_structural_footprint,
		layers_from_nodes, nest_flattened_plant_chunk, placement_noise,
		stick_material_from_palette, woody_grove_scene_chunks, CanopyProxySite, FlatTerrainSample,
		GroveCellVariant, GroveExtent, GroveFrontend, DEFAULT_GROVE_EXTENT_XZ,
		ULTRA_LOW_CANOPY_BIN_METERS,
	};

	pub const ORCHARD_STRUCTURAL_HIGH_FACTOR: f32 = 2.0;
	pub const ORCHARD_STRUCTURAL_MEDIUM_FACTOR: f32 = 5.0;
	pub const ORCHARD_STRUCTURAL_LOW_FACTOR: f32 = 12.0;

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct OrchardParams {
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

		/// Number of unit-height Storybook archetypes (`unit_from_num(0..n)`). Caps unique
		/// merged-mesh handles for High/Medium.
		#[arg(long, default_value_t = 100)]
		pub tree_variants: u32,

		#[arg(skip)]
		resolved_placements: Option<Vec<GroveCellVariant<OrchardCell>>>,
	}

	impl Default for OrchardParams {
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
				terrain: FlatTerrainSample { elevation: 0.35, steepness: 0.10 },
				tree_variants: 100,
				resolved_placements: None,
			}
		}
	}

	impl OrchardParams {
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

		pub fn placements(&self) -> Vec<GroveCellVariant<OrchardCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.placements_on(&self.terrain)
		}

		/// Select placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn placements_on(
			&self,
			world: &impl crate::GroveWorldSample,
		) -> Vec<GroveCellVariant<OrchardCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.grove.assemble(definition()).populate(&self.extent, world)
		}

		pub fn build(&self) -> Orchard {
			self.build_on(&self.terrain)
		}

		/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> Orchard {
			Orchard::from_placements(
				&self.placements_on(world),
				self.grove.noise,
				&self.extent,
				self.tree_variants,
			)
		}
	}

	#[derive(Clone)]
	pub struct OrchardPlant {
		pub placement: Placement,
		pub(crate) tree: Arc<StorybookTree>,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	}

	#[derive(Clone, Component)]
	pub struct Orchard {
		pub plants: Arc<[OrchardPlant]>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl Orchard {
		pub fn from_placements(
			placements: &[GroveCellVariant<OrchardCell>],
			grove_noise: NoiseParams,
			extent: &GroveExtent,
			tree_variants: u32,
		) -> Self {
			let plants: Arc<[OrchardPlant]> = placements
				.iter()
				.map(|placed| grow_plant(placed, grove_noise, tree_variants))
				.collect::<Vec<_>>()
				.into();
			let (structural_center, footprint_radius) = grove_structural_footprint(extent);
			Self { plants, structural_center, footprint_radius, extent: *extent }
		}

		/// High/Medium plant hosts — one lazy producer so begin does not clone every tree.
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
				Some(nest_flattened_plant_chunk(
					Arc::clone(&plant.tree),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
				))
			})]
		}

		fn canopy_sites(&self) -> Vec<CanopyProxySite> {
			self.plants
				.iter()
				.filter_map(|plant| {
					canopy_proxy_site(&plant.tree, plant.placement, &plant.ball_material)
				})
				.collect()
		}
	}

	fn grow_plant(
		placed: &GroveCellVariant<OrchardCell>,
		grove_noise: NoiseParams,
		tree_variants: u32,
	) -> OrchardPlant {
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

		let OrchardItem::Storybook(story) = placed.variant.item();
		let geometry = story.build_with_noise(build_noise);
		let mut params = StorybookTreeParams::default();
		params.geometry = geometry;
		let (unit_params, world_size) = params.into_unit_from_num(variant);
		let placement = Placement::new(placed.position, 0.0)
			.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4)));

		OrchardPlant {
			placement,
			tree: Arc::new(unit_params.build()),
			stick_material,
			ball_material,
			frond_material,
		}
	}

	impl VegetationComponents for Orchard {
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
				ORCHARD_STRUCTURAL_HIGH_FACTOR,
				ORCHARD_STRUCTURAL_MEDIUM_FACTOR,
				ORCHARD_STRUCTURAL_LOW_FACTOR,
			))
		}
	}

	impl LodScene for Orchard {
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
	Orchard, OrchardParams, OrchardPlant, ORCHARD_STRUCTURAL_HIGH_FACTOR,
	ORCHARD_STRUCTURAL_LOW_FACTOR, ORCHARD_STRUCTURAL_MEDIUM_FACTOR,
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
		let dist = OrchardCell::distribution();
		assert_eq!(dist.len(), 3);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, CULTIVATED_EMPTY_WEIGHT);
		assert_eq!(dist.buckets[1].item, Some(OrchardCell::FruitingStorybook));
		assert_eq!(dist.buckets[1].weight, 1.5);
		assert_eq!(dist.buckets[2].item, Some(OrchardCell::PaleBloomStorybook));
		assert_eq!(dist.buckets[2].weight, 0.75);
		Ok(())
	}

	#[test]
	fn placed_share_targets_cultivated_fill() -> Result<()> {
		let dist = OrchardCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!(
			(0.94..=0.96).contains(&share),
			"placed share {share} outside cultivated ~95% target"
		);
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let OrchardItem::Storybook(fruiting) = OrchardCell::FruitingStorybook.item();
		assert_eq!(fruiting.height, UnitRange::new(5.0, 10.0));
		assert_eq!(fruiting.canopy_density, MODERATE_CANOPY_DENSITY);

		let OrchardItem::Storybook(pale) = OrchardCell::PaleBloomStorybook.item();
		assert_eq!(pale.height, UnitRange::new(5.0, 9.0));
		Ok(())
	}

	#[test]
	fn placement_constraints_use_full_elevation_and_rfc_steepness() -> Result<()> {
		let dist = OrchardCell::distribution();
		for bucket in dist.buckets.iter().filter(|b| b.item.is_some()) {
			assert_eq!(bucket.constraints.elevation.start, 0.0);
			assert_eq!(bucket.constraints.elevation.end, 1.0);
		}
		let fruiting = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(OrchardCell::FruitingStorybook))
			.ok_or_else(|| anyhow::anyhow!("missing fruiting bucket"))?;
		assert_eq!(fruiting.constraints.steepness.end, 0.30);

		let pale = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(OrchardCell::PaleBloomStorybook))
			.ok_or_else(|| anyhow::anyhow!("missing pale bloom bucket"))?;
		assert_eq!(pale.constraints.steepness.end, 0.28);
		Ok(())
	}

	#[test]
	#[ignore = "placement constraints deferred to forest-layer normalization"]
	fn steep_slope_rejects_fruiting_but_allows_pale_on_gentler_band() -> Result<()> {
		let prepared =
			OrchardCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let gentle = FlatTerrainSample { elevation: 0.40, steepness: 0.25 };
		let fruiting_outcome = prepared.select_from(
			1,
			Vec3::new(5.0, 0.40, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&gentle,
		);
		match fruiting_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, OrchardCell::FruitingStorybook);
			}
			other => anyhow::bail!("expected FruitingStorybook on gentle slope, got {other:?}"),
		}
		let steep = FlatTerrainSample { elevation: 0.40, steepness: 0.32 };
		let steep_outcome = prepared.select_from(
			1,
			Vec3::new(5.0, 0.40, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&steep,
		);
		match steep_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_ne!(variant, OrchardCell::FruitingStorybook);
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [OrchardCell::FruitingStorybook, OrchardCell::PaleBloomStorybook] {
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
	fn placement_uses_tight_centroid_offset_and_uniform_scale() -> Result<()> {
		let def = definition();
		assert_eq!(def.placement.offset, UnitRange::new(-0.5, 0.5));
		assert_eq!(def.placement.scale, UnitRange::new(1.0, 1.0));
		Ok(())
	}

	#[test]
	fn populated_grove_is_deterministic_and_non_empty() -> Result<()> {
		let noise = crate::grove::GroveFrontend::default().noise;
		let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(120.0, 1.0, 120.0));
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.10 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}

	#[cfg(feature = "render")]
	mod render_tests {
		use super::*;
		use crate::orchard::OrchardParams;
		use bevy::math::bounding::Aabb3d;
		use bevy::prelude::{Entity, Transform};
		use chico_vegetation_components::VegetationComponents;
		use lod::gen::{LodScene, LodSceneLevel};
		use lod::lod_ref::LodRef;

		fn small_grove() -> crate::orchard::Orchard {
			OrchardParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(33.0, 1.0, 33.0)))
				.build()
		}

		#[test]
		fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
			let grove = small_grove();
			assert!(!grove.plants.is_empty(), "expected placed orchard trees");

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
				anyhow::bail!("High orchard should wrap plant chunks");
			};
			assert_eq!(parts.len(), 1, "expected one lazy plant producer");
			let lod::SceneChunk::Lazy { remaining_primitives, remaining_weight, .. } = &parts[0]
			else {
				anyhow::bail!("High orchard plants should be SceneChunk::Lazy");
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
				anyhow::bail!("Low orchard should emit one flattened canopy collection");
			};
			assert_eq!(weight, chico_vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT);
			Ok(())
		}

		#[test]
		fn tree_variants_quantize_archetypes() -> Result<()> {
			use std::collections::HashSet;

			let mut params = OrchardParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(120.0, 1.0, 120.0)));
			params.tree_variants = 4;
			let grove = params.build();
			assert!(!grove.plants.is_empty(), "expected placed orchard trees");
			for plant in grove.plants.iter() {
				assert!(
					(plant.tree.geometry.height() - 1.0).abs() < 1e-4,
					"expected unit height, got {}",
					plant.tree.geometry.height()
				);
			}
			let seeds: HashSet<i32> =
				grove.plants.iter().map(|p| p.tree.geometry.canopy_noise.seed).collect();
			assert!(seeds.len() <= 4, "expected ≤4 unique unit seeds, got {}", seeds.len());
			Ok(())
		}
	}
}
