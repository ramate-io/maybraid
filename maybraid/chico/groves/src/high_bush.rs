//! High Bush — well-known moderate-density tall shrub understory grove
//! ([RFC-183 §3.4.5.4](../../../../rfc/rfc-000-000-183-chico-vegetation/03-04-cellular-groves/05-well-known-understory-groves/04-high-bush/README.md),
//! [#312](https://github.com/ramate-io/maybraid/issues/312)).
//!
//! Common High Bush forms at 1.0–2.5 m: substantial shrub masses that shape sightlines and
//! local movement. Each placement is a [`HighBushShoots`](../../tree-components/src/high_bush_shoots/assembly.rs)
//! bush with dual stick and canopy palettes; forest-layer attachment remains a follow-up.

use std::ops::RangeInclusive;

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

/// RFC `projection_count: Moderate` — all High Bush varietals.
const MODERATE_PROJECTION_RADIAL: UnitRange = UnitRange::new(0.32, 0.48);
const MODERATE_PROJECTION_VERTICAL: UnitRange = UnitRange::new(0.58, 0.78);

/// Authored High Bush grove definition.
///
/// Cell footprint is the midpoint of the RFC's `CELL_SIZE_RANGE` (`3.5..8.0`). The offset range
/// is signed and ± one cell so placements break the underlying grid instead of clustering near
/// cell centers.
pub fn definition() -> GroveDefinition<HighBushCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(5.75),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-5.75, 5.75),
		),
		distribution: HighBushCell::distribution(),
	}
}

/// Ordered high-bush varietals ([RFC-183 §3.4.5.4]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighBushCell {
	GreenHighBush,
	DenseHighBush,
	DryHighBush,
	BerryHighBush,
	CopperCaneHighBush,
}

/// Typed authored geometry for one high-bush varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HighBushItem {
	Bush(&'static HighBushBush),
}

/// Authored geometry ranges for one Common High Bush shrub.
#[derive(Debug, Clone, PartialEq)]
pub struct HighBushBush {
	pub height: UnitRange,
	pub shoot_count: RangeInclusive<u32>,
	pub branch_depth: RangeInclusive<u32>,
	/// RFC `projection_count` — horizontal splay in shoot direction mix.
	pub radial_strength: UnitRange,
	/// RFC `projection_count` — upward bias in shoot direction mix.
	pub vertical_bias: UnitRange,
	/// Terminal foliage radius in **world meters** (render converts to a height fraction).
	pub leaf_radius: UnitRange,
}

const GREEN_HIGH_BUSH: HighBushBush = HighBushBush {
	height: UnitRange::new(1.00, 3.20),
	shoot_count: 7..=10,
	branch_depth: 2..=4,
	radial_strength: MODERATE_PROJECTION_RADIAL,
	vertical_bias: MODERATE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.06, 0.12),
};

const DENSE_HIGH_BUSH: HighBushBush = HighBushBush {
	height: UnitRange::new(1.40, 3.50),
	shoot_count: 8..=12,
	branch_depth: 3..=5,
	radial_strength: MODERATE_PROJECTION_RADIAL,
	vertical_bias: MODERATE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.07, 0.14),
};

const DRY_HIGH_BUSH: HighBushBush = HighBushBush {
	height: UnitRange::new(1.00, 3.00),
	shoot_count: 6..=9,
	branch_depth: 2..=3,
	radial_strength: MODERATE_PROJECTION_RADIAL,
	vertical_bias: MODERATE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.05, 0.10),
};

const BERRY_HIGH_BUSH: HighBushBush = HighBushBush {
	height: UnitRange::new(1.20, 2.80),
	shoot_count: 7..=10,
	branch_depth: 2..=4,
	radial_strength: MODERATE_PROJECTION_RADIAL,
	vertical_bias: MODERATE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.06, 0.12),
};

const COPPER_CANE_HIGH_BUSH: HighBushBush = HighBushBush {
	height: UnitRange::new(1.20, 2.50),
	shoot_count: 7..=11,
	branch_depth: 2..=4,
	radial_strength: MODERATE_PROJECTION_RADIAL,
	vertical_bias: MODERATE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.06, 0.12),
};

const GREEN_HIGH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("shrub_bark", "green_brown"),
	PaletteSlot::new("dark_bark", "gray_brown"),
]);
const DENSE_HIGH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_bark", "wet_brown"),
	PaletteSlot::new("green_brown", "shrub_bark"),
]);
const DRY_HIGH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_bark", "tan_brown"),
	PaletteSlot::new("gray_brown", "straw_brown"),
]);
const BERRY_HIGH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("shrub_bark", "dark_bark"),
	PaletteSlot::new("green_brown", "wet_brown"),
]);
const COPPER_CANE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("copper_red", "orange_bark"),
	PaletteSlot::new("red_brown", "dark_bark"),
]);

