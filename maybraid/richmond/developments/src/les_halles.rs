//! Flattened Les Halles development: one mixed-use monotower + per-floor stairs + roof.
//!
//! Hosts are siblings — no parent development LodScene. Each storey,
//! each per-floor shaft stairwell, and the outer [`PitchedRoof`] is its own host.

use bevy_math::bounding::Aabb3d;
use bevy_math::{Vec2, Vec3};
use procedural_common::NoiseParams;
use richmond_building_components::panels::PanelStyle;
use richmond_buildings::{
	Confines, ConnectingStairwell, FillableRegions, Fit, FitError, MixedUseLesHallesMonotower,
	MixedUseLesHallesStorey, PitchedRoof, PitchedRoofParams, RoofHalf, StairwellKind, WellAabb,
	WellSide, TREAD_FILL_DEFAULT,
};

/// Sibling LOD host emitted by [`MixedUseLesHallesDevelopment`].
#[derive(Debug, Clone, PartialEq)]
pub enum MixedUseLesHallesHost {
	Storey(MixedUseLesHallesStorey),
	Stairwell(ConnectingStairwell),
	Roof(PitchedRoof),
}

/// One mixed-use Les Halles monotower exploded into flattened hosts.
#[derive(Debug, Clone, PartialEq)]
pub struct MixedUseLesHallesDevelopment {
	pub tower: MixedUseLesHallesMonotower,
	pub stairwells: Vec<ConnectingStairwell>,
	pub roof: PitchedRoof,
}

impl MixedUseLesHallesDevelopment {
	/// Storeys, then one rectangular stairwell per floor per shaft, then the roof.
	pub fn hosts(&self) -> Vec<MixedUseLesHallesHost> {
		let mut out = Vec::with_capacity(self.tower.floors.len() + self.stairwells.len() + 1);
		for floor in &self.tower.floors {
			out.push(MixedUseLesHallesHost::Storey(floor.clone()));
		}
		for stairwell in &self.stairwells {
			out.push(MixedUseLesHallesHost::Stairwell(stairwell.clone()));
		}
		out.push(MixedUseLesHallesHost::Roof(self.roof.clone()));
		out
	}
}

impl Fit for MixedUseLesHallesDevelopment {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let (tower, _) = MixedUseLesHallesMonotower::fit_to_confines(confines, noise)?;
		if tower.floors.is_empty() {
			return Err(FitError::TooSmall { reason: "storeys" });
		}
		let stairwells = stairwells_for(&tower);
		let roof = roof_for(&tower);
		Ok((
			Self { tower, stairwells, roof },
			FillableRegions { within: Vec::new(), atop: Vec::new() },
		))
	}
}

/// Courtyard-facing cardinal of a shaft: the face whose outward is most toward
/// the plan center. Mid-side shafts have one inner face; corners pick the
/// stronger of the two inner axes.
pub fn courtyard_well_side(center_xz: Vec3, shaft: Aabb3d) -> WellSide {
	let mid = Vec3::from((shaft.min + shaft.max) * 0.5);
	let toward = Vec2::new(center_xz.x - mid.x, center_xz.z - mid.z);
	let x_side = if toward.x >= 0.0 { WellSide::PosX } else { WellSide::NegX };
	let z_side = if toward.y >= 0.0 { WellSide::PosZ } else { WellSide::NegZ };
	if toward.x.abs() >= toward.y.abs() {
		x_side
	} else {
		z_side
	}
}

fn stairwells_for(tower: &MixedUseLesHallesMonotower) -> Vec<ConnectingStairwell> {
	let last_i = tower.floors.len().saturating_sub(1);
	let mut out = Vec::new();
	for (floor_i, floor) in tower.floors.iter().enumerate() {
		let plan = floor.floor_plan();
		let upper_landing = floor_i == last_i;
		for shaft in &plan.shaft_bounds {
			let side = courtyard_well_side(plan.center_xz, *shaft);
			let well = WellAabb::from_plan(
				Vec3::from(shaft.min),
				Vec3::from(shaft.max),
				side,
				side,
				TREAD_FILL_DEFAULT,
			);
			out.push(
				ConnectingStairwell::from_well_kind(
					PanelStyle::RoughStonework,
					well,
					StairwellKind::Rectangular,
				)
				.with_upper_landing(upper_landing),
			);
		}
	}
	out
}

fn roof_for(tower: &MixedUseLesHallesMonotower) -> PitchedRoof {
	let plan = tower
		.floors
		.last()
		.expect("fit_to_confines rejects an empty stack")
		.floor_plan();
	let eave_y = plan.center_xz.y + plan.storey_height;
	let rise = (plan.outer.x.min(plan.outer.y) * 0.12).clamp(3.0, 8.0);
	let ridge_inset = (plan.outer.x * 0.08).clamp(1.5, 6.0);
	let params = PitchedRoofParams::rectangular_hip(plan.outer, eave_y + rise, eave_y, ridge_inset);
	let delta = Vec3::new(plan.center_xz.x, 0.0, plan.center_xz.z);
	PitchedRoof::new(offset_roof_params(params, delta))
}

