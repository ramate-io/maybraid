//! Arid Conifer Sapling — well-known low-density dry young conifer lower-canopy grove
//! ([RFC-183 §3.4.6.6], [#327](https://github.com/ramate-io/maybraid/issues/327)).
//!
//! Sparse Friend's, Northern, and rare Liam's Conifer saplings on dry exposed terrain. Forest-layer
//! attachment remains a follow-up.

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

/// Standard arid sapling height band ([`2.0`, `4.0`] m).
const ARID_SAPLING_HEIGHT: UnitRange = UnitRange::new(2.0, 4.0);
/// Sparse sampled canopy-density band ([`0.0`, `0.35`]).
const SPARSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.04, 0.15);
/// Ultra-sparse sampled canopy-density band ([`0.0`, `0.15`]).
const ULTRA_SPARSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.01, 0.18);

/// Authored Arid Conifer Sapling grove definition.
///
/// Cell footprint at the RFC midpoint (`13.5` m). The offset range is signed and ± one cell so
/// placements break the underlying grid.
pub fn definition() -> GroveDefinition<AridConiferSaplingCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(13.5),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-13.5, 13.5),
		),
		distribution: AridConiferSaplingCell::distribution(),
	}
}

/// Ordered arid-conifer-sapling varietals ([RFC-183 §3.4.6.6]); the explicit `None` bucket lives
/// only in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AridConiferSaplingCell {
	DryFriendSapling,
	DryNorthernSapling,
	WispyDryFriendSapling,
	WispyDryNorthernSapling,
	BareDryFriendSapling,
	BareDryNorthernSapling,
	DryLiamsConiferSapling,
}

/// Typed authored geometry for one arid-conifer-sapling varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AridConiferSaplingItem {
	FriendsConifer(&'static AridConiferSaplingFriendsConifer),
	NorthernConifer(&'static AridConiferSaplingNorthernConifer),
	LiamsConifer(&'static AridConiferSaplingLiamsConifer),
}

/// Authored geometry ranges for one dry Friend's Conifer sapling form.
#[derive(Debug, Clone, PartialEq)]
pub struct AridConiferSaplingFriendsConifer {
	pub height: UnitRange,
	/// World-space stalk base radius (RFC `0.025 × H`).
	pub stalk_radius: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse canopy density at render time.
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one dry Northern Conifer sapling form.
#[derive(Debug, Clone, PartialEq)]
pub struct AridConiferSaplingNorthernConifer {
	pub height: UnitRange,
	/// World-space stalk base radius (Northern `0.032 × H`).
	pub stalk_radius: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse canopy density at render time.
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one dry Liam's Conifer sapling form.
#[derive(Debug, Clone, PartialEq)]
pub struct AridConiferSaplingLiamsConifer {
	pub height: UnitRange,
	/// World-space stalk base radius (RFC `0.025 × H`).
	pub stalk_radius: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse tuft density at render time.
	pub canopy_density: UnitRange,
}

const DRY_FRIEND_SAPLING: AridConiferSaplingFriendsConifer = AridConiferSaplingFriendsConifer {
	height: ARID_SAPLING_HEIGHT,
	stalk_radius: UnitRange::new(0.05, 0.10),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const DRY_NORTHERN_SAPLING: AridConiferSaplingNorthernConifer = AridConiferSaplingNorthernConifer {
	height: ARID_SAPLING_HEIGHT,
	stalk_radius: UnitRange::new(0.064, 0.128),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const WISPY_DRY_FRIEND_SAPLING: AridConiferSaplingFriendsConifer =
	AridConiferSaplingFriendsConifer {
		height: ARID_SAPLING_HEIGHT,
		stalk_radius: UnitRange::new(0.05, 0.10),
		canopy_density: ULTRA_SPARSE_CANOPY_DENSITY,
	};

const WISPY_DRY_NORTHERN_SAPLING: AridConiferSaplingNorthernConifer =
	AridConiferSaplingNorthernConifer {
		height: ARID_SAPLING_HEIGHT,
		stalk_radius: UnitRange::new(0.064, 0.128),
		canopy_density: ULTRA_SPARSE_CANOPY_DENSITY,
	};

const BARE_DRY_FRIEND_SAPLING: AridConiferSaplingFriendsConifer =
	AridConiferSaplingFriendsConifer {
		height: ARID_SAPLING_HEIGHT,
		stalk_radius: UnitRange::new(0.05, 0.09),
		canopy_density: ULTRA_SPARSE_CANOPY_DENSITY,
	};

const BARE_DRY_NORTHERN_SAPLING: AridConiferSaplingNorthernConifer =
	AridConiferSaplingNorthernConifer {
		height: ARID_SAPLING_HEIGHT,
		stalk_radius: UnitRange::new(0.064, 0.115),
		canopy_density: ULTRA_SPARSE_CANOPY_DENSITY,
	};

const DRY_LIAMS_SAPLING: AridConiferSaplingLiamsConifer = AridConiferSaplingLiamsConifer {
	height: ARID_SAPLING_HEIGHT,
	stalk_radius: UnitRange::new(0.05, 0.10),
	canopy_density: ULTRA_SPARSE_CANOPY_DENSITY,
};

const DRY_FRIEND_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_conifer_bark", "tan_bark"),
	PaletteSlot::new("gray_brown", "sun_baked_bark"),
]);

const DRY_FRIEND_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("sage_green", "dusty_green"),
	PaletteSlot::new("deep_green", "olive_green"),
]);

const DRY_NORTHERN_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_gray_bark", "dark_bark"),
	PaletteSlot::new("tan_bark", "conifer_bark"),
]);

