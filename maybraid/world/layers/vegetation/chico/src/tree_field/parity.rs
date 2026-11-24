//! Hopscotch biome-spread GPU vs legacy census on the real `ChicoGroveHost` path.

use std::collections::HashMap;
use std::time::Instant;

use bevy::prelude::*;
use bevy::shape::Aabb3d;
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneLevel, LodSceneStatus};
use vegetation_groves::GroveExtent;

use super::{owner_settings, GpuFieldKind, GroveGpuVisuals};
use crate::grove::forest_world_sample;
use crate::{
	neighbor_layers, presenting_recipes, select_layering, ChicoGroveHost, ForestExtent,
	ForestLayer, ForestSelection, LayeringKind, DEFAULT_FOREST_GROVE_TILE_XZ,
	GROVE_PRESENT_RADIUS_M,
};

const PARITY_REL: f32 = 0.20;

#[derive(Clone, Copy, Debug, Default)]
struct SizeStats {
	count: usize,
	mean: f32,
	min: f32,
	max: f32,
}

impl SizeStats {
	fn from_values(values: &[f32]) -> Self {
		if values.is_empty() {
			return Self { count: 0, mean: 0.0, min: 0.0, max: 0.0 };
		}
		let min = values.iter().copied().fold(f32::INFINITY, f32::min);
		let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
		let mean = values.iter().sum::<f32>() / values.len() as f32;
		Self { count: values.len(), mean, min, max }
	}

	fn range(self) -> f32 {
		(self.max - self.min).max(0.0)
	}

	fn rel_mean(self, other: Self) -> f32 {
		rel(self.mean, other.mean)
	}

	fn rel_range(self, other: Self) -> f32 {
		if self.count < 3 || other.count < 3 {
			return 0.0;
		}
		rel(self.range(), other.range())
	}
}

#[derive(Clone, Debug)]
struct TileParity {
	layering: LayeringKind,
	origin: Vec3,
	layers_legacy: u32,
	layers_gpu: u32,
	trees_legacy: usize,
	trees_gpu: usize,
	bushes_legacy: usize,
	bushes_gpu: usize,
	grass_legacy: usize,
	grass_gpu: usize,
	tree_h_legacy: SizeStats,
	tree_h_gpu: SizeStats,
	tree_r_gpu: SizeStats,
	bush_h_legacy: SizeStats,
	bush_h_gpu: SizeStats,
	bush_r_gpu: SizeStats,
	grass_h_legacy: SizeStats,
	grass_h_gpu: SizeStats,
	capped: bool,
}

impl TileParity {
	fn from_hosts(layering: LayeringKind, origin: Vec3, hosts: &[ChicoGroveHost]) -> Self {
		let mut layers_legacy = 0u32;
		let mut layers_gpu = 0u32;
		let mut trees_legacy = 0usize;
		let mut trees_gpu = 0usize;
		let mut bushes_legacy = 0usize;
		let mut bushes_gpu = 0usize;
		let mut grass_legacy = 0usize;
		let mut grass_gpu = 0usize;
		let mut tree_h_legacy = Vec::new();
		let mut tree_h_gpu = Vec::new();
		let mut tree_r_gpu = Vec::new();
		let mut bush_h_legacy = Vec::new();
		let mut bush_h_gpu = Vec::new();
		let mut bush_r_gpu = Vec::new();
		let mut grass_h_legacy = Vec::new();
		let mut grass_h_gpu = Vec::new();
		let mut capped = false;
		let max_trees = owner_settings().max_plants as usize;

		for host in hosts {
			let sites = host.tile.plant_sites();
			if !sites.is_empty() {
				layers_legacy += 1;
			}
			let visuals = host.gpu_visuals();
			if visuals.replaces_kits() {
				layers_gpu += 1;
			} else if sites.is_empty() {
				// empty on both arms
			} else {
				// GPU failed to admit a nonempty grove
			}
			match GroveGpuVisuals::field_kind(&host.tile, host.layer) {
				GpuFieldKind::Tree => {
					trees_legacy += sites.len();
					tree_h_legacy.extend(sites.iter().map(|site| site.height_m));
					if let Some(grove) = &visuals.trees {
						trees_gpu += grove.0.plants.len();
						capped |= sites.len() > max_trees;
						tree_h_gpu.extend(grove.0.plants.iter().map(|plant| plant.height_m));
						tree_r_gpu.extend(grove.0.plants.iter().map(|plant| plant.crown_radius_m));
					}
				}
				GpuFieldKind::Bush => {
					bushes_legacy += sites.len();
					bush_h_legacy.extend(sites.iter().map(|site| site.height_m));
					if let Some(patch) = &visuals.bushes {
						let plants = patch.plants();
						bushes_gpu += plants.len();
						capped |= sites.len() > 4_096;
						bush_h_gpu.extend(plants.iter().map(|plant| plant.descriptor.height_m));
						bush_r_gpu
							.extend(plants.iter().map(|plant| plant.descriptor.crown_radius_m));
					}
				}
				GpuFieldKind::Grass => {
					grass_legacy += sites.len();
					grass_h_legacy.extend(sites.iter().map(|site| site.height_m));
					if let Some(patch) = &visuals.grass {
						let blades = patch.descriptors();
						grass_gpu += blades.len();
						grass_h_gpu.extend(blades.iter().map(|blade| blade.height_m));
					}
				}
			}
		}

		Self {
			layering,
			origin,
			layers_legacy,
			layers_gpu,
			trees_legacy,
			trees_gpu,
			bushes_legacy,
			bushes_gpu,
			grass_legacy,
			grass_gpu,
			tree_h_legacy: SizeStats::from_values(&tree_h_legacy),
			tree_h_gpu: SizeStats::from_values(&tree_h_gpu),
			tree_r_gpu: SizeStats::from_values(&tree_r_gpu),
			bush_h_legacy: SizeStats::from_values(&bush_h_legacy),
			bush_h_gpu: SizeStats::from_values(&bush_h_gpu),
			bush_r_gpu: SizeStats::from_values(&bush_r_gpu),
			grass_h_legacy: SizeStats::from_values(&grass_h_legacy),
			grass_h_gpu: SizeStats::from_values(&grass_h_gpu),
			capped,
		}
	}

