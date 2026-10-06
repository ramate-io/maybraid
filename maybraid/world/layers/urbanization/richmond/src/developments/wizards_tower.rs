//! [`WizardsTowerCell`]: one solitary wizard's tower.

use procedural_common::{NoiseParams, SeededHash};

use super::site::DevelopmentKind;
use super::solitary::{SolitaryCell, SolitaryEnvelope, SolitaryKind, SolitaryPlan};
use crate::archetype_generation::ArchetypeGenerator;
use crate::artifact::BuiltDevelopment;
use crate::cell::RING_FORT_MAX_FOOTPRINT;
use crate::finish::{DevelopmentFinish, DevelopmentFinishRole};

pub struct WizardsTowerKind;

pub type WizardsTowerCell<G> = SolitaryCell<WizardsTowerKind, G>;

impl SolitaryKind for WizardsTowerKind {
	const KIND: DevelopmentKind = DevelopmentKind::WizardsTower;

	fn envelope() -> SolitaryEnvelope {
		SolitaryEnvelope {
			min_footprint: 24.0,
			max_footprint: RING_FORT_MAX_FOOTPRINT.min(42.0),
			min_height: 48.0,
			max_height: 104.0,
			rotates: true,
		}
	}

	fn finish(hash: SeededHash) -> DevelopmentFinish {
		DevelopmentFinish::pick_for_role(hash, DevelopmentFinishRole::WizardsTower, false)
	}

	fn built(plan: &SolitaryPlan, noise: NoiseParams) -> Option<BuiltDevelopment> {
		let finish = plan.finish.clone();
		let mut development =
			ArchetypeGenerator::build_wizards_tower(plan.cell, plan.confines(), noise)?;
		development.building.building =
			development.building.building.with_finish(finish.wall, finish.roof);
		Some(BuiltDevelopment::WizardsTower(Box::new(development)))
	}
}
