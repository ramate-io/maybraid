//! [`ShepherdsVillageCell`]: deterministic 4×4 jittered Shepherds Village placement.

use std::marker::PhantomData;

use bevy::math::bounding::Aabb3d;
use lod::gen::{GenerationScheme, Id, OriginalId};
use lod::hcsg::HcsgStorage;
use procedural_common::{Bounds2, NoiseParams, SeededHash};
use urbanization_developments::ShepherdsVillage;

use super::site::{DevelopmentKind, DevelopmentSite};
use super::DevelopmentPad;
use crate::artifact::BuiltDevelopment;
use crate::cell::cell_salt;
use crate::config::DevelopmentConfig;
use crate::ground::{GroundSampler, RichmondGround, SiteGround};
use crate::pad::{PadComplex, PadParams, PlacedBuildingPad};
use crate::scatter::bounds_intersect;
use crate::shepherds_fit::{fit_shepherds_building, shepherds_recipe, ShepherdsBuildingKind};
use crate::storage::column_bounds;

/// A Shepherds Village over ground `G`: one pad per hut or house.
pub struct ShepherdsVillageCell<G> {
	pub cell: Aabb3d,
	pub pads: Vec<DevelopmentPad>,
	pub village: ShepherdsVillage,
	_ground: PhantomData<fn() -> G>,
}

impl<G> ShepherdsVillageCell<G> {
	pub fn built(&self) -> BuiltDevelopment {
		BuiltDevelopment::ShepherdsVillage(Box::new(self.village.clone()))
	}
}

impl<G: RichmondGround> GenerationScheme<HcsgStorage> for ShepherdsVillageCell<G> {
	fn original_ids_for(storage: &mut HcsgStorage, region: Aabb3d) -> Vec<OriginalId> {
		DevelopmentSite::ids_of_kind(storage, region, DevelopmentKind::ShepherdsVillage)
	}

	fn build_with_id(storage: &mut HcsgStorage, id: Id) -> Option<(Self, Aabb3d)> {
		let (site, config) =
			DevelopmentSite::planned(storage, id, DevelopmentKind::ShepherdsVillage)?;
		let bounds = column_bounds(site.cell);
		let mut ground = GroundSampler::<G>::new(storage, bounds);
		let (village, pads) = build_shepherds_village(&mut ground, site.cell, &config)?;
		Some((Self { cell: site.cell, pads, village, _ground: PhantomData }, bounds))
	}
}

fn build_shepherds_village(
	ground: &mut impl SiteGround,
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
		if ground.hydro_overlaps(coarse_pad.bounds) {
			continue;
		}
		let Some(height) = ground.height_upper_on_rect(
			candidate.center,
			PadParams::shepherds().influence_half(candidate.footprint * 0.5),
			candidate.yaw,
		) else {
			continue;
		};

		let kind = if matches!(candidate.kind, ShepherdsBuildingKind::House) {
			ShepherdsBuildingKind::House
		} else {
			ShepherdsBuildingKind::Hut
		};
		let noise = NoiseParams {
			seed: config.seed as i32 ^ (candidate.slot as i32 * 7919),
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
		if ground.hydro_overlaps(complex.bounds) {
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
	use buildings::Fit;
	use std::sync::Arc;
	use urbanization_developments::{ShepherdsBuilding, ShepherdsHut, ShepherdsVillageBuilding};

	use crate::ground::tests::FlatGround;
	use crate::shepherds_fit::shepherds_recipe;

	#[test]
	fn flat_dry_ground_places_buildings_on_their_pads() -> anyhow::Result<()> {
		let cell = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(300.0, 1.0, 300.0));
		let mut ground = FlatGround { height: 9.0, wet: |_| false };
		let (village, pads) =
			build_shepherds_village(&mut ground, cell, &DevelopmentConfig::default())
				.ok_or_else(|| anyhow::anyhow!("village on flat ground"))?;
		anyhow::ensure!(!village.buildings.is_empty());
		anyhow::ensure!(pads.len() == village.buildings.len());
		anyhow::ensure!(pads.iter().all(|pad| (pad.height - 9.0).abs() < 1e-3));
		let mut wet = FlatGround { height: 9.0, wet: |_| true };
		anyhow::ensure!(
			build_shepherds_village(&mut wet, cell, &DevelopmentConfig::default()).is_none(),
			"water everywhere rejects every hut"
		);
		Ok(())
	}

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
		let confines = buildings::Confines::new(
			Aabb3d::from_min_max(
				Vec3::new(center.x - 3.0, 10.0, center.y - 4.0),
				Vec3::new(center.x + 3.0, 16.0, center.y + 4.0),
			),
			yaw,
			buildings::Openings::new(),
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