fn offset_roof_params(mut params: PitchedRoofParams, delta: Vec3) -> PitchedRoofParams {
	for half in &mut params.halves {
		*half = offset_roof_half(half.clone(), delta);
	}
	params
}

fn offset_roof_half(half: RoofHalf, delta: Vec3) -> RoofHalf {
	RoofHalf {
		ridge_line: (half.ridge_line.0 + delta, half.ridge_line.1 + delta),
		eave_line: (half.eave_line.0 + delta, half.eave_line.1 + delta),
		wall_line: (half.wall_line.0 + delta, half.wall_line.1 + delta),
		..half
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;

	fn large_tower_bounds() -> Aabb3d {
		Aabb3d::from_min_max(Vec3::new(-36.0, 0.0, -27.0), Vec3::new(36.0, 16.0, 27.0))
	}

	fn fit_dev(seed: i32) -> MixedUseLesHallesDevelopment {
		let confines = Confines::from_bounds(large_tower_bounds());
		let noise = NoiseParams { seed, ..NoiseParams::default() };
		MixedUseLesHallesDevelopment::fit_to_confines(&confines, noise).unwrap().0
	}

	#[test]
	fn courtyard_side_mid_south_is_pos_z() {
		let center = Vec3::new(0.0, 0.0, 0.0);
		let shaft = Aabb3d::from_min_max(Vec3::new(-1.0, 0.0, -10.0), Vec3::new(1.0, 4.0, -8.0));
		assert_eq!(courtyard_well_side(center, shaft), WellSide::PosZ);
	}

	#[test]
	fn courtyard_side_corner_sw_picks_stronger_inner() {
		let center = Vec3::new(0.0, 0.0, 0.0);
		// Equal inner components → X wins (`abs(x) >= abs(z)`).
		let shaft =
			Aabb3d::from_min_max(Vec3::new(-12.0, 0.0, -12.0), Vec3::new(-10.0, 4.0, -10.0));
		assert_eq!(courtyard_well_side(center, shaft), WellSide::PosX);
	}

	#[test]
	fn development_flattens_storeys_stairs_and_roof() {
		let dev = fit_dev(42);
		assert!(dev.tower.floor_count() >= 2);
		let shafts = dev.tower.floors[0].floor_plan().shaft_bounds.len();
		assert!(shafts >= 1);
		assert_eq!(dev.stairwells.len(), dev.tower.floor_count() * shafts);
		let hosts = dev.hosts();
		assert_eq!(hosts.len(), dev.tower.floor_count() + dev.stairwells.len() + 1);
		assert!(matches!(hosts.first(), Some(MixedUseLesHallesHost::Storey(_))));
		assert!(matches!(hosts.last(), Some(MixedUseLesHallesHost::Roof(_))));
	}

	#[test]
	fn stairwells_are_rectangular_same_side_courtyard() {
		let dev = fit_dev(7);
		let last_i = dev.tower.floor_count() - 1;
		let mut i = 0usize;
		for (floor_i, floor) in dev.tower.floors.iter().enumerate() {
			let plan = floor.floor_plan();
			for shaft in &plan.shaft_bounds {
				let well = dev.stairwells[i].well();
				let side = courtyard_well_side(plan.center_xz, *shaft);
				assert_eq!(dev.stairwells[i].kind(), StairwellKind::Rectangular);
				assert_eq!(well.walk_on, side);
				assert_eq!(well.walk_off, side);
				assert_eq!(well.walk_on, well.walk_off);
				if floor_i == last_i {
					assert!(dev.stairwells[i].upper_landing().is_some(), "top gallery pad");
				} else {
					assert!(dev.stairwells[i].upper_landing().is_none(), "shared-face omit");
				}
				i += 1;
			}
		}
		assert_eq!(i, dev.stairwells.len());
	}

	#[test]
	fn roof_sits_on_outer_rectangle_at_last_gallery() {
		let dev = fit_dev(11);
		let plan = dev.tower.floors.last().unwrap().floor_plan();
		let eave_y = plan.center_xz.y + plan.storey_height;
		let eave = dev.roof.params().halves[0].eave_line.0;
		assert!((eave.y - eave_y).abs() < 1e-3, "eave y {eave:?} vs {eave_y}");
		assert!(
			((eave.x - plan.center_xz.x).abs() - plan.outer.x * 0.5).abs() < 1e-3,
			"eave x {eave:?} vs outer {}",
			plan.outer.x
		);
	}
}
