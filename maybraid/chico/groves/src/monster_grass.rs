//! Monster Grass — well-known oversized understory blade grove
//! ([RFC-183 §3.4.5.2](../../../../rfc/rfc-000-000-183-chico-vegetation/03-04-cellular-groves/05-well-known-understory-groves/02-monster-grass/README.md),
//! [#308](https://github.com/ramate-io/maybraid/issues/308)).
//!
//! Dense 2–6 m understory blades for jungle, swamp, and elder-tree floors — structurally
//! Braid Grass at monster scale. Authored cells resolve to [`GroveTuftPatch`] (single-clump
//! cells use `clump_count = 1`). Under `render`, [`MonsterGrassParams::build`] grows
//! [`TuftPatch`](chico_sbs_trees::TuftPatch) plants, then
//! [`TuftPatch::merge_placed`](chico_sbs_trees::TuftPatch::merge_placed) folds them down to
//! [`MonsterGrassParams::merge_collections`] (default 100) so foliage LOD probes stay bounded.
//! Leaf materials are not applied yet; [`MonsterGrassCell::palette_mix`] keeps the authored
//! color ranges.

use std::ops::RangeInclusive;

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, GroveTuftPatch,
	PaletteMix, PaletteSlot, PlacementConstraints,
};

/// Authored Monster Grass grove definition.
///
/// Cell footprint is denser than the RFC's nominal `4.0..9.0` grid (like Braid Grass) so preview
/// groves read as continuous tall understory rather than sparse screens. The offset range is
/// signed and ± one cell so placements break the underlying grid instead of clustering near
/// cell centers.
pub fn definition() -> GroveDefinition<MonsterGrassCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(2.5),
		placement: GrovePlacementRanges::new(UnitRange::new(0.85, 1.15), UnitRange::new(-2.5, 2.5)),
		distribution: MonsterGrassCell::distribution(),
	}
}

/// Ordered monster-grass varietals ([RFC-183 §3.4.5.2]); the explicit `None` bucket lives only
/// in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonsterGrassCell {
	GiantWetBlade,
	BroadJungleBlade,
	PaleGiantReed,
	RedRibbedBlade,
	GiantWetBladePatch,
	BroadJungleBladePatch,
	PaleGiantReedPatch,
	RedRibbedBladePatch,
}

/// Authored geometry ranges for one monster-grass blade clump.
#[derive(Debug, Clone, PartialEq)]
pub struct MonsterGrassClump {
	pub height: UnitRange,
	/// Blade width as a **fraction of blade length**. The RFC's absolute widths describe the
	/// clump footprint, not blade thickness.
	pub width_factor: UnitRange,
	pub blade_count: RangeInclusive<u32>,
	pub bend_segments: RangeInclusive<u32>,
	/// RFC `droop` — splay/sag departure from vertical on upward-biased blade tufts.
	pub max_tilt_radians: UnitRange,
}

/// Shared blade thickness band: ~2.5–4.5 % of blade length — broader than Braid Grass for the
/// heavy, wall-like read at 2–6 m.
const BLADE_WIDTH_FACTOR: UnitRange = UnitRange::new(0.025, 0.045);
/// Match default [`chico_sbs_trees::TuftPatch`] kink budget (1–3 segments, not a tall polyline).
const BEND_SEGMENTS: RangeInclusive<u32> = 1..=3;
const SINGLE: RangeInclusive<u32> = 1..=1;
const NO_EXTENT: UnitRange = UnitRange::new(0.0, 0.0);
const NO_SPREAD: UnitRange = UnitRange::new(0.0, 0.0);

const GIANT_WET_BLADE_CLUMP: MonsterGrassClump = MonsterGrassClump {
	height: UnitRange::new(2.00, 6.00),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: 10..=28,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: UnitRange::new(0.25, 0.70),
};

const BROAD_JUNGLE_BLADE_CLUMP: MonsterGrassClump = MonsterGrassClump {
	height: UnitRange::new(2.50, 5.00),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: 8..=24,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: UnitRange::new(0.35, 0.85),
};

const PALE_GIANT_REED_CLUMP: MonsterGrassClump = MonsterGrassClump {
	height: UnitRange::new(2.00, 4.50),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: 8..=22,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: UnitRange::new(0.15, 0.50),
};