const DRY_NORTHERN_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("blue_sage", "dusty_green"),
	PaletteSlot::new("dark_green", "olive_green"),
]);

const WISPY_DRY_FRIEND_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("sun_baked_bark", "dry_conifer_bark"),
	PaletteSlot::new("gray_brown", "tan_bark"),
]);

const WISPY_DRY_FRIEND_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dusty_green", "sage_green"),
	PaletteSlot::new("olive_green", "deep_green"),
]);

const WISPY_DRY_NORTHERN_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_gray_bark", "sun_baked_bark"),
	PaletteSlot::new("conifer_bark", "tan_bark"),
]);

const WISPY_DRY_NORTHERN_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("blue_sage", "sage_green"),
	PaletteSlot::new("olive_green", "dusty_green"),
]);

const BARE_DRY_FRIEND_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("tan_bark", "sun_baked_bark"),
	PaletteSlot::new("dry_conifer_bark", "gray_brown"),
]);

const BARE_DRY_FRIEND_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("sage_green", "olive_green"),
	PaletteSlot::new("dusty_green", "blue_sage"),
]);

const BARE_DRY_NORTHERN_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("conifer_bark", "dry_gray_bark"),
	PaletteSlot::new("sun_baked_bark", "gray_brown"),
]);

const BARE_DRY_NORTHERN_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dusty_green", "blue_sage"),
	PaletteSlot::new("dark_green", "olive_green"),
]);

const DRY_LIAMS_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_gray_bark", "sun_baked_bark"),
	PaletteSlot::new("tan_bark", "dry_conifer_bark"),
]);

const DRY_LIAMS_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("blue_sage", "sage_green"),
	PaletteSlot::new("dusty_green", "olive_green"),
]);

