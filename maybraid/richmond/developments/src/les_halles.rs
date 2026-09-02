//! Flattened Les Halles development: one mixed-use monotower + stairs + roof.
//!
//! Hosts are siblings — no parent development LodScene. Each storey, each
//! storey-to-storey shaft stairwell, and the outer [`PitchedRoof`] is its own
//! host. The top storey has a gallery ceiling and no stairwell; the penultimate
//! well walks off on that last gallery.

use bevy_math::bounding::Aabb3d;
use bevy_math::{Vec2, Vec3};
use procedural_common::{NoiseConfig, NoiseParams};
use richmond_building_components::panels::PanelStyle;
use richmond_buildings::{
	Confines, ConnectingStairwell, FillableRegions, Fit, FitError, LesHallesCommercialUsage,
	LesHallesFloorPlan, LesHallesLivableUsage, LesHallesParameterized, LesHallesUsagePlan,
	MixedUseLesHallesMonotower, MixedUseLesHallesStorey, Openings, PitchedRoof, PitchedRoofParams,
	RectRingFloorSlab, RoofHalf, StairwellKind, WellAabb, WellSide,
};

/// [`fit_windows_on_run`] emits nothing below this density.
const WINDOW_DENSITY_GATE: f32 = 0.08;
/// Floor used when the monotower sample falls under [`WINDOW_DENSITY_GATE`].
const MIN_WINDOW_DENSITY: f32 = 0.35;
/// Tread fill so wall-hugging flights stay wider than a 0.8 m capsule.
const WALK_TREAD_FILL: f32 = 0.85;
/// Eave drip past the outer facade (meters).
const ROOF_DRIP_M: f32 = 0.8;
/// Same salt as [`MixedUseLesHallesMonotower`] floor noise.
const SALT_FLOOR: f32 = 11.0;

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
	/// Storeys, then one rectangular stairwell per climb per shaft, then the roof.
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
		let (mut tower, _) = MixedUseLesHallesMonotower::fit_to_confines(confines, noise)?;
		if tower.floors.is_empty() {
			return Err(FitError::TooSmall { reason: "storeys" });
		}
		finish_tower(&mut tower, noise)?;
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

fn finish_tower(
	tower: &mut MixedUseLesHallesMonotower,
	noise: NoiseParams,
) -> Result<(), FitError> {
	let rebuild_all = tower.parameterized.opening_density < WINDOW_DENSITY_GATE;
	if rebuild_all {
		tower.parameterized.opening_density = MIN_WINDOW_DENSITY;
	}
	let last_i = tower.floors.len().saturating_sub(1);
	let start = if rebuild_all { 0 } else { last_i };
	let slots = tower.shaft_slots.clone();
	let params = tower.parameterized.clone();
	for i in start..tower.floors.len() {
		let commercial = tower.floors[i].is_commercial();
		let confines = storey_confines(tower.floors[i].floor_plan(), &params, &slots);
		let ceiling = if i == last_i { RectRingFloorSlab::Solid } else { RectRingFloorSlab::None };
		let (floor_plan, regions) = LesHallesFloorPlan::from_parameterized_with_ceiling(
			params.clone(),
			&confines,
			ceiling,
		)?;
		let floor_noise = floor_noise(noise, i);
		tower.floors[i] = if commercial {
			let (usage, _) = LesHallesCommercialUsage::paint(regions, floor_noise)?;
			MixedUseLesHallesStorey::Commercial { floor_plan, usage }
		} else {
			let (usage, _) = LesHallesLivableUsage::paint(regions, floor_noise)?;
			MixedUseLesHallesStorey::Livable { floor_plan, usage }
		};
	}
	Ok(())
}

fn storey_confines(
	plan: &LesHallesFloorPlan,
	params: &LesHallesParameterized,
	slots: &[usize],
) -> Confines {
	let y0 = plan.center_xz.y;
	let h = plan.storey_height;
	let hx = plan.outer.x * 0.5;
	let hz = plan.outer.y * 0.5;
	let c = plan.center_xz;
	let bounds = Aabb3d::from_min_max(
		Vec3::new(c.x - hx, y0, c.z - hz),
		Vec3::new(c.x + hx, y0 + h, c.z + hz),
	);
	let empty = Confines::new(bounds, plan.roll, Openings::new());
	let openings = LesHallesFloorPlan::shaft_requests_for_slots(params, &empty, slots);
	Confines::new(bounds, plan.roll, openings)
}

fn floor_noise(noise: NoiseParams, floor_i: usize) -> NoiseParams {
	let mut n = noise;
	n.seed = noise.seed.wrapping_add(floor_i as i32 * 97);
	let _ = NoiseConfig::new(n).sample_unit_4d(0.0, floor_i as f32, 0.0, SALT_FLOOR);
	n
}

