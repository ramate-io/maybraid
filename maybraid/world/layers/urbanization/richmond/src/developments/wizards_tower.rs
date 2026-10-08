//! [`WizardsTowerCell`]: one solitary wizard's tower.

use buildings::Fit;
use procedural_common::{NoiseParams, SeededHash};
use urbanization_developments::{
	DevelopmentFinish, DevelopmentFinishRole, PlacedBuilding, SolitaryWizardsTower,
};

use super::site::DevelopmentKind;
use super::terrace::{TerraceCell, TerraceEnvelope, TerraceKind, TerracePlan};
use crate::artifact::BuiltDevelopment;
use crate::cell::RING_FORT_MAX_FOOTPRINT;

pub struct WizardsTowerKind;

pub type WizardsTowerCell<G> = TerraceCell<WizardsTowerKind, G>;

impl TerraceKind for WizardsTowerKind {
	const KIND: DevelopmentKind = DevelopmentKind::WizardsTower;

	fn envelope() -> TerraceEnvelope {
		TerraceEnvelope {
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

	fn built(plan: &TerracePlan, noise: NoiseParams) -> Option<BuiltDevelopment> {
		let (placed, _) =
			PlacedBuilding::<SolitaryWizardsTower>::fit_to_confines(&plan.confines(), noise)
				.ok()?;
		let finish = plan.finish.clone();
		Some(BuiltDevelopment::WizardsTower(Box::new(
			placed.map(|tower| tower.with_finish(finish.wall, finish.roof)),
		)))
	}
}
