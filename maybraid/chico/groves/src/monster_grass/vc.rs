use super::{
	MONSTER_GRASS_STRUCTURAL_HIGH_FACTOR, MONSTER_GRASS_STRUCTURAL_LOW_FACTOR,
	MONSTER_GRASS_STRUCTURAL_MEDIUM_FACTOR,
};
use std::sync::Arc;

use bevy::prelude::*;
use chico_sbs_trees::{QuantizedPlant, TuftPatch};
use chico_vegetation_components::{
	FoliageNode, FrondCollection, FrondRun, Layers, Placement, StickNode, StructuralLod,
	VegetationComponents,
};
use clap::Args;
use lod::gen::LodSceneLevel;
use material_ref::MaterialRef;
use procedural_common::{noise_params_from_scalar_str, NoiseParams};

use super::{
	definition, MonsterGrassCell, BROAD_JUNGLE_BLADE, BROAD_JUNGLE_BLADE_PATCH, GIANT_WET_BLADE,
	GIANT_WET_BLADE_PATCH, PALE_GIANT_REED, PALE_GIANT_REED_PATCH, RED_RIBBED_BLADE,
	RED_RIBBED_BLADE_PATCH,
};
use crate::grove::{
	remixed_tuft_plant,
	vc_tuft::{
		grow_placed_tuft_params, horizontal_grid_proxy_placements, surface_normal_at,
		surface_samples_from_plants, upright_proxy_run,
	},
	FlatTerrainSample, GroveCellVariant, GroveExtent, GrovePreviewParams,
};

/// Authoring / CLI parameters for Monster Grass.
#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct MonsterGrassParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<MonsterGrassCell>,

	#[arg(
		long,
		default_value = "0,1,0.20,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
		help_heading = "Foliage Surface Noise",
	)]
	pub foliage_noise: NoiseParams,

	/// Cap foliage LOD collections after growing placements into square XZ bins
	/// (`ceil(sqrt(n))` on a side; `0` = one collection per placement).
	#[arg(long, default_value_t = 0)]
	pub merge_collections: usize,

	/// Number of unit-scale tuft-patch archetypes (`unit_from_num(0..n)`). Caps unique
	/// merged-mesh handles for High/Medium.
	#[arg(long, default_value_t = 100)]
	pub patch_variants: u32,
}

impl Default for MonsterGrassParams {
	fn default() -> Self {
		Self {
			preview: GrovePreviewParams::default().with_terrain(FlatTerrainSample::default()),
			foliage_noise: NoiseParams::from_scalar(0.0, 1.0, 0.20, 1),
			merge_collections: 0,
			patch_variants: 100,
		}
	}
}

crate::impl_grove_preview_params!(MonsterGrassParams, MonsterGrassCell);

impl MonsterGrassParams {
	// preview accessors via impl_grove_preview_params!
	pub fn with_resolved_placements(
		resolved_placements: Vec<GroveCellVariant<MonsterGrassCell>>,
		terrain: FlatTerrainSample,
		foliage_noise: NoiseParams,
	) -> Self {
		Self {
			preview: GrovePreviewParams::default()
				.with_terrain(terrain)
				.with_resolved_placements(resolved_placements),
			foliage_noise,
			merge_collections: 0,
			patch_variants: 100,
		}
	}