	fn flags(self: &TileParity) -> Vec<&'static str> {
		let mut flags = Vec::new();
		if rel(self.layers_legacy as f32, self.layers_gpu as f32) > PARITY_REL {
			flags.push("layers");
		}
		if rel(self.trees_legacy as f32, self.trees_gpu as f32) > PARITY_REL {
			flags.push("trees");
		}
		if rel(self.bushes_legacy as f32, self.bushes_gpu as f32) > PARITY_REL {
			flags.push("bushes");
		}
		if rel(self.grass_legacy as f32, self.grass_gpu as f32) > PARITY_REL {
			flags.push("grass");
		}
		if self.tree_h_legacy.rel_mean(self.tree_h_gpu) > PARITY_REL {
			flags.push("tree-h-mean");
		}
		if self.tree_h_legacy.rel_range(self.tree_h_gpu) > PARITY_REL {
			flags.push("tree-h-range");
		}
		if self.bush_h_legacy.rel_mean(self.bush_h_gpu) > PARITY_REL {
			flags.push("bush-h-mean");
		}
		if self.bush_h_legacy.rel_range(self.bush_h_gpu) > PARITY_REL {
			flags.push("bush-h-range");
		}
		if self.grass_h_legacy.rel_mean(self.grass_h_gpu) > PARITY_REL {
			flags.push("grass-h-mean");
		}
		if self.grass_h_legacy.rel_range(self.grass_h_gpu) > PARITY_REL {
			flags.push("grass-h-range");
		}
		if self.capped {
			flags.push("capped");
		}
		flags
	}
}

fn rel(a: f32, b: f32) -> f32 {
	let denom = a.abs().max(b.abs());
	if denom < 1e-4 {
		0.0
	} else {
		(a - b).abs() / denom
	}
}

fn lod_at(translation: Vec3) -> (Transform, Aabb3d) {
	(Transform::from_translation(translation), Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE))
}

