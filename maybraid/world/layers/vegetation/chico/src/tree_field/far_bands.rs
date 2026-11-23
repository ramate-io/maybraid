//! Per-band GPU vs legacy instance counts on the real `ChicoGroveHost` path.

use bevy::camera::CameraProjection;
use bevy::math::{Vec3, Vec4};
use bevy::prelude::{PerspectiveProjection, Transform};
use lod::scene::LodSceneLevel;
use vegetation_bush_field::{BushField, BushFieldSettings, BushSelector};
use vegetation_grass_field::{
	grass_patch_visible, woody_grove_visible, GrassField, GrassFieldSettings, GrassSelector,
	WoodyGroveRecord,
};
use vegetation_groves::GroveExtent;
use vegetation_shaders::wesl::{GrassViewUniform, LodThresholds};
use vegetation_tree_field::TreeSelector;

use super::{owner_settings, GpuFieldKind, GroveGpuVisuals};
use crate::grove::forest_world_sample;
use crate::{ChicoGroveHost, ForestGroveKind, ForestGroveRecipe, ForestLayer, LayerDropOut};

const PARITY_REL: f32 = 0.20;
const DISTANCES_M: [f32; 4] = [50.0, 200.0, 450.0, 900.0];

#[derive(Clone, Copy, Debug)]
struct BandCounts {
	trees: usize,
	bushes: usize,
	grass: usize,
}

fn look_view(origin: Vec3, target: Vec3, thresholds: LodThresholds) -> GrassViewUniform {
	let view_from_world = Transform::from_translation(origin)
		.looking_at(target, Vec3::Y)
		.to_matrix()
		.inverse();
	let projection = PerspectiveProjection {
		fov: 2.0,
		aspect_ratio: 16.0 / 9.0,
		near: 0.1,
		far: 2500.0,
		..Default::default()
	};
	let clip = projection.get_clip_from_view() * view_from_world;
	GrassViewUniform {
		clip_from_world: clip,
		world_from_clip: clip.inverse(),
		world_position: origin.extend(0.0),
		prev_world_position: origin.extend(0.0),
		resolution: Vec4::new(1920.0, 1080.0, 1.0, 0.0),
		wind: Vec4::new(0.0, 0.0, 0.0, 0.0),
		lod_distances: Vec4::new(0.0, thresholds.near_m, thresholds.mid_m, thresholds.card_m),
		lod_density: Vec4::new(
			thresholds.near_density,
			thresholds.mid_density,
			thresholds.card_density,
			thresholds.min_height_px,
		),
		counts: Vec4::new(0.0, 262_144.0, 0.0, 0.0),
		light_direction: Vec4::new(0.4, 0.8, 0.3, 0.0),
	}
}

fn legacy_level(distance_m: f32) -> LodSceneLevel {
	if distance_m <= 400.0 {
		LodSceneLevel::High
	} else if distance_m <= 520.0 {
		LodSceneLevel::Medium
	} else {
		LodSceneLevel::Low
	}
}

fn woody_grove_record(
	bounds_min: Vec3,
	bounds_max: Vec3,
	plant_count: u32,
	branch_count: u32,
	leaf_count: u32,
) -> WoodyGroveRecord {
	WoodyGroveRecord {
		aabb_min: bounds_min,
		aabb_max: bounds_max,
		plant_off: 0,
		plant_count,
		branch_off: 0,
		branch_count,
		leaf_off: 0,
		leaf_count,
		flags: WoodyGroveRecord::FLAG_ACTIVE,
		..Default::default()
	}
}

fn representative_plant_height_m(height_m: impl Iterator<Item = f32>) -> f32 {
	height_m.max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)).unwrap_or(1.0)
}

fn grow_stacked() -> Vec<ChicoGroveHost> {
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
	let world = forest_world_sample();
	[
		(ForestGroveKind::TemperateLowerMassives, ForestLayer::LowerCanopy),
		(ForestGroveKind::LowBush, ForestLayer::Understory),
		(ForestGroveKind::TallGrass, ForestLayer::Tufts),
	]
	.into_iter()
	.map(|(kind, layer)| {
		ChicoGroveHost::new(ForestGroveRecipe::uniform(kind, extent).grow(&world), layer)
	})
	.collect()
}

