//! Stand-up food stall: counters on long customer passages + kitchen remainder.
//!
//! **Semantically:** quick-service counter facing the gallery door(s), prep space
//! behind.
//!
//! **Programmatically:** parameterized counter choices on passage long faces →
//! pack counters with clearance → kitchen = max-empty remainder (soft-fail if
//! either stage cannot meet mins).

pub mod parameterized;

pub use parameterized::{BitesStallParameterized, BitesStallPlan};

use lod::gen::LodSceneLevel;
use procedural_common::NoiseParams;
use richmond_building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};

use crate::fit::{Confines, FillableRegions, Fit, FitError};

use super::label_util::label_filling_aabb;

#[derive(Debug, Clone, PartialEq)]
pub struct BitesStall {
	/// Higher-order type label covering the whole stall.
	pub stall_type: LabelNode,
	pub bites_counters: Vec<LabelNode>,
	pub bites_kitchen: LabelNode,
}

impl BitesStall {
	pub fn from_plan(plan: BitesStallPlan, confines: &Confines) -> Self {
		let style = plan.parameterized.style;
		let bites_counters = plan
			.counter_aabbs
			.iter()
			.map(|aabb| label_filling_aabb(style, "BitesCounter", aabb, confines.roll))
			.collect();
		Self {
			stall_type: label_filling_aabb(
				LabelStyle::Yellow,
				"BitesStall",
				&confines.bounds,
				confines.roll,
			),
			bites_counters,
			bites_kitchen: label_filling_aabb(
				LabelStyle::Orange,
				"BitesKitchen",
				&plan.kitchen_aabb,
				confines.roll,
			),
		}
	}
}

impl Fit for BitesStall {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = BitesStallParameterized::sample(confines, noise)?;
		let plan = BitesStallPlan::from_parameterized(params, confines)?;
		Ok((Self::from_plan(plan, confines), FillableRegions::empty()))
	}
}

impl BuildingComponents for BitesStall {
	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		let mut labels = vec![self.stall_type.clone()];
		labels.extend(self.bites_counters.iter().cloned());
		labels.push(self.bites_kitchen.clone());
		Layers::from_free(labels)
	}
}

#[cfg(test)]
mod tests {
	use super::super::stall_layout::{BitesPassageSpec, EligibleBitesPassage};
	use super::*;
	use crate::openings::{Opening, OpeningId, Openings};
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;
	use procedural_common::OptionalFaceBand;

	fn two_south_doors() -> Confines {
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("door_a"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(0.5, 0.0, -0.2),
				Vec3::new(3.0, 2.2, 0.2),
			)),
		);
		openings.insert(
			OpeningId::new("door_b"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(5.0, 0.0, -0.2),
				Vec3::new(7.5, 2.2, 0.2),
			)),
		);
		Confines::new(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(10.0, 3.0, 6.0)), 0.0, openings)
	}

	fn both_counters() -> BitesStallParameterized {
		let confines = two_south_doors();
		let eligible = EligibleBitesPassage::collect(&confines);
		assert_eq!(eligible.len(), 2);
		BitesStallParameterized {
			style: LabelStyle::Cyan,
			passages: eligible
				.into_iter()
				.map(|passage| BitesPassageSpec {
					passage,
					counter: OptionalFaceBand { place: true, along: 1.5, depth: 0.8, along_t: 0.0 },
				})
				.collect(),
		}
	}

	#[test]
	fn parameterized_places_both_counters() {
		let confines = two_south_doors();
		let plan = BitesStallPlan::from_parameterized(both_counters(), &confines).unwrap();
		assert_eq!(plan.counter_aabbs.len(), 2);
		let stall = BitesStall::from_plan(plan, &confines);
		assert_eq!(stall.stall_type.text, "BitesStall");
		assert_eq!(stall.bites_kitchen.text, "BitesKitchen");
	}

	#[test]
	fn kitchen_claims_gap_and_full_behind_counters() {
		let confines = two_south_doors();
		let plan = BitesStallPlan::from_parameterized(both_counters(), &confines).unwrap();
		let stall = BitesStall::from_plan(plan, &confines);
		assert!(
			stall.bites_kitchen.placement.scale.x >= 8.0,
			"kitchen width {}",
			stall.bites_kitchen.placement.scale.x
		);
	}

	#[test]
	fn sample_fits_with_at_least_one_counter() {
		let (stall, _) =
			BitesStall::fit_to_confines(&two_south_doors(), NoiseParams::default()).unwrap();
		assert!(!stall.bites_counters.is_empty());
	}

	#[test]
	fn sparse_counters_still_fit() {
		let confines = two_south_doors();
		let mut params = both_counters();
		params.passages[1].counter.place = false;
		let plan = BitesStallPlan::from_parameterized(params, &confines).unwrap();
		assert_eq!(plan.counter_aabbs.len(), 1);
	}

	#[test]
	fn no_long_passage_does_not_fit() {
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("short"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(1.0, 0.0, -0.2),
				Vec3::new(2.5, 2.2, 0.2),
			)),
		);
		let confines = Confines::new(
			Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(6.0, 3.0, 5.0)),
			0.0,
			openings,
		);
		assert!(matches!(
			BitesStall::fit_to_confines(&confines, NoiseParams::default()),
			Err(FitError::TooSmall { .. })
		));
	}

	#[test]
	fn east_long_passages_fit() {
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("door_a"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(4.75, 0.0, 0.4),
				Vec3::new(5.25, 2.2, 5.0),
			)),
		);
		openings.insert(
			OpeningId::new("door_b"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(4.75, 0.0, 7.0),
				Vec3::new(5.25, 2.2, 11.5),
			)),
		);
		let confines = Confines::new(
			Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(5.0, 3.2, 12.0)),
			0.0,
			openings,
		);
		let (stall, _) = BitesStall::fit_to_confines(&confines, NoiseParams::default()).unwrap();
		assert!(!stall.bites_counters.is_empty());
	}

	#[test]
	fn shallow_stall_fails_kitchen_min() {
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("door"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(0.2, 0.0, -0.2),
				Vec3::new(5.8, 2.2, 0.2),
			)),
		);
		let confines = Confines::new(
			Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(6.0, 3.0, 2.2)),
			0.0,
			openings,
		);
		let eligible = EligibleBitesPassage::collect(&confines);
		let params = BitesStallParameterized {
			style: LabelStyle::Cyan,
			passages: eligible
				.into_iter()
				.map(|passage| BitesPassageSpec {
					passage,
					counter: OptionalFaceBand { place: true, along: 5.0, depth: 1.0, along_t: 0.0 },
				})
				.collect(),
		};
		assert!(matches!(
			BitesStallPlan::from_parameterized(params, &confines),
			Err(FitError::TooSmall { .. })
		));
	}
}