const RED_RIBBED_BLADE_CLUMP: MonsterGrassClump = MonsterGrassClump {
	height: UnitRange::new(2.20, 4.20),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: 10..=24,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: UnitRange::new(0.20, 0.65),
};

const GIANT_WET_BLADE: GroveTuftPatch<MonsterGrassClump> = GroveTuftPatch {
	clump: GIANT_WET_BLADE_CLUMP,
	clump_count: SINGLE,
	patch_extent_xz: NO_EXTENT,
	base_spread: NO_SPREAD,
};

const BROAD_JUNGLE_BLADE: GroveTuftPatch<MonsterGrassClump> = GroveTuftPatch {
	clump: BROAD_JUNGLE_BLADE_CLUMP,
	clump_count: SINGLE,
	patch_extent_xz: NO_EXTENT,
	base_spread: NO_SPREAD,
};

const PALE_GIANT_REED: GroveTuftPatch<MonsterGrassClump> = GroveTuftPatch {
	clump: PALE_GIANT_REED_CLUMP,
	clump_count: SINGLE,
	patch_extent_xz: NO_EXTENT,
	base_spread: NO_SPREAD,
};

const RED_RIBBED_BLADE: GroveTuftPatch<MonsterGrassClump> = GroveTuftPatch {
	clump: RED_RIBBED_BLADE_CLUMP,
	clump_count: SINGLE,
	patch_extent_xz: NO_EXTENT,
	base_spread: NO_SPREAD,
};

const GIANT_WET_BLADE_PATCH: GroveTuftPatch<MonsterGrassClump> = GroveTuftPatch {
	clump: GIANT_WET_BLADE_CLUMP,
	clump_count: 3..=5,
	patch_extent_xz: UnitRange::new(1.8, 4.4),
	base_spread: UnitRange::new(0.25, 0.50),
};

const BROAD_JUNGLE_BLADE_PATCH: GroveTuftPatch<MonsterGrassClump> = GroveTuftPatch {
	clump: BROAD_JUNGLE_BLADE_CLUMP,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(1.6, 4.8),
	base_spread: UnitRange::new(0.30, 0.55),
};

const PALE_GIANT_REED_PATCH: GroveTuftPatch<MonsterGrassClump> = GroveTuftPatch {
	clump: PALE_GIANT_REED_CLUMP,
	clump_count: 3..=5,
	patch_extent_xz: UnitRange::new(2.0, 2.8),
	base_spread: UnitRange::new(0.20, 0.45),
};

const RED_RIBBED_BLADE_PATCH: GroveTuftPatch<MonsterGrassClump> = GroveTuftPatch {
	clump: RED_RIBBED_BLADE_CLUMP,
	clump_count: 2..=5,
	patch_extent_xz: UnitRange::new(1.8, 4.4),
	base_spread: UnitRange::new(0.25, 0.50),
};