fn gpu_counts_with(
	hosts: &[ChicoGroveHost],
	distance_m: f32,
	tree_lod: LodThresholds,
	bush_lod: LodThresholds,
	grass_lod: LodThresholds,
) -> BandCounts {
	let look = Vec3::new(50.0, 4.0, 50.0);
	let camera = Vec3::new(50.0, 20.0, 50.0 - distance_m);
	let mut trees = 0usize;
	let mut bushes = 0usize;
	let mut grass = 0usize;
	for host in hosts {
		let visuals = host.gpu_visuals();
		if let Some(grove) = &visuals.trees {
			let field = &grove.0;
			let view = look_view(camera, look, tree_lod);
			let settings = owner_settings();
			let record = woody_grove_record(
				field.bounds.min,
				field.bounds.max,
				field.plants.len() as u32,
				field.branches.len() as u32,
				field.leaves.len() as u32,
			);
			let height = representative_plant_height_m(field.plants.iter().map(|p| p.height_m));
			if woody_grove_visible(&view, tree_lod, record, settings.wind.speed, height) {
				let selector = TreeSelector::from_view(view, &settings);
				trees += field
					.plants
					.iter()
					.filter(|plant| selector.plant_bucket(plant).is_some())
					.count();
			}
		}
		if let Some(patch) = &visuals.bushes {
			let settings = BushFieldSettings::default();
			let field = BushField::rebuild([patch], &settings);
			let view = look_view(camera, look, bush_lod);
			let record = woody_grove_record(
				field.bounds.min,
				field.bounds.max,
				field.plants.len() as u32,
				field.branches.len() as u32,
				field.leaves.len() as u32,
			);
			let height = representative_plant_height_m(field.plants.iter().map(|p| p.height_m));
			if field.grove_residency
				&& woody_grove_visible(&view, bush_lod, record, settings.wind.speed, height)
			{
				let selector = BushSelector::from_view(view, &settings);
				bushes += field
					.plants
					.iter()
					.filter(|plant| selector.plant_bucket(plant).is_some())
					.count();
			} else if !field.grove_residency {
				let selector = BushSelector::from_view(view, &settings);
				bushes += field
					.plants
					.iter()
					.filter(|plant| selector.plant_bucket(plant).is_some())
					.count();
			}
		}
		if let Some(patch) = &visuals.grass {
			let settings = GrassFieldSettings::default();
			let field = GrassField::rebuild([patch], &settings);
			let view = look_view(camera, look, grass_lod);
			let selector = GrassSelector::from_view(view, &settings);
			if field.grove_residency {
				for patch in &field.patches {
					if !grass_patch_visible(&view, grass_lod, patch) {
						continue;
					}
					let start = patch.blade_offset as usize;
					let end = start + patch.blade_count as usize;
					grass += field.blades[start..end.min(field.blades.len())]
						.iter()
						.filter(|blade| selector.classify(blade).is_some())
						.count();
				}
			} else {
				grass += field.blades.iter().filter(|blade| selector.classify(blade).is_some()).count();
			}
		}
	}
	BandCounts { trees, bushes, grass }
}

fn gpu_counts(hosts: &[ChicoGroveHost], distance_m: f32) -> BandCounts {
	gpu_counts_with(
		hosts,
		distance_m,
		LodThresholds::grove_tree(),
		LodThresholds::grove_bush(),
		LodThresholds::grove_grass(),
	)
}

fn gpu_counts_old_cutoffs(hosts: &[ChicoGroveHost], distance_m: f32) -> BandCounts {
	gpu_counts_with(
		hosts,
		distance_m,
		LodThresholds::tree_default(),
		LodThresholds::bush_default(),
		LodThresholds::field_default(),
	)
}

fn legacy_counts(hosts: &[ChicoGroveHost], distance_m: f32) -> BandCounts {
	let level = legacy_level(distance_m);
	let mut trees = 0usize;
	let mut bushes = 0usize;
	let mut grass = 0usize;
	for host in hosts {
		let sites = host.tile.plant_sites().len();
		let drop = LayerDropOut::for_stacked(host.layer, host.tile.is_tuft());
		if drop.omits(level) {
			continue;
		}
		match GroveGpuVisuals::field_kind(&host.tile, host.layer) {
			GpuFieldKind::Tree => trees += sites,
			GpuFieldKind::Bush => bushes += sites,
			GpuFieldKind::Grass => grass += sites,
		}
	}
	BandCounts { trees, bushes, grass }
}

