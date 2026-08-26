//! Date Grove — high-density cultivated Date Palm upper-canopy grove
//! ([RFC-183 §3.4.7.9], [#357](https://github.com/ramate-io/maybraid/issues/357)).
//!
//! Single moderate-crown date palm form with tight cell offset on warm flat terrain.
//! Forest-layer attachment remains a follow-up.

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

/// Moderate sampled crown-density band ([`0.35`, `0.65`]).
const MODERATE_CROWN_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);

/// Authored Date Grove definition.
///
/// Cell footprint sits at the RFC midpoint (`12.0` m). Placements stay on cell centroids with only
/// ±`0.5` m horizontal jitter for regular palm rows.
pub fn definition() -> GroveDefinition<DateGroveCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(12.0),
		placement: GrovePlacementRanges::new(UnitRange::new(1.0, 1.0), UnitRange::new(-0.5, 0.5)),
		distribution: DateGroveCell::distribution(),
	}
}

/// Ordered date-grove varietals ([RFC-183 §3.4.7.9]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateGroveCell {
	FruitingDatePalm,
}

/// Typed authored geometry for one date-grove varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DateGroveItem {
	DatePalm(&'static DateGroveDatePalm),
}

/// Authored geometry ranges for one Date Palm form.
#[derive(Debug, Clone, PartialEq)]
pub struct DateGroveDatePalm {
	pub height: UnitRange,
	pub crown_density: UnitRange,
}

const FRUITING_DATE_PALM: DateGroveDatePalm =
	DateGroveDatePalm { height: UnitRange::new(5.0, 8.0), crown_density: MODERATE_CROWN_DENSITY };

const DATE_PALM_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_bark", "tan_bark"),
	PaletteSlot::new("date_trunk", "dry_brown"),
]);

const DATE_PALM_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_green", "olive_green"),
	PaletteSlot::new("fresh_green", "yellow_green"),
]);

/// Explicit `None` weight so ~`95%` of cells receive a palm (`0.05` empty vs `0.95` placed).
const CULTIVATED_EMPTY_WEIGHT: f32 = 0.05;
const CULTIVATED_PLACED_WEIGHT: f32 = 0.95;

impl DateGroveCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// `None` weight `0.05` against placed weight `0.95` yields a `0.95` placed share for
	/// regular grove planting.
	pub fn distribution() -> GroveDistribution<Self> {
		let fruiting_date =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.30));
		GroveDistribution::new(vec![
			GroveBucket::none(CULTIVATED_EMPTY_WEIGHT),
			GroveBucket::placed(CULTIVATED_PLACED_WEIGHT, fruiting_date, Self::FruitingDatePalm),
		])
	}

	pub fn item(self) -> DateGroveItem {
		match self {
			Self::FruitingDatePalm => DateGroveItem::DatePalm(&FRUITING_DATE_PALM),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		DATE_PALM_STICK_MIX
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		DATE_PALM_CANOPY_MIX
	}
}

#[cfg(feature = "render")]
mod vc {
	use bevy::math::bounding::Aabb3d;
	use bevy::prelude::*;
	use bevy::scene::prelude::Scene;
	use std::sync::Arc;