impl MonsterGrassCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `4.6` (RFC relative proportions); the `None` weight of `1.5` puts
	/// the placed share at `4.6 / 6.1 ≈ 0.75`. Patches carry `3.68` of the placed weight;
	/// single-anchor clumps share the remaining `0.92`.
	pub fn distribution() -> GroveDistribution<Self> {
		let low_wet =
			PlacementConstraints::new(UnitRange::new(0.0, 0.75), UnitRange::new(0.0, 0.50));
		let red_ribbed =
			PlacementConstraints::new(UnitRange::new(0.0, 0.75), UnitRange::new(0.0, 0.60));
		GroveDistribution::new(vec![
			GroveBucket::none(1.5),
			GroveBucket::placed(0.40, low_wet, Self::GiantWetBlade),
			GroveBucket::placed(0.30, low_wet, Self::BroadJungleBlade),
			GroveBucket::placed(0.15, low_wet, Self::PaleGiantReed),
			GroveBucket::placed(0.07, red_ribbed, Self::RedRibbedBlade),
			GroveBucket::placed(1.60, low_wet, Self::GiantWetBladePatch),
			GroveBucket::placed(1.20, low_wet, Self::BroadJungleBladePatch),
			GroveBucket::placed(0.60, low_wet, Self::PaleGiantReedPatch),
			GroveBucket::placed(0.28, red_ribbed, Self::RedRibbedBladePatch),
		])
	}

	/// Authored tuft-patch layout for this varietal (single-clump cells use `clump_count = 1`).
	pub fn patch(self) -> &'static GroveTuftPatch<MonsterGrassClump> {
		match self {
			Self::GiantWetBlade => &GIANT_WET_BLADE,
			Self::BroadJungleBlade => &BROAD_JUNGLE_BLADE,
			Self::PaleGiantReed => &PALE_GIANT_REED,
			Self::RedRibbedBlade => &RED_RIBBED_BLADE,
			Self::GiantWetBladePatch => &GIANT_WET_BLADE_PATCH,
			Self::BroadJungleBladePatch => &BROAD_JUNGLE_BLADE_PATCH,
			Self::PaleGiantReedPatch => &PALE_GIANT_REED_PATCH,
			Self::RedRibbedBladePatch => &RED_RIBBED_BLADE_PATCH,
		}
	}

	/// Authored palette ranges for this varietal.
	///
	/// Not applied while VegetationComponents presentation uses procedural frond kits; kept as
	/// the reference for restoring leaf materials / `WithPalette` later.
	pub fn palette_mix(self) -> PaletteMix {
		const GIANT_WET_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("deep_green", "wet_green"),
			PaletteSlot::new("blue_green", "dark_green"),
			PaletteSlot::new("emerald_green", "fresh_green"),
		]);
		const BROAD_JUNGLE_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("lush_green", "bright_green"),
			PaletteSlot::new("wet_green", "lime_green"),
			PaletteSlot::new("dark_green", "blue_green"),
		]);
		const PALE_GIANT_REED_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("yellow_green", "pale_straw"),
			PaletteSlot::new("dry_green", "tan_green"),
			PaletteSlot::new("light_green", "fresh_green"),
		]);
		const RED_RIBBED_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("dark_red", "deep_green"),
			PaletteSlot::new("copper_red", "wet_green"),
			PaletteSlot::new("red_green", "blue_green"),
		]);
		match self {
			Self::GiantWetBlade | Self::GiantWetBladePatch => GIANT_WET_MIX,
			Self::BroadJungleBlade | Self::BroadJungleBladePatch => BROAD_JUNGLE_MIX,
			Self::PaleGiantReed | Self::PaleGiantReedPatch => PALE_GIANT_REED_MIX,
			Self::RedRibbedBlade | Self::RedRibbedBladePatch => RED_RIBBED_MIX,
		}
	}
}

#[cfg(feature = "render")]
mod vc {
	use bevy::prelude::*;
	use chico_sbs_trees::TuftPatch;
	use chico_vegetation_components::{
		FoliageNode, Layers, Placement, StickNode, VegetationComponents, VegetationStructuralLodProbe,
	};
	use clap::Args;
	use lod::gen::LodSceneLevel;
	use procedural_common::{noise_params_from_scalar_str, NoiseParams};

	use super::{definition, MonsterGrassCell};
	use crate::grove::{
		placement_noise, FlatTerrainSample, GroveCellVariant, GroveExtent, GroveFrontend,
		DEFAULT_GROVE_EXTENT_XZ,
	};

