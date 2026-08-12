//! Strange Oasis — well-known sparse oasis lower-canopy grove
//! ([RFC-183 §3.4.6.2], [#323](https://github.com/ramate-io/maybraid/issues/323)).
//!
//! Compact date palms with rare Penmarch torch and Storybook accents in wet desert pockets.
//! Forest-layer attachment remains a follow-up.

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

/// Sparse sampled canopy-density band ([`0.0`, `0.35`]).
const SPARSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.0, 0.35);
/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);
/// Sparse..moderate sampled canopy-density band.
const SPARSE_TO_MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.0, 0.65);

/// Authored Strange Oasis grove definition.
///
/// Cell footprint sits at the RFC midpoint (`12.0` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<StrangeOasisCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(8.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-12.0, 12.0),
		),
		distribution: StrangeOasisCell::distribution(),
	}
}

/// Ordered strange-oasis varietals ([RFC-183 §3.4.6.2]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrangeOasisCell {
	CompactDatePalm,
	TorchAccent,
	RedTorchAccent,
	OasisStorybook,
}

/// Typed authored geometry for one strange-oasis varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StrangeOasisItem {
	DatePalm(&'static StrangeOasisDatePalm),
	Torch(&'static StrangeOasisTorch),
	Storybook(&'static StrangeOasisStorybook),
}

/// Authored geometry ranges for one compact Date Palm form.
#[derive(Debug, Clone, PartialEq)]
pub struct StrangeOasisDatePalm {
	pub height: UnitRange,
	pub crown_density: UnitRange,
}

/// Authored geometry ranges for one Penmarch Torch accent (standard or red-stick palette).
#[derive(Debug, Clone, PartialEq)]
pub struct StrangeOasisTorch {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one oasis Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct StrangeOasisStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const COMPACT_DATE_PALM: StrangeOasisDatePalm = StrangeOasisDatePalm {
	height: UnitRange::new(3.0, 5.0),
	crown_density: MODERATE_CANOPY_DENSITY,
};

const TORCH_ACCENT: StrangeOasisTorch = StrangeOasisTorch {
	height: UnitRange::new(3.0, 7.0),
	stalk_radius: UnitRange::new(0.12, 0.24),
	canopy_spread: UnitRange::new(1.2, 3.5),
	canopy_density: SPARSE_TO_MODERATE_CANOPY_DENSITY,
};

const RED_TORCH_ACCENT: StrangeOasisTorch = StrangeOasisTorch {
	height: UnitRange::new(3.0, 6.5),
	stalk_radius: UnitRange::new(0.12, 0.22),
	canopy_spread: UnitRange::new(1.2, 3.2),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const OASIS_STORYBOOK: StrangeOasisStorybook = StrangeOasisStorybook {
	height: UnitRange::new(4.0, 6.0),
	stalk_radius: UnitRange::new(0.20, 0.32),
	canopy_spread: UnitRange::new(1.6, 3.6),
	canopy_density: SPARSE_TO_MODERATE_CANOPY_DENSITY,
};

const DATE_PALM_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_bark", "tan_bark"),
	PaletteSlot::new("dry_brown", "gray_brown"),
]);

const DATE_PALM_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("yellow_green", "date_green"),
]);

const TORCH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_bark", "ornamental_bark"),
	PaletteSlot::new("gray_brown", "tan_brown"),
]);

const TORCH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "olive_green"),
	PaletteSlot::new("flower_yellow", "fresh_green"),
]);

const RED_TORCH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("desert_red_bark", "copper_red"),
	PaletteSlot::new("orange_bark", "dark_bark"),
]);

const RED_TORCH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "fresh_green"),
	PaletteSlot::new("flower_yellow", "light_green"),
]);

const STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_bark", "brown_bark"),
	PaletteSlot::new("gray_brown", "tan_brown"),
]);

const STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("green", "light_green"),
	PaletteSlot::new("olive_green", "fresh_green"),
]);

