//! Vineyard — high-density cultivated Rory-trained vine upper-canopy grove
//! ([RFC-183 §3.4.7.8], [#355](https://github.com/ramate-io/maybraid/issues/355)).
//!
//! Low trained-vine rows with very tight cell offset and grape-like palettes. Forest-layer
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

/// Authored Vineyard grove definition.
///
/// Cell footprint sits at the RFC midpoint (`4.5` m). Placements stay on cell centroids with only
/// ±`0.5` m horizontal jitter for regular vine rows.
pub fn definition() -> GroveDefinition<VineyardCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(4.5),
		placement: GrovePlacementRanges::new(UnitRange::new(1.0, 1.0), UnitRange::new(-0.5, 0.5)),
		distribution: VineyardCell::distribution(),
	}
}

/// Ordered vineyard varietals ([RFC-183 §3.4.7.8]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VineyardCell {
	TrainedVineRory,
}

/// Typed authored geometry for one vineyard varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VineyardItem {
	Rory(&'static VineyardRory),
}

/// Authored geometry ranges for one trained-vine Rory form.
#[derive(Debug, Clone, PartialEq)]
pub struct VineyardRory {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const TRAINED_VINE_RORY: VineyardRory = VineyardRory {
	height: UnitRange::new(1.5, 3.0),
	stalk_radius: UnitRange::new(0.045, 0.090),
	canopy_spread: UnitRange::new(1.0, 2.4),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const VINE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("vine_bark", "red_brown"),
	PaletteSlot::new("weathered_bark", "gray_brown"),
]);

const VINE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("grape_green", "fresh_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);

/// Explicit `None` weight so ~`95%` of cells receive a vine (`0.05` empty vs `0.95` placed).
const CULTIVATED_EMPTY_WEIGHT: f32 = 0.05;
const CULTIVATED_PLACED_WEIGHT: f32 = 0.95;

impl VineyardCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// `None` weight `0.05` against placed weight `0.95` yields a `0.95` placed share for
	/// regular row planting.
	pub fn distribution() -> GroveDistribution<Self> {
		let trained_vine =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.34));
		GroveDistribution::new(vec![
			GroveBucket::none(CULTIVATED_EMPTY_WEIGHT),
			GroveBucket::placed(CULTIVATED_PLACED_WEIGHT, trained_vine, Self::TrainedVineRory),
		])
	}

	pub fn item(self) -> VineyardItem {
		match self {
			Self::TrainedVineRory => VineyardItem::Rory(&TRAINED_VINE_RORY),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		VINE_STICK_MIX
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		VINE_CANOPY_MIX
	}
}

