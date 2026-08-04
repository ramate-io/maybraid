//! Shared greedy furniture pack loop for livable-quarter rooms.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::Vec3;
use procedural_common::{aabb2_area, aabb3_to_plan, NoiseConfig, NoiseParams, PlanAxes};

use crate::fit::{Confines, FitError};
use crate::usage_areas::clearance::PassageClearance;
use crate::placer::predicates::{all_pass, PredicateCtx};
use crate::placer::{
	pick_kind, FreeExtentKnobs, KindSpec, OccupiedBudget, ProposeKnobs, WallLongKnobs,
};
use crate::placer::{try_free_extent, try_wall_long};

const WALL_EPS: f32 = 0.08;
const PROPOSE_ATTEMPTS: u32 = 14;

/// Host plan + passage keep-outs for a livable-quarter pack.
pub struct PackHost {
	pub host3: Aabb3d,
	pub host: Aabb2d,
	pub clearances: Vec<Aabb2d>,
	pub room_area: f32,
}

/// Knobs shared by livable-quarter furniture loops.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PackKnobs {
	pub spaciousness: f32,
	pub furniture_occupancy: f32,
	pub structure_occupancy: f32,
	pub max_steps: u32,
}

impl PackKnobs {
	pub fn budget(&self, room_area: f32) -> OccupiedBudget {
		OccupiedBudget::new(room_area, self.furniture_occupancy, self.structure_occupancy)
	}
}

/// Initialize host geometry and passage clearance bands.
pub fn init_host(confines: &Confines) -> Result<PackHost, FitError> {
	let host3 = confines.bounds;
	let host = aabb3_to_plan(&host3, PlanAxes::XZ);
	let passage_faces = PassageClearance::collect_faces(confines, host);
	if passage_faces.is_empty() {
		return Err(FitError::TooSmall {
			reason: "passage",
		});
	}
	let clearances = PassageClearance::bands_std(host, &passage_faces);
	let room_area = aabb2_area(host).max(1e-4);
	Ok(PackHost {
		host3,
		host,
		clearances,
		room_area,
	})
}

/// Plan-area of an AABB footprint on XZ.
pub fn xz_area(aabb: &Aabb3d) -> f32 {
	let e = aabb.max - aabb.min;
	e.x.max(0.0) * e.z.max(0.0)
}

fn propose_candidate<Kind>(
	spec: &KindSpec<Kind>,
	spaciousness: f32,
	host3: &Aabb3d,
	host: Aabb2d,
	clearances: &[Aabb2d],
	cfg: &NoiseConfig,
	salt: u32,
	center: Vec3,
) -> Option<Aabb3d> {
	match spec.propose {
		ProposeKnobs::FreeExtent {
			min_x,
			max_x,
			min_z,
			max_z,
			height,
			prefer_wall,
		} => {
			let sx = cfg.sample_range_f32_4d(
				min_x * spaciousness,
				max_x * spaciousness,
				center.x,
				center.y,
				center.z,
				salt as f32 + 1.0,
			);
			let sz = cfg.sample_range_f32_4d(
				min_z * spaciousness,
				max_z * spaciousness,
				center.x,
				center.y,
				center.z,
				salt as f32 + 2.0,
			);
			let h = height * spaciousness.min(1.2);
			try_free_extent(
				host3,
				host,
				clearances,
				cfg,
				salt,
				FreeExtentKnobs {
					extent: Vec3::new(sx, h, sz),
					prefer_wall,
					wall_eps: WALL_EPS,
					attempts: PROPOSE_ATTEMPTS,
				},
			)
		}
		ProposeKnobs::WallLong {
			along_min,
			along_max,
			depth_min,
			depth_max,
			height,
		} => {
			let along = cfg.sample_range_f32_4d(
				along_min * spaciousness,
				along_max * spaciousness,
				center.x,
				center.y,
				center.z,
				salt as f32 + 3.0,
			);
			let depth = cfg.sample_range_f32_4d(
				depth_min * spaciousness,
				depth_max * spaciousness,
				center.x,
				center.y,
				center.z,
				salt as f32 + 4.0,
			);
			let h = height * spaciousness.min(1.2);
			try_wall_long(
				host3,
				host,
				clearances,
				cfg,
				salt,
				WallLongKnobs {
					extent: Vec3::new(along, h, depth),
					wall_eps: WALL_EPS,
					attempts: PROPOSE_ATTEMPTS,
				},
			)
		}
		ProposeKnobs::EnclosedRoom => None,
	}
}

fn count_kind<Kind: Copy + PartialEq>(placed: &[(Kind, Aabb3d)], kind: Kind) -> usize {
	placed.iter().filter(|(k, _)| *k == kind).count()
}

/// Greedy kind loop: pick → propose → predicate → budget → commit.
pub fn pack_kinds<Kind: Copy + PartialEq>(
	catalog: &[KindSpec<Kind>],
	knobs: PackKnobs,
	host: &mut PackHost,
	confines: &Confines,
	noise: NoiseParams,
	soft_goal_met: impl Fn(&[(Kind, Aabb3d)]) -> bool,
) -> Result<Vec<(Kind, Aabb3d)>, FitError> {
	let cfg = NoiseConfig::new(noise);
	let center = confines.center();
	let mut budget = knobs.budget(host.room_area);
	let mut placed = Vec::new();

	for step in 0..knobs.max_steps {
		if budget.furniture_full() {
			break;
		}
		let Some(kind) = pick_kind(
			catalog,
			&cfg,
			step,
			soft_goal_met(&placed),
			|k| count_kind(&placed, k),
		) else {
			break;
		};
		let Some(spec) = catalog.iter().find(|s| s.id == kind) else {
			continue;
		};
		let Some(candidate) = propose_candidate(
			spec,
			knobs.spaciousness,
			&host.host3,
			host.host,
			&host.clearances,
			&cfg,
			step + 100,
			center,
		) else {
			continue;
		};
		let plan = aabb3_to_plan(&candidate, PlanAxes::XZ);
		let ctx = PredicateCtx {
			host: host.host,
			candidate: plan,
			clearances: &host.clearances,
			door_clear: None,
			wall_eps: WALL_EPS,
		};
		if !all_pass(spec.predicates, ctx) {
			continue;
		}
		let add = xz_area(&candidate);
		if !budget.accepts(add, spec.structure_budget) {
			continue;
		}
		budget.commit(add);
		host.clearances.push(plan);
		placed.push((kind, candidate));
	}

	Ok(placed)
}