impl StrangeOasisCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `3.23` (RFC relative proportions); the `None` weight of `14.0` puts
	/// the placed share at `3.23 / 17.23 ≈ 0.19`, mid RFC `DENSITY_RANGE` (`0.08..0.24`).
	pub fn distribution() -> GroveDistribution<Self> {
		let date_palm =
			PlacementConstraints::new(UnitRange::new(0.0, 0.38), UnitRange::new(0.0, 0.28));
		let torch = PlacementConstraints::new(UnitRange::new(0.0, 0.45), UnitRange::new(0.0, 0.34));
		let red_torch =
			PlacementConstraints::new(UnitRange::new(0.0, 0.40), UnitRange::new(0.0, 0.40));
		let storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 0.42), UnitRange::new(0.0, 0.32));
		GroveDistribution::new(vec![
			GroveBucket::none(10.0),
			GroveBucket::placed(2.0, date_palm, Self::CompactDatePalm),
			GroveBucket::placed(0.30, torch, Self::TorchAccent),
			GroveBucket::placed(0.18, red_torch, Self::RedTorchAccent),
			GroveBucket::placed(0.75, storybook, Self::OasisStorybook),
		])
	}

	pub fn item(self) -> StrangeOasisItem {
		match self {
			Self::CompactDatePalm => StrangeOasisItem::DatePalm(&COMPACT_DATE_PALM),
			Self::TorchAccent => StrangeOasisItem::Torch(&TORCH_ACCENT),
			Self::RedTorchAccent => StrangeOasisItem::Torch(&RED_TORCH_ACCENT),
			Self::OasisStorybook => StrangeOasisItem::Storybook(&OASIS_STORYBOOK),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::CompactDatePalm => DATE_PALM_STICK_MIX,
			Self::TorchAccent => TORCH_STICK_MIX,
			Self::RedTorchAccent => RED_TORCH_STICK_MIX,
			Self::OasisStorybook => STORYBOOK_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::CompactDatePalm => DATE_PALM_CANOPY_MIX,
			Self::TorchAccent => TORCH_CANOPY_MIX,
			Self::RedTorchAccent => RED_TORCH_CANOPY_MIX,
			Self::OasisStorybook => STORYBOOK_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
mod vc {
	use bevy::prelude::*;
	use chico_sbs_geometry::DatePalmSbs;
	use chico_sbs_trees::{
		DatePalm, DatePalmParams, PalmCrown, PalmCrownParams, PenmarchTorch, PenmarchTorchParams,
		StorybookTree, StorybookTreeParams,
	};
	use chico_vegetation_components::{
		FoliageNode, Layers, Placement, StickNode, StructuralLod, VegetationComponents,
	};
	use clap::Args;
	use lod::gen::LodSceneLevel;
	use material_ref::MaterialRef;
	use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};

	use super::{definition, StrangeOasisCell, StrangeOasisItem};
	use crate::grove::{
		canopy_ball_material_from_palette, canopy_proxy_site, canopy_proxy_site_nested,
		flatten_foliage_nodes, flatten_foliage_nodes_nested, flatten_stick_nodes,
		foliage_low_canopy_balls, foliage_ultra_low_merged_balls, frond_material_from_palette,
		grove_detail_level, grove_structural_footprint, layers_from_nodes, placement_noise,
		stick_material_from_palette, CanopyProxySite, FlatTerrainSample, GroveCellVariant,
		GroveExtent, GroveFrontend, DEFAULT_GROVE_EXTENT_XZ, ULTRA_LOW_CANOPY_BIN_METERS,
	};

	pub const STRANGE_OASIS_STRUCTURAL_HIGH_FACTOR: f32 = 2.0;
	pub const STRANGE_OASIS_STRUCTURAL_MEDIUM_FACTOR: f32 = 5.0;
	pub const STRANGE_OASIS_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

	/// Authoring / CLI parameters for Strange Oasis.
	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct StrangeOasisParams {
		#[command(flatten, next_help_heading = "Grove")]
		pub grove: GroveFrontend,

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
		resolved_placements: Option<Vec<GroveCellVariant<StrangeOasisCell>>>,
	}

	impl Default for StrangeOasisParams {
		fn default() -> Self {
			Self {
				grove: GroveFrontend::default(),
				leaf_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
				extent: GroveExtent::new(
					Vec3::ZERO,
					Vec3::new(DEFAULT_GROVE_EXTENT_XZ, 1.0, DEFAULT_GROVE_EXTENT_XZ),
				),
				terrain: FlatTerrainSample::default(),
				resolved_placements: None,
			}
		}
	}

	impl StrangeOasisParams {
		pub fn with_resolved_placements(
			resolved_placements: Vec<GroveCellVariant<StrangeOasisCell>>,
			terrain: FlatTerrainSample,
			leaf_surface_noise: NoiseParams,
		) -> Self {
			Self {
				grove: GroveFrontend::default(),
				leaf_surface_noise,
				extent: GroveExtent::new(
					Vec3::ZERO,
					Vec3::new(DEFAULT_GROVE_EXTENT_XZ, 1.0, DEFAULT_GROVE_EXTENT_XZ),
				),
				terrain,
				resolved_placements: Some(resolved_placements),
			}
		}

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

		pub fn placements(&self) -> Vec<GroveCellVariant<StrangeOasisCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.grove.assemble(definition()).populate(&self.extent, &self.terrain)
		}

		pub fn build(&self) -> StrangeOasis {
			StrangeOasis::from_placements(&self.placements(), self.grove.noise, &self.extent)
		}
	}

	#[derive(Clone)]
	enum StrangeOasisKind {
		/// Columnar trunk + unit PalmCrown at tip
		/// ([`PalmCrownParams::unit_full_for_height_from_num`]).
		DatePalm {
			trunk: DatePalm,
			crown: PalmCrown,
			crown_local: Placement,
		},
		Torch(PenmarchTorch),
		Storybook(StorybookTree),
	}

	#[derive(Clone)]
	pub struct StrangeOasisPlant {
		pub placement: Placement,
		kind: StrangeOasisKind,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	}

	#[derive(Clone)]
	pub struct StrangeOasis {
		pub plants: Vec<StrangeOasisPlant>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl StrangeOasis {
		pub fn from_placements(
			placements: &[GroveCellVariant<StrangeOasisCell>],
			grove_noise: NoiseParams,
			extent: &GroveExtent,
		) -> Self {
			let plants = placements
				.iter()
				.map(|placed| grow_plant(placed, grove_noise))
				.collect();
			let (structural_center, footprint_radius) = grove_structural_footprint(extent);
			Self {
				plants,
				structural_center,
				footprint_radius,
				extent: *extent,
			}
		}

		fn stick_nodes(&self, level: LodSceneLevel) -> Vec<StickNode> {
			let mut out = Vec::new();
			for plant in &self.plants {
				match &plant.kind {
					StrangeOasisKind::DatePalm { trunk, .. } => {
						out.extend(flatten_stick_nodes(
							trunk,
							plant.placement,
							&plant.stick_material,
							level,
						));
					}
					StrangeOasisKind::Torch(t) => {
						out.extend(flatten_stick_nodes(
							t,
							plant.placement,
							&plant.stick_material,
							level,
						));
					}
					StrangeOasisKind::Storybook(t) => {
						out.extend(flatten_stick_nodes(
							t,
							plant.placement,
							&plant.stick_material,
							level,
						));
					}
				}
			}
			out
		}

		fn foliage_nodes(&self, level: LodSceneLevel) -> Vec<FoliageNode> {
			let mut out = Vec::new();
			for plant in &self.plants {
				match &plant.kind {
					StrangeOasisKind::DatePalm { crown, crown_local, .. } => {
						out.extend(flatten_foliage_nodes_nested(
							crown,
							plant.placement,
							*crown_local,
							&plant.ball_material,
							&plant.frond_material,
							level,
						));
					}
					StrangeOasisKind::Torch(t) => {
						out.extend(flatten_foliage_nodes(
							t,
							plant.placement,
							&plant.ball_material,
							&plant.frond_material,
							level,
						));
					}
					StrangeOasisKind::Storybook(t) => {
						out.extend(flatten_foliage_nodes(
							t,
							plant.placement,
							&plant.ball_material,
							&plant.frond_material,
							level,
						));
					}
				}
			}
			out
		}

		fn canopy_sites(&self) -> Vec<CanopyProxySite> {
			self.plants
				.iter()
				.filter_map(|plant| {
					let material = &plant.ball_material;
					match &plant.kind {
						StrangeOasisKind::DatePalm { crown, crown_local, .. } => {
							canopy_proxy_site_nested(
								crown,
								plant.placement,
								*crown_local,
								material,
							)
						}
						StrangeOasisKind::Torch(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						StrangeOasisKind::Storybook(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
					}
				})
				.collect()
		}
	}

	fn grow_plant(
		placed: &GroveCellVariant<StrangeOasisCell>,
		grove_noise: NoiseParams,
	) -> StrangeOasisPlant {
		let build_noise = placement_noise(grove_noise, placed.position);
		let stick_seed = build_noise.seed;
		let canopy_seed = build_noise.seed.wrapping_add(31);
		let stick_material =
			stick_material_from_palette(Some(placed.variant.stick_palette_mix()), stick_seed);
		let ball_material = canopy_ball_material_from_palette(
			Some(placed.variant.canopy_palette_mix()),
			canopy_seed,
		);
		let frond_material = frond_material_from_palette(
			Some(placed.variant.canopy_palette_mix()),
			canopy_seed,
		);
		let placement =
			Placement::new(placed.position, 0.0).with_scale(Vec3::splat(placed.scale.max(1e-4)));

		let kind = match placed.variant.item() {
			StrangeOasisItem::DatePalm(palm) => {
				let geometry = palm.build_with_noise(build_noise);
				let mut trunk_params = DatePalmParams::default();
				trunk_params.geometry = geometry.clone();
				let trunk = trunk_params.build();
				let tip = DatePalmSbs::trunk_tip_from_chain(&trunk.chain);
				let crown_seed = build_noise.seed.unsigned_abs();
				// Quantize topology to unit crown; Placement restores height-band meters.
				let (unit_crown, world_size) =
					PalmCrownParams::unit_full_for_height_from_num(geometry.height(), crown_seed);
				let crown = unit_crown.build();
				let crown_local =
					Placement::new(tip, 0.0).with_scale(Vec3::splat(world_size.max(1e-4)));
				StrangeOasisKind::DatePalm { trunk, crown, crown_local }
			}
			StrangeOasisItem::Torch(torch) => {
				let geometry = torch.build_with_noise(build_noise);
				let mut params = PenmarchTorchParams::default();
				params.geometry = geometry;
				StrangeOasisKind::Torch(params.build())
			}
			StrangeOasisItem::Storybook(story) => {
				let geometry = story.build_with_noise(build_noise);
				let mut params = StorybookTreeParams::default();
				params.geometry = geometry;
				StrangeOasisKind::Storybook(params.build())
			}
		};

		StrangeOasisPlant {
			placement,
			kind,
			stick_material,
			ball_material,
			frond_material,
		}
	}

	impl VegetationComponents for StrangeOasis {
		fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
			match grove_detail_level(level) {
				Some(detail) => layers_from_nodes(self.stick_nodes(detail)),
				None => Layers::new(),
			}
		}

		fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
			match level {
				LodSceneLevel::High | LodSceneLevel::Medium => {
					layers_from_nodes(self.foliage_nodes(level))
				}
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
					STRANGE_OASIS_STRUCTURAL_HIGH_FACTOR,
					STRANGE_OASIS_STRUCTURAL_MEDIUM_FACTOR,
					STRANGE_OASIS_STRUCTURAL_LOW_FACTOR,
				),
			)
		}
	}
}

