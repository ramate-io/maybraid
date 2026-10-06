//! [`TempleComplexCell`]: a sanctum campus on one terrace.

use procedural_common::{NoiseParams, SeededHash};

use super::site::DevelopmentKind;
use super::solitary::{SolitaryCell, SolitaryEnvelope, SolitaryKind, SolitaryPlan};
use crate::archetype_generation::ArchetypeGenerator;
use crate::artifact::BuiltDevelopment;
use crate::cell::RING_FORT_MAX_FOOTPRINT;
use crate::finish::{DevelopmentFinish, DevelopmentFinishRole};

pub struct TempleComplexKind;

pub type TempleComplexCell<G> = SolitaryCell<TempleComplexKind, G>;

impl SolitaryKind for TempleComplexKind {
	const KIND: DevelopmentKind = DevelopmentKind::TempleComplex;

	fn envelope() -> SolitaryEnvelope {
		SolitaryEnvelope {
			min_footprint: 130.0,
			max_footprint: RING_FORT_MAX_FOOTPRINT.min(190.0),
			min_height: 48.0,
			max_height: 64.0,
			rotates: false,
		}
	}

	fn finish(hash: SeededHash) -> DevelopmentFinish {
		DevelopmentFinish::pick_for_role(hash, DevelopmentFinishRole::Temple, false)
	}

	fn built(plan: &SolitaryPlan, noise: NoiseParams) -> Option<BuiltDevelopment> {
		let finish = plan.finish.clone();
		Some(BuiltDevelopment::TempleComplex(Box::new(
			ArchetypeGenerator::build_temple_complex(plan.cell, &plan.confines(), noise)?
				.with_finish(finish.wall, finish.roof),
		)))
	}
}
