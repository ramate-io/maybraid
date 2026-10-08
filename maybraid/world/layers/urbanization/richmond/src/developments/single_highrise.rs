//! [`SingleHighriseCell`]: one tower on a yawed terrace.

use buildings::Fit;
use procedural_common::{NoiseParams, SeededHash};
use urbanization_developments::{PlacedBuilding, SingleHighrise};

use super::site::DevelopmentKind;
use super::solitary::{SolitaryCell, SolitaryEnvelope, SolitaryKind, SolitaryPlan};
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
		let (placed, _) =
			PlacedBuilding::<SingleHighrise>::fit_to_confines(&plan.confines(), noise).ok()?;
		Some(BuiltDevelopment::SingleHighrise(Box::new(
			placed.map(|tower| tower.with_wall_material(plan.finish.wall.clone())),
		)))
	}
}