const GREEN_HIGH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("dark_green", "light_green"),
	PaletteSlot::new("blue_green", "emerald_green"),
]);
const DENSE_HIGH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("wet_green", "fresh_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);
const DRY_HIGH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "dry_green"),
	PaletteSlot::new("tan_green", "pale_green"),
	PaletteSlot::new("straw_brown", "green"),
]);
const BERRY_HIGH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "leaf_green"),
	PaletteSlot::new("berry_red", "deep_green"),
	PaletteSlot::new("berry_blue", "fresh_green"),
]);
const COPPER_CANE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("yellow_green", "light_green"),
	PaletteSlot::new("berry_red", "leaf_green"),
]);

impl HighBushCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `4.65` (RFC relative proportions); the `None` weight of `11.0` puts
	/// the placed share at `4.65 / 15.65 ≈ 0.30`, inside the RFC's `DENSITY_RANGE`
	/// (`0.16..0.42`).
	pub fn distribution() -> GroveDistribution<Self> {
		GroveDistribution::new(vec![
			GroveBucket::none(11.0),
			GroveBucket::placed(
				2.0,
				PlacementConstraints::new(UnitRange::new(0.0, 0.40), UnitRange::new(0.0, 0.32)),
				Self::GreenHighBush,
			),
			GroveBucket::placed(
				1.25,
				PlacementConstraints::new(UnitRange::new(0.0, 0.40), UnitRange::new(0.0, 0.32)),
				Self::DenseHighBush,
			),
			GroveBucket::placed(
				0.75,
				PlacementConstraints::new(UnitRange::new(0.0, 0.40), UnitRange::new(0.0, 0.32)),
				Self::DryHighBush,
			),
			GroveBucket::placed(
				0.35,
				PlacementConstraints::new(UnitRange::new(0.0, 0.40), UnitRange::new(0.0, 0.32)),
				Self::BerryHighBush,
			),
			GroveBucket::placed(
				0.30,
				PlacementConstraints::new(UnitRange::new(0.05, 0.45), UnitRange::new(0.0, 0.58)),
				Self::CopperCaneHighBush,
			),
		])
	}

	pub fn item(self) -> HighBushItem {
		match self {
			Self::GreenHighBush => HighBushItem::Bush(&GREEN_HIGH_BUSH),
			Self::DenseHighBush => HighBushItem::Bush(&DENSE_HIGH_BUSH),
			Self::DryHighBush => HighBushItem::Bush(&DRY_HIGH_BUSH),
			Self::BerryHighBush => HighBushItem::Bush(&BERRY_HIGH_BUSH),
			Self::CopperCaneHighBush => HighBushItem::Bush(&COPPER_CANE_HIGH_BUSH),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::GreenHighBush => GREEN_HIGH_STICK_MIX,
			Self::DenseHighBush => DENSE_HIGH_STICK_MIX,
			Self::DryHighBush => DRY_HIGH_STICK_MIX,
			Self::BerryHighBush => BERRY_HIGH_STICK_MIX,
			Self::CopperCaneHighBush => COPPER_CANE_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::GreenHighBush => GREEN_HIGH_CANOPY_MIX,
			Self::DenseHighBush => DENSE_HIGH_CANOPY_MIX,
			Self::DryHighBush => DRY_HIGH_CANOPY_MIX,
			Self::BerryHighBush => BERRY_HIGH_CANOPY_MIX,
			Self::CopperCaneHighBush => COPPER_CANE_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
mod vc {
	use std::sync::Arc;

	use bevy::math::bounding::Aabb3d;
	use bevy::prelude::*;
	use bevy::scene::prelude::Scene;
	use chico_sbs_trees::{HighBushShoots, HighBushShootsParams};
	use chico_vegetation_components::{
		FoliageNode, Layers, Placement, StickNode, StructuralLod, VegetationComponents,
	};
	use clap::Args;
	use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
	use lod::lod_ref::LodRef;
	use lod::{lod_host_scene_pending, SceneChunk};
	use material_ref::MaterialRef;
	use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};

	use super::{definition, HighBushCell, HighBushItem};
	use crate::grove::{
		canopy_ball_material_from_palette, canopy_proxy_site, foliage_low_canopy_balls,
		foliage_ultra_low_merged_balls, frond_material_from_palette, grove_detail_level,
		grove_lod_culls, grove_lod_level, grove_lod_status, grove_structural_footprint,
		layers_from_nodes, nest_flattened_plant_chunk, placement_noise, stick_material_from_palette,
		woody_grove_scene_chunks, CanopyProxySite, FlatTerrainSample, GroveCellVariant,
		GroveExtent, GroveFrontend, DEFAULT_GROVE_EXTENT_XZ, ULTRA_LOW_CANOPY_BIN_METERS,
	};
	use crate::grove::vc_tuft::{patch_variant_index, variant_noise};

	pub const HIGH_BUSH_STRUCTURAL_HIGH_FACTOR: f32 = 2.0;
	pub const HIGH_BUSH_STRUCTURAL_MEDIUM_FACTOR: f32 = 5.0;
	pub const HIGH_BUSH_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct HighBushParams {
		#[command(flatten, next_help_heading = "Grove")]
		pub grove: GroveFrontend,

		#[arg(
			long,
			default_value = "0,1.0,1.0,1",
			value_parser = noise_params_from_scalar_str,
			value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
			help_heading = "The noise applied to the chains of sticks in bushes",
		)]
		pub bush_chain_noise: NoiseParams,

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

		/// Number of unit-height bush archetypes (`unit_from_num(0..n)`). Caps unique
		/// merged-mesh handles for High/Medium.
		#[arg(long, default_value_t = 100)]
		pub tree_variants: u32,

		#[arg(skip)]
		resolved_placements: Option<Vec<GroveCellVariant<HighBushCell>>>,
	}

	impl Default for HighBushParams {
		fn default() -> Self {
			Self {
				grove: GroveFrontend::default(),
				bush_chain_noise: NoiseParams::from_scalar(0.0, 1.0, 1.0, 1),
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

	impl HighBushParams {
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

		pub fn placements(&self) -> Vec<GroveCellVariant<HighBushCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.placements_on(&self.terrain)
		}

		/// Select placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn placements_on(
			&self,
			world: &impl crate::GroveWorldSample,
		) -> Vec<GroveCellVariant<HighBushCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.grove.assemble(definition()).populate(&self.extent, world)
		}

		pub fn build(&self) -> HighBush {
			self.build_on(&self.terrain)
		}

		/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> HighBush {
			HighBush::from_placements(
				&self.placements_on(world),
				self.grove.noise,
				self.bush_chain_noise,
				&self.extent,
				self.tree_variants,
			)
		}
	}

	#[derive(Clone)]
	pub struct HighBushPlant {
		pub placement: Placement,
		pub(crate) bush: Arc<HighBushShoots>,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	}

	#[derive(Clone, Component)]
	pub struct HighBush {
		pub plants: Arc<[HighBushPlant]>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl HighBush {
		pub fn from_placements(
			placements: &[GroveCellVariant<HighBushCell>],
			grove_noise: NoiseParams,
			bush_chain_noise: NoiseParams,
			extent: &GroveExtent,
			tree_variants: u32,
		) -> Self {
			let plants: Arc<[HighBushPlant]> = placements
				.iter()
				.map(|placed| grow_plant(placed, grove_noise, bush_chain_noise, tree_variants))
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
					Arc::clone(&plant.bush),
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
					canopy_proxy_site(&plant.bush, plant.placement, &plant.ball_material)
				})
				.collect()
		}
	}

	fn grow_plant(
		placed: &GroveCellVariant<HighBushCell>,
		grove_noise: NoiseParams,
		bush_chain_noise: NoiseParams,
		tree_variants: u32,
	) -> HighBushPlant {
		let variant = patch_variant_index(placed.position, tree_variants);
		let build_noise = variant_noise(grove_noise, variant);
		let chain_noise = variant_noise(bush_chain_noise, variant);
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

		let HighBushItem::Bush(bush) = placed.variant.item();
		let mut shape = bush.build_with_noise(build_noise);
		shape.chain_noise = chain_noise;
		let (unit_params, world_size) = HighBushShootsParams::new(shape).into_unit_from_num(variant);
		let placement = Placement::new(placed.position, 0.0)
			.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4)));

		HighBushPlant {
			placement,
			bush: Arc::new(unit_params.build()),
			stick_material,
			ball_material,
			frond_material,
		}
	}

	impl VegetationComponents for HighBush {
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
				HIGH_BUSH_STRUCTURAL_HIGH_FACTOR,
				HIGH_BUSH_STRUCTURAL_MEDIUM_FACTOR,
				HIGH_BUSH_STRUCTURAL_LOW_FACTOR,
			))
		}
	}

	impl LodScene for HighBush {
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

		fn small_grove() -> HighBush {
			HighBushParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(40.0, 1.0, 40.0)))
				.build()
		}

		#[test]
		fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
			let grove = small_grove();
			assert!(!grove.plants.is_empty(), "expected placed high bushes");

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
				anyhow::bail!("High high-bush should wrap plant chunks");
			};
			assert_eq!(parts.len(), 1, "expected one lazy plant producer");
			let lod::SceneChunk::Lazy { remaining_primitives, remaining_weight, .. } = &parts[0]
			else {
				anyhow::bail!("High high-bush plants should be SceneChunk::Lazy");
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
				anyhow::bail!("Low high-bush should emit one flattened canopy collection");
			};
			assert_eq!(weight, chico_vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT);
			Ok(())
		}

		#[test]
		fn tree_variants_quantize_archetypes() -> Result<()> {
			use std::collections::HashSet;

			let mut params = HighBushParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(40.0, 1.0, 40.0)));
			params.tree_variants = 4;
			let grove = params.build();
			assert!(!grove.plants.is_empty(), "expected placed high bushes");
			for plant in grove.plants.iter() {
				assert!(
					(plant.bush.shape.height - 1.0).abs() < 1e-4,
					"expected unit height, got {}",
					plant.bush.shape.height
				);
			}
			let seeds: HashSet<i32> =
				grove.plants.iter().map(|p| p.bush.shape.chain_noise.seed).collect();
			assert!(seeds.len() <= 4, "expected ≤4 unique unit seeds, got {}", seeds.len());
			Ok(())
		}
	}
}

