//! Monster Grass — well-known oversized understory blade grove
//! ([RFC-183 §3.4.5.2](../../../../rfc/rfc-000-000-183-chico-vegetation/03-04-cellular-groves/05-well-known-understory-groves/02-monster-grass/README.md),
//! [#308](https://github.com/ramate-io/maybraid/issues/308)).
//!
//! Dense 2–6 m understory blades for jungle, swamp, and elder-tree floors — structurally
//! Braid Grass at monster scale. Authored cells resolve to [`GroveTuftPatch`] (single-clump
//! cells use `clump_count = 1`). Under `render`, [`MonsterGrassParams::build`] grows
//! [`TuftPatch`](chico_sbs_trees::TuftPatch) plants quantized through
//! [`TuftPatchParams::into_unit_from_num`](chico_sbs_trees::TuftPatchParams::into_unit_from_num)
//! (`patch_variants`, default `100`) so High/Medium share archetypal MultiSceneMerge meshes.
//! Optional
//! [`TuftPatch::merge_placed`](chico_sbs_trees::TuftPatch::merge_placed) fold via
//! [`MonsterGrassParams::merge_collections`] (`0` = one collection per placement).
//!
//! Structural LOD (× grove footprint): High (full clumps); Medium = ~¼ of High tufts
//! (same geometry, thinned); Low ≈ one upright proxy per ~8 cells; UltraLow = 2×2 carpets.
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
		FoliageNode, FrondCollection, FrondRun, Layers, Placement, StickNode, StructuralLod,
		VegetationComponents, FROND_KIT_HALF_X,
	};
	use clap::Args;
	use lod::gen::LodSceneLevel;
	use procedural_common::{noise_params_from_scalar_str, NoiseParams};

	use super::{definition, MonsterGrassCell};
	use crate::grove::{
		FlatTerrainSample, GroveCellVariant, GroveExtent, GroveFrontend, DEFAULT_GROVE_EXTENT_XZ,
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

		/// Cap foliage LOD collections after growing placements (`0` = no fold, one per placement).
		#[arg(long, default_value_t = 0)]
		pub merge_collections: usize,

		/// Number of unit-scale tuft-patch archetypes (`unit_from_num(0..n)`). Caps unique
		/// merged-mesh handles for High/Medium.
		#[arg(long, default_value_t = 100)]
		pub patch_variants: u32,

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
				merge_collections: 0,
				patch_variants: 100,
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
				merge_collections: 0,
				patch_variants: 100,
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
				self.patch_variants,
			)
		}
	}

	/// Stable archetype index in `0..variants` from world XZ.
	fn patch_variant_index(position: Vec3, variants: u32) -> u32 {
		let variants = variants.max(1);
		let h = position
			.x
			.to_bits()
			.wrapping_mul(0x9e3779b9)
			.wrapping_add(position.z.to_bits().wrapping_mul(0x85ebca77));
		h % variants
	}

	/// Noise keyed by variant id (not world position) so the same archetype rebuilds identically.
	fn variant_noise(base: NoiseParams, variant: u32) -> NoiseParams {
		NoiseParams {
			seed: base.seed ^ (variant as i32).wrapping_mul(0x45d9f3b),
			..base
		}
	}

	/// One grove-local [`TuftPatch`] collection (placement already baked when merged).
	#[derive(Clone, Debug)]
	pub struct MonsterGrassPlant {
		pub placement: Placement,
		pub patch: TuftPatch,
	}

	/// Structural High band (× footprint): full authored clumps.
	pub const MONSTER_GRASS_STRUCTURAL_HIGH_FACTOR: f32 = 2.0;
	/// Structural Medium band (× footprint): ~¼ of High tufts (same blade geometry).
	pub const MONSTER_GRASS_STRUCTURAL_MEDIUM_FACTOR: f32 = 5.0;
	/// Structural Low band (× footprint): one upright proxy per ~8 placement cells; beyond → UltraLow.
	pub const MONSTER_GRASS_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

	/// Keep every Nth plant for Medium (¼ density).
	const MEDIUM_TUFT_STRIDE: usize = 4;

	const PROXY_HEIGHT_LOW: f32 = 4.5;
	/// Carpet float height (world Y); kept small — this is not blade length.
	const PROXY_HEIGHT_ULTRA: f32 = 0.6;
	/// World-space vertical thickness for UltraLow XZ carpets (local Z → up).
	const ULTRA_CARPET_THICKNESS: f32 = 0.35;
	const ULTRA_GRID: u32 = 2;
	/// Square bin side in placement-cell units so area ≈ 8 cells (`√8 × √8` = `2√2`).
	const LOW_CELL_STRIDE: f32 = 2.0 * std::f32::consts::SQRT_2;

	/// Built Monster Grass grove: composed [`TuftPatch`] plants for VegetationComponents.
	#[derive(Clone, Debug)]
	pub struct MonsterGrass {
		pub plants: Vec<MonsterGrassPlant>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
		pub cell_extent_xz: Vec2,
	}

	impl MonsterGrass {
		/// Grow every placement into a unit [`TuftPatch`] archetype; fold when
		/// `merge_collections > 0`.
		pub fn from_placements(
			placements: &[GroveCellVariant<MonsterGrassCell>],
			foliage_noise: NoiseParams,
			extent: &GroveExtent,
			merge_collections: usize,
			patch_variants: u32,
		) -> Self {
			let variants = patch_variants.max(1);
			let grown = placements.iter().map(|placed| {
				let variant = patch_variant_index(placed.position, variants);
				let noise = variant_noise(foliage_noise, variant);
				let mut params = placed.variant.patch().build_tuft_patch(noise);
				params.shape.noise_amplitude = foliage_noise.amplitude;
				params.shape.noise_frequency = foliage_noise.frequency;
				let (unit_params, world_size) = params.into_unit_from_num(variant);
				let placement = Placement::new(placed.position, 0.0)
					.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4)));
				(placement, unit_params.build())
			});
			// Unmerged: keep plant placement (unit runs). Fold: bake placements into runs.
			let plants = if merge_collections == 0 {
				grown
					.map(|(placement, patch)| MonsterGrassPlant { placement, patch })
					.collect()
			} else {
				TuftPatch::merge_placed(grown, merge_collections)
					.into_iter()
					.map(|patch| MonsterGrassPlant { placement: Placement::IDENTITY, patch })
					.collect()
			};
			let span = extent.max() - extent.min();
			let half = span * 0.5;
			let footprint_radius = half.x.max(half.z).max(1.0);
			Self {
				plants,
				structural_center: extent.min() + Vec3::new(half.x, half.y.max(1.0), half.z),
				footprint_radius,
				extent: *extent,
				cell_extent_xz: definition().cell_extent_xz,
			}
		}

		/// Emit foliage nodes: unit-local collection geometry + plant pose on the node.
		///
		/// [`FoliageNode`] composes the plant pose for LOD probe / bounds; merge parts stay
		/// unit-local so MultiSceneMerge cache keys are shared across placements.
		fn foliage_nodes_for_plant(
			plant: &MonsterGrassPlant,
			level: LodSceneLevel,
		) -> impl Iterator<Item = FoliageNode> + '_ {
			plant.patch.foliage_nodes_for_level(level).flatten().into_iter().map(|mut node| {
				node.placement = plant.placement.compose_child(node.placement);
				node
			})
		}

		fn foliage_high(&self) -> Vec<FoliageNode> {
			self.plants
				.iter()
				.flat_map(|plant| Self::foliage_nodes_for_plant(plant, LodSceneLevel::High))
				.collect()
		}

		/// Same High tuft geometry, keeping ~¼ of plants for a denser→proxy transition.
		fn foliage_medium(&self) -> Vec<FoliageNode> {
			self.plants
				.iter()
				.enumerate()
				.filter(|(i, _)| i % MEDIUM_TUFT_STRIDE == 0)
				.flat_map(|(_, plant)| Self::foliage_nodes_for_plant(plant, LodSceneLevel::High))
				.collect()
		}

		/// One upright proxy per ~8 placement cells, blending anchors in each bin.
		fn foliage_low(&self) -> Vec<FoliageNode> {
			self.foliage_cell_proxies(LOW_CELL_STRIDE, PROXY_HEIGHT_LOW)
		}

		/// Upright proxies from occupied placement-cell bins of side `cell_stride` cells.
		fn foliage_cell_proxies(&self, cell_stride: f32, height: f32) -> Vec<FoliageNode> {
			use std::collections::HashMap;

			let bin_x = (self.cell_extent_xz.x * cell_stride).max(1e-3);
			let bin_z = (self.cell_extent_xz.y * cell_stride).max(1e-3);
			let origin = self.extent.min();
			let mut bins: HashMap<(i32, i32), (Vec3, f32, u32)> = HashMap::new();

			for plant in &self.plants {
				let patch = &plant.patch;
				let width = clump_proxy_width(patch);
				for anchor in &patch.anchors {
					let world =
						plant.placement.compose_child(Placement::new(*anchor, 0.0)).translation;
					let ix = ((world.x - origin.x) / bin_x).floor() as i32;
					let iz = ((world.z - origin.z) / bin_z).floor() as i32;
					let entry = bins.entry((ix, iz)).or_insert((Vec3::ZERO, 0.0, 0));
					entry.0 += world;
					entry.1 += width;
					entry.2 = entry.2.saturating_add(1);
				}
			}

			let mut runs = Vec::with_capacity(bins.len());
			for ((ix, iz), (sum_pos, sum_width, count)) in bins {
				let n = (count as f32).max(1.0);
				let mean = sum_pos / n;
				// Cover the bin footprint while preserving blended blade width.
				let width = (sum_width / n).max(bin_x.max(bin_z) * 0.5) * n.sqrt();
				let cx = origin.x + (ix as f32 + 0.5) * bin_x;
				let cz = origin.z + (iz as f32 + 0.5) * bin_z;
				// Prefer bin center so proxies sit on the coarse grid; pull slightly toward mass.
				let base = Vec3::new(cx, 0.0, cz).lerp(Vec3::new(mean.x, 0.0, mean.z), 0.35);
				if let Some(run) = upright_proxy_run(base, width, height) {
					runs.push(run);
				}
			}
			collection_nodes(runs, self.structural_center, self.footprint_radius)
		}

		/// Four flat XZ carpet segments covering a 2×2 subdivision of the grove extent.
		///
		/// Emitted as separate frond nodes (not one [`FrondCollection`]): collection
		/// Low/UltraLow merge rebuilds via [`Placement::frond_segment`], which maps a
		/// large “width” onto world up and turns carpets into walls.
		fn foliage_ultra_low(&self) -> Vec<FoliageNode> {
			horizontal_grid_proxy_placements(&self.extent, ULTRA_GRID, PROXY_HEIGHT_ULTRA)
				.into_iter()
				.map(FoliageNode::straight_frond_segment)
				.collect()
		}
	}

	fn clump_proxy_width(patch: &TuftPatch) -> f32 {
		let n = patch.clump_count.max(1) as f32;
		if patch.patch_extent_xz > 1e-3 {
			(patch.patch_extent_xz / n.sqrt()).max(0.5)
		} else {
			1.2
		}
	}

	fn upright_proxy_run(base: Vec3, width: f32, height: f32) -> Option<FrondRun> {
		let start = Vec3::new(base.x, 0.0, base.z);
		Placement::frond_segment(start, Vec3::Y, height, width.max(1e-3))
			.map(|p| FrondRun::from_placements([p]))
	}

	/// Flat XZ carpet tiles: rachis along +X, blade width along ±Z, thin on world up.
	///
	/// Do not use [`Placement::frond_segment`] with `dir = X` and cell-sized `width` —
	/// that path leaves kit width near world up. Also keep `scale.z` small: after this
	/// basis, local Z is vertical, so matching Z to the width scale builds walls.
	fn horizontal_grid_proxy_placements(
		extent: &GroveExtent,
		divisions: u32,
		height: f32,
	) -> Vec<Placement> {
		let divisions = divisions.max(1);
		let min = extent.min();
		let max = extent.max();
		let span = max - min;
		let cell_x = (span.x / divisions as f32).max(1e-3);
		let cell_z = (span.z / divisions as f32).max(1e-3);
		// local X → world Z (width), local Y → world X (length), local Z → world Y (thickness).
		let rotation = Quat::from_mat3(&Mat3::from_cols(Vec3::Z, Vec3::X, Vec3::Y));
		let scale_x = (cell_z * 0.5 / FROND_KIT_HALF_X).max(1e-4);
		let scale_z = (ULTRA_CARPET_THICKNESS / FROND_KIT_HALF_X).max(1e-4);
		let y = height * 0.5;
		let mut out = Vec::with_capacity((divisions * divisions) as usize);
		for ix in 0..divisions {
			for iz in 0..divisions {
				let x0 = min.x + ix as f32 * cell_x;
				let z0 = min.z + iz as f32 * cell_z;
				let cz = z0 + cell_z * 0.5;
				out.push(
					Placement::new(Vec3::new(x0, y, cz), 0.0)
						.with_rotation(rotation)
						.with_scale(Vec3::new(scale_x, cell_x, scale_z)),
				);
			}
		}
		out
	}

	fn collection_nodes(runs: Vec<FrondRun>, center: Vec3, radius: f32) -> Vec<FoliageNode> {
		if runs.is_empty() {
			return Vec::new();
		}
		vec![FoliageNode::frond_collection(
			FrondCollection::new(runs).with_probe(center, radius),
			Placement::IDENTITY,
		)]
	}

	impl VegetationComponents for MonsterGrass {
		fn stick_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StickNode> {
			Layers::new()
		}

		fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
			let nodes = match level {
				LodSceneLevel::High => self.foliage_high(),
				LodSceneLevel::Medium => self.foliage_medium(),
				LodSceneLevel::Low => self.foliage_low(),
				LodSceneLevel::UltraLow
				| LodSceneLevel::Distance(_)
				| LodSceneLevel::Resolution(_) => self.foliage_ultra_low(),
			};
			Layers::from_free(nodes)
		}

		fn structural_lod(&self) -> Option<StructuralLod> {
			Some(
				StructuralLod::new(self.structural_center, self.footprint_radius)
					.with_factors(
						MONSTER_GRASS_STRUCTURAL_HIGH_FACTOR,
						MONSTER_GRASS_STRUCTURAL_MEDIUM_FACTOR,
						MONSTER_GRASS_STRUCTURAL_LOW_FACTOR,
					)
					.with_preserve_ultra_low(true),
			)
		}
	}
}

