//! [`RingFortCell`]: one walled ring fort.

use buildings::Fit;
use procedural_common::{NoiseParams, SeededHash};
use urbanization_developments::{PlacedBuilding, RingFort};

use super::site::DevelopmentKind;
use super::solitary::{SolitaryCell, SolitaryEnvelope, SolitaryKind, SolitaryPlan};
use crate::artifact::BuiltDevelopment;
use crate::cell::{
	RING_FORT_MAX_CONFINES_HEIGHT, RING_FORT_MAX_FOOTPRINT, RING_FORT_MIN_CONFINES_HEIGHT,
	RING_FORT_MIN_FOOTPRINT,
};
use crate::finish::DevelopmentFinish;

pub struct RingFortKind;

pub type RingFortCell<G> = SolitaryCell<RingFortKind, G>;

impl SolitaryKind for RingFortKind {
	const KIND: DevelopmentKind = DevelopmentKind::RingFort;

	fn envelope() -> SolitaryEnvelope {
		SolitaryEnvelope {
			min_footprint: RING_FORT_MIN_FOOTPRINT,
			max_footprint: RING_FORT_MAX_FOOTPRINT,
			min_height: RING_FORT_MIN_CONFINES_HEIGHT,
			max_height: RING_FORT_MAX_CONFINES_HEIGHT,
			rotates: true,
		}
	}

	fn finish(hash: SeededHash) -> DevelopmentFinish {
		DevelopmentFinish::pick(hash)
	}

	fn built(plan: &SolitaryPlan, noise: NoiseParams) -> Option<BuiltDevelopment> {
		let (placed, _) =
			PlacedBuilding::<RingFort>::fit_to_confines(&plan.confines(), noise).ok()?;
		let finish = plan.finish.clone();
		Some(BuiltDevelopment::RingFort(Box::new(
			placed.map(|fort| fort.with_finish(finish.wall, finish.roof)),
		)))
	}
}