#[cfg(feature = "render")]
pub use vc::{
	HighBush, HighBushParams, HighBushPlant, HIGH_BUSH_STRUCTURAL_HIGH_FACTOR,
	HIGH_BUSH_STRUCTURAL_LOW_FACTOR, HIGH_BUSH_STRUCTURAL_MEDIUM_FACTOR,
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
		let dist = HighBushCell::distribution();
		assert_eq!(dist.len(), 6);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 11.0);
		assert_eq!(dist.buckets[1].item, Some(HighBushCell::GreenHighBush));
		assert_eq!(dist.buckets[1].weight, 2.0);
		assert_eq!(dist.buckets[2].item, Some(HighBushCell::DenseHighBush));
		assert_eq!(dist.buckets[2].weight, 1.25);
		assert_eq!(dist.buckets[3].item, Some(HighBushCell::DryHighBush));
		assert_eq!(dist.buckets[3].weight, 0.75);
		assert_eq!(dist.buckets[4].item, Some(HighBushCell::BerryHighBush));
		assert_eq!(dist.buckets[4].weight, 0.35);
		assert_eq!(dist.buckets[5].item, Some(HighBushCell::CopperCaneHighBush));
		assert_eq!(dist.buckets[5].weight, 0.30);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = HighBushCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.16..=0.42).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn bush_geometry_follows_authored_bands() -> Result<()> {
		for cell in [
			HighBushCell::GreenHighBush,
			HighBushCell::DenseHighBush,
			HighBushCell::DryHighBush,
			HighBushCell::BerryHighBush,
			HighBushCell::CopperCaneHighBush,
		] {
			let HighBushItem::Bush(bush) = cell.item();
			assert!(bush.height.start >= 1.00);
			assert!(bush.height.end <= 3.50);
			assert!(*bush.shoot_count.start() >= 6);
			assert!(*bush.shoot_count.end() <= 12);
			assert!(*bush.branch_depth.start() >= 2);
			assert!(*bush.branch_depth.end() <= 5);
			assert!(bush.leaf_radius.start >= 0.05);
			assert!(bush.leaf_radius.end <= 0.14);
		}
		Ok(())
	}

	#[test]
	#[ignore = "placement constraints deferred to forest-layer normalization"]
	fn constraint_first_fit_fallback() -> Result<()> {
		// BerryHighBush (index 4) rejects steepness 0.40; first-fit falls to CopperCaneHighBush
		// (index 5), which allows steepness up to 0.58.
		let prepared =
			HighBushCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.40 };
		let outcome = prepared.select_from(
			4,
			Vec3::new(5.0, 0.35, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, HighBushCell::CopperCaneHighBush);
			}
			other => anyhow::bail!("expected CopperCaneHighBush fallback, got {other:?}"),
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
