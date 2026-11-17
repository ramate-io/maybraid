//! Authentic pre-change fitting policy, retained only for offline comparison.
//! Legacy usage is not deterministic; gameplay invariants replace layout parity.
use super::*;
use crate::{CurtainRing, RingFort, Terrace, TerraceDevelopment};
use buildings::{
	LesHallesArcadeUsage, LesHallesCommercialUsage, LesHallesFloorPlan, LesHallesLivableUsage,
	LesHallesOpeningProgram, LesHallesUsagePlan, Openings,
};
use std::cell::Cell;
use std::hint::black_box;
use std::time::{Duration, Instant};
const SALT_FLOOR: f32 = 11.0;
thread_local! { static LEGACY: Cell<bool> = const { Cell::new(false) }; }
pub(crate) fn legacy_enabled() -> bool {
	LEGACY.with(Cell::get)
}
struct LegacyMode(bool);
impl LegacyMode {
	fn new(legacy: bool) -> Self {
		Self(LEGACY.with(|mode| mode.replace(legacy)))
	}
}
impl Drop for LegacyMode {
	fn drop(&mut self) {
		LEGACY.with(|mode| mode.set(self.0));
	}
}
fn with_mode<T>(legacy: bool, run: impl FnOnce() -> T) -> T {
	let _mode = LegacyMode::new(legacy);
	run()
}
pub(super) fn legacy_fit(
	confines: &Confines,
	noise: NoiseParams,
	sample_shell: fn(&Confines, NoiseParams) -> Result<LesHallesParameterized, FitError>,
) -> Result<(MixedUseLesHallesDevelopment, FillableRegions), FitError> {
	let (mut tower, _) = MixedUseLesHallesMonotower::fit_sampled(confines, noise, sample_shell)?;
	finish_tower(&mut tower, noise)?;
	MixedUseLesHallesDevelopment::assemble(tower, noise)
}
pub(crate) fn legacy_open_top(ring: &mut CurtainRing) {
	if legacy_enabled() {
		open_last_storey_for_terrace(ring);
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
		let arcade = tower.floors[i].is_arcade();
		let commercial = tower.floors[i].is_commercial();
		let confines = storey_confines(tower.floors[i].floor_plan(), &params, &slots);
		let ceiling = if i == last_i { RectRingFloorSlab::Solid } else { RectRingFloorSlab::None };
		let program = if arcade {
			LesHallesOpeningProgram::GroundArcade
		} else {
			LesHallesOpeningProgram::CommercialGallery
		};
		let (floor_plan, regions) = LesHallesFloorPlan::from_parameterized_with(
			params.clone(),
			&confines,
			ceiling,
			program,
		)?;
		let floor_noise = floor_noise(noise, i);
		tower.floors[i] = if arcade {
			let keep_outs = LesHallesArcadeUsage::pillar_keep_outs_of(&floor_plan);
			let (usage, _) =
				LesHallesArcadeUsage::paint_avoiding(regions, floor_noise, &keep_outs)?;
			MixedUseLesHallesStorey::Arcade { floor_plan, usage, wall_material: None }
		} else if commercial {
			let (usage, _) = LesHallesCommercialUsage::paint(regions, floor_noise)?;
			MixedUseLesHallesStorey::Commercial { floor_plan, usage, wall_material: None }
		} else {
			let (usage, _) = LesHallesLivableUsage::paint(regions, floor_noise)?;
			MixedUseLesHallesStorey::Livable { floor_plan, usage, wall_material: None }
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

fn open_last_storey_for_terrace(ring: &mut CurtainRing) {
	let Some(last) = ring.halles.tower.floors.last_mut() else {
		return;
	};
	let plan = last.floor_plan_mut();
	let mut params = plan.gallery.params().clone();
	params.ceiling = RectRingFloorSlab::None;
	plan.gallery = params.build();
	plan.ceiling = RectRingFloorSlab::None;
}

fn traced_terraces() -> [Terrace; 2] {
	[
		([1998.8796, 0.0, 1130.1075], [2400.0, 1.0, 1610.566], 35.708984),
		([800.0, 0.0, 1610.566], [1104.924, 1.0, 2086.4426], -193.13463),
	]
	.map(|(min, max, height)| {
		Terrace::new::<RingFort>(
			Aabb3d::from_min_max(Vec3::from_array(min), Vec3::from_array(max)),
			height,
			1337,
		)
	})
}

fn check_development(dev: &MixedUseLesHallesDevelopment, curtain: bool) -> anyhow::Result<()> {
	anyhow::ensure!(!dev.tower.floors.is_empty());
	anyhow::ensure!(dev.tower.parameterized.opening_density >= WINDOW_DENSITY_GATE);
	let last = dev.tower.floors.len() - 1;
	for (i, floor) in dev.tower.floors.iter().enumerate() {
		let plan = floor.floor_plan();
		anyhow::ensure!(plan.center_xz.is_finite() && plan.outer.is_finite());
		anyhow::ensure!(plan.outer.min_element() > 0.0 && plan.storey_height > 0.0);
		anyhow::ensure!(plan.gallery.has_ceiling() == (!curtain && i == last));
		anyhow::ensure!(floor.is_arcade() == (i == 0));
		if i > 0 {
			anyhow::ensure!(floor.is_commercial() == (i <= dev.tower.n_commercial));
		}
		anyhow::ensure!(plan.shaft_bounds.len() == dev.tower.shaft_slots.len());
		anyhow::ensure!(plan.shaft_inbound.len() == plan.shaft_bounds.len());
		for (shaft, ids) in plan.shaft_bounds.iter().zip(&plan.shaft_inbound) {
			anyhow::ensure!(!ids.is_empty(), "shaft lost owning input");
			for id in ids {
				let opening = plan
					.openings
					.get(id)
					.ok_or_else(|| anyhow::anyhow!("missing shaft owner {id:?}"))?;
				anyhow::ensure!(
					opening.label == buildings::OpeningLabel::Shaft && opening.bounds == *shaft
				);
			}
		}
	}
	let expected_stairs = last * dev.tower.shaft_slots.len();
	anyhow::ensure!(dev.stairwells.len() == expected_stairs, "missing stair connectivity");
	Ok(())
}

#[test]
fn finalized_shell_precedes_single_usage_pass_across_seed_corpus() -> anyhow::Result<()> {
	let confines =
		Confines::from_bounds(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(72.0, 20.0, 72.0)));
	let mut accepted = [0usize; 2];
	let mut commercial = 0;
	for seed in 0..16 {
		let noise = NoiseParams { seed, ..NoiseParams::default() };
		for curtain in [false, true] {
			let fitted = if curtain {
				MixedUseLesHallesDevelopment::fit_curtain(&confines, noise)
			} else {
				MixedUseLesHallesDevelopment::fit_to_confines(&confines, noise)
			};
			if let Ok((dev, _)) = fitted {
				check_development(&dev, curtain)?;
				accepted[usize::from(curtain)] += 1;
				commercial += dev.tower.floors.iter().filter(|floor| floor.is_commercial()).count();
			}
		}
	}
	anyhow::ensure!(
		accepted.iter().all(|count| *count >= 8),
		"too few accepted fixtures {accepted:?}"
	);
	anyhow::ensure!(commercial > 0, "commercial program not covered");
	Ok(())
}

fn zero_density(
	confines: &Confines,
	noise: NoiseParams,
) -> Result<LesHallesParameterized, FitError> {
	let mut params = LesHallesParameterized::sample_monotower(confines, noise)?;
	params.opening_density = 0.0;
	Ok(params)
}

#[test]
fn low_window_density_is_finalized_before_usage_fitting() -> anyhow::Result<()> {
	let confines =
		Confines::from_bounds(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(72.0, 20.0, 72.0)));
	let (development, _) = MixedUseLesHallesDevelopment::fit_finalized(
		&confines,
		NoiseParams { seed: 42, ..NoiseParams::default() },
		zero_density,
		RectRingFloorSlab::Solid,
	)?;
	anyhow::ensure!(development.tower.parameterized.opening_density == MIN_WINDOW_DENSITY);
	for floor in &development.tower.floors {
		anyhow::ensure!(floor.floor_plan().parameterized.opening_density == MIN_WINDOW_DENSITY);
	}
	check_development(&development, false)
}

#[test]
fn explicit_inbound_shaft_identity_survives_density_finalization() -> anyhow::Result<()> {
	let owner = buildings::OpeningId::new("authored_stair_connection");
	let mut openings = Openings::new();
	openings.insert(
		owner.clone(),
		buildings::Opening::new(
			Aabb3d::from_min_max(Vec3::new(1.0, 0.0, 1.0), Vec3::new(3.0, 20.0, 3.0)),
			buildings::OpeningLabel::Shaft,
		),
	);
	let confines =
		Confines::new(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(72.0, 20.0, 72.0)), 0.0, openings);
	let (development, _) = MixedUseLesHallesDevelopment::fit_finalized(
		&confines,
		NoiseParams { seed: 42, ..NoiseParams::default() },
		zero_density,
		RectRingFloorSlab::Solid,
	)?;
	check_development(&development, false)?;
	let mut first_slot_and_xz = None;
	for floor in &development.tower.floors {
		let plan = floor.floor_plan();
		anyhow::ensure!(plan.parameterized.opening_density == MIN_WINDOW_DENSITY);
		let owned = plan
			.shaft_inbound
			.iter()
			.position(|ids| ids.contains(&owner))
			.ok_or_else(|| anyhow::anyhow!("explicit inbound shaft lost ownership"))?;
		let opening = plan
			.openings
			.get(&owner)
			.ok_or_else(|| anyhow::anyhow!("explicit inbound identity absent from openings"))?;
		anyhow::ensure!(opening.label == buildings::OpeningLabel::Shaft);
		anyhow::ensure!(opening.bounds == plan.shaft_bounds[owned]);
		anyhow::ensure!(opening.bounds.min.y == plan.center_xz.y);
		anyhow::ensure!(opening.bounds.max.y == plan.center_xz.y + plan.storey_height);
		let xz = (
			opening.bounds.min.x,
			opening.bounds.min.z,
			opening.bounds.max.x,
			opening.bounds.max.z,
		);
		let mapped = (plan.shaft_slots[owned], xz);
		if let Some(expected) = first_slot_and_xz {
			anyhow::ensure!(
				mapped == expected,
				"input shaft is not vertically aligned across storeys"
			);
		} else {
			first_slot_and_xz = Some(mapped);
		}
	}
	anyhow::ensure!(first_slot_and_xz.is_some());
	Ok(())
}

