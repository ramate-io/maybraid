//! [`SkybridgeBazaarCell`]: towers joined by skybridges on one terrace.

use procedural_common::{NoiseParams, SeededHash};

use super::site::DevelopmentKind;
use super::terrace::{TerraceCell, TerraceEnvelope, TerraceKind, TerracePlan};
use crate::archetype_generation::ArchetypeGenerator;
use crate::artifact::BuiltDevelopment;
use crate::cell::RING_FORT_MAX_FOOTPRINT;
use crate::finish::{DevelopmentFinish, DevelopmentFinishRole};

pub struct SkybridgeBazaarKind;

pub type SkybridgeBazaarCell<G> = TerraceCell<SkybridgeBazaarKind, G>;

impl TerraceKind for SkybridgeBazaarKind {
	const KIND: DevelopmentKind = DevelopmentKind::SkybridgeBazaar;

	fn envelope() -> TerraceEnvelope {
		TerraceEnvelope {
			min_footprint: 160.0,
			max_footprint: RING_FORT_MAX_FOOTPRINT.min(220.0),
			min_height: 64.0,
			max_height: 96.0,
			rotates: false,
		}
	}

	fn finish(hash: SeededHash) -> DevelopmentFinish {
		DevelopmentFinish::pick_for_role(hash, DevelopmentFinishRole::Connector, false)
	}

	fn built(plan: &TerracePlan, noise: NoiseParams) -> Option<BuiltDevelopment> {
		Some(BuiltDevelopment::SkybridgeBazaar(Box::new(
			ArchetypeGenerator::build_skybridge_bazaar(plan.cell, &plan.confines(), noise)?
				.with_bridge_material(plan.finish.wall.clone()),
		)))
	}
}
