//! Kitchen plan packer: wall counter run + optional island.

use bevy_math::bounding::Aabb3d;
use procedural_common::NoiseParams;

use crate::fit::{Confines, FitError};
use crate::placer::{
	CommitEffect, KindSpec, Predicate, ProgramTier, ProposeKnobs, SoftGoalRole,
};

use crate::usage_areas::livable_quarters::pack::{init_host, pack_kinds, PackKnobs};

pub const MIN_AREA: f32 = 2.4 * 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KitchenKind {
	CounterRun,
	Island,
	Filler,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct KitchenPacked {
	pub counter_runs: Vec<Aabb3d>,
	pub islands: Vec<Aabb3d>,
	pub fillers: Vec<Aabb3d>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KitchenRegions {
	pub spaciousness: f32,
	pub occupancy: f32,
}

impl KitchenRegions {
	fn catalog() -> &'static [KindSpec<KitchenKind>] {
		static CATALOG: [KindSpec<KitchenKind>; 3] = [
			KindSpec {
				id: KitchenKind::CounterRun,
				tier: ProgramTier::Appointed,
				weight: 1.0,
				max_count: Some(2),
				soft_goal: SoftGoalRole::None,
				propose: ProposeKnobs::WallLong {
					along_min: 1.8,
					along_max: 3.6,
					depth_min: 0.55,
					depth_max: 0.75,
					height: 0.9,
				},
				predicates: &[Predicate::InHost, Predicate::ClearOfKeepOuts, Predicate::LongFaceOnWall],
				commit: CommitEffect::SolidFootprint,
				structure_budget: false,
			},
			KindSpec {
				id: KitchenKind::Island,
				tier: ProgramTier::Appointed,
				weight: 0.55,
				max_count: Some(1),
				soft_goal: SoftGoalRole::None,
				propose: ProposeKnobs::FreeExtent {
					min_x: 0.9,
					max_x: 1.6,
					min_z: 0.7,
					max_z: 1.2,
					height: 0.9,
					prefer_wall: false,
				},
				predicates: &[Predicate::InHost, Predicate::ClearOfKeepOuts],
				commit: CommitEffect::SolidFootprint,
				structure_budget: false,
			},
			KindSpec {
				id: KitchenKind::Filler,
				tier: ProgramTier::Filler,
				weight: 0.35,
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
			},
		];
		&CATALOG
	}

	pub fn pack(&self, confines: &Confines, noise: NoiseParams) -> Result<KitchenPacked, FitError> {
		let mut host = init_host(confines)?;
		if host.room_area + 1e-3 < MIN_AREA {
			return Err(FitError::TooSmall {
				reason: "kitchen",
			});
		}
		let mut packed = KitchenPacked::default();
		let catalog = Self::catalog();
		let placed = pack_kinds(
			catalog,
			PackKnobs {
				spaciousness: self.spaciousness,
				furniture_occupancy: self.occupancy,
				structure_occupancy: self.occupancy.max(0.75),
				max_steps: 16,
			},
			&mut host,
			confines,
			noise,
			|p| p.iter().any(|(k, _)| *k == KitchenKind::CounterRun),
		)?;
		for (kind, aabb) in placed {
			match kind {
				KitchenKind::CounterRun => packed.counter_runs.push(aabb),
				KitchenKind::Island => packed.islands.push(aabb),
				KitchenKind::Filler => packed.fillers.push(aabb),
			}
		}
		if packed.counter_runs.is_empty() {
			return Err(FitError::TooSmall {
				reason: "kitchen counter",
			});
		}
		Ok(packed)
	}
}
