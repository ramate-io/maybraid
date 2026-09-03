//! Shepherds Commune: hysteresis connectivity graph, then pads, then buildings.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use durham_terrain_models::{TerrainCellLayout, TerrainEntryStore};
use procedural_common::{Bounds2, HysteresisConfig, HysteresisGraph, NoiseParams, SeededHash};
use richmond_developments::{ShepherdsCommune, ShepherdsCommuneCorridor};

use crate::config::DevelopmentConfig;
use crate::connectivity::{corridor_levels, ConnectivityGraph};
use crate::development::{cell_salt, DevelopmentPad};
use crate::hydro::{composed_height_upper_on_rect, terrain_hydro_overlaps};
use crate::pad::{PadComplex, PadParams, PlacedBuildingPad};
use crate::scatter::{bounds_intersect, ScatterCandidate};
use crate::shepherds_fit::{
	fit_shepherds_building, sample_shepherds_footprint, sample_shepherds_kind, shepherds_recipe,
	ShepherdsBuildingKind,
};

const CELL_INSET: f32 = 32.0;
/// Capsule half-width before berm. With berm 2 this is a ~20 m flatten so a
/// `res_2=5` origin cell (~5 m pitch) actually samples the corridor.
const PATH_HALF_WIDTH: f32 = 8.0;
const MIN_PATH_LEN: f32 = 16.0;
const MIN_BUILDINGS: usize = 2;
/// Maximum tree-edge slope (rise/run) when BFS-assigning pad heights from the peak.
const MAX_PATH_GRADE: f32 = 0.15;
/// Keep one compact commune reasonably close to its highest site.
const MAX_COMMUNE_RELIEF: f32 = 8.0;

#[derive(Debug, Clone, Copy)]
struct CommuneSite {
	hash: SeededHash,
	kind: ShepherdsBuildingKind,
	footprint: Vec2,
	yaw: f32,
}

pub fn build_shepherds_commune(
	store: &TerrainEntryStore,
	layout: &TerrainCellLayout,
	cell: Aabb3d,
	config: &DevelopmentConfig,
) -> Option<(ShepherdsCommune, Vec<DevelopmentPad>)> {
	let root = SeededHash::new(config.seed.wrapping_add(cell_salt(cell)));
	let walk = Bounds2::from_xz(
		cell.min.x + CELL_INSET,
		cell.min.z + CELL_INSET,
		cell.max.x - CELL_INSET,
		cell.max.z - CELL_INSET,
	);
	if walk.min.x >= walk.max.x || walk.min.y >= walk.max.y {
		return None;
	}

	let head = sample_endpoint(root, 11, walk);
	let toe = sample_endpoint(root, 17, walk);
	if head.distance(toe) < MIN_PATH_LEN {
		return None;
	}
	let degree = if root.unit(23) < 0.5 { 2 } else { 3 };
	let graph = HysteresisGraph::with_degree(
		degree,
		walk,
		root.seed.wrapping_add(29),
		head,
		toe,
		&HysteresisConfig {
			max_segments: 12,
			step_len: 18.0,
			snap_radius: 16.0,
			connect_radius: 28.0,
			..HysteresisConfig::default()
		},
	);
	let conn = ConnectivityGraph::from_hysteresis(&graph)?;

	// Resolve the actual site plans before elevation assignment so the terrain
	// sample covers each building's complete flatten + ease influence.
	let sites: Vec<CommuneSite> = conn
		.keypoints
		.iter()
		.enumerate()
		.map(|(i, _)| {
			let hash =
				SeededHash::new(root.seed.wrapping_add((i as u32 + 1).wrapping_mul(0x9E37_79B9)));
			let kind = sample_shepherds_kind(hash);
			CommuneSite {
				hash,
				kind,
				footprint: sample_shepherds_footprint(hash, kind),
				yaw: conn.yaw_at(i),
			}
		})
		.collect();
	let mut natural_height = vec![None; conn.keypoints.len()];
	for (i, (p, site)) in conn.keypoints.iter().zip(&sites).enumerate() {
		natural_height[i] = composed_height_upper_on_rect(
			store,
			layout,
			*p,
			PadParams::shepherds().influence_half(site.footprint * 0.5),
			site.yaw,
		);
	}
	let mut key_height = conn.assign_graded_heights(&natural_height, MAX_PATH_GRADE);
	raise_toward_peak(&mut key_height, MAX_COMMUNE_RELIEF);

	let recipe = shepherds_recipe();
	let mut pads = Vec::new();
	let mut kept_corridors = Vec::new();
	let mut kept_links = Vec::new();
	for corridor in &conn.corridors {
		let Some(ha) = key_height[corridor.from_key] else {
			continue;
		};
		let Some(hb) = key_height[corridor.to_key] else {
			continue;
		};
		let path_len = corridor.arclength();
		if path_len < MIN_PATH_LEN {
			continue;
		}
		let levels = corridor_levels(&corridor.path, ha, hb);
		let complex = PadComplex::graded_polyline(
			&corridor.path,
			&levels,
			PATH_HALF_WIDTH,
			PadParams::path(),
		);
		if terrain_hydro_overlaps(store, layout, cell, complex.bounds) {
			continue;
		}
		let height = 0.5 * (ha + hb);
		pads.push(DevelopmentPad { height, complex });
		kept_corridors.push(ShepherdsCommuneCorridor { path: corridor.path.clone(), levels });
		kept_links.push((corridor.from_key, corridor.to_key));
	}
	if kept_corridors.is_empty() {
		return None;
	}

	let mut buildings = Vec::new();
	let mut occupied = Vec::<Bounds2>::new();
	for (i, center) in conn.keypoints.iter().copied().enumerate() {
		let Some(height) = key_height[i] else {
			continue;
		};
		if kept_links.iter().all(|&(a, b)| a != i && b != i) {
			continue;
		}
		let site = sites[i];
		let hash = site.hash;
		let kind = site.kind;
		let footprint = site.footprint;
		let yaw = site.yaw;
		let candidate = ScatterCandidate { slot: i, center, yaw, footprint, kind };
		let occupied_bounds = recipe.collision_bounds(&candidate);
		if occupied.iter().any(|b| bounds_intersect(*b, occupied_bounds)) {
			continue;
		}
		let coarse = PadComplex::building_skirt(
			center,
			footprint * 0.5,
			yaw,
			height,
			PadParams::shepherds(),
		);
		if terrain_hydro_overlaps(store, layout, cell, coarse.bounds) {
			continue;
		}
		let noise =
			NoiseParams { seed: config.seed as i32 ^ (i as i32 * 7919), ..NoiseParams::default() };
		let Some(placed) =
			fit_shepherds_building(kind, center, yaw, footprint, height, hash, noise)
		else {
			continue;
		};
		let complex = placed.pad_complex(PadParams::shepherds());
		if terrain_hydro_overlaps(store, layout, cell, complex.bounds) {
			continue;
		}
		buildings.push(placed);
		pads.push(DevelopmentPad { height, complex });
		occupied.push(occupied_bounds);
	}

	if buildings.len() < MIN_BUILDINGS {
		return None;
	}
	Some((ShepherdsCommune::new(cell, buildings, kept_corridors), pads))
}