#[cfg(feature = "render")]
pub use vc::{
	MonsterGrass, MonsterGrassParams, MonsterGrassPlant, MONSTER_GRASS_STRUCTURAL_HIGH_FACTOR,
	MONSTER_GRASS_STRUCTURAL_LOW_FACTOR, MONSTER_GRASS_STRUCTURAL_MEDIUM_FACTOR,
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
			// Unit archetypes keep runs patch-local; world pose lives on the plant placement.
			assert!(
				(grove.plants[0].placement.translation - Vec3::new(1.0, 0.0, 2.0)).length() < 1e-4
			);
			assert!(grove.plants[0].patch.patch_extent_xz <= 1.0 + 1e-4);
			let base = grove.plants[0].patch.frond_runs()[0].segments[0].placement.translation;
			assert!(
				base.x.abs() < 2.0 && base.z.abs() < 2.0,
				"unit-local blade base should stay near patch origin, got {base:?}"
			);
			Ok(())
		}

		#[test]
		fn patch_variants_quantize_archetypes() -> Result<()> {
			use crate::grove::GroveCellVariant;
			use std::collections::HashSet;

			let placements: Vec<_> = (0..40)
				.map(|i| {
					GroveCellVariant::new(
						MonsterGrassCell::GiantWetBlade,
						Vec3::new(i as f32 * 3.0, 0.0, (i % 5) as f32),
						1.0,
					)
				})
				.collect();
			let mut params = MonsterGrassParams::with_resolved_placements(
				placements,
				FlatTerrainSample::default(),
				NoiseParams::default(),
			);
			params.patch_variants = 4;
			let grove = params.build();
			let seeds: HashSet<i32> =
				grove.plants.iter().map(|p| p.patch.shape.seed).collect();
			assert!(
				seeds.len() <= 4,
				"expected ≤4 unique unit seeds, got {}",
				seeds.len()
			);
			Ok(())
		}

		#[test]
		fn build_without_fold_keeps_one_plant_per_placement() -> Result<()> {
			use crate::grove::GroveCellVariant;

			let placements: Vec<_> = (0..12)
				.map(|i| {
					GroveCellVariant::new(
						MonsterGrassCell::GiantWetBlade,
						Vec3::new(i as f32, 0.0, 0.0),
						1.0,
					)
				})
				.collect();
			let grove = MonsterGrassParams::with_resolved_placements(
				placements,
				FlatTerrainSample::default(),
				NoiseParams::default(),
			)
			.build();
			assert_eq!(grove.plants.len(), 12);
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
			use crate::grove::WithPalette;
			use bevy::prelude::StandardMaterial;

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

		#[test]
		fn structural_lod_thins_to_proxy_grids() -> Result<()> {
			use crate::grove::{GroveCellVariant, DEFAULT_GROVE_EXTENT_XZ};
			use crate::monster_grass::{
				MONSTER_GRASS_STRUCTURAL_HIGH_FACTOR, MONSTER_GRASS_STRUCTURAL_LOW_FACTOR,
				MONSTER_GRASS_STRUCTURAL_MEDIUM_FACTOR,
			};
			use chico_vegetation_components::VegetationComponents;
			use lod::gen::LodSceneLevel;

			let placements: Vec<_> = (0..8)
				.map(|i| {
					GroveCellVariant::new(
						MonsterGrassCell::GiantWetBlade,
						Vec3::new((i % 4) as f32 * 5.0, 0.0, (i / 4) as f32 * 5.0),
						1.0,
					)
				})
				.collect();
			let grove = MonsterGrassParams::with_resolved_placements(
				placements,
				FlatTerrainSample::default(),
				NoiseParams::default(),
			)
			.with_extent(GroveExtent::new(
				Vec3::ZERO,
				Vec3::new(DEFAULT_GROVE_EXTENT_XZ, 1.0, DEFAULT_GROVE_EXTENT_XZ),
			))
			.build();

			let high_n = grove.foliage_nodes_for_level(LodSceneLevel::High).len();
			let medium_n = grove.foliage_nodes_for_level(LodSceneLevel::Medium).len();
			assert!(high_n >= 1);
			// Medium keeps every 4th plant (~¼ of High tufts).
			assert_eq!(medium_n, high_n.div_ceil(4));
			assert_eq!(grove.foliage_nodes_for_level(LodSceneLevel::Low).len(), 1);
			assert_eq!(grove.foliage_nodes_for_level(LodSceneLevel::UltraLow).len(), 4);

			let low_runs = grove
				.foliage_nodes_for_level(LodSceneLevel::Low)
				.flatten()
				.first()
				.and_then(|n| n.geometry.as_frond_collection().map(|c| c.runs.len()))
				.unwrap_or(0);
			// Low bins (~√8 cells ≈ 7.1 m) merge the 5 m lattice → 3 occupied bins.
			assert_eq!(low_runs, 3);

			let probe = grove.structural_lod().expect("probe");
			assert!((probe.high_factor - MONSTER_GRASS_STRUCTURAL_HIGH_FACTOR).abs() < 1e-5);
			assert!((probe.medium_factor - MONSTER_GRASS_STRUCTURAL_MEDIUM_FACTOR).abs() < 1e-5);
			assert!((probe.low_factor - MONSTER_GRASS_STRUCTURAL_LOW_FACTOR).abs() < 1e-5);
			assert!(probe.preserve_ultra_low);
			Ok(())
		}

		#[test]
		fn medium_keeps_quarter_of_high_tufts() -> Result<()> {
			use crate::grove::GroveCellVariant;
			use chico_vegetation_components::VegetationComponents;
			use lod::gen::LodSceneLevel;

			let placements: Vec<_> = (0..16)
				.map(|i| {
					let ix = i % 4;
					let iz = i / 4;
					GroveCellVariant::new(
						MonsterGrassCell::GiantWetBlade,
						Vec3::new(ix as f32 * 2.5 + 1.25, 0.0, iz as f32 * 2.5 + 1.25),
						1.0,
					)
				})
				.collect();
			let grove = MonsterGrassParams::with_resolved_placements(
				placements,
				FlatTerrainSample::default(),
				NoiseParams::default(),
			)
			.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(10.0, 1.0, 10.0)))
			.build();

			let high_n = grove.foliage_nodes_for_level(LodSceneLevel::High).len();
			let medium_n = grove.foliage_nodes_for_level(LodSceneLevel::Medium).len();
			assert_eq!(high_n, 16);
			assert_eq!(medium_n, 4);
			Ok(())
		}
	}
}
