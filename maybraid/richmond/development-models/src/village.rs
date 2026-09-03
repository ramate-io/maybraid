//! Deterministic 4×4 jittered Shepherds Village placement.

use std::sync::Arc;

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use durham_terrain_models::{TerrainCellLayout, TerrainEntryStore};
use procedural_common::{Bounds2, NoiseParams, SeededHash};
use richmond_building_components::panels::PanelStyle;
use richmond_buildings::{Confines, Fit, Openings};
use richmond_developments::{
	ShepherdsBuilding, ShepherdsFinish, ShepherdsHouse, ShepherdsHut, ShepherdsVillage,
	ShepherdsVillageBuilding, HOUSE_MAX_FOOTPRINT, HOUSE_MIN_FOOTPRINT, HUT_HEIGHT,
	HUT_MAX_FOOTPRINT, HUT_MIN_FOOTPRINT,
};

use crate::config::DevelopmentConfig;
use crate::development::{cell_salt, DevelopmentPad};
use crate::finish::DevelopmentFinish;
use crate::hydro::{composed_height_at, terrain_hydro_overlaps};
use crate::pad::{PadComplex, PadParams, PlacedBuildingPad};
use crate::scatter::{bounds_intersect, ScatterChoice, ScatterRecipe};

#[derive(Debug, Clone, Copy)]
enum ShepherdsBuildingKind {
	House,
	Hut,
}

fn shepherds_recipe() -> ScatterRecipe<ShepherdsBuildingKind> {
	ScatterRecipe {
		grid_side: 4,
		min_count: 6,
		max_count: 10,
		cell_inset: 32.0,
		jitter: 6.0,
		clearance: 3.0,
		choices: vec![
			ScatterChoice {
				kind: ShepherdsBuildingKind::House,
				weight: 1.0,
				min_footprint: HOUSE_MIN_FOOTPRINT,
				max_footprint: HOUSE_MAX_FOOTPRINT,
			},
			ScatterChoice {
				kind: ShepherdsBuildingKind::Hut,
				weight: 1.0,
				min_footprint: HUT_MIN_FOOTPRINT,
				max_footprint: HUT_MAX_FOOTPRINT,
			},
		],
	}
}

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
		let is_house = matches!(candidate.kind, ShepherdsBuildingKind::House);
		let footprint = candidate.footprint;
		let yaw = candidate.yaw;
		let center = candidate.center;
		let occupied_bounds = recipe.collision_bounds(&candidate);
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
			seed: config.seed as i32 ^ candidate.slot as i32 * 7919,
			..NoiseParams::default()
		};
		let building = if is_house {
			let Ok((house, _)) = ShepherdsHouse::fit_to_confines(&confines, noise) else {
				continue;
			};
			let wooden = house.wall_style == PanelStyle::RibAndPlank;
			let finish = DevelopmentFinish::pick_shepherds(hash, wooden);
			ShepherdsBuilding::House(Arc::new(
				house.with_finish(ShepherdsFinish { wall: finish.wall, roof: finish.roof }),
			))
		} else {
			let Ok((hut, _)) = ShepherdsHut::fit_to_confines(&confines, noise) else {
				continue;
			};
			let wooden = hut.wall_style == PanelStyle::RibAndPlank;
			let finish = DevelopmentFinish::pick_shepherds(hash, wooden);
			ShepherdsBuilding::Hut(Arc::new(
				hut.with_finish(ShepherdsFinish { wall: finish.wall, roof: finish.roof }),
			))
		};

		let placed = ShepherdsVillageBuilding {
			center_xz: center,
			yaw,
			footprint,
			ground_height: height,
			building,
		};
		let complex = placed.pad_complex(PadParams::default());
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
	use bevy::math::Vec2;
	use richmond_buildings::Fit;

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
		let confines = Confines::new(
			Aabb3d::from_min_max(
				Vec3::new(center.x - 3.0, 10.0, center.y - 4.0),
				Vec3::new(center.x + 3.0, 16.0, center.y + 4.0),
			),
			yaw,
			Openings::new(),
		);
		let hut = ShepherdsHut::fit_to_confines(&confines, NoiseParams::default()).expect("hut").0;
		let placed = ShepherdsVillageBuilding {
			center_xz: center,
			yaw,
			footprint: Vec2::new(6.0, 8.0),
			ground_height: 10.0,
			building: ShepherdsBuilding::Hut(Arc::new(hut)),
		};
		let pad = placed.pad_complex(PadParams::default());
		let local = Vec2::new(2.5, 3.5);
		let (s, c) = yaw.sin_cos();
		let spawned = center + Vec2::new(c * local.x + s * local.y, -s * local.x + c * local.y);
		assert_eq!(
			pad.classification_at(spawned.x, spawned.y),
			Some(crate::pad::PadStage::Flatten)
		);
	}
}