	use chico_sbs_trees::{DatePalm, DatePalmParams, QuantizedPlant};
	use chico_vegetation_components::{
		FoliageNode, Layers, Placement, StickNode, StructuralLod, VegetationComponents,
	};
	use clap::Args;
	use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
	use lod::lod_ref::LodRef;
	use lod::{lod_host_scene_pending, SceneChunk};
	use material_ref::MaterialRef;
	use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};

	use super::{definition, DateGroveCell, DateGroveItem, FRUITING_DATE_PALM};
	use crate::grove::vc_tuft::{patch_variant_index, variant_noise};
	use crate::grove::{
		canopy_ball_material_from_palette, canopy_proxy_crown, foliage_ultra_low_merged_balls,
		frond_material_from_palette, grove_detail_level_keep_low, grove_lod_culls, grove_lod_level,
		grove_lod_status, grove_structural_footprint, layers_from_nodes,
		nest_flattened_plant_chunk, placement_noise, stick_material_from_palette,
		woody_grove_scene_chunks_keep_low_plants, CanopyProxySite, FlatTerrainSample,
		GroveCellVariant, GroveExtent, GroveFrontend, DEFAULT_GROVE_EXTENT_XZ,
		ULTRA_LOW_CANOPY_BIN_METERS,
	};

	pub const DATE_GROVE_STRUCTURAL_HIGH_FACTOR: f32 = 2.0;
	pub const DATE_GROVE_STRUCTURAL_MEDIUM_FACTOR: f32 = 5.0;
	pub const DATE_GROVE_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct DateGroveParams {
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

		/// Number of unit-height plant archetypes (`unit_from_num(0..n)`). Caps unique
		/// merged-mesh handles for High/Medium.
		#[arg(long, default_value_t = 100)]
		pub tree_variants: u32,

		#[arg(skip)]
		resolved_placements: Option<Vec<GroveCellVariant<DateGroveCell>>>,
	}

	impl Default for DateGroveParams {
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
				tree_variants: 100,
				resolved_placements: None,
			}
		}
	}

	impl DateGroveParams {
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

		pub fn placements(&self) -> Vec<GroveCellVariant<DateGroveCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.placements_on(&self.terrain)
		}

		/// Select placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn placements_on(
			&self,
			world: &impl crate::GroveWorldSample,
		) -> Vec<GroveCellVariant<DateGroveCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.grove.assemble(definition()).populate(&self.extent, world)
		}

		pub fn build(&self) -> DateGrove {
			self.build_on(&self.terrain)
		}

		/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> DateGrove {
			DateGrove::from_placements(
				&self.placements_on(world),
				self.grove.noise,
				&self.extent,
				self.tree_variants,
			)
		}
	}

	/// Cache identity for fruiting Date Grove remixes of [`DatePalm`].
	struct DateGrovePalm;

	impl QuantizedPlant for DateGrovePalm {
		type Unit = DatePalm;

		fn build_unit(num: u32) -> (DatePalm, f32) {
			let noise = variant_noise(GroveFrontend::default().noise, num);
			let mut params = DatePalmParams::default();
			params.geometry = FRUITING_DATE_PALM.build_with_noise(noise);
			let (unit, world_size) = params.into_unit_from_num(num);
			(unit.build(), world_size)
		}
	}

	#[derive(Clone)]
	pub struct DateGrovePlant {
		pub placement: Placement,
		pub(crate) tree: Arc<DatePalm>,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	}

	#[derive(Clone, Component)]
	pub struct DateGrove {
		pub plants: Arc<[DateGrovePlant]>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl DateGrove {
		pub fn from_placements(
			placements: &[GroveCellVariant<DateGroveCell>],
			grove_noise: NoiseParams,
			extent: &GroveExtent,
			tree_variants: u32,
		) -> Self {
			let plants: Arc<[DateGrovePlant]> = placements
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
					canopy_proxy_crown(&plant.tree, plant.placement, &plant.ball_material)
				})
				.collect()
		}
	}

	fn grow_plant(
		placed: &GroveCellVariant<DateGroveCell>,
		grove_noise: NoiseParams,
		tree_variants: u32,
	) -> DateGrovePlant {
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

		let DateGroveItem::DatePalm(_) = placed.variant.item();
		let (tree, world_size) = DateGrovePalm::grow_num(variant);

		DateGrovePlant {
			placement: Placement::new(placed.position, 0.0)
				.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
			tree,
			stick_material,
			ball_material,
			frond_material,
		}
	}

	impl VegetationComponents for DateGrove {
		fn stick_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StickNode> {
			Layers::new()
		}

		fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
			match level {
				LodSceneLevel::High | LodSceneLevel::Medium | LodSceneLevel::Low => Layers::new(),
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
				DATE_GROVE_STRUCTURAL_HIGH_FACTOR,
				DATE_GROVE_STRUCTURAL_MEDIUM_FACTOR,
				DATE_GROVE_STRUCTURAL_LOW_FACTOR,
			))
		}
	}

	impl LodScene for DateGrove {
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
			match grove_detail_level_keep_low(level) {
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
			woody_grove_scene_chunks_keep_low_plants(
				level,
				lod_ref,
				self.nest_plant_chunks(lod_ref),
				self,
			)
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

		fn small_grove() -> DateGrove {
			DateGroveParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0)))
				.build()
		}

		#[test]
		fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
			let grove = small_grove();
			assert!(!grove.plants.is_empty(), "expected placed date-grove plants");

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
				anyhow::bail!("High date-grove should wrap plant chunks");
			};
			assert_eq!(parts.len(), 1, "expected one lazy plant producer");
			let lod::SceneChunk::Lazy { remaining_primitives, remaining_weight, .. } = &parts[0]
			else {
				anyhow::bail!("High date-grove plants should be SceneChunk::Lazy");
			};
			assert_eq!(*remaining_primitives, grove.plants.len());
			assert_eq!(*remaining_weight as usize, grove.plants.len());

			assert_eq!(grove.stick_nodes_for_level(LodSceneLevel::Low).len(), 0);
			assert_eq!(grove.foliage_nodes_for_level(LodSceneLevel::Low).len(), 0);
			assert!(!grove.foliage_nodes_for_level(LodSceneLevel::UltraLow).flatten().is_empty());
			let low = grove.scene_chunks_with_level(&lod_ref, LodSceneLevel::Low);
			let lod::SceneChunk::SubChunks(parts) = low else {
				anyhow::bail!("Low date-grove should nest plant chunks");
			};
			assert_eq!(parts.len(), 1, "expected one lazy plant producer");
			let lod::SceneChunk::Lazy { remaining_primitives, remaining_weight, .. } = &parts[0]
			else {
				anyhow::bail!("Low date-grove plants should be SceneChunk::Lazy");
			};
			assert_eq!(*remaining_primitives, grove.plants.len());
			assert_eq!(*remaining_weight as usize, grove.plants.len());
			Ok(())
		}

		#[test]
		fn tree_variants_quantize_archetypes() -> Result<()> {
			use std::collections::HashSet;
			use std::sync::Arc;

			let mut params = DateGroveParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0)));
			params.tree_variants = 4;
			let grove = params.build();
			assert!(!grove.plants.is_empty(), "expected placed date-grove plants");
			for plant in grove.plants.iter() {
				assert!(
					(plant.tree.geometry.height() - 1.0).abs() < 1e-4,
					"expected unit height, got {}",
					plant.tree.geometry.height()
				);
			}
			let seeds: HashSet<i32> =
				grove.plants.iter().map(|p| p.tree.geometry.trunk_noise.seed).collect();
			assert!(seeds.len() <= 4, "expected ≤4 unique unit seeds, got {}", seeds.len());
			let ptrs: HashSet<_> = grove.plants.iter().map(|p| Arc::as_ptr(&p.tree)).collect();
			assert!(
				grove.plants.len() > ptrs.len(),
				"expected repeated variants to share one unit Arc"
			);
			Ok(())
		}
	}
}

