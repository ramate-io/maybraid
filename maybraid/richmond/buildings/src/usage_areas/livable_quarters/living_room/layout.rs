//! Living room plan packer: primary seating + optional secondary + filler.

use bevy_math::bounding::Aabb3d;
use procedural_common::NoiseParams;

use crate::fit::{Confines, FitError};
use crate::placer::{
	CommitEffect, KindSpec, Predicate, ProgramTier, ProposeKnobs, SoftGoalRole,
};

use crate::usage_areas::livable_quarters::pack::{init_host, pack_kinds, PackKnobs};

pub const MIN_AREA: f32 = 3.0 * 3.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LivingKind {
	PrimarySeating,
	SecondarySeating,
	Filler,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LivingRoomPacked {
	pub primary_seating: Vec<Aabb3d>,
	pub secondary_seating: Vec<Aabb3d>,
	pub fillers: Vec<Aabb3d>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LivingRoomRegions {
	pub spaciousness: f32,
	pub occupancy: f32,
}

impl LivingRoomRegions {
	fn catalog() -> &'static [KindSpec<LivingKind>] {
		static CATALOG: [KindSpec<LivingKind>; 3] = [
			KindSpec {
				id: LivingKind::PrimarySeating,
				tier: ProgramTier::Appointed,
				weight: 1.0,
				max_count: Some(1),
				soft_goal: SoftGoalRole::None,
				// Sofa: long face on a wall (more reliable than free+prefer_wall).
				propose: ProposeKnobs::WallLong {
					along_min: 1.35,
					along_max: 2.2,
					depth_min: 0.7,
					depth_max: 0.95,
					height: 0.85,
				},
				predicates: &[
					Predicate::InHost,
					Predicate::ClearOfKeepOuts,
					Predicate::LongFaceOnWall,
				],
				commit: CommitEffect::SolidFootprint,
				structure_budget: false,
			},
			KindSpec {
				id: LivingKind::SecondarySeating,
				tier: ProgramTier::Appointed,
				weight: 0.65,
				max_count: Some(1),
				soft_goal: SoftGoalRole::None,
				propose: ProposeKnobs::WallLong {
					along_min: 0.85,
					along_max: 1.35,
					depth_min: 0.65,
					depth_max: 0.9,
					height: 0.85,
				},
				predicates: &[
					Predicate::InHost,
					Predicate::ClearOfKeepOuts,
					Predicate::LongFaceOnWall,
				],
				commit: CommitEffect::SolidFootprint,
				structure_budget: false,
			},
			KindSpec {
				id: LivingKind::Filler,
				tier: ProgramTier::Filler,
				weight: 0.45,
				max_count: Some(2),
				soft_goal: SoftGoalRole::None,
				propose: ProposeKnobs::FreeExtent {
					min_x: 0.45,
					max_x: 0.75,
					min_z: 0.45,
					max_z: 0.75,
					height: 0.55,
					prefer_wall: false,
				},
				predicates: &[Predicate::InHost, Predicate::ClearOfKeepOuts],
				commit: CommitEffect::SolidFootprint,
				structure_budget: false,
			},
		];
		&CATALOG
	}

	pub fn pack(
		&self,
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<LivingRoomPacked, FitError> {
		let mut host = init_host(confines)?;
		if host.room_area + 1e-3 < MIN_AREA {
			return Err(FitError::TooSmall {
				reason: "living room",
			});
		}
		let mut packed = LivingRoomPacked::default();
		let placed = pack_kinds(
			Self::catalog(),
			PackKnobs {
				spaciousness: self.spaciousness,
				furniture_occupancy: self.occupancy,
				structure_occupancy: self.occupancy.max(0.72),
				max_steps: 18,
			},
			&mut host,
			confines,
			noise,
			|p| p.iter().any(|(k, _)| *k == LivingKind::PrimarySeating),
		)?;
		for (kind, aabb) in placed {
			match kind {
				LivingKind::PrimarySeating => packed.primary_seating.push(aabb),
				LivingKind::SecondarySeating => packed.secondary_seating.push(aabb),
				LivingKind::Filler => packed.fillers.push(aabb),
			}
		}
		if packed.primary_seating.is_empty() {
			return Err(FitError::TooSmall {
				reason: "living room seating",
			});
		}
		Ok(packed)
	}
}