	pub fn build(&self) -> MonsterGrass {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> MonsterGrass {
		MonsterGrass::from_placements(
			&self.placements_on(world),
			self.foliage_noise,
			&self.extent,
			self.merge_collections,
			self.patch_variants,
		)
	}
}

fn default_foliage() -> NoiseParams {
	NoiseParams::from_scalar(0.0, 1.0, 0.20, 1)
}

remixed_tuft_plant!(GiantWetBlade, GIANT_WET_BLADE, default_foliage());
remixed_tuft_plant!(BroadJungleBlade, BROAD_JUNGLE_BLADE, default_foliage());
remixed_tuft_plant!(PaleGiantReed, PALE_GIANT_REED, default_foliage());
remixed_tuft_plant!(RedRibbedBlade, RED_RIBBED_BLADE, default_foliage());
remixed_tuft_plant!(GiantWetBladePatch, GIANT_WET_BLADE_PATCH, default_foliage());
remixed_tuft_plant!(BroadJungleBladePatch, BROAD_JUNGLE_BLADE_PATCH, default_foliage());
remixed_tuft_plant!(PaleGiantReedPatch, PALE_GIANT_REED_PATCH, default_foliage());
remixed_tuft_plant!(RedRibbedBladePatch, RED_RIBBED_BLADE_PATCH, default_foliage());

/// One grove-local [`TuftPatch`] collection (placement already baked when merged).
#[derive(Clone, Debug)]
pub struct MonsterGrassPlant {
	pub placement: Placement,
	pub patch: Arc<TuftPatch>,
	/// Chico frond material with one palette-picked color.
	pub material: MaterialRef,
}

/// Keep every Nth plant for Medium (¼ density).
const MEDIUM_TUFT_STRIDE: usize = 4;

const PROXY_HEIGHT_LOW: f32 = 4.5;
/// Carpet float height along the surface normal; kept small — this is not blade length.
const PROXY_HEIGHT_ULTRA: f32 = 0.6;
const ULTRA_GRID: u32 = 2;
/// Square bin side in placement-cell units so area ≈ 8 cells (`√8 × √8` = `2√2`).
const LOW_CELL_STRIDE: f32 = 2.0 * std::f32::consts::SQRT_2;

/// Built Monster Grass grove: composed [`TuftPatch`] plants for VegetationComponents.
#[derive(Clone, Debug, Component)]
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
		let plants: Vec<MonsterGrassPlant> = grow_placed_tuft_params(
			placements,
			foliage_noise,
			merge_collections,
			patch_variants,
			extent,
			|cell, variant| {
				let mix = cell.palette_mix();
				let (patch, world_size) = match cell {
					MonsterGrassCell::GiantWetBlade => GiantWetBlade::grow_num(variant),
					MonsterGrassCell::BroadJungleBlade => BroadJungleBlade::grow_num(variant),
					MonsterGrassCell::PaleGiantReed => PaleGiantReed::grow_num(variant),
					MonsterGrassCell::RedRibbedBlade => RedRibbedBlade::grow_num(variant),
					MonsterGrassCell::GiantWetBladePatch => GiantWetBladePatch::grow_num(variant),
					MonsterGrassCell::BroadJungleBladePatch => {
						BroadJungleBladePatch::grow_num(variant)
					}
					MonsterGrassCell::PaleGiantReedPatch => PaleGiantReedPatch::grow_num(variant),
					MonsterGrassCell::RedRibbedBladePatch => RedRibbedBladePatch::grow_num(variant),
				};
				(patch, world_size, mix)
			},
		)
		.into_iter()
		.map(|plant| MonsterGrassPlant {
			placement: plant.placement,
			patch: plant.patch,
			material: plant.material,
		})
		.collect();
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
		let material = plant.material.clone();
		plant
			.patch
			.foliage_nodes_for_level(level)
			.flatten()
			.into_iter()
			.map(move |mut node| {
				node.placement = plant.placement.compose_child(node.placement);
				node.with_material(material.clone())
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
		let samples = surface_samples_from_plants(
			self.plants.iter().map(|p| (&p.placement, p.patch.as_ref())),
		);

		for plant in &self.plants {
			let patch = &plant.patch;
			let width = clump_proxy_width(patch);
			for anchor in &patch.anchors {
				let world = plant.placement.compose_child(Placement::new(*anchor, 0.0)).translation;
				let ix = ((world.x - origin.x) / bin_x).floor() as i32;
				let iz = ((world.z - origin.z) / bin_z).floor() as i32;
				let entry = bins.entry((ix, iz)).or_insert((Vec3::ZERO, 0.0, 0));
				entry.0 += world;
				entry.1 += width;
				entry.2 = entry.2.saturating_add(1);
			}
		}

		let material = self.plants.first().map(|p| p.material.clone()).unwrap_or_default();
		let mut runs = Vec::with_capacity(bins.len());
		let normal_eps = bin_x.max(bin_z) * 0.5;
		for ((ix, iz), (sum_pos, sum_width, count)) in bins {
			let n = (count as f32).max(1.0);
			let mean = sum_pos / n;
			// Cover the bin footprint while preserving blended blade width.
			let width = (sum_width / n).max(bin_x.max(bin_z) * 0.5) * n.sqrt();
			let cx = origin.x + (ix as f32 + 0.5) * bin_x;
			let cz = origin.z + (iz as f32 + 0.5) * bin_z;
			// Prefer bin center on XZ; keep plant mean Y so proxies sit on terrain.
			let base_xz = Vec3::new(cx, 0.0, cz).lerp(Vec3::new(mean.x, 0.0, mean.z), 0.35);
			let base = Vec3::new(base_xz.x, mean.y, base_xz.z);
			let up = surface_normal_at(&samples, base.x, base.z, normal_eps);
			if let Some(run) = upright_proxy_run(base, up, width, height) {
				runs.push(run);
			}
		}
		collection_nodes(runs, self.structural_center, self.footprint_radius, material)
	}

	/// Four slope-aligned carpet segments covering a 2×2 subdivision of the grove extent.
	///
	/// Emitted as separate frond nodes (not one [`FrondCollection`]): collection
	/// Low/UltraLow merge rebuilds via [`Placement::frond_segment`], which maps a
	/// large “width” onto world up and turns carpets into walls.
	fn foliage_ultra_low(&self) -> Vec<FoliageNode> {
		let material = self.plants.first().map(|p| p.material.clone()).unwrap_or_default();
		let samples = surface_samples_from_plants(
			self.plants.iter().map(|p| (&p.placement, p.patch.as_ref())),
		);
		horizontal_grid_proxy_placements(&self.extent, ULTRA_GRID, PROXY_HEIGHT_ULTRA, &samples)
			.into_iter()
			.map(|placement| {
				FoliageNode::straight_frond_segment(placement).with_material(material.clone())
			})
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

fn collection_nodes(
	runs: Vec<FrondRun>,
	center: Vec3,
	radius: f32,
	material: MaterialRef,
) -> Vec<FoliageNode> {
	if runs.is_empty() {
		return Vec::new();
	}
	vec![FoliageNode::frond_collection(
		FrondCollection::new(runs).with_probe(center, radius),
		Placement::IDENTITY,
	)
	.with_material(material)]
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
			LodSceneLevel::UltraLow | LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_) => {
				self.foliage_ultra_low()
			}
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

crate::impl_tuft_grove_lod!(MonsterGrass);
