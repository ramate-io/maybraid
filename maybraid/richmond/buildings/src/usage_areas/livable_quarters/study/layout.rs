//! Study plan packer: wall desk + optional bookcase filler.

use bevy_math::bounding::Aabb3d;
use procedural_common::NoiseParams;

use crate::fit::{Confines, FitError};
use crate::placer::{
	CommitEffect, KindSpec, Predicate, ProgramTier, ProposeKnobs, SoftGoalRole,
};

use crate::usage_areas::livable_quarters::pack::{init_host, pack_kinds, PackKnobs};

pub const MIN_AREA: f32 = 2.0 * 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StudyKind {
	Desk,
	Bookcase,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct StudyPacked {
	pub desks: Vec<Aabb3d>,
	pub bookcases: Vec<Aabb3d>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StudyRegions {
	pub spaciousness: f32,
	pub occupancy: f32,
}

impl StudyRegions {
	fn catalog() -> &'static [KindSpec<StudyKind>] {
		static CATALOG: [KindSpec<StudyKind>; 2] = [
			KindSpec {
				id: StudyKind::Desk,
				tier: ProgramTier::Appointed,
				weight: 1.0,
				max_count: Some(1),
				soft_goal: SoftGoalRole::None,
				propose: ProposeKnobs::WallLong {
					along_min: 1.0,
					along_max: 1.8,
					depth_min: 0.55,
					depth_max: 0.75,
					height: 0.75,
				},
				predicates: &[Predicate::InHost, Predicate::ClearOfKeepOuts, Predicate::LongFaceOnWall],
				commit: CommitEffect::SolidFootprint,
				structure_budget: false,
			},
			KindSpec {
				id: StudyKind::Bookcase,
				tier: ProgramTier::Filler,
				weight: 0.55,
				max_count: Some(2),
				soft_goal: SoftGoalRole::None,
				propose: ProposeKnobs::WallLong {
					along_min: 0.7,
					along_max: 1.2,
					depth_min: 0.35,
					depth_max: 0.5,
					height: 1.8,
				},
				predicates: &[Predicate::InHost, Predicate::ClearOfKeepOuts, Predicate::LongFaceOnWall],
				commit: CommitEffect::SolidFootprint,
				structure_budget: false,
			},
		];
		&CATALOG
	}

	pub fn pack(&self, confines: &Confines, noise: NoiseParams) -> Result<StudyPacked, FitError> {
		let mut host = init_host(confines)?;
		if host.room_area + 1e-3 < MIN_AREA {
			return Err(FitError::TooSmall {
				reason: "study",
			});
		}
		let mut packed = StudyPacked::default();
		let placed = pack_kinds(
			Self::catalog(),
			PackKnobs {
				spaciousness: self.spaciousness,
				furniture_occupancy: self.occupancy,
				structure_occupancy: self.occupancy.max(0.68),
				max_steps: 12,
			},
			&mut host,
			confines,
			noise,
			|p| p.iter().any(|(k, _)| *k == StudyKind::Desk),
		)?;
		for (kind, aabb) in placed {
			match kind {
				StudyKind::Desk => packed.desks.push(aabb),
				StudyKind::Bookcase => packed.bookcases.push(aabb),
			}
		}
		if packed.desks.is_empty() {
			return Err(FitError::TooSmall {
				reason: "study desk",
			});
		}
		Ok(packed)
	}
}