impl AridConiferSaplingCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `4.7` (two sparse pair, four ultra-sparse accents, rare Liam's); the
	/// `None` weight of `24.0` puts the placed share at `4.7 / 28.7 ≈ 0.16`, mid RFC
	/// `DENSITY_RANGE` (`0.08..0.24`).
	/// Placement constraints are unconstrained until RFC elevation bands land ([#327](https://github.com/ramate-io/maybraid/issues/327)).
	pub fn distribution() -> GroveDistribution<Self> {
		GroveDistribution::new(vec![
			GroveBucket::none(24.0),
			GroveBucket::placed(0.5, PlacementConstraints::UNCONSTRAINED, Self::DryFriendSapling),
			GroveBucket::placed(0.5, PlacementConstraints::UNCONSTRAINED, Self::DryNorthernSapling),
			GroveBucket::placed(
				1.0,
				PlacementConstraints::UNCONSTRAINED,
				Self::WispyDryFriendSapling,
			),
			GroveBucket::placed(
				1.0,
				PlacementConstraints::UNCONSTRAINED,
				Self::WispyDryNorthernSapling,
			),
			GroveBucket::placed(
				0.75,
				PlacementConstraints::UNCONSTRAINED,
				Self::BareDryFriendSapling,
			),
			GroveBucket::placed(
				0.75,
				PlacementConstraints::UNCONSTRAINED,
				Self::BareDryNorthernSapling,
			),
			GroveBucket::placed(
				0.2,
				PlacementConstraints::UNCONSTRAINED,
				Self::DryLiamsConiferSapling,
			),
		])
	}

	pub fn item(self) -> AridConiferSaplingItem {
		match self {
			Self::DryFriendSapling => AridConiferSaplingItem::FriendsConifer(&DRY_FRIEND_SAPLING),
			Self::WispyDryFriendSapling => {
				AridConiferSaplingItem::FriendsConifer(&WISPY_DRY_FRIEND_SAPLING)
			}
			Self::BareDryFriendSapling => {
				AridConiferSaplingItem::FriendsConifer(&BARE_DRY_FRIEND_SAPLING)
			}
			Self::DryNorthernSapling => {
				AridConiferSaplingItem::NorthernConifer(&DRY_NORTHERN_SAPLING)
			}
			Self::WispyDryNorthernSapling => {
				AridConiferSaplingItem::NorthernConifer(&WISPY_DRY_NORTHERN_SAPLING)
			}
			Self::BareDryNorthernSapling => {
				AridConiferSaplingItem::NorthernConifer(&BARE_DRY_NORTHERN_SAPLING)
			}
			Self::DryLiamsConiferSapling => {
				AridConiferSaplingItem::LiamsConifer(&DRY_LIAMS_SAPLING)
			}
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::DryFriendSapling => DRY_FRIEND_SAPLING_STICK_MIX,
			Self::WispyDryFriendSapling => WISPY_DRY_FRIEND_SAPLING_STICK_MIX,
			Self::BareDryFriendSapling => BARE_DRY_FRIEND_SAPLING_STICK_MIX,
			Self::DryNorthernSapling => DRY_NORTHERN_SAPLING_STICK_MIX,
			Self::WispyDryNorthernSapling => WISPY_DRY_NORTHERN_SAPLING_STICK_MIX,
			Self::BareDryNorthernSapling => BARE_DRY_NORTHERN_SAPLING_STICK_MIX,
			Self::DryLiamsConiferSapling => DRY_LIAMS_SAPLING_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::DryFriendSapling => DRY_FRIEND_SAPLING_CANOPY_MIX,
			Self::WispyDryFriendSapling => WISPY_DRY_FRIEND_SAPLING_CANOPY_MIX,
			Self::BareDryFriendSapling => BARE_DRY_FRIEND_SAPLING_CANOPY_MIX,
			Self::DryNorthernSapling => DRY_NORTHERN_SAPLING_CANOPY_MIX,
			Self::WispyDryNorthernSapling => WISPY_DRY_NORTHERN_SAPLING_CANOPY_MIX,
			Self::BareDryNorthernSapling => BARE_DRY_NORTHERN_SAPLING_CANOPY_MIX,
			Self::DryLiamsConiferSapling => DRY_LIAMS_SAPLING_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
mod vc {
	use std::sync::Arc;

	use crate::grove::FlatTerrainSample;
	use bevy::math::bounding::Aabb3d;
	use bevy::prelude::*;
	use bevy::scene::prelude::Scene;
	use chico_sbs_trees::{
		FriendsConifer, FriendsConiferParams, LiamsConifer, LiamsConiferParams, NorthernConifer,
		NorthernConiferParams,
	};
	use chico_vegetation_components::{
		FoliageNode, Layers, Placement, StickNode, StructuralLod, VegetationComponents,
	};
	use clap::Args;
	use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
	use lod::lod_ref::LodRef;
	use lod::{lod_host_scene_pending, SceneChunk};
	use material_ref::MaterialRef;
	use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};

	use super::{definition, AridConiferSaplingCell, AridConiferSaplingItem};
	use crate::grove::vc_tuft::{patch_variant_index, variant_noise};
	use crate::grove::{
		canopy_ball_material_from_palette, canopy_proxy_site, foliage_low_canopy_balls,
		foliage_ultra_low_merged_balls, frond_material_from_palette, grove_detail_level,
		grove_lod_culls, grove_lod_level, grove_lod_status, grove_structural_footprint,
		layers_from_nodes, nest_flattened_plant_chunk, placement_noise,
		stick_material_from_palette, woody_grove_scene_chunks, CanopyProxySite, GroveCellVariant,
		GroveExtent, GroveFrontend, DEFAULT_GROVE_EXTENT_XZ, ULTRA_LOW_CANOPY_BIN_METERS,
	};

	pub const ARID_CONIFER_SAPLING_STRUCTURAL_HIGH_FACTOR: f32 = 2.0;
	pub const ARID_CONIFER_SAPLING_STRUCTURAL_MEDIUM_FACTOR: f32 = 5.0;
	pub const ARID_CONIFER_SAPLING_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct AridConiferSaplingParams {
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
		resolved_placements: Option<Vec<GroveCellVariant<AridConiferSaplingCell>>>,
	}

	impl Default for AridConiferSaplingParams {
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

	impl AridConiferSaplingParams {
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

		pub fn placements(&self) -> Vec<GroveCellVariant<AridConiferSaplingCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.placements_on(&self.terrain)
		}

		/// Select placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn placements_on(
			&self,
			world: &impl crate::GroveWorldSample,
		) -> Vec<GroveCellVariant<AridConiferSaplingCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.grove.assemble(definition()).populate(&self.extent, world)
		}

		pub fn build(&self) -> AridConiferSapling {
			self.build_on(&self.terrain)
		}

		/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> AridConiferSapling {
			AridConiferSapling::from_placements(
				&self.placements_on(world),
				self.grove.noise,
				&self.extent,
				self.tree_variants,
			)
		}
	}

	#[derive(Clone)]
	enum AridConiferSaplingKind {
		Friends(Arc<FriendsConifer>),
		Northern(Arc<NorthernConifer>),
		Liams(Arc<LiamsConifer>),
	}

	#[derive(Clone)]
	pub struct AridConiferSaplingPlant {
		pub placement: Placement,
		kind: AridConiferSaplingKind,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	}

	#[derive(Clone, Component)]
	pub struct AridConiferSapling {
		pub plants: Arc<[AridConiferSaplingPlant]>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl AridConiferSapling {
		pub fn from_placements(
			placements: &[GroveCellVariant<AridConiferSaplingCell>],
			grove_noise: NoiseParams,
			extent: &GroveExtent,
			tree_variants: u32,
		) -> Self {
			let plants: Arc<[AridConiferSaplingPlant]> = placements
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
					AridConiferSaplingKind::Friends(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					AridConiferSaplingKind::Northern(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					AridConiferSaplingKind::Liams(t) => nest_flattened_plant_chunk(
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
						AridConiferSaplingKind::Friends(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						AridConiferSaplingKind::Northern(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						AridConiferSaplingKind::Liams(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
					}
				})
				.collect()
		}
	}

	fn grow_plant(
		placed: &GroveCellVariant<AridConiferSaplingCell>,
		grove_noise: NoiseParams,
		tree_variants: u32,
	) -> AridConiferSaplingPlant {
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
			AridConiferSaplingItem::FriendsConifer(conifer) => {
				let geometry = conifer.build_with_noise(build_noise);
				let mut params = FriendsConiferParams::default();
				params.geometry = geometry;
				let (unit_params, world_size) = params.into_unit_from_num(variant);
				AridConiferSaplingPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: AridConiferSaplingKind::Friends(Arc::new(unit_params.build())),
					stick_material,
					ball_material,
					frond_material,
				}
			}
			AridConiferSaplingItem::NorthernConifer(conifer) => {
				let geometry = conifer.build_with_noise(build_noise);
				let mut params = NorthernConiferParams::default();
				params.geometry = geometry;
				let (unit_params, world_size) = params.into_unit_from_num(variant);
				AridConiferSaplingPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: AridConiferSaplingKind::Northern(Arc::new(unit_params.build())),
					stick_material,
					ball_material,
					frond_material,
				}
			}
			AridConiferSaplingItem::LiamsConifer(conifer) => {
				let geometry = conifer.build_with_noise(build_noise);
				let mut params = LiamsConiferParams::default();
				params.geometry = geometry;
				let (unit_params, world_size) = params.into_unit_from_num(variant);
				AridConiferSaplingPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: AridConiferSaplingKind::Liams(Arc::new(unit_params.build())),
					stick_material,
					ball_material,
					frond_material,
				}
			}
		}
	}

	impl VegetationComponents for AridConiferSapling {
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
				ARID_CONIFER_SAPLING_STRUCTURAL_HIGH_FACTOR,
				ARID_CONIFER_SAPLING_STRUCTURAL_MEDIUM_FACTOR,
				ARID_CONIFER_SAPLING_STRUCTURAL_LOW_FACTOR,
			))
		}
	}

	impl LodScene for AridConiferSapling {
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

		fn small_grove() -> AridConiferSapling {
			AridConiferSaplingParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0)))
				.build()
		}

		fn plant_height(plant: &AridConiferSaplingPlant) -> f32 {
			match &plant.kind {
				AridConiferSaplingKind::Friends(t) => t.geometry.height(),
				AridConiferSaplingKind::Northern(t) => t.geometry.height(),
				AridConiferSaplingKind::Liams(t) => t.geometry.scale.stalk_height,
			}
		}

		fn plant_seed(plant: &AridConiferSaplingPlant) -> i32 {
			match &plant.kind {
				AridConiferSaplingKind::Friends(t) => t.geometry.canopy_noise.seed,
				AridConiferSaplingKind::Northern(t) => t.geometry.liams.canopy_noise.seed,
				AridConiferSaplingKind::Liams(t) => t.geometry.canopy_noise.seed,
			}
		}

		#[test]
		fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
			let grove = small_grove();
			assert!(!grove.plants.is_empty(), "expected placed arid-conifer-sapling plants");

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
				anyhow::bail!("High arid-conifer-sapling should wrap plant chunks");
			};
			assert_eq!(parts.len(), 1, "expected one lazy plant producer");
			let lod::SceneChunk::Lazy { remaining_primitives, remaining_weight, .. } = &parts[0]
			else {
				anyhow::bail!("High arid-conifer-sapling plants should be SceneChunk::Lazy");
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
				anyhow::bail!("Low arid-conifer-sapling should emit one flattened canopy collection");
			};
			assert_eq!(weight, chico_vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT);
			Ok(())
		}

		#[test]
		fn tree_variants_quantize_archetypes() -> Result<()> {
			use std::collections::HashSet;

			let mut params = AridConiferSaplingParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0)));
			params.tree_variants = 4;
			let grove = params.build();
			assert!(!grove.plants.is_empty(), "expected placed arid-conifer-sapling plants");
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
	AridConiferSapling, AridConiferSaplingParams, AridConiferSaplingPlant,
	ARID_CONIFER_SAPLING_STRUCTURAL_HIGH_FACTOR, ARID_CONIFER_SAPLING_STRUCTURAL_LOW_FACTOR,
	ARID_CONIFER_SAPLING_STRUCTURAL_MEDIUM_FACTOR,
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
		let dist = AridConiferSaplingCell::distribution();
		assert_eq!(dist.len(), 8);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 24.0);
		assert_eq!(dist.buckets[1].item, Some(AridConiferSaplingCell::DryFriendSapling));
		assert_eq!(dist.buckets[1].weight, 0.5);
		assert_eq!(dist.buckets[2].item, Some(AridConiferSaplingCell::DryNorthernSapling));
		assert_eq!(dist.buckets[2].weight, 0.5);
		assert_eq!(dist.buckets[3].item, Some(AridConiferSaplingCell::WispyDryFriendSapling));
		assert_eq!(dist.buckets[3].weight, 1.0);
		assert_eq!(dist.buckets[4].item, Some(AridConiferSaplingCell::WispyDryNorthernSapling));
		assert_eq!(dist.buckets[4].weight, 1.0);
		assert_eq!(dist.buckets[5].item, Some(AridConiferSaplingCell::BareDryFriendSapling));
		assert_eq!(dist.buckets[5].weight, 0.75);
		assert_eq!(dist.buckets[6].item, Some(AridConiferSaplingCell::BareDryNorthernSapling));
		assert_eq!(dist.buckets[6].weight, 0.75);
		assert_eq!(dist.buckets[7].item, Some(AridConiferSaplingCell::DryLiamsConiferSapling));
		assert_eq!(dist.buckets[7].weight, 0.2);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = AridConiferSaplingCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.08..=0.24).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let AridConiferSaplingItem::FriendsConifer(friend) =
			AridConiferSaplingCell::DryFriendSapling.item()
		else {
			anyhow::bail!("expected dry friend sapling item");
		};
		assert_eq!(friend.canopy_density, SPARSE_CANOPY_DENSITY);

		let AridConiferSaplingItem::FriendsConifer(wispy) =
			AridConiferSaplingCell::WispyDryFriendSapling.item()
		else {
			anyhow::bail!("expected wispy dry friend sapling item");
		};
		assert_eq!(wispy.canopy_density, ULTRA_SPARSE_CANOPY_DENSITY);

		let AridConiferSaplingItem::LiamsConifer(liams) =
			AridConiferSaplingCell::DryLiamsConiferSapling.item()
		else {
			anyhow::bail!("expected dry liams sapling item");
		};
		assert_eq!(liams.height, ARID_SAPLING_HEIGHT);
		assert_eq!(liams.canopy_density, ULTRA_SPARSE_CANOPY_DENSITY);
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
