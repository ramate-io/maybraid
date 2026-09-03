//! Deterministic 4×4 jittered Shepherds Village placement.

use bevy::math::bounding::Aabb3d;
use durham_terrain_models::{TerrainCellLayout, TerrainEntryStore};
use procedural_common::{Bounds2, NoiseParams, SeededHash};
use richmond_developments::ShepherdsVillage;

use crate::config::DevelopmentConfig;
use crate::development::{cell_salt, DevelopmentPad};
use crate::hydro::{composed_height_at, terrain_hydro_overlaps};
use crate::pad::{PadComplex, PadParams, PlacedBuildingPad};
use crate::scatter::bounds_intersect;
use crate::shepherds_fit::{fit_shepherds_building, shepherds_recipe, ShepherdsBuildingKind};

pub fn build_shepherds_village(
	store: &TerrainEntryStore,
	layout: &TerrainCellLayout,
	cell: Aabb3d,
	config: &DevelopmentConfig,
) -> Option<(ShepherdsVillage, Vec<DevelopmentPad>)> {
	let root = SeededHash::new(config.seed.wrapping_add(cell_salt(cell)));
	let recipe = shepherds_recipe();
	let plan = recipe.plan(cell, root);

	let mut buildings = Vec::with_capacity(plan.target_count);
	let mut pads = Vec::with_capacity(plan.target_count);
	let mut occupied = Vec::<Bounds2>::with_capacity(plan.target_count);
	for candidate in plan.candidates {
		if buildings.len() >= plan.target_count {
			break;
		}
		let hash = SeededHash::new(
			root.seed.wrapping_add((candidate.slot as u32 + 1).wrapping_mul(0x9E37_79B9)),
		);
		let occupied_bounds = recipe.collision_bounds(&candidate);
		if occupied.iter().any(|b| bounds_intersect(*b, occupied_bounds)) {
			continue;
		}

		let coarse_pad = PadComplex::building_skirt(
			candidate.center,
			candidate.footprint * 0.5,
			candidate.yaw,
			0.0,
			PadParams::shepherds(),
		);
		if terrain_hydro_overlaps(store, layout, cell, coarse_pad.bounds) {
			continue;
		}
		let Some(height) =
			composed_height_at(store, layout, candidate.center.x, candidate.center.y)
		else {
			continue;
		};

		let kind = if matches!(candidate.kind, ShepherdsBuildingKind::House) {
			ShepherdsBuildingKind::House
		} else {
			ShepherdsBuildingKind::Hut
		};
		let noise = NoiseParams {
			seed: config.seed as i32 ^ candidate.slot as i32 * 7919,
			..NoiseParams::default()
		};
		let Some(placed) = fit_shepherds_building(
			kind,
			candidate.center,
			candidate.yaw,
			candidate.footprint,
			height,
			hash,
			noise,
		) else {
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

	if buildings.is_empty() {
		None
	} else {
		Some((ShepherdsVillage::new(cell, buildings), pads))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::math::{Vec2, Vec3};
	use richmond_buildings::Fit;
	use richmond_developments::{ShepherdsBuilding, ShepherdsHut, ShepherdsVillageBuilding};
	use std::sync::Arc;

	use crate::shepherds_fit::shepherds_recipe;

	#[test]
	fn jittered_centers_stay_inside_the_cell_inset() {
		let cell = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(200.0, 1.0, 200.0));
		let plan = shepherds_recipe().plan(cell, SeededHash::new(0));
		for candidate in plan.candidates {
			let center = candidate.center;
			assert!((32.0..=168.0).contains(&center.x));
			assert!((32.0..=168.0).contains(&center.y));
		}
	}

	#[test]
	fn continuous_yaw_changes_the_collision_envelope() {
		let center = Vec2::splat(100.0);
		let recipe = shepherds_recipe();
		let mut candidate = crate::scatter::ScatterCandidate {
			slot: 0,
			center,
			yaw: 0.0,
			footprint: Vec2::new(24.0, 12.0),
			kind: ShepherdsBuildingKind::House,
		};
		let aligned = recipe.collision_bounds(&candidate);
		candidate.yaw = std::f32::consts::FRAC_PI_4;
		let yawed = recipe.collision_bounds(&candidate);
		assert_ne!(aligned.max - aligned.min, yawed.max - yawed.min);
	}

	#[test]
	fn exact_pad_follows_the_spawned_hut() {
		let center = Vec2::new(80.0, 120.0);
		let yaw = std::f32::consts::FRAC_PI_4;
		let confines = richmond_buildings::Confines::new(
			Aabb3d::from_min_max(
				Vec3::new(center.x - 3.0, 10.0, center.y - 4.0),
				Vec3::new(center.x + 3.0, 16.0, center.y + 4.0),
			),
			yaw,
			richmond_buildings::Openings::new(),
		);
		let hut = ShepherdsHut::fit_to_confines(&confines, NoiseParams::default()).expect("hut").0;
		let placed = ShepherdsVillageBuilding {
			center_xz: center,
			yaw,
			footprint: Vec2::new(6.0, 8.0),
			ground_height: 10.0,
			building: ShepherdsBuilding::Hut(Arc::new(hut)),
		};
		let pad = placed.pad_complex(PadParams::shepherds());
		let local = Vec2::new(2.5, 3.5);
		let (s, c) = yaw.sin_cos();
		let spawned = center + Vec2::new(c * local.x + s * local.y, -s * local.x + c * local.y);
		assert_eq!(
			pad.classification_at(spawned.x, spawned.y),
			Some(crate::pad::PadStage::Flatten)
		);
	}
}