fn with_lod<R>(translation: Vec3, f: impl FnOnce(&LodRef<'_>) -> R) -> R {
	with_lod_move(translation, translation, f)
}

fn with_lod_move<R>(previous: Vec3, current: Vec3, f: impl FnOnce(&LodRef<'_>) -> R) -> R {
	let (prev, bounds) = lod_at(previous);
	let curr = Transform::from_translation(current);
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &prev,
		current_transform: &curr,
		bounds: &bounds,
	};
	f(&lod_ref)
}

fn assert_gpu_cpu_lod_high_only(host: &ChicoGroveHost, origin: Vec3) {
	let near = origin + Vec3::new(50.0, 20.0, 80.0);
	let far = origin + Vec3::new(50.0, 20.0, GROVE_PRESENT_RADIUS_M + 500.0);
	assert!(near.distance(origin) < GROVE_PRESENT_RADIUS_M);
	assert!(far.distance(origin) > GROVE_PRESENT_RADIUS_M);
	with_lod(near, |lod_ref| {
		assert_eq!(host.scene_lod_level(lod_ref), LodSceneLevel::High);
		assert_eq!(
			host.scene_chunks_with_level(lod_ref, LodSceneLevel::High).total_primitives(),
			1
		);
	});
	with_lod(far, |lod_ref| {
		assert_eq!(host.scene_lod_level(lod_ref), LodSceneLevel::High);
		assert_eq!(host.scene_lod_level_from_levels(&[]), LodSceneLevel::High);
		let culls = host.scene_lod_culls(lod_ref, LodSceneLevel::High);
		assert!(!culls.should_cull(LodSceneLevel::High));
		assert!(culls.should_cull(LodSceneLevel::Medium));
		assert!(culls.should_cull(LodSceneLevel::Low));
		assert!(culls.should_cull(LodSceneLevel::UltraLow));
	});
	with_lod_move(near, far, |lod_ref| {
		assert_eq!(host.scene_lod_status(lod_ref), LodSceneStatus::Unchanged);
	});
}

fn center_grove_tile(forest: ForestExtent) -> GroveExtent {
	let c = forest.center();
	ForestExtent::grove_tiles_overlapping(Aabb3d::from_min_max(
		Vec3::new(c.x - 1.0, 0.0, c.z - 1.0),
		Vec3::new(c.x + 1.0, 1.0, c.z + 1.0),
	))
	.into_iter()
	.next()
	.unwrap_or_else(|| {
		GroveExtent::new(
			c - Vec3::new(
				DEFAULT_FOREST_GROVE_TILE_XZ * 0.5,
				0.0,
				DEFAULT_FOREST_GROVE_TILE_XZ * 0.5,
			),
			c + Vec3::new(
				DEFAULT_FOREST_GROVE_TILE_XZ * 0.5,
				1.0,
				DEFAULT_FOREST_GROVE_TILE_XZ * 0.5,
			),
		)
	})
}

fn grow_stacked(forest: ForestExtent, tile: GroveExtent) -> Vec<ChicoGroveHost> {
	let selection = ForestSelection::default();
	let layers = selection.layers_for(forest);
	let neighbors = neighbor_layers(forest, selection.noise);
	let world = forest_world_sample();
	let mut hosts = Vec::new();
	for layer in ForestLayer::ALL {
		for recipe in presenting_recipes(layer.kind(layers), tile, forest, &neighbors, layer) {
			hosts.push(ChicoGroveHost::new(recipe.grow(&world), layer));
		}
	}
	hosts
}

fn biome_samples() -> Vec<(LayeringKind, ForestExtent, GroveExtent)> {
	let selection = ForestSelection::default();
	let mut found = HashMap::new();
	for ix in -16..=16 {
		for iz in -16..=16 {
			let forest = ForestExtent::from_cell_index(ix, iz);
			let kind = select_layering(selection.noise, forest.center());
			found.entry(kind).or_insert_with(|| (kind, forest, center_grove_tile(forest)));
		}
	}
	let mut samples: Vec<_> = found.into_values().collect();
	samples.sort_by_key(|(kind, forest, _)| {
		format!("{kind:?}:{:.0},{:.0}", forest.center().x, forest.center().z)
	});
	samples
}

fn walk_waypoints() -> Vec<(LayeringKind, ForestExtent, GroveExtent)> {
	let selection = ForestSelection::default();
	let ix = ForestExtent::cell_index_containing(Vec3::new(-4950.0, 0.0, 0.0)).0;
	(-4..=5)
		.map(|iz| {
			let forest = ForestExtent::from_cell_index(ix, iz);
			let kind = select_layering(selection.noise, forest.center());
			(kind, forest, center_grove_tile(forest))
		})
		.collect()
}

fn print_table(rows: &[TileParity]) {
	eprintln!(
		"layering\torigin.xz\tlayers L/G\ttrees L/G\tbushes L/G\tgrass L/G\ttree-h mean L/G\tbush-h mean L/G\tgrass-h mean L/G\ttree-r/bush-r GPU\tflags"
	);
	for row in rows {
		let flags = row.flags();
		eprintln!(
			"{:?}\t({:.0},{:.0})\t{}/{} \t{}/{}\t{}/{}\t{}/{}\t{:.2}/{:.2}\t{:.2}/{:.2}\t{:.2}/{:.2}\t{:.2}/{:.2}\t{}",
			row.layering,
			row.origin.x,
			row.origin.z,
			row.layers_legacy,
			row.layers_gpu,
			row.trees_legacy,
			row.trees_gpu,
			row.bushes_legacy,
			row.bushes_gpu,
			row.grass_legacy,
			row.grass_gpu,
			row.tree_h_legacy.mean,
			row.tree_h_gpu.mean,
			row.bush_h_legacy.mean,
			row.bush_h_gpu.mean,
			row.grass_h_legacy.mean,
			row.grass_h_gpu.mean,
			row.tree_r_gpu.mean,
			row.bush_r_gpu.mean,
			if flags.is_empty() { "ok".into() } else { flags.join(",") }
		);
	}
}

#[test]
fn hopscotch_biome_tiles_keep_counts_layers_and_sizes() {
	let _lock = GroveGpuVisuals::lock_admission();
	GroveGpuVisuals::admit(true);
	let samples = biome_samples();
	assert!(
		samples.len() >= 8,
		"expected hopscotch to cover many layerings, got {}",
		samples.len()
	);

	let mut rows = Vec::new();
	for (layering, forest, tile) in &samples {
		let hosts = grow_stacked(*forest, *tile);
		for host in &hosts {
			let sites = host.tile.plant_sites().len();
			assert!(
				sites == 0 || host.gpu_visuals().replaces_kits(),
				"{layering:?} left {sites} plants on kits at {:?}",
				host.layer
			);
		}
		for host in &hosts {
			if host.gpu_visuals().replaces_kits() {
				assert_gpu_cpu_lod_high_only(host, tile.min());
			}
		}
		rows.push(TileParity::from_hosts(*layering, tile.min(), &hosts));
	}

	print_table(&rows);
	let flagged: Vec<_> = rows
		.iter()
		.filter(|row| !row.flags().is_empty())
		.map(|row| (row.layering, row.flags()))
		.collect();
	GroveGpuVisuals::admit(false);
	assert!(flagged.is_empty(), "GPU vs legacy off by >20% on hopscotch tiles: {flagged:?}");
}

#[test]
fn hopscotch_world_walk_streams_gpu_hosts_without_kit_chunks() {
	let _lock = GroveGpuVisuals::lock_admission();
	GroveGpuVisuals::admit(true);
	let waypoints = walk_waypoints();
	assert!(waypoints.len() >= 8, "walk should cross several forest cells");

	let mut layerings = Vec::new();
	let mut previous: Option<Vec3> = None;
	let mut streamed_out = false;
	let mut streamed_in = false;

	for (i, (layering, forest, tile)) in waypoints.iter().enumerate() {
		layerings.push(*layering);
		let started = Instant::now();
		let hosts = grow_stacked(*forest, *tile);
		let grow_ms = started.elapsed().as_secs_f32() * 1000.0;
		let mut trees = 0usize;
		let mut bushes = 0usize;
		let mut grass = 0usize;
		for host in &hosts {
			let sites = host.tile.plant_sites();
			assert!(
				sites.is_empty() || host.gpu_visuals().replaces_kits(),
				"walk {i} {layering:?} leftover kits"
			);
			if host.gpu_visuals().replaces_kits() {
				assert_gpu_cpu_lod_high_only(host, tile.min());
			}
			trees +=
				host.gpu_visuals().trees.as_ref().map(|grove| grove.0.plants.len()).unwrap_or(0);
			bushes +=
				host.gpu_visuals().bushes.as_ref().map(|patch| patch.samples.len()).unwrap_or(0);
			grass +=
				host.gpu_visuals().grass.as_ref().map(|patch| patch.samples.len()).unwrap_or(0);
		}
		let origin = tile.min();
		if let Some(prev) = previous {
			if prev != origin {
				streamed_out = true;
				streamed_in = true;
			}
		}
		previous = Some(origin);
		eprintln!(
			"walk[{i}] {:?} cell=({:.0},{:.0}) hosts={} trees={} bushes={} grass={} grow_ms={grow_ms:.1}",
			layering,
			forest.center().x,
			forest.center().z,
			hosts.len(),
			trees,
			bushes,
			grass,
		);
	}

	let unique: std::collections::HashSet<_> = layerings.iter().copied().collect();
	GroveGpuVisuals::admit(false);
	assert!(unique.len() >= 3, "walk should cross multiple biomes, got {unique:?}");
	assert!(streamed_out && streamed_in, "walk must stream grove tiles in and out");
}
