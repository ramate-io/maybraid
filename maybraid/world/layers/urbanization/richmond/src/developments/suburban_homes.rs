//! [`SuburbanHomesCell`]: a neighborhood of homes on one terrace.

use procedural_common::{NoiseParams, SeededHash};
use urbanization_developments::{DevelopmentFinish, DevelopmentFinishRole, SuburbanHomes};

use super::site::DevelopmentKind;
use super::terrace::{TerraceCell, TerraceEnvelope, TerraceKind, TerracePlan};
use crate::artifact::BuiltDevelopment;
use crate::cell::RING_FORT_MAX_FOOTPRINT;

pub struct SuburbanHomesKind;

pub type SuburbanHomesCell<G> = TerraceCell<SuburbanHomesKind, G>;

impl TerraceKind for SuburbanHomesKind {
	const KIND: DevelopmentKind = DevelopmentKind::SuburbanHomes;

	fn envelope() -> TerraceEnvelope {
		TerraceEnvelope {
			min_footprint: 190.0,
			max_footprint: RING_FORT_MAX_FOOTPRINT.min(230.0),
			min_height: 12.0,
			max_height: 16.0,
			rotates: false,
		}
	}

	fn finish(hash: SeededHash) -> DevelopmentFinish {
		DevelopmentFinish::pick_for_role(hash, DevelopmentFinishRole::SuburbanHome, false)
	}

	fn built(plan: &TerracePlan, noise: NoiseParams) -> Option<BuiltDevelopment> {
		Some(BuiltDevelopment::SuburbanHomes(Box::new(SuburbanHomes::fit(
			plan.cell,
			&plan.confines(),
			noise,
		)?)))
	}
}
