//! [`TempleComplexCell`]: a sanctum campus on one terrace.

use procedural_common::{NoiseParams, SeededHash};
use urbanization_developments::{DevelopmentFinish, DevelopmentFinishRole, TempleComplex};

use super::site::DevelopmentKind;
use super::terrace::{TerraceCell, TerraceEnvelope, TerraceKind, TerracePlan};
use crate::artifact::BuiltDevelopment;
use crate::cell::RING_FORT_MAX_FOOTPRINT;

pub struct TempleComplexKind;

pub type TempleComplexCell<G> = TerraceCell<TempleComplexKind, G>;

impl TerraceKind for TempleComplexKind {
	const KIND: DevelopmentKind = DevelopmentKind::TempleComplex;

	fn envelope() -> TerraceEnvelope {
		TerraceEnvelope {
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

	fn built(plan: &TerracePlan, noise: NoiseParams) -> Option<BuiltDevelopment> {
		let finish = plan.finish.clone();
		Some(BuiltDevelopment::TempleComplex(Box::new(
			TempleComplex::fit(plan.cell, &plan.confines(), noise)?
				.with_finish(finish.wall, finish.roof),
		)))
	}
}
