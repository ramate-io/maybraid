//! [`LesHallesCell`]: one mixed-use Les Halles hall.

use buildings::Fit;
use procedural_common::{NoiseParams, SeededHash};
use urbanization_developments::{MixedUseLesHallesDevelopment, PlacedBuilding};

use super::site::DevelopmentKind;
use super::solitary::{SolitaryCell, SolitaryEnvelope, SolitaryKind, SolitaryPlan};
use crate::artifact::BuiltDevelopment;
use crate::cell::{available_footprint, MAX_CONFINES_HEIGHT, MIN_CONFINES_HEIGHT, MIN_FOOTPRINT};
use crate::finish::DevelopmentFinish;

pub struct LesHallesKind;

pub type LesHallesCell<G> = SolitaryCell<LesHallesKind, G>;

impl SolitaryKind for LesHallesKind {
	const KIND: DevelopmentKind = DevelopmentKind::LesHalles;

	fn envelope() -> SolitaryEnvelope {
		SolitaryEnvelope {
			min_footprint: MIN_FOOTPRINT,
			max_footprint: available_footprint(),
			min_height: MIN_CONFINES_HEIGHT,
			max_height: MAX_CONFINES_HEIGHT,
			rotates: true,
		}
	}

	fn finish(hash: SeededHash) -> DevelopmentFinish {
		DevelopmentFinish::pick(hash)
	}

	fn built(plan: &SolitaryPlan, noise: NoiseParams) -> Option<BuiltDevelopment> {
		let (placed, _) = PlacedBuilding::<MixedUseLesHallesDevelopment>::fit_to_confines(
			&plan.confines(),
			noise,
		)
		.ok()?;
		let finish = plan.finish.clone();
		Some(BuiltDevelopment::LesHalles(Box::new(
			placed.map(|halles| halles.with_finish(finish.wall, finish.roof)),
		)))
	}
}