fn gpu_submitted(hosts: &[ChicoGroveHost], distance_m: f32) -> (u32, u32) {
	let look = Vec3::new(50.0, 4.0, 50.0);
	let camera = Vec3::new(50.0, 20.0, 50.0 - distance_m);
	let mut wood = 0u32;
	let mut leaves = 0u32;
	for host in hosts {
		let Some(grove) = &host.gpu_visuals().trees else {
			continue;
		};
		let view = look_view(camera, look, LodThresholds::grove_tree());
		let lists = TreeSelector::from_view(view, &owner_settings()).compact(
			&grove.0.plants,
			&grove.0.branches,
			&grove.0.leaves,
		);
		wood += lists.submitted_wood;
		leaves += lists.submitted_foliage;
	}
	(wood, leaves)
}

fn rel(a: usize, b: usize) -> f32 {
	let denom = a.max(b) as f32;
	if denom < 1.0 {
		0.0
	} else {
		(a as f32 - b as f32).abs() / denom
	}
}

#[test]
fn grove_gpu_far_bands_match_legacy_instance_counts() {
	let _lock = GroveGpuVisuals::lock_admission();
	GroveGpuVisuals::admit(true);
	let hosts = grow_stacked();
	assert!(hosts.iter().any(|host| host.gpu_visuals().trees.is_some()));
	assert!(hosts.iter().any(|host| host.gpu_visuals().bushes.is_some()));
	assert!(hosts.iter().any(|host| host.gpu_visuals().grass.is_some()));

	eprintln!("distance_m\tlegacy T/B/G\tGPU T/B/G\told-cutoff T/B/G");
	let mut near_leaves = None;
	for distance_m in DISTANCES_M {
		let gpu = gpu_counts(&hosts, distance_m);
		let old = gpu_counts_old_cutoffs(&hosts, distance_m);
		let legacy = legacy_counts(&hosts, distance_m);
		let submitted = gpu_submitted(&hosts, distance_m);
		eprintln!(
			"{:.0}\t{}/{}/{}\t{}/{}/{}\t{}/{}/{}\twood={} leaves={}",
			distance_m,
			legacy.trees,
			legacy.bushes,
			legacy.grass,
			gpu.trees,
			gpu.bushes,
			gpu.grass,
			old.trees,
			old.bushes,
			old.grass,
			submitted.0,
			submitted.1
		);
		if distance_m == 50.0 {
			near_leaves = Some(submitted.1);
		}
		if distance_m == 900.0 {
			if let Some(near) = near_leaves {
				assert!(
					submitted.1 < near,
					"GPU LOD should thin leaves from 50 m ({near}) to 900 m ({})",
					submitted.1
				);
			}
		}
		assert!(
			rel(gpu.trees, legacy.trees) <= PARITY_REL,
			"trees at {distance_m} m: GPU {} vs legacy {}",
			gpu.trees,
			legacy.trees
		);
		let bush_cut = LodThresholds::grove_bush().card_m;
		let grass_cut = LodThresholds::grove_grass().card_m;
		if distance_m <= bush_cut {
			assert!(
				rel(gpu.bushes, legacy.bushes) <= PARITY_REL,
				"bushes at {distance_m} m: GPU {} vs legacy {}",
				gpu.bushes,
				legacy.bushes
			);
		} else if distance_m > bush_cut {
			assert_eq!(
				gpu.bushes, 0,
				"whole-grove bush residency drops beyond card_m ({bush_cut} m) at {distance_m} m"
			);
		}
		if distance_m <= grass_cut {
			assert!(
				rel(gpu.grass, legacy.grass) <= PARITY_REL,
				"grass at {distance_m} m: GPU {} vs legacy {}",
				gpu.grass,
				legacy.grass
			);
		} else if distance_m > grass_cut {
			assert_eq!(
				gpu.grass, 0,
				"whole-grove grass residency drops beyond card_m ({grass_cut} m) at {distance_m} m"
			);
		}
		if distance_m >= 200.0 && distance_m < bush_cut {
			assert!(
				gpu.bushes > old.bushes,
				"grove bush card_m should keep understory past playground default at {distance_m} m"
			);
		}
		if distance_m >= 200.0 && distance_m < grass_cut {
			assert!(
				gpu.grass > old.grass,
				"grove grass card_m should keep tufts past playground default at {distance_m} m"
			);
		}
	}
	GroveGpuVisuals::admit(false);
}
