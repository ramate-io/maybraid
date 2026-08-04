//! Dining room plan packer: host-scaled table + optional filler.

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::fit::{Confines, FitError};
use crate::placer::{
	try_free_extent, CommitEffect, FreeExtentKnobs, KindSpec, Predicate, ProgramTier, ProposeKnobs,
	SoftGoalRole,
};

use crate::usage_areas::livable_quarters::pack::{
	init_host, pack_kinds, xz_area, PackKnobs, PackHost,
};

pub const MIN_AREA: f32 = 2.2 * 2.0;

const WALL_EPS: f32 = 0.08;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiningKind {
	Filler,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DiningRoomPacked {
	pub tables: Vec<Aabb3d>,
	pub fillers: Vec<Aabb3d>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiningRoomRegions {
	pub spaciousness: f32,
	pub occupancy: f32,
}

impl DiningRoomRegions {
	fn filler_catalog() -> &'static [KindSpec<DiningKind>] {
		static CATALOG: [KindSpec<DiningKind>; 1] = [KindSpec {
			id: DiningKind::Filler,
			tier: ProgramTier::Filler,
			weight: 0.4,
			max_count: Some(2),
			soft_goal: SoftGoalRole::None,
			propose: ProposeKnobs::FreeExtent {
				min_x: 0.35,
				max_x: 0.55,
				min_z: 0.35,
				max_z: 0.55,
				height: 0.5,
				prefer_wall: true,
			},
			predicates: &[Predicate::InHost, Predicate::ClearOfKeepOuts],
			commit: CommitEffect::SolidFootprint,
			structure_budget: false,
		}];
		&CATALOG
	}

	pub fn pack(
		&self,
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<DiningRoomPacked, FitError> {
		let mut host = init_host(confines)?;
		if host.room_area + 1e-3 < MIN_AREA {
			return Err(FitError::TooSmall {
				reason: "dining room",
			});
		}
		let cfg = NoiseConfig::new(noise);
		let Some(table) = place_table(&host, &cfg, self.spaciousness) else {
			return Err(FitError::TooSmall {
				reason: "dining table",
			});
		};
		host.clearances
			.push(procedural_common::aabb3_to_plan(&table, procedural_common::PlanAxes::XZ));

		let mut packed = DiningRoomPacked {
			tables: vec![table],
			fillers: Vec::new(),
		};

		// Soft-goal already met (table placed); pack optional fillers under remaining budget.
		let table_frac = xz_area(&packed.tables[0]) / host.room_area;
		let remain = (self.occupancy - table_frac).max(0.05);
		let placed = pack_kinds(
			Self::filler_catalog(),
			PackKnobs {
				spaciousness: self.spaciousness,
				furniture_occupancy: remain,
				structure_occupancy: remain.max(0.7),
				max_steps: 8,
			},
			&mut host,
			confines,
			noise,
			|_p| true,
		)?;
		for (kind, aabb) in placed {
			match kind {
				DiningKind::Filler => packed.fillers.push(aabb),
			}
		}
		Ok(packed)
	}
}

/// Size the table from the host plan: grow with the long/short spans when space allows.
fn place_table(host: &PackHost, cfg: &NoiseConfig, spaciousness: f32) -> Option<Aabb3d> {
	let span_x = (host.host.max.x - host.host.min.x).max(0.0);
	let span_z = (host.host.max.y - host.host.min.y).max(0.0);
	let long_span = span_x.max(span_z);
	let short_span = span_x.min(span_z);

	// Target ~42–72% of the long axis and ~28–48% of the short axis, scaled by spaciousness.
	let along_lo = (long_span * 0.42 * spaciousness.min(1.35)).clamp(1.2, 4.0);
	let along_hi = (long_span * 0.72 * spaciousness.min(1.45))
		.clamp(along_lo + 0.15, 6.0)
		.min(long_span - 0.35);
	let depth_lo = (short_span * 0.28 * spaciousness.min(1.25)).clamp(0.65, 1.4);
	let depth_hi = (short_span * 0.48 * spaciousness.min(1.35))
		.clamp(depth_lo + 0.1, 2.2)
		.min(short_span - 0.35);

	if along_hi < along_lo + 1e-3 || depth_hi < depth_lo + 1e-3 {
		return None;
	}

	let c = Vec3::new(
		(host.host.min.x + host.host.max.x) * 0.5,
		host.host3.min.y,
		(host.host.min.y + host.host.max.y) * 0.5,
	);
	let along = cfg.sample_range_f32_4d(along_lo, along_hi, c.x, c.y, c.z, 60.0);
	let depth = cfg.sample_range_f32_4d(depth_lo, depth_hi, c.x, c.y, c.z, 61.0);
	let height = 0.75 * spaciousness.min(1.15);

	// Prefer long axis along the room's longer plan span; try the swap as fallback.
	let primary = if span_x >= span_z {
		Vec3::new(along, height, depth)
	} else {
		Vec3::new(depth, height, along)
	};
	let secondary = Vec3::new(primary.z, primary.y, primary.x);

	for (i, extent) in [primary, secondary].into_iter().enumerate() {
		if let Some(table) = try_free_extent(
			&host.host3,
			host.host,
			&host.clearances,
			cfg,
			70 + i as u32,
			FreeExtentKnobs {
				extent,
				prefer_wall: false,
				wall_eps: WALL_EPS,
				attempts: 24,
			},
		) {
			return Some(table);
		}
	}
	None
}
