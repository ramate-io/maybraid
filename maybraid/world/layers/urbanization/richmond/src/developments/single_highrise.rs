//! [`SingleHighriseCell`]: one tower on a yawed terrace.

use buildings::Fit;
use procedural_common::{NoiseParams, SeededHash};
use urbanization_developments::{
	DevelopmentFinish, DevelopmentFinishRole, PlacedBuilding, SingleHighrise,
};

use super::site::DevelopmentKind;
use super::terrace::{TerraceCell, TerraceEnvelope, TerraceKind, TerracePlan};
use crate::artifact::BuiltDevelopment;
use crate::cell::RING_FORT_MAX_FOOTPRINT;

pub struct SingleHighriseKind;

pub type SingleHighriseCell<G> = TerraceCell<SingleHighriseKind, G>;

impl TerraceKind for SingleHighriseKind {
	const KIND: DevelopmentKind = DevelopmentKind::SingleHighrise;

	fn envelope() -> TerraceEnvelope {
		TerraceEnvelope {
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

	fn built(plan: &TerracePlan, noise: NoiseParams) -> Option<BuiltDevelopment> {
		let (placed, _) =
			PlacedBuilding::<SingleHighrise>::fit_to_confines(&plan.confines(), noise).ok()?;
		Some(BuiltDevelopment::SingleHighrise(Box::new(
			placed.map(|tower| tower.with_wall_material(plan.finish.wall.clone())),
		)))
	}
}