#[cfg(feature = "render")]
mod vc {
	use bevy::math::bounding::Aabb3d;
	use bevy::prelude::*;
	use bevy::scene::prelude::Scene;
	use chico_sbs_trees::{RorysHeadTrained, RorysHeadTrainedParams};
	use chico_vegetation_components::{
		FoliageNode, Layers, Placement, StickNode, StructuralLod, VegetationComponents,
	};
	use clap::Args;
	use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
	use lod::lod_ref::LodRef;
	use lod::{lod_host_scene_pending, SceneChunk};
	use material_ref::MaterialRef;
	use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};

	use super::{definition, VineyardCell, VineyardItem};
	use crate::grove::{
		canopy_ball_material_from_palette, canopy_proxy_site, foliage_low_canopy_balls,
		foliage_ultra_low_merged_balls, frond_material_from_palette, grove_detail_level,
		grove_lod_culls, grove_lod_level, grove_lod_status, grove_structural_footprint,
		layers_from_nodes, nest_placed_plant_chunk, placement_noise, stick_material_from_palette,
		woody_grove_scene_chunks, CanopyProxySite, FlatTerrainSample, GroveCellVariant, GroveExtent,
		GroveFrontend, DEFAULT_GROVE_EXTENT_XZ, ULTRA_LOW_CANOPY_BIN_METERS,
	};

	pub const VINEYARD_STRUCTURAL_HIGH_FACTOR: f32 = 2.0;
	pub const VINEYARD_STRUCTURAL_MEDIUM_FACTOR: f32 = 5.0;
	pub const VINEYARD_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct VineyardParams {
		#[command(flatten, next_help_heading = "Grove")]
		pub grove: GroveFrontend,

		#[arg(
			long,
			default_value = "0,1.0,1.0,1",
			value_parser = noise_params_from_scalar_str,
			value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
			help_heading = "The noise applied to the chains of sticks in vines",
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
		resolved_placements: Option<Vec<GroveCellVariant<VineyardCell>>>,
	}

	impl Default for VineyardParams {
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
				terrain: FlatTerrainSample { elevation: 0.35, steepness: 0.12 },
				resolved_placements: None,
			}
		}
	}

	impl VineyardParams {
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

		pub fn placements(&self) -> Vec<GroveCellVariant<VineyardCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.grove.assemble(definition()).populate(&self.extent, &self.terrain)
		}

		pub fn build(&self) -> Vineyard {
			Vineyard::from_placements(&self.placements(), self.grove.noise, &self.extent)
		}
	}

	#[derive(Clone)]
	pub struct VineyardPlant {
		pub placement: Placement,
		tree: RorysHeadTrained,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	}

	#[derive(Clone, Component)]
	pub struct Vineyard {
		pub plants: Vec<VineyardPlant>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl Vineyard {
		pub fn from_placements(
			placements: &[GroveCellVariant<VineyardCell>],
			grove_noise: NoiseParams,
			extent: &GroveExtent,
		) -> Self {
			let plants = placements.iter().map(|placed| grow_plant(placed, grove_noise)).collect();
			let (structural_center, footprint_radius) = grove_structural_footprint(extent);
			Self {
				plants,
				structural_center,
				footprint_radius,
				extent: *extent,
			}
		}

		fn nest_plant_chunks(&self, lod_ref: &LodRef) -> Vec<SceneChunk> {
			self.plants
				.iter()
				.map(|plant| {
					nest_placed_plant_chunk(
						plant.tree.clone(),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						lod_ref,
					)
				})
				.collect()
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
		placed: &GroveCellVariant<VineyardCell>,
		grove_noise: NoiseParams,
	) -> VineyardPlant {
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

		let VineyardItem::Rory(vine) = placed.variant.item();
		let geometry = vine.build_with_noise(build_noise);
		let mut params = RorysHeadTrainedParams::default();
		params.geometry = geometry;

		VineyardPlant {
			placement,
			tree: params.build(),
			stick_material,
			ball_material,
			frond_material,
		}
	}

	impl VegetationComponents for Vineyard {
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
				| LodSceneLevel::Resolution(_) => layers_from_nodes(
					foliage_ultra_low_merged_balls(&self.canopy_sites(), ULTRA_LOW_CANOPY_BIN_METERS),
				),
			}
		}

		fn structural_lod(&self) -> Option<StructuralLod> {
			Some(
				StructuralLod::new(self.structural_center, self.footprint_radius).with_factors(
					VINEYARD_STRUCTURAL_HIGH_FACTOR,
					VINEYARD_STRUCTURAL_MEDIUM_FACTOR,
					VINEYARD_STRUCTURAL_LOW_FACTOR,
				),
			)
		}
	}

	impl LodScene for Vineyard {
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
						self, lod_ref, level, &mut children,
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
	Vineyard, VineyardParams, VineyardPlant, VINEYARD_STRUCTURAL_HIGH_FACTOR,
	VINEYARD_STRUCTURAL_LOW_FACTOR, VINEYARD_STRUCTURAL_MEDIUM_FACTOR,
};


#[cfg(test)]
mod tests {
	use super::*;
	use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveExtent};
	use anyhow::Result;
	use bevy_math::Vec3;

	#[test]
	fn distribution_matches_rfc_order_and_weights() -> Result<()> {
		let dist = VineyardCell::distribution();
		assert_eq!(dist.len(), 2);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, CULTIVATED_EMPTY_WEIGHT);
		assert_eq!(dist.buckets[1].item, Some(VineyardCell::TrainedVineRory));
		assert_eq!(dist.buckets[1].weight, CULTIVATED_PLACED_WEIGHT);
		Ok(())
	}

	#[test]
	fn placed_share_targets_cultivated_fill() -> Result<()> {
		let dist = VineyardCell::distribution();
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
	fn placement_uses_tight_centroid_offset_and_uniform_scale() -> Result<()> {
		let def = definition();
		assert_eq!(def.placement.offset, UnitRange::new(-0.5, 0.5));
		assert_eq!(def.placement.scale, UnitRange::new(1.0, 1.0));
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let VineyardItem::Rory(vine) = VineyardCell::TrainedVineRory.item();
		assert_eq!(vine.height, UnitRange::new(1.5, 3.0));
		assert_eq!(vine.canopy_spread, UnitRange::new(1.0, 2.4));
		assert_eq!(vine.canopy_density, SPARSE_CANOPY_DENSITY);
		Ok(())
	}

	#[test]
	fn placement_constraints_use_full_elevation_and_rfc_steepness() -> Result<()> {
		let dist = VineyardCell::distribution();
		let vine = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(VineyardCell::TrainedVineRory))
			.ok_or_else(|| anyhow::anyhow!("missing trained vine bucket"))?;
		assert_eq!(vine.constraints.elevation.start, 0.0);
		assert_eq!(vine.constraints.elevation.end, 1.0);
		assert_eq!(vine.constraints.steepness.end, 0.34);
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [VineyardCell::TrainedVineRory] {
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
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0));
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.12 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
