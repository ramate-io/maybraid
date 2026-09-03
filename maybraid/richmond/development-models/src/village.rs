//! Deterministic 4×4 jittered Shepherds Village placement.

use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use durham_terrain_models::{TerrainCellLayout, TerrainEntryStore};
use procedural_common::{Bounds2, NoiseParams, SeededHash};
use richmond_building_components::panels::PanelStyle;
use richmond_buildings::{Confines, Fit, Openings};
use richmond_developments::{
	ShepherdsBuilding, ShepherdsFinish, ShepherdsHouse, ShepherdsHut, ShepherdsVillage,
	ShepherdsVillageBuilding, HOUSE_MAX_FOOTPRINT, HOUSE_MIN_FOOTPRINT, HUT_HEIGHT,
	HUT_MAX_FOOTPRINT, HUT_MIN_FOOTPRINT,
};

use crate::cell::{sample_confines_yaw, yawed_plan_aabb_extent};
use crate::config::DevelopmentConfig;
use crate::development::DevelopmentPad;
use crate::finish::DevelopmentFinish;
use crate::hydro::{composed_height_at, terrain_hydro_overlaps};
use crate::pad::{PadComplex, PadNode, PadParams};

const GRID_SIDE: usize = 4;
const MIN_BUILDINGS: usize = 6;
const MAX_BUILDINGS: usize = 10;
const CELL_INSET: f32 = 32.0;
const JITTER: f32 = 6.0;
const BUILDING_CLEARANCE: f32 = 3.0;

#[derive(Debug, Clone, Copy)]
struct Candidate {
	index: usize,
	priority: f32,
}

pub fn build_shepherds_village(
	store: &TerrainEntryStore,
	layout: &TerrainCellLayout,
	cell: Aabb3d,
	config: &DevelopmentConfig,
) -> Option<(ShepherdsVillage, Vec<DevelopmentPad>)> {
	let root = SeededHash::new(config.seed.wrapping_add(cell_salt(cell)));
	let target = MIN_BUILDINGS
		+ (root.unit(101) * (MAX_BUILDINGS - MIN_BUILDINGS + 1) as f32).floor() as usize;
	let mut candidates: Vec<Candidate> = (0..GRID_SIDE * GRID_SIDE)
		.map(|index| Candidate { index, priority: root.unit(200 + index as u32) })
		.collect();
	candidates.sort_by(|a, b| a.priority.total_cmp(&b.priority));

	let mut buildings = Vec::with_capacity(target);
	let mut pads = Vec::with_capacity(target);
	let mut occupied = Vec::<Bounds2>::with_capacity(target);
	for candidate in candidates {
		if buildings.len() >= target {
			break;
		}
		let hash = SeededHash::new(
			root.seed.wrapping_add((candidate.index as u32 + 1).wrapping_mul(0x9E37_79B9)),
		);
		let is_house = hash.unit(1) < 0.5;
		let footprint = if is_house {
			Vec2::new(
				lerp(HOUSE_MIN_FOOTPRINT, HOUSE_MAX_FOOTPRINT, hash.unit(2)),
				lerp(HOUSE_MIN_FOOTPRINT, HOUSE_MAX_FOOTPRINT, hash.unit(3)),
			)
		} else {
			Vec2::new(
				lerp(HUT_MIN_FOOTPRINT, HUT_MAX_FOOTPRINT, hash.unit(2)),
				lerp(HUT_MIN_FOOTPRINT, HUT_MAX_FOOTPRINT, hash.unit(3)),
			)
		};
		let yaw = sample_confines_yaw(hash.unit(4));
		let center = candidate_center(cell, candidate.index, hash);
		let occupied_bounds = rotated_envelope_bounds(center, footprint, yaw, BUILDING_CLEARANCE);
		if occupied.iter().any(|b| bounds_intersect(*b, occupied_bounds)) {
			continue;
		}

		let coarse_pad =
			PadComplex::building_skirt(center, footprint * 0.5, yaw, 0.0, PadParams::default());
		if terrain_hydro_overlaps(store, layout, cell, coarse_pad.bounds) {
			continue;
		}
		let Some(height) = composed_height_at(store, layout, center.x, center.y) else {
			continue;
		};

		let authored_height = if is_house { 2.0 * 3.0 } else { HUT_HEIGHT };
		let confines = Confines::new(
			Aabb3d::from_min_max(
				Vec3::new(center.x - footprint.x * 0.5, height, center.y - footprint.y * 0.5),
				Vec3::new(
					center.x + footprint.x * 0.5,
					height + authored_height,
					center.y + footprint.y * 0.5,
				),
			),
			yaw,
			Openings::new(),
		);
		let noise = NoiseParams {
			seed: config.seed as i32 ^ candidate.index as i32 * 7919,
			..NoiseParams::default()
		};
		let building = if is_house {
			let Ok((house, _)) = ShepherdsHouse::fit_to_confines(&confines, noise) else {
				continue;
			};
			let wooden = house.wall_style == PanelStyle::RibAndPlank;
			let finish = DevelopmentFinish::pick_shepherds(hash, wooden);
			ShepherdsBuilding::House(
				house.with_finish(ShepherdsFinish { wall: finish.wall, roof: finish.roof }),
			)
		} else {
			let Ok((hut, _)) = ShepherdsHut::fit_to_confines(&confines, noise) else {
				continue;
			};
			let wooden = hut.wall_style == PanelStyle::RibAndPlank;
			let finish = DevelopmentFinish::pick_shepherds(hash, wooden);
			ShepherdsBuilding::Hut(
				hut.with_finish(ShepherdsFinish { wall: finish.wall, roof: finish.roof }),
			)
		};

		let complex = exact_pad_for(&building, center, yaw, height);
		if terrain_hydro_overlaps(store, layout, cell, complex.bounds) {
			continue;
		}
		buildings.push(ShepherdsVillageBuilding { center_xz: center, yaw, footprint, building });
		pads.push(DevelopmentPad { height, complex });
		occupied.push(occupied_bounds);
	}

	if buildings.is_empty() {
		None
	} else {
		Some((ShepherdsVillage::new(cell, buildings), pads))
	}
}