#[test]
fn traced_forts_keep_owned_shafts_and_deck_clearances() -> anyhow::Result<()> {
	for terrace in traced_terraces() {
		let fort =
			RingFort::fit_terrace(&terrace).ok_or_else(|| anyhow::anyhow!("traced fort failed"))?;
		anyhow::ensure!(fort.building.connected.topology_is_valid());
		let ring = fort.building.ring().ok_or_else(|| anyhow::anyhow!("missing ring"))?;
		check_development(&ring.halles, true)?;
		anyhow::ensure!(fort.building.keeps().count() == 4);
		anyhow::ensure!(!fort.building.terrace_stairs.is_empty());
		let last = ring.last_plan().ok_or_else(|| anyhow::anyhow!("missing last plan"))?;
		for shaft in &last.shaft_bounds {
			let center = (Vec3::from(shaft.min) + Vec3::from(shaft.max)) * 0.5;
			anyhow::ensure!(
				!fort.building.terrace.deck().floor_covers_xz(center.x, center.z),
				"deck closes shaft"
			);
		}
	}
	Ok(())
}

#[test]
fn malformed_or_tiny_confines_still_return_fit_errors() -> anyhow::Result<()> {
	let tiny = Confines::from_bounds(Aabb3d::from_min_max(Vec3::ZERO, Vec3::splat(0.01)));
	for curtain in [false, true] {
		let result = if curtain {
			MixedUseLesHallesDevelopment::fit_curtain(&tiny, NoiseParams::default())
		} else {
			MixedUseLesHallesDevelopment::fit_to_confines(&tiny, NoiseParams::default())
		};
		anyhow::ensure!(result.is_err());
	}
	Ok(())
}

