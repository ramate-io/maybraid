//! Palm Shade — sparse upper-canopy grove with Waialea and Date Palm variants
//! ([RFC-183 §3.4.7.10], [#332](https://github.com/ramate-io/maybraid/issues/332)).
//!
//! Tower Waialea columns, dense lower Waialea crowns, and clustered Date Palms for oasis shade.
//! Forest-layer attachment remains a follow-up.

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

/// Authored Palm Shade grove definition.
///
/// Cell footprint sits at the RFC midpoint (`24.0` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<PalmShadeCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(24.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-24.0, 24.0),
		),
		distribution: PalmShadeCell::distribution(),
	}
}

/// Ordered palm-shade varietals ([RFC-183 §3.4.7.10]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PalmShadeCell {
	TowerWaialeaPalm,
	LowerWaialeaPalm,
	ShadeDatePalm,
	ClusterDatePalm,
}

/// Typed authored geometry for one palm-shade varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PalmShadeItem {
	WaialeaPalm(&'static PalmShadeWaialeaPalm),
	DatePalm(&'static PalmShadeDatePalm),
}

/// Authored geometry ranges for one Waialea Palm form.
#[derive(Debug, Clone, PartialEq)]
pub struct PalmShadeWaialeaPalm {
	pub height: UnitRange,
	pub crown_density: UnitRange,
}

/// Authored geometry ranges for one Date Palm form.
#[derive(Debug, Clone, PartialEq)]
pub struct PalmShadeDatePalm {
	pub height: UnitRange,
	pub crown_density: UnitRange,
}

const TOWER_WAIALEA_PALM: PalmShadeWaialeaPalm = PalmShadeWaialeaPalm {
	height: UnitRange::new(20.0, 40.0),
	crown_density: MODERATE_CANOPY_DENSITY,
};

const LOWER_WAIALEA_PALM: PalmShadeWaialeaPalm =
	PalmShadeWaialeaPalm { height: UnitRange::new(8.0, 20.0), crown_density: DENSE_CANOPY_DENSITY };

const SHADE_DATE_PALM: PalmShadeDatePalm =
	PalmShadeDatePalm { height: UnitRange::new(6.0, 20.0), crown_density: MODERATE_CANOPY_DENSITY };

const CLUSTER_DATE_PALM: PalmShadeDatePalm =
	PalmShadeDatePalm { height: UnitRange::new(6.0, 12.0), crown_density: DENSE_CANOPY_DENSITY };

const WAIALEA_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_bark", "tan_bark"),
	PaletteSlot::new("wet_brown", "green_brown"),
]);

const WAIALEA_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("wet_green", "lime_green"),
]);

const SHADE_DATE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_bark", "date_trunk"),
	PaletteSlot::new("tan_bark", "dry_brown"),
]);

const SHADE_DATE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_green", "olive_green"),
	PaletteSlot::new("fresh_green", "yellow_green"),
]);

const CLUSTER_DATE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("date_trunk", "dry_brown"),
	PaletteSlot::new("tan_bark", "palm_bark"),
]);

const CLUSTER_DATE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_green", "olive_green"),
	PaletteSlot::new("yellow_green", "fresh_green"),
]);