fn candidate_center(cell: Aabb3d, index: usize, hash: SeededHash) -> Vec2 {
	let ix = index % GRID_SIDE;
	let iz = index / GRID_SIDE;
	let min = Vec2::new(cell.min.x + CELL_INSET, cell.min.z + CELL_INSET);
	let max = Vec2::new(cell.max.x - CELL_INSET, cell.max.z - CELL_INSET);
	let denom = (GRID_SIDE - 1) as f32;
	let base =
		Vec2::new(lerp(min.x, max.x, ix as f32 / denom), lerp(min.y, max.y, iz as f32 / denom));
	let jitter =
		Vec2::new(lerp(-JITTER, JITTER, hash.unit(5)), lerp(-JITTER, JITTER, hash.unit(6)));
	(base + jitter).clamp(min, max)
}

fn rotated_envelope_bounds(center: Vec2, footprint: Vec2, yaw: f32, clearance: f32) -> Bounds2 {
	let half = yawed_plan_aabb_extent(footprint.x, footprint.y, yaw) * 0.5 + Vec2::splat(clearance);
	Bounds2 { min: center - half, max: center + half }
}

fn exact_pad_for(building: &ShepherdsBuilding, center: Vec2, yaw: f32, height: f32) -> PadComplex {
	let (s, c) = yaw.sin_cos();
	let nodes = building
		.footprint_rects()
		.into_iter()
		.map(|rect| {
			let rect_center = (rect.min + rect.max) * 0.5;
			let local = rect_center - center;
			let rotated_center =
				center + Vec2::new(c * local.x + s * local.y, -s * local.x + c * local.y);
			PadNode::rectangular_flatten(
				rotated_center,
				(rect.max - rect.min) * 0.5,
				yaw,
				height,
				PadParams::default(),
			)
		})
		.collect();
	PadComplex::from_nodes(nodes)
}

fn bounds_intersect(a: Bounds2, b: Bounds2) -> bool {
	a.min.x <= b.max.x && b.min.x <= a.max.x && a.min.y <= b.max.y && b.min.y <= a.max.y
}

fn cell_salt(cell: Aabb3d) -> u32 {
	cell.min.x.to_bits().wrapping_mul(73856093) ^ cell.min.z.to_bits().wrapping_mul(19349663)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
	a + (b - a) * t.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
	use super::*;
	use richmond_buildings::Fit;

	#[test]
	fn jittered_centers_stay_inside_the_cell_inset() {
		let cell = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(200.0, 1.0, 200.0));
		for i in 0..16 {
			let center = candidate_center(cell, i, SeededHash::new(i as u32));
			assert!((32.0..=168.0).contains(&center.x));
			assert!((32.0..=168.0).contains(&center.y));
		}
	}

	#[test]
	fn continuous_yaw_changes_the_collision_envelope() {
		let center = Vec2::splat(100.0);
		let aligned = rotated_envelope_bounds(center, Vec2::new(24.0, 12.0), 0.0, 0.0);
		let yawed = rotated_envelope_bounds(
			center,
			Vec2::new(24.0, 12.0),
			std::f32::consts::FRAC_PI_4,
			0.0,
		);
		assert_ne!(aligned.max - aligned.min, yawed.max - yawed.min);
	}

	#[test]
	fn exact_pad_follows_the_spawned_hut() {
		let center = Vec2::new(80.0, 120.0);
		let yaw = std::f32::consts::FRAC_PI_4;
		let confines = Confines::new(
			Aabb3d::from_min_max(
				Vec3::new(center.x - 3.0, 10.0, center.y - 4.0),
				Vec3::new(center.x + 3.0, 16.0, center.y + 4.0),
			),
			yaw,
			Openings::new(),
		);
		let hut = ShepherdsHut::fit_to_confines(&confines, NoiseParams::default()).expect("hut").0;
		let building = ShepherdsBuilding::Hut(hut);
		let pad = exact_pad_for(&building, center, yaw, 10.0);
		let local = Vec2::new(2.5, 3.5);
		let (s, c) = yaw.sin_cos();
		let spawned = center + Vec2::new(c * local.x + s * local.y, -s * local.x + c * local.y);
		assert_eq!(
			pad.classification_at(spawned.x, spawned.y),
			Some(crate::pad::PadStage::Flatten)
		);
	}
}