#[test]
#[ignore = "offline alternating complete RingFort fits and bounded feasibility corpus"]
fn single_paint_full_fit_benchmark() -> anyhow::Result<()> {
	for (index, terrace) in traced_terraces().iter().enumerate() {
		for legacy in [true, false] {
			anyhow::ensure!(
				with_mode(legacy, || RingFort::fit_terrace(terrace)).is_some(),
				"benchmark arm failed"
			);
		}
		let mut totals = [Duration::ZERO; 2];
		let mut calls = [0usize; 2];
		let mut samples = [Vec::new(), Vec::new()];
		for batch in 0..8 {
			for arm in if batch % 2 == 0 { [0, 1] } else { [1, 0] } {
				for _ in 0..8 {
					let start = Instant::now();
					let fitted = black_box(with_mode(arm == 0, || {
						RingFort::fit_terrace(black_box(terrace))
					}))
					.ok_or_else(|| anyhow::anyhow!("timed arm failed"))?;
					let elapsed = start.elapsed();
					totals[arm] += elapsed;
					samples[arm].push(elapsed.as_nanos());
					calls[arm] += 1;
					black_box(fitted);
				}
			}
		}
		for (arm, values) in samples.iter().enumerate() {
			println!("SINGLE_PAINT_SAMPLES fixture={index} arm={arm} ns={values:?}");
		}
		println!(
			"SINGLE_PAINT_BENCH fixture={index} calls={calls:?} total_ns={:?}",
			totals.map(|t| t.as_nanos())
		);
	}
	let mut success = [[0usize; 2]; 2];
	for seed in 0..24 {
		for (family, bounds) in
			[(0, Vec3::new(72.0, 20.0, 72.0)), (1, Vec3::new(160.0, 12.0, 160.0))]
		{
			let confines = Confines::from_bounds(Aabb3d::from_min_max(Vec3::ZERO, bounds));
			let noise = NoiseParams { seed, ..NoiseParams::default() };
			let mut outcomes = [false; 2];
			for arm in 0..2 {
				let fitted = with_mode(arm == 0, || {
					if family == 0 {
						MixedUseLesHallesDevelopment::fit_to_confines(&confines, noise).is_ok()
					} else {
						RingFort::fit_to_confines(&confines, noise).is_ok()
					}
				});
				success[family][arm] += usize::from(fitted);
				outcomes[arm] = fitted;
			}
			if outcomes[0] != outcomes[1] {
				println!("SINGLE_PAINT_FEASIBILITY_CHANGE family={family} seed={seed} legacy_candidate={outcomes:?}");
			}
		}
	}
	println!("SINGLE_PAINT_CORPUS seeds=24 normal_then_fort_legacy_candidate={success:?}");
	Ok(())
}