#[cfg(feature = "render")]
pub use vc::{
	DateGrove, DateGroveParams, DateGrovePlant, DATE_GROVE_STRUCTURAL_HIGH_FACTOR,
	DATE_GROVE_STRUCTURAL_LOW_FACTOR, DATE_GROVE_STRUCTURAL_MEDIUM_FACTOR,
};

#[cfg(test)]
mod tests {
	use super::*;
	use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveExtent};
	use anyhow::Result;
	use bevy_math::Vec3;

	#[test]
	fn distribution_matches_rfc_order_and_weights() -> Result<()> {
		let dist = DateGroveCell::distribution();
		assert_eq!(dist.len(), 2);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, CULTIVATED_EMPTY_WEIGHT);
		assert_eq!(dist.buckets[1].item, Some(DateGroveCell::FruitingDatePalm));
		assert_eq!(dist.buckets[1].weight, CULTIVATED_PLACED_WEIGHT);
		Ok(())
	}

	#[test]
	fn placed_share_targets_cultivated_fill() -> Result<()> {
		let dist = DateGroveCell::distribution();
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
		let DateGroveItem::DatePalm(palm) = DateGroveCell::FruitingDatePalm.item();
		assert_eq!(palm.height, UnitRange::new(5.0, 8.0));
		assert_eq!(palm.crown_density, MODERATE_CROWN_DENSITY);
		Ok(())
	}

	#[test]
	fn placement_constraints_use_full_elevation_and_rfc_steepness() -> Result<()> {
		let dist = DateGroveCell::distribution();
		let palm = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(DateGroveCell::FruitingDatePalm))
			.ok_or_else(|| anyhow::anyhow!("missing fruiting date palm bucket"))?;
		assert_eq!(palm.constraints.elevation.start, 0.0);
		assert_eq!(palm.constraints.elevation.end, 1.0);
		assert_eq!(palm.constraints.steepness.end, 0.30);
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [DateGroveCell::FruitingDatePalm] {
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
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(120.0, 1.0, 120.0));
		let terrain = FlatTerrainSample { elevation: 0.30, steepness: 0.10 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