fn raise_toward_peak(heights: &mut [Option<f32>], max_relief: f32) {
	let peak = heights.iter().flatten().copied().max_by(f32::total_cmp);
	let Some(peak) = peak else {
		return;
	};
	let floor = peak - max_relief.max(0.0);
	for height in heights.iter_mut().flatten() {
		*height = height.max(floor);
	}
}

fn sample_endpoint(hash: SeededHash, salt: u32, bounds: Bounds2) -> Vec2 {
	Vec2::new(
		lerp(bounds.min.x, bounds.max.x, hash.unit(salt)),
		lerp(bounds.min.y, bounds.max.y, hash.unit(salt.wrapping_add(1))),
	)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
	a + (b - a) * t.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::pad::PadStage;
	use bevy::math::Vec3;
	use richmond_developments::HOUSE_MAX_FOOTPRINT;

	#[test]
	fn hysteresis_walk_stays_inside_the_cell_inset() -> anyhow::Result<()> {
		let cell = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(200.0, 1.0, 200.0));
		let walk = Bounds2::from_xz(
			cell.min.x + CELL_INSET,
			cell.min.z + CELL_INSET,
			cell.max.x - CELL_INSET,
			cell.max.z - CELL_INSET,
		);
		let graph = HysteresisGraph::with_degree(
			2,
			walk,
			7,
			Vec2::new(40.0, 40.0),
			Vec2::new(160.0, 160.0),
			&HysteresisConfig::default(),
		);
		for p in &graph.nodes {
			anyhow::ensure!((CELL_INSET..=200.0 - CELL_INSET).contains(&p.x));
			anyhow::ensure!((CELL_INSET..=200.0 - CELL_INSET).contains(&p.y));
		}
		Ok(())
	}

	#[test]
	fn connecting_grade_is_narrower_than_a_house() -> anyhow::Result<()> {
		let complex = PadComplex::graded_polyline(
			&[Vec2::ZERO, Vec2::new(40.0, 0.0)],
			&[12.0, 16.0],
			PATH_HALF_WIDTH,
			PadParams::path(),
		);
		anyhow::ensure!(complex.classification_at(20.0, 0.0) == Some(PadStage::Grade));
		let flatten_half = PATH_HALF_WIDTH + PadParams::path().berm;
		anyhow::ensure!(
			complex.classification_at(20.0, flatten_half + 0.5) != Some(PadStage::Grade)
		);
		anyhow::ensure!(
			complex.classification_at(20.0, HOUSE_MAX_FOOTPRINT * 0.5) != Some(PadStage::Grade),
			"path flatten should not be as wide as a house"
		);
		anyhow::ensure!(
			PATH_HALF_WIDTH + PadParams::path().berm >= 8.0,
			"path core should span more than one ~5 m terrain sample"
		);
		Ok(())
	}

	#[test]
	fn commune_sites_stay_near_the_peak() -> anyhow::Result<()> {
		let mut heights = vec![Some(100.0), Some(72.0), Some(95.0), None];
		raise_toward_peak(&mut heights, 8.0);
		anyhow::ensure!(heights[0] == Some(100.0));
		anyhow::ensure!(heights[1] == Some(92.0));
		anyhow::ensure!(heights[2] == Some(95.0));
		anyhow::ensure!(heights[3].is_none());
		Ok(())
	}
}