#[test]
#[ignore = "offline workload counts for the paired benchmark and seed corpus"]
fn single_paint_workload_census() -> anyhow::Result<()> {
	fn fort_counts(fort: &RingFort) -> anyhow::Result<(usize, usize)> {
		let ring = fort.building.ring().ok_or_else(|| anyhow::anyhow!("missing ring"))?;
		Ok((
			ring.halles.tower.floor_count(),
			fort.building.keeps().map(|keep| keep.storey_count()).sum(),
		))
	}
	for (fixture, terrace) in traced_terraces().iter().enumerate() {
		for legacy in [true, false] {
			let fort = with_mode(legacy, || RingFort::fit_terrace(terrace))
				.ok_or_else(|| anyhow::anyhow!("timed workload fixture failed"))?;
			println!("SINGLE_PAINT_TIMED_WORKLOAD fixture={fixture} legacy={legacy} ring_keep_storeys={:?}",fort_counts(&fort)?);
		}
	}
	for seed in 0..24 {
		let noise = NoiseParams { seed, ..NoiseParams::default() };
		for family in 0..2 {
			let extent = if family == 0 {
				Vec3::new(72.0, 20.0, 72.0)
			} else {
				Vec3::new(160.0, 12.0, 160.0)
			};
			let confines = Confines::from_bounds(Aabb3d::from_min_max(Vec3::ZERO, extent));
			for legacy in [true, false] {
				let counts = if family == 0 {
					with_mode(legacy, || {
						MixedUseLesHallesDevelopment::fit_to_confines(&confines, noise)
					})
					.ok()
					.map(|(dev, _)| (dev.tower.floor_count(), 0))
				} else {
					with_mode(legacy, || RingFort::fit_to_confines(&confines, noise))
						.ok()
						.map(|(fort, _)| fort_counts(&fort))
						.transpose()?
				};
				println!("SINGLE_PAINT_CORPUS_WORKLOAD family={family} seed={seed} legacy={legacy} ring_keep_storeys={counts:?}");
			}
		}
	}
	Ok(())
}