	/// Authoring / CLI parameters for Monster Grass.
	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct MonsterGrassParams {
		#[command(flatten, next_help_heading = "Grove")]
		pub grove: GroveFrontend,

		#[arg(
			long,
			default_value = "0,1,0.20,1",
			value_parser = noise_params_from_scalar_str,
			value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
			help_heading = "Foliage Surface Noise",
		)]
		pub foliage_noise: NoiseParams,

		#[arg(skip)]
		pub extent: GroveExtent,

		#[command(flatten, next_help_heading = "Terrain")]
		pub terrain: FlatTerrainSample,

		/// Cap foliage LOD collections / probes after growing placements (merge nearby patches).
		#[arg(long, default_value_t = 100)]
		pub merge_collections: usize,

		#[arg(skip)]
		resolved_placements: Option<Vec<GroveCellVariant<MonsterGrassCell>>>,
	}

	impl Default for MonsterGrassParams {
		fn default() -> Self {
			Self {
				grove: GroveFrontend::default(),
				foliage_noise: NoiseParams::from_scalar(0.0, 1.0, 0.20, 1),
				extent: GroveExtent::new(
					Vec3::ZERO,
					Vec3::new(DEFAULT_GROVE_EXTENT_XZ, 1.0, DEFAULT_GROVE_EXTENT_XZ),
				),
				terrain: FlatTerrainSample::default(),
				merge_collections: 100,
				resolved_placements: None,
			}
		}
	}

	impl MonsterGrassParams {
		/// Render precomputed placements instead of selecting live from the grove frontend.
		pub fn with_resolved_placements(
			resolved_placements: Vec<GroveCellVariant<MonsterGrassCell>>,
			terrain: FlatTerrainSample,
			foliage_noise: NoiseParams,
		) -> Self {
			Self {
				grove: GroveFrontend::default(),
				foliage_noise,
				extent: GroveExtent::new(
					Vec3::ZERO,
					Vec3::new(DEFAULT_GROVE_EXTENT_XZ, 1.0, DEFAULT_GROVE_EXTENT_XZ),
				),
				terrain,
				merge_collections: 100,
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

		/// Effective vegetation cell footprint (frontend override or authored).
		pub fn cell_extent_xz(&self) -> Vec2 {
			self.grove.definition(definition()).cell_extent_xz
		}

		pub fn placement_cells(&self) -> Vec<gimme_gen::Cell> {
			self.extent.subdivide_xz(self.cell_extent_xz())
		}

		pub fn placements(&self) -> Vec<GroveCellVariant<MonsterGrassCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.grove.assemble(definition()).populate(&self.extent, &self.terrain)
		}

		/// Grow placements into the VegetationComponents grove.
		pub fn build(&self) -> MonsterGrass {
			MonsterGrass::from_placements(
				&self.placements(),
				self.foliage_noise,
				&self.extent,
				self.merge_collections,
			)
		}
	}

	/// One grove-local [`TuftPatch`] collection (placement already baked when merged).
	#[derive(Clone, Debug)]
	pub struct MonsterGrassPlant {
		pub placement: Placement,
		pub patch: TuftPatch,
	}

	/// Built Monster Grass grove: composed [`TuftPatch`] plants for VegetationComponents.
	#[derive(Clone, Debug)]
	pub struct MonsterGrass {
		pub plants: Vec<MonsterGrassPlant>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
	}

	impl MonsterGrass {
		/// Grow every placement into a [`TuftPatch`], then merge down to `merge_collections`.
		pub fn from_placements(
			placements: &[GroveCellVariant<MonsterGrassCell>],
			foliage_noise: NoiseParams,
			extent: &GroveExtent,
			merge_collections: usize,
		) -> Self {
			let grown = placements.iter().map(|placed| {
				let noise = placement_noise(foliage_noise, placed.position);
				let mut params = placed.variant.patch().build_tuft_patch(noise);
				params.shape.noise_amplitude = foliage_noise.amplitude;
				params.shape.noise_frequency = foliage_noise.frequency;
				(
					Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat(placed.scale.max(1e-4))),
					params.build(),
				)
			});
			let plants = TuftPatch::merge_placed(grown, merge_collections)
				.into_iter()
				.map(|patch| MonsterGrassPlant {
					placement: Placement::IDENTITY,
					patch,
				})
				.collect();
			let span = extent.max() - extent.min();
			let half = span * 0.5;
			let footprint_radius = half.x.max(half.z).max(1.0);
			Self {
				plants,
				structural_center: extent.min() + Vec3::new(half.x, half.y.max(1.0), half.z),
				footprint_radius,
			}
		}
	}

	impl VegetationComponents for MonsterGrass {
		fn stick_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StickNode> {
			Layers::new()
		}

		fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
			let mut nodes = Vec::new();
			for plant in &self.plants {
				for mut node in plant.patch.foliage_nodes_for_level(level).flatten() {
					node.placement = plant.placement.compose_child(node.placement);
					nodes.push(node);
				}
			}
			Layers::from_free(nodes)
		}

		fn structural_lod_probe(&self) -> Option<VegetationStructuralLodProbe> {
			Some(VegetationStructuralLodProbe::new(
				self.structural_center,
				self.footprint_radius,
			))
		}
	}
}