#[cfg(feature = "render")]
pub use vc::{
	StrangeOasis, StrangeOasisParams, StrangeOasisPlant, STRANGE_OASIS_STRUCTURAL_HIGH_FACTOR,
	STRANGE_OASIS_STRUCTURAL_LOW_FACTOR, STRANGE_OASIS_STRUCTURAL_MEDIUM_FACTOR,
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
		let dist = StrangeOasisCell::distribution();
		assert_eq!(dist.len(), 5);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 10.0);
		assert_eq!(dist.buckets[1].item, Some(StrangeOasisCell::CompactDatePalm));
		assert_eq!(dist.buckets[1].weight, 2.0);
		assert_eq!(dist.buckets[2].item, Some(StrangeOasisCell::TorchAccent));
		assert_eq!(dist.buckets[2].weight, 0.30);
		assert_eq!(dist.buckets[3].item, Some(StrangeOasisCell::RedTorchAccent));
		assert_eq!(dist.buckets[3].weight, 0.18);
		assert_eq!(dist.buckets[4].item, Some(StrangeOasisCell::OasisStorybook));
		assert_eq!(dist.buckets[4].weight, 0.75);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = StrangeOasisCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.08..=0.25).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let StrangeOasisItem::DatePalm(palm) = StrangeOasisCell::CompactDatePalm.item() else {
			anyhow::bail!("expected date palm item");
		};
		assert_eq!(palm.height, UnitRange::new(3.0, 5.0));
		assert_eq!(palm.crown_density, MODERATE_CANOPY_DENSITY);

		let StrangeOasisItem::Storybook(story) = StrangeOasisCell::OasisStorybook.item() else {
			anyhow::bail!("expected storybook item");
		};
		assert_eq!(story.height, UnitRange::new(4.0, 6.0));

		let StrangeOasisItem::Torch(torch) = StrangeOasisCell::RedTorchAccent.item() else {
			anyhow::bail!("expected red torch item");
		};
		assert_eq!(torch.height, UnitRange::new(3.0, 6.5));
		assert_eq!(torch.canopy_density, SPARSE_CANOPY_DENSITY);
		Ok(())
	}

	#[test]
	fn red_torch_accepts_steeper_slope_than_compact_date_palm() -> Result<()> {
		let prepared =
			StrangeOasisCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.32 };
		let red_outcome = prepared.select_from(
			3,
			Vec3::new(5.0, 0.25, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match red_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, StrangeOasisCell::RedTorchAccent);
			}
			other => anyhow::bail!("expected RedTorchAccent on moderate slope, got {other:?}"),
		}
		let palm_outcome = prepared.select_from(
			1,
			Vec3::new(5.0, 0.25, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match palm_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_ne!(variant, StrangeOasisCell::CompactDatePalm);
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
		}
		Ok(())
	}

	#[test]
	fn high_elevation_rejects_oasis_floor_variants() -> Result<()> {
		let prepared =
			StrangeOasisCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let terrain = FlatTerrainSample { elevation: 0.45, steepness: 0.15 };
		let outcome = prepared.select_from(
			1,
			Vec3::new(5.0, 0.45, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_ne!(variant, StrangeOasisCell::CompactDatePalm);
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
		}
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