fn stairwells_for(tower: &MixedUseLesHallesMonotower) -> Vec<ConnectingStairwell> {
	let n = tower.floors.len();
	if n < 2 {
		return Vec::new();
	}
	let last_well_i = n - 2;
	let mut out = Vec::new();
	for (floor_i, floor) in tower.floors.iter().enumerate().take(last_well_i + 1) {
		let plan = floor.floor_plan();
		let upper_landing = floor_i == last_well_i;
		for shaft in &plan.shaft_bounds {
			let side = courtyard_well_side(plan.center_xz, *shaft);
			let well = WellAabb::from_plan(
				Vec3::from(shaft.min),
				Vec3::from(shaft.max),
				side,
				side,
				WALK_TREAD_FILL,
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
	let params = hip_over_outer(plan.outer, eave_y + rise, eave_y, ridge_inset, ROOF_DRIP_M);
	let delta = Vec3::new(plan.center_xz.x, 0.0, plan.center_xz.z);
	PitchedRoof::new(offset_roof_params(params, delta))
}

/// Hip whose wall plates sit on `outer` and eaves drip past that footprint.
fn hip_over_outer(
	outer: Vec2,
	ridge_height: f32,
	eave_height: f32,
	ridge_inset: f32,
	drip: f32,
) -> PitchedRoofParams {
	let half_x = outer.x * 0.5;
	let half_z = outer.y * 0.5;
	let eave_x = half_x + drip;
	let eave_z = half_z + drip;
	let ridge_half = (half_x - ridge_inset).max(0.0);
	let ridge =
		(Vec3::new(-ridge_half, ridge_height, 0.0), Vec3::new(ridge_half, ridge_height, 0.0));
	let eave_pos =
		(Vec3::new(-eave_x, eave_height, eave_z), Vec3::new(eave_x, eave_height, eave_z));
	let eave_neg =
		(Vec3::new(-eave_x, eave_height, -eave_z), Vec3::new(eave_x, eave_height, -eave_z));
	let wall_pos =
		(Vec3::new(-half_x, eave_height, half_z), Vec3::new(half_x, eave_height, half_z));
	let wall_neg =
		(Vec3::new(-half_x, eave_height, -half_z), Vec3::new(half_x, eave_height, -half_z));
	let ends = (true, true);
	let pos = RoofHalf::new(ridge, eave_pos, wall_pos)
		.draw_in_wall_line(true)
		.draw_in_half_hip(ends);
	let neg = RoofHalf::new(ridge, eave_neg, wall_neg)
		.draw_in_wall_line(true)
		.draw_in_half_hip(ends);
	PitchedRoofParams::new([pos, neg])
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
	use richmond_buildings::OpeningLabel;

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
		let climbs = dev.tower.floor_count() - 1;
		assert_eq!(dev.stairwells.len(), climbs * shafts);
		let hosts = dev.hosts();
		assert_eq!(hosts.len(), dev.tower.floor_count() + dev.stairwells.len() + 1);
		assert!(matches!(hosts.first(), Some(MixedUseLesHallesHost::Storey(_))));
		assert!(matches!(hosts.last(), Some(MixedUseLesHallesHost::Roof(_))));
	}

	#[test]
	fn top_floor_has_ceiling_and_no_stairwell() {
		let dev = fit_dev(42);
		let last = dev.tower.floors.last().unwrap().floor_plan();
		assert!(last.gallery.has_ceiling());
		for floor in dev.tower.floors.iter().rev().skip(1) {
			assert!(!floor.floor_plan().gallery.has_ceiling());
		}
		let last_y0 = last.center_xz.y;
		assert!(dev.stairwells.iter().all(|s| s.well().min().y + 1e-3 < last_y0));
	}

	#[test]
	fn shell_has_outer_apertures() {
		let dev = fit_dev(1337);
		assert!(
			dev.tower.floors.iter().all(|f| {
				f.floor_plan().openings.iter().any(|(id, o)| {
					id.as_str().contains("outer_aperture")
						&& matches!(o.label, OpeningLabel::Aperture)
				})
			}),
			"expected outer apertures on every storey"
		);
	}

	#[test]
	fn stairwells_are_rectangular_same_side_courtyard() {
		let dev = fit_dev(7);
		let last_well_i = dev.tower.floor_count() - 2;
		let mut i = 0usize;
		for (floor_i, floor) in dev.tower.floors.iter().enumerate() {
			if floor_i > last_well_i {
				break;
			}
			let plan = floor.floor_plan();
			for shaft in &plan.shaft_bounds {
				let well = dev.stairwells[i].well();
				let side = courtyard_well_side(plan.center_xz, *shaft);
				assert_eq!(dev.stairwells[i].kind(), StairwellKind::Rectangular);
				assert_eq!(well.walk_on, side);
				assert_eq!(well.walk_off, side);
				assert_eq!(well.walk_on, well.walk_off);
				if floor_i == last_well_i {
					assert!(dev.stairwells[i].upper_landing().is_some(), "last-gallery pad");
				} else {
					assert!(dev.stairwells[i].upper_landing().is_none(), "shared-face omit");
				}
				i += 1;
			}
		}
		assert_eq!(i, dev.stairwells.len());
	}

	#[test]
	fn roof_eaves_drip_past_outer_wall_plates() {
		let dev = fit_dev(11);
		let plan = dev.tower.floors.last().unwrap().floor_plan();
		let eave_y = plan.center_xz.y + plan.storey_height;
		let eave = dev.roof.params().halves[0].eave_line.0;
		let wall = dev.roof.params().halves[0].wall_line.0;
		assert!((eave.y - eave_y).abs() < 1e-3, "eave y {eave:?} vs {eave_y}");
		assert!(
			((eave.z - plan.center_xz.z).abs() - (plan.outer.y * 0.5 + ROOF_DRIP_M)).abs() < 1e-3,
			"eave z {eave:?} vs outer {} + drip",
			plan.outer.y
		);
		assert!(
			((wall.z - plan.center_xz.z).abs() - plan.outer.y * 0.5).abs() < 1e-3,
			"wall z {wall:?} vs outer {}",
			plan.outer.y
		);
	}
}
