//! [`WizardsTowerCell`]: one solitary wizard's tower.

use buildings::Fit;
use procedural_common::{NoiseParams, SeededHash};
use urbanization_developments::{PlacedBuilding, SolitaryWizardsTower};

use super::site::DevelopmentKind;
use super::solitary::{SolitaryCell, SolitaryEnvelope, SolitaryKind, SolitaryPlan};
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
		let (placed, _) =
			PlacedBuilding::<SolitaryWizardsTower>::fit_to_confines(&plan.confines(), noise)
				.ok()?;
		let finish = plan.finish.clone();
		Some(BuiltDevelopment::WizardsTower(Box::new(
			placed.map(|tower| tower.with_finish(finish.wall, finish.roof)),
		)))
	}
}