#[cfg(feature = "render")]
pub use vc::{MonsterGrass, MonsterGrassParams, MonsterGrassPlant};

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
		let dist = MonsterGrassCell::distribution();
		assert_eq!(dist.len(), 9);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 1.5);
		assert_eq!(dist.buckets[1].item, Some(MonsterGrassCell::GiantWetBlade));
		assert_eq!(dist.buckets[1].weight, 0.40);
		assert_eq!(dist.buckets[5].item, Some(MonsterGrassCell::GiantWetBladePatch));
		assert_eq!(dist.buckets[5].weight, 1.60);
		assert_eq!(dist.buckets[8].item, Some(MonsterGrassCell::RedRibbedBladePatch));
		assert_eq!(dist.buckets[8].weight, 0.28);
		Ok(())
	}

	#[test]
	fn placed_share_matches_dense_understory_target() -> Result<()> {
		let dist = MonsterGrassCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!(
			(0.70..=0.80).contains(&share),
			"placed share {share} outside dense understory band (~75 %)"
		);
		Ok(())
	}

	#[test]
	fn patches_outweigh_single_clumps() -> Result<()> {
		let placed_weight = |multi: bool| -> f32 {
			MonsterGrassCell::distribution()
				.buckets
				.iter()
				.filter(|b| {
					b.item.is_some_and(|cell| {
						let patch = cell.patch();
						(*patch.clump_count.end() > 1) == multi
					})
				})
				.map(|b| b.weight)
				.sum()
		};
		assert!(
			placed_weight(true) > 2.0 * placed_weight(false),
			"multi-clump patches should dominate placed weight"
		);
		Ok(())
	}

	#[test]
	fn palette_mix_keeps_authored_color_slots() -> Result<()> {
		for cell in [
			MonsterGrassCell::GiantWetBlade,
			MonsterGrassCell::BroadJungleBlade,
			MonsterGrassCell::PaleGiantReed,
			MonsterGrassCell::RedRibbedBlade,
			MonsterGrassCell::GiantWetBladePatch,
		] {
			let palette = cell.palette_mix();
			assert!(!palette.slots.is_empty(), "expected palette slots for {cell:?}");
			for slot in palette.slots {
				assert!(!slot.start.0.is_empty(), "empty start token for {cell:?}");
				assert!(!slot.end.0.is_empty(), "empty end token for {cell:?}");
			}
		}
		Ok(())
	}

	#[test]
	fn bend_segments_match_tuft_patch_budget() -> Result<()> {
		for cell in [
			MonsterGrassCell::GiantWetBlade,
			MonsterGrassCell::BroadJungleBlade,
			MonsterGrassCell::PaleGiantReed,
			MonsterGrassCell::RedRibbedBlade,
			MonsterGrassCell::GiantWetBladePatch,
		] {
			let segs = &cell.patch().clump.bend_segments;
			assert!(*segs.start() >= 1);
			assert!(*segs.end() <= 3, "{cell:?} bend_segments {segs:?} exceeds 1..=3");
		}
		Ok(())
	}

	#[test]
	fn single_cells_are_one_clump_patches() -> Result<()> {
		for cell in [
			MonsterGrassCell::GiantWetBlade,
			MonsterGrassCell::BroadJungleBlade,
			MonsterGrassCell::PaleGiantReed,
			MonsterGrassCell::RedRibbedBlade,
		] {
			let patch = cell.patch();
			assert_eq!(*patch.clump_count.start(), 1);
			assert_eq!(*patch.clump_count.end(), 1);
			assert!(patch.clump.height.start >= 2.0);
			assert!(patch.clump.height.end <= 6.0);
		}
		Ok(())
	}

	#[test]
	fn patch_wraps_giant_wet_blade_clump() -> Result<()> {
		let patch = MonsterGrassCell::GiantWetBladePatch.patch();
		assert_eq!(patch.clump, GIANT_WET_BLADE_CLUMP);
		assert!(*patch.clump_count.start() >= 3);
		assert!(patch.patch_extent_xz.start >= 1.2);
		Ok(())
	}

	#[test]
	fn constraint_first_fit_fallback() -> Result<()> {
		let prepared =
			MonsterGrassCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.55 };
		let outcome = prepared.select_from(
			3,
			Vec3::new(5.0, 0.35, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, MonsterGrassCell::RedRibbedBlade);
			}
			other => anyhow::bail!("expected RedRibbedBlade fallback, got {other:?}"),
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

	#[cfg(feature = "render")]
	mod render_tests {
		use super::*;
		use crate::grove::placement_noise;
		use crate::monster_grass::MonsterGrassParams;

		#[test]
		fn clump_geometry_builds_within_authored_ranges() -> Result<()> {
			let noise = placement_noise(NoiseParams::default(), Vec3::new(5.0, 0.0, 5.0));
			for cell in [
				MonsterGrassCell::GiantWetBlade,
				MonsterGrassCell::BroadJungleBlade,
				MonsterGrassCell::PaleGiantReed,
				MonsterGrassCell::RedRibbedBlade,
			] {
				let patch = cell.patch();
				let clump = &patch.clump;
				let item = patch.build_tuft_patch(noise);
				assert_eq!(item.clump_count, 1);
				assert!(item.shape.blade_length >= clump.height.start.min(clump.height.end));
				assert!(item.shape.blade_length <= clump.height.start.max(clump.height.end));
				assert!(clump.bend_segments.contains(&item.shape.bend_segments));
				assert!(item.shape.bend_segments <= 3);
			}
			Ok(())
		}

		#[test]
		fn build_composes_tuft_patches() -> Result<()> {
			use crate::grove::GroveCellVariant;

			let placement = GroveCellVariant::new(
				MonsterGrassCell::GiantWetBlade,
				Vec3::new(1.0, 0.0, 2.0),
				1.0,
			);
			let grove = MonsterGrassParams::with_resolved_placements(
				vec![placement],
				FlatTerrainSample::default(),
				NoiseParams::default(),
			)
			.build();
			assert_eq!(grove.plants.len(), 1);
			assert_eq!(grove.plants[0].patch.clump_count, 1);
			// Placement is baked into frond runs when merging.
			let base = grove.plants[0].patch.frond_runs()[0].segments[0]
				.placement
				.translation;
			assert!((base.x - 1.0).abs() < 0.5 && (base.z - 2.0).abs() < 0.5);
			Ok(())
		}

		#[test]
		fn build_merges_down_to_collection_cap() -> Result<()> {
			use crate::grove::GroveCellVariant;

			let placements: Vec<_> = (0..40)
				.map(|i| {
					GroveCellVariant::new(
						MonsterGrassCell::GiantWetBlade,
						Vec3::new((i % 8) as f32 * 3.0, 0.0, (i / 8) as f32 * 3.0),
						1.0,
					)
				})
				.collect();
			let mut params = MonsterGrassParams::with_resolved_placements(
				placements,
				FlatTerrainSample::default(),
				NoiseParams::default(),
			);
			params.merge_collections = 5;
			let grove = params.build();
			assert_eq!(grove.plants.len(), 5);
			Ok(())
		}

		#[test]
		fn palette_resolves_to_authored_color() -> Result<()> {
			use bevy::prelude::StandardMaterial;
			use crate::grove::WithPalette;

			for cell in [
				MonsterGrassCell::GiantWetBlade,
				MonsterGrassCell::BroadJungleBlade,
				MonsterGrassCell::PaleGiantReed,
				MonsterGrassCell::RedRibbedBlade,
				MonsterGrassCell::GiantWetBladePatch,
			] {
				let palette = cell.palette_mix();
				let mut allowed = Vec::new();
				for slot in palette.slots {
					allowed.extend(slot.start.resolve());
					allowed.extend(slot.end.resolve());
				}
				assert!(!allowed.is_empty(), "unresolved palette tokens for {cell:?}");
				let material =
					StandardMaterial::with_palette(StandardMaterial::default(), palette, 7);
				assert!(allowed.contains(&material.base_color));
			}
			Ok(())
		}
	}
}