impl PalmShadeCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `3.2` (RFC relative proportions); the `None` weight of `10.7` puts
	/// the placed share at `3.2 / 14.0 ≈ 0.23`, mid RFC `DENSITY_RANGE` (`0.08..0.24`).
	pub fn distribution() -> GroveDistribution<Self> {
		let tower_waialea =
			PlacementConstraints::new(UnitRange::new(0.0, 0.46), UnitRange::new(0.0, 0.56));
		let lower_waialea =
			PlacementConstraints::new(UnitRange::new(0.0, 0.50), UnitRange::new(0.0, 0.62));
		let shade_date =
			PlacementConstraints::new(UnitRange::new(0.0, 0.52), UnitRange::new(0.0, 0.42));
		let cluster_date =
			PlacementConstraints::new(UnitRange::new(0.0, 0.44), UnitRange::new(0.0, 0.36));
		GroveDistribution::new(vec![
			GroveBucket::none(10.7),
			GroveBucket::placed(0.8, tower_waialea, Self::TowerWaialeaPalm),
			GroveBucket::placed(0.8, lower_waialea, Self::LowerWaialeaPalm),
			GroveBucket::placed(1.0, shade_date, Self::ShadeDatePalm),
			GroveBucket::placed(0.6, cluster_date, Self::ClusterDatePalm),
		])
	}

	pub fn item(self) -> PalmShadeItem {
		match self {
			Self::TowerWaialeaPalm => PalmShadeItem::WaialeaPalm(&TOWER_WAIALEA_PALM),
			Self::LowerWaialeaPalm => PalmShadeItem::WaialeaPalm(&LOWER_WAIALEA_PALM),
			Self::ShadeDatePalm => PalmShadeItem::DatePalm(&SHADE_DATE_PALM),
			Self::ClusterDatePalm => PalmShadeItem::DatePalm(&CLUSTER_DATE_PALM),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::TowerWaialeaPalm | Self::LowerWaialeaPalm => WAIALEA_STICK_MIX,
			Self::ShadeDatePalm => SHADE_DATE_STICK_MIX,
			Self::ClusterDatePalm => CLUSTER_DATE_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::TowerWaialeaPalm | Self::LowerWaialeaPalm => WAIALEA_CANOPY_MIX,
			Self::ShadeDatePalm => SHADE_DATE_CANOPY_MIX,
			Self::ClusterDatePalm => CLUSTER_DATE_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
mod vc {
	use std::sync::Arc;

	use bevy::math::bounding::Aabb3d;
	use bevy::prelude::*;
	use bevy::scene::prelude::Scene;
	use chico_sbs_trees::{DatePalm, DatePalmParams, WaialeaPalm, WaialeaPalmParams};
	use chico_vegetation_components::{
		FoliageNode, Layers, Placement, StickNode, StructuralLod, VegetationComponents,
	};
	use clap::Args;
	use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
	use lod::lod_ref::LodRef;
	use lod::{lod_host_scene_pending, SceneChunk};
	use material_ref::MaterialRef;
	use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};

	use super::{definition, PalmShadeCell, PalmShadeItem};
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

	pub const PALM_SHADE_STRUCTURAL_HIGH_FACTOR: f32 = 5.0;
	pub const PALM_SHADE_STRUCTURAL_MEDIUM_FACTOR: f32 = 20.0;
	pub const PALM_SHADE_STRUCTURAL_LOW_FACTOR: f32 = 30.0;

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct PalmShadeParams {
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
		resolved_placements: Option<Vec<GroveCellVariant<PalmShadeCell>>>,
	}

	impl Default for PalmShadeParams {
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

	impl PalmShadeParams {
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

		pub fn placements(&self) -> Vec<GroveCellVariant<PalmShadeCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.placements_on(&self.terrain)
		}

		/// Select placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn placements_on(
			&self,
			world: &impl crate::GroveWorldSample,
		) -> Vec<GroveCellVariant<PalmShadeCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.grove.assemble(definition()).populate(&self.extent, world)
		}

		pub fn build(&self) -> PalmShade {
			self.build_on(&self.terrain)
		}

		/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> PalmShade {
			PalmShade::from_placements(
				&self.placements_on(world),
				self.grove.noise,
				&self.extent,
				self.tree_variants,
			)
		}
	}

	#[derive(Clone)]
	enum PalmShadeKind {
		Waialea(Arc<WaialeaPalm>),
		Date(Arc<DatePalm>),
	}

	#[derive(Clone)]
	pub struct PalmShadePlant {
		pub placement: Placement,
		kind: PalmShadeKind,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	}

	#[derive(Clone, Component)]
	pub struct PalmShade {
		pub plants: Arc<[PalmShadePlant]>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl PalmShade {
		pub fn from_placements(
			placements: &[GroveCellVariant<PalmShadeCell>],
			grove_noise: NoiseParams,
			extent: &GroveExtent,
			tree_variants: u32,
		) -> Self {
			let plants: Arc<[PalmShadePlant]> = placements
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
					PalmShadeKind::Waialea(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					PalmShadeKind::Date(t) => nest_flattened_plant_chunk(
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
						PalmShadeKind::Waialea(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						PalmShadeKind::Date(t) => canopy_proxy_site(t, plant.placement, material),
					}
				})
				.collect()
		}
	}

	fn grow_plant(
		placed: &GroveCellVariant<PalmShadeCell>,
		grove_noise: NoiseParams,
		tree_variants: u32,
	) -> PalmShadePlant {
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
			PalmShadeItem::WaialeaPalm(palm) => {
				let geometry = palm.build_with_noise(build_noise);
				let mut params = WaialeaPalmParams::default();
				params.geometry = geometry;
				let (unit_params, world_size) = params.into_unit_from_num(variant);
				PalmShadePlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: PalmShadeKind::Waialea(Arc::new(unit_params.build())),
					stick_material,
					ball_material,
					frond_material,
				}
			}
			PalmShadeItem::DatePalm(palm) => {
				let geometry = palm.build_with_noise(build_noise);
				let mut params = DatePalmParams::default();
				params.geometry = geometry;
				let (unit_params, world_size) = params.into_unit_from_num(variant);
				PalmShadePlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: PalmShadeKind::Date(Arc::new(unit_params.build())),
					stick_material,
					ball_material,
					frond_material,
				}
			}
		}
	}

	impl VegetationComponents for PalmShade {
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
				PALM_SHADE_STRUCTURAL_HIGH_FACTOR,
				PALM_SHADE_STRUCTURAL_MEDIUM_FACTOR,
				PALM_SHADE_STRUCTURAL_LOW_FACTOR,
			))
		}
	}

	impl LodScene for PalmShade {
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

		fn small_grove() -> PalmShade {
			PalmShadeParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(160.0, 1.0, 160.0)))
				.build()
		}

		fn plant_height(plant: &PalmShadePlant) -> f32 {
			match &plant.kind {
				PalmShadeKind::Waialea(t) => t.geometry.height(),
				PalmShadeKind::Date(t) => t.geometry.height(),
			}
		}

		fn plant_seed(plant: &PalmShadePlant) -> i32 {
			match &plant.kind {
				PalmShadeKind::Waialea(t) => t.geometry.trunk_noise.seed,
				PalmShadeKind::Date(t) => t.geometry.trunk_noise.seed,
			}
		}

		#[test]
		fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
			let grove = small_grove();
			assert!(!grove.plants.is_empty(), "expected placed palm-shade plants");

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
				anyhow::bail!("High palm-shade should wrap plant chunks");
			};
			assert_eq!(parts.len(), 1, "expected one lazy plant producer");
			let lod::SceneChunk::Lazy { remaining_primitives, remaining_weight, .. } = &parts[0]
			else {
				anyhow::bail!("High palm-shade plants should be SceneChunk::Lazy");
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
				anyhow::bail!("Low palm-shade should emit one flattened canopy collection");
			};
			assert_eq!(weight, chico_vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT);
			Ok(())
		}

		#[test]
		fn tree_variants_quantize_archetypes() -> Result<()> {
			use std::collections::HashSet;

			let mut params = PalmShadeParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(160.0, 1.0, 160.0)));
			params.tree_variants = 4;
			let grove = params.build();
			assert!(!grove.plants.is_empty(), "expected placed palm-shade plants");
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
	PalmShade, PalmShadeParams, PalmShadePlant, PALM_SHADE_STRUCTURAL_HIGH_FACTOR,
	PALM_SHADE_STRUCTURAL_LOW_FACTOR, PALM_SHADE_STRUCTURAL_MEDIUM_FACTOR,
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
		let dist = PalmShadeCell::distribution();
		assert_eq!(dist.len(), 5);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 10.7);
		assert_eq!(dist.buckets[1].item, Some(PalmShadeCell::TowerWaialeaPalm));
		assert_eq!(dist.buckets[1].weight, 0.8);
		assert_eq!(dist.buckets[2].item, Some(PalmShadeCell::LowerWaialeaPalm));
		assert_eq!(dist.buckets[2].weight, 0.8);
		assert_eq!(dist.buckets[3].item, Some(PalmShadeCell::ShadeDatePalm));
		assert_eq!(dist.buckets[3].weight, 1.0);
		assert_eq!(dist.buckets[4].item, Some(PalmShadeCell::ClusterDatePalm));
		assert_eq!(dist.buckets[4].weight, 0.6);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = PalmShadeCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.08..=0.25).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let PalmShadeItem::WaialeaPalm(tower) = PalmShadeCell::TowerWaialeaPalm.item() else {
			anyhow::bail!("expected tower waialea item");
		};
		assert_eq!(tower.height, UnitRange::new(20.0, 40.0));
		assert_eq!(tower.crown_density, MODERATE_CANOPY_DENSITY);

		let PalmShadeItem::WaialeaPalm(lower) = PalmShadeCell::LowerWaialeaPalm.item() else {
			anyhow::bail!("expected lower waialea item");
		};
		assert_eq!(lower.height, UnitRange::new(8.0, 20.0));
		assert_eq!(lower.crown_density, DENSE_CANOPY_DENSITY);

		let PalmShadeItem::DatePalm(shade) = PalmShadeCell::ShadeDatePalm.item() else {
			anyhow::bail!("expected shade date palm item");
		};
		assert_eq!(shade.height, UnitRange::new(6.0, 20.0));
		assert_eq!(shade.crown_density, MODERATE_CANOPY_DENSITY);

		let PalmShadeItem::DatePalm(cluster) = PalmShadeCell::ClusterDatePalm.item() else {
			anyhow::bail!("expected cluster date palm item");
		};
		assert_eq!(cluster.height, UnitRange::new(6.0, 12.0));
		assert_eq!(cluster.crown_density, DENSE_CANOPY_DENSITY);
		Ok(())
	}

	#[test]
	fn placement_constraints_match_rfc() -> Result<()> {
		let dist = PalmShadeCell::distribution();
		let tower = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(PalmShadeCell::TowerWaialeaPalm))
			.ok_or_else(|| anyhow::anyhow!("missing tower waialea bucket"))?;
		assert_eq!(tower.constraints.elevation.end, 0.46);
		assert_eq!(tower.constraints.steepness.end, 0.56);

		let lower = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(PalmShadeCell::LowerWaialeaPalm))
			.ok_or_else(|| anyhow::anyhow!("missing lower waialea bucket"))?;
		assert_eq!(lower.constraints.elevation.end, 0.50);
		assert_eq!(lower.constraints.steepness.end, 0.62);

		let shade = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(PalmShadeCell::ShadeDatePalm))
			.ok_or_else(|| anyhow::anyhow!("missing shade date palm bucket"))?;
		assert_eq!(shade.constraints.elevation.end, 0.52);
		assert_eq!(shade.constraints.steepness.end, 0.42);

		let cluster = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(PalmShadeCell::ClusterDatePalm))
			.ok_or_else(|| anyhow::anyhow!("missing cluster date palm bucket"))?;
		assert_eq!(cluster.constraints.elevation.end, 0.44);
		assert_eq!(cluster.constraints.steepness.end, 0.36);
		Ok(())
	}

	#[test]
	fn steep_slope_rejects_cluster_date_but_allows_shade_date() -> Result<()> {
		let prepared =
			PalmShadeCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let terrain = FlatTerrainSample { elevation: 0.30, steepness: 0.38 };
		let shade_outcome = prepared.select_from(
			3,
			Vec3::new(5.0, 0.30, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match shade_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, PalmShadeCell::ShadeDatePalm);
			}
			other => anyhow::bail!("expected ShadeDatePalm on moderate slope, got {other:?}"),
		}
		let cluster_outcome = prepared.select_from(
			5,
			Vec3::new(5.0, 0.30, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match cluster_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_ne!(variant, PalmShadeCell::ClusterDatePalm);
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			PalmShadeCell::TowerWaialeaPalm,
			PalmShadeCell::LowerWaialeaPalm,
			PalmShadeCell::ShadeDatePalm,
			PalmShadeCell::ClusterDatePalm,
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
