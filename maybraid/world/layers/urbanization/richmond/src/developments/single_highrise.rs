//! [`SingleHighriseCell`]: one tower on a yawed terrace.

use procedural_common::{NoiseParams, SeededHash};

use super::site::DevelopmentKind;
use super::solitary::{SolitaryCell, SolitaryEnvelope, SolitaryKind, SolitaryPlan};
use crate::archetype_generation::ArchetypeGenerator;
use crate::artifact::BuiltDevelopment;
use crate::cell::RING_FORT_MAX_FOOTPRINT;
use crate::finish::{DevelopmentFinish, DevelopmentFinishRole};

pub struct SingleHighriseKind;

pub type SingleHighriseCell<G> = SolitaryCell<SingleHighriseKind, G>;

impl SolitaryKind for SingleHighriseKind {
	const KIND: DevelopmentKind = DevelopmentKind::SingleHighrise;

	fn envelope() -> SolitaryEnvelope {
		SolitaryEnvelope {
			min_footprint: 42.5,
			max_footprint: RING_FORT_MAX_FOOTPRINT.min(65.0),
			min_height: 48.0,
			max_height: 104.0,
			rotates: true,
		}
	}

	fn finish(hash: SeededHash) -> DevelopmentFinish {
		DevelopmentFinish::pick_for_role(hash, DevelopmentFinishRole::Highrise, false)
	}

	fn built(plan: &SolitaryPlan, noise: NoiseParams) -> Option<BuiltDevelopment> {
		let mut development =
			ArchetypeGenerator::build_single_highrise(plan.cell, plan.confines(), noise)?;
		development.building.building =
			development.building.building.with_wall_material(plan.finish.wall.clone());
		Some(BuiltDevelopment::SingleHighrise(Box::new(development)))
	}
}
