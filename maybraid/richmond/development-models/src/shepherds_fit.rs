//! Shared house / hut fitting used by Shepherds Village and Shepherds Commune.

use std::sync::Arc;

use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use procedural_common::{NoiseParams, SeededHash};
use richmond_building_components::panels::PanelStyle;
use richmond_buildings::{Confines, Fit, Openings};
use richmond_developments::{
	ShepherdsBuilding, ShepherdsFinish, ShepherdsHouse, ShepherdsHut, ShepherdsVillageBuilding,
	HOUSE_MAX_FOOTPRINT, HOUSE_MIN_FOOTPRINT, HOUSE_STOREY_HEIGHT, HUT_HEIGHT, HUT_MAX_FOOTPRINT,
	HUT_MIN_FOOTPRINT,
};

use crate::finish::DevelopmentFinish;
use crate::scatter::{ScatterChoice, ScatterRecipe};

#[derive(Debug, Clone, Copy)]
pub(crate) enum ShepherdsBuildingKind {
	House,
	Hut,
}

pub(crate) fn shepherds_recipe() -> ScatterRecipe<ShepherdsBuildingKind> {
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

pub(crate) fn shepherds_authored_height(kind: ShepherdsBuildingKind) -> f32 {
	match kind {
		ShepherdsBuildingKind::House => 2.0 * HOUSE_STOREY_HEIGHT,
		ShepherdsBuildingKind::Hut => HUT_HEIGHT,
	}
}

pub(crate) fn sample_shepherds_kind(hash: SeededHash) -> ShepherdsBuildingKind {
	if hash.unit(1) < 0.5 {
		ShepherdsBuildingKind::House
	} else {
		ShepherdsBuildingKind::Hut
	}
}

pub(crate) fn sample_shepherds_footprint(hash: SeededHash, kind: ShepherdsBuildingKind) -> Vec2 {
	let (lo, hi) = match kind {
		ShepherdsBuildingKind::House => (HOUSE_MIN_FOOTPRINT, HOUSE_MAX_FOOTPRINT),
		ShepherdsBuildingKind::Hut => (HUT_MIN_FOOTPRINT, HUT_MAX_FOOTPRINT),
	};
	Vec2::new(lerp(lo, hi, hash.unit(2)), lerp(lo, hi, hash.unit(3)))
}

pub(crate) fn fit_shepherds_building(
	kind: ShepherdsBuildingKind,
	center: Vec2,
	yaw: f32,
	footprint: Vec2,
	height: f32,
	hash: SeededHash,
	noise: NoiseParams,
) -> Option<ShepherdsVillageBuilding> {
	let authored_height = shepherds_authored_height(kind);
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
	let building = match kind {
		ShepherdsBuildingKind::House => {
			let (house, _) = ShepherdsHouse::fit_to_confines(&confines, noise).ok()?;
			let wooden = house.wall_style == PanelStyle::RibAndPlank;
			let finish = DevelopmentFinish::pick_shepherds(hash, wooden);
			ShepherdsBuilding::House(Arc::new(
				house.with_finish(ShepherdsFinish { wall: finish.wall, roof: finish.roof }),
			))
		}
		ShepherdsBuildingKind::Hut => {
			let (hut, _) = ShepherdsHut::fit_to_confines(&confines, noise).ok()?;
			let wooden = hut.wall_style == PanelStyle::RibAndPlank;
			let finish = DevelopmentFinish::pick_shepherds(hash, wooden);
			ShepherdsBuilding::Hut(Arc::new(
				hut.with_finish(ShepherdsFinish { wall: finish.wall, roof: finish.roof }),
			))
		}
	};
	Some(ShepherdsVillageBuilding {
		center_xz: center,
		yaw,
		footprint,
		ground_height: height,
		building,
	})
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
	a + (b - a) * t.clamp(0.0, 1.0)
}
