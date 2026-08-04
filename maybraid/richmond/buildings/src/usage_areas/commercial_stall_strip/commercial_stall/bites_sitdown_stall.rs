//! Bites sit-down: composes bites counters + opening-face seating + kitchen.
//!
//! Constraints:
//! - Counter rules from [`super::bites_stall::BitesStall`] (via composed parameterized).
//! - [`BitesSeatingArea`] ≥1×1, may abut counters, must share ≥1m border with
//!   the **long opening face** of a Passage, and may abut the kitchen.
//! - Kitchen ≥1×1, ≥1m from counters, may abut seating.
//! Soft-fail ([`FitError::TooSmall`]) if any region cannot be reserved.

pub mod parameterized;

pub use parameterized::{BitesSitdownParameterized, BitesSitdownPlan};

use lod::gen::LodSceneLevel;
use procedural_common::NoiseParams;
use richmond_building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};

use crate::fit::{Confines, FillableRegions, Fit, FitError};

use super::label_util::label_filling_aabb;

#[derive(Debug, Clone, PartialEq)]
pub struct BitesSitdownStall {
	pub stall_type: LabelNode,
	pub bites_counters: Vec<LabelNode>,
	pub bites_kitchen: LabelNode,
	pub bites_seating_area: LabelNode,
}

impl BitesSitdownStall {
	pub fn from_plan(plan: BitesSitdownPlan, confines: &Confines) -> Self {
		let style = plan.parameterized.style();
		let bites_counters = plan
			.counter_aabbs
			.iter()
			.map(|aabb| label_filling_aabb(style, "BitesCounter", aabb, confines.roll))
			.collect();
		Self {
			stall_type: label_filling_aabb(
				LabelStyle::Yellow,
				"BitesSitdownStall",
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
			bites_seating_area: label_filling_aabb(
				LabelStyle::Green,
				"BitesSeatingArea",
				&plan.seating_aabb,
				confines.roll,
			),
		}
	}
}

impl Fit for BitesSitdownStall {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = BitesSitdownParameterized::sample(confines, noise)?;
		let plan = BitesSitdownPlan::from_parameterized(params, confines)?;
		Ok((Self::from_plan(plan, confines), FillableRegions::empty()))
	}
}

impl BuildingComponents for BitesSitdownStall {
	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		let mut labels = vec![self.stall_type.clone()];
		labels.extend(self.bites_counters.iter().cloned());
		labels.push(self.bites_kitchen.clone());
		labels.push(self.bites_seating_area.clone());
		Layers::from_free(labels)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;
	use crate::openings::{Opening, OpeningId, OpeningLabel, Openings};
	use procedural_common::{
		aabb2_area, aabb3_to_plan, contacts_opening_face, passage_opening_face, PlanAxes,
	};
	use super::super::bites_stall::BitesStallParameterized;
	use super::super::stall_layout::{BitesCounterChoice, BITES_SEATING_FACE_CONTACT};

	fn roomy_south() -> Confines {
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("door_a"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(0.5, 0.0, -0.2),
				Vec3::new(3.5, 2.2, 0.2),
			)),
		);
		openings.insert(
			OpeningId::new("door_b"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(6.0, 0.0, -0.2),
				Vec3::new(9.0, 2.2, 0.2),
			)),
		);
		Confines::new(
			Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(12.0, 3.2, 8.0)),
			0.0,
			openings,
		)
	}

	fn roomy_params(seating_area_target: f32) -> BitesSitdownParameterized {
		BitesSitdownParameterized {
			base: BitesStallParameterized {
				style: LabelStyle::Cyan,
				counters: vec![
					BitesCounterChoice {
						place: true,
						along: 1.5,
						depth: 0.8,
						along_t: 0.0,
					},
					BitesCounterChoice {
						place: true,
						along: 1.5,
						depth: 0.8,
						along_t: 0.0,
					},
				],
			},
			seating_area_target,
			seating_seed_depth: 1.5,
			seating_along_t: 0.5,
		}
	}

	#[test]
	fn sitdown_emits_counters_seating_kitchen() {
		let confines = roomy_south();
		let plan =
			BitesSitdownPlan::from_parameterized(roomy_params(35.0), &confines).unwrap();
		let stall = BitesSitdownStall::from_plan(plan, &confines);
		assert!(!stall.bites_counters.is_empty());
		assert_eq!(stall.stall_type.text, "BitesSitdownStall");
		assert_eq!(stall.bites_seating_area.text, "BitesSeatingArea");
		assert!(stall.bites_seating_area.placement.scale.x >= 1.0);
		assert!(stall.bites_seating_area.placement.scale.z >= 1.0);
		assert!(stall.bites_kitchen.placement.scale.x >= 1.0);
		assert!(stall.bites_kitchen.placement.scale.z >= 1.0);
	}

	#[test]
	fn seating_shares_one_meter_opening_face() {
		let confines = roomy_south();
		let plan =
			BitesSitdownPlan::from_parameterized(roomy_params(35.0), &confines).unwrap();
		let seating = plan.seating_aabb;
		let seat2 = aabb3_to_plan(&seating, PlanAxes::XZ);
		let host = aabb3_to_plan(&confines.bounds, PlanAxes::XZ);
		let ok = confines.openings.iter().any(|(_, o)| {
			if !matches!(o.label, OpeningLabel::Passage) {
				return false;
			}
			let Some(face) = passage_opening_face(host, aabb3_to_plan(&o.bounds, PlanAxes::XZ))
			else {
				return false;
			};
			contacts_opening_face(seat2, face, BITES_SEATING_FACE_CONTACT)
		});
		assert!(ok, "seating must share ≥1m with a passage long face");
	}

	#[test]
	fn seating_grows_toward_area_target() {
		let confines = roomy_south();
		let target = 40.0_f32;
		let plan =
			BitesSitdownPlan::from_parameterized(roomy_params(target), &confines).unwrap();
		let seat_area = aabb2_area(aabb3_to_plan(&plan.seating_aabb, PlanAxes::XZ));
		let kit_area = aabb2_area(aabb3_to_plan(&plan.kitchen_aabb, PlanAxes::XZ));
		assert!(
			seat_area + 1.0 >= target.min(30.0),
			"seating area {seat_area} should approach target {target}"
		);
		// Seating should not be a thin leftover beside a dominant kitchen.
		assert!(
			seat_area + 1.0 >= kit_area * 0.45,
			"seating {seat_area} dominated by kitchen {kit_area}"
		);
	}

	#[test]
	fn sample_fit_works() {
		let (stall, _) =
			BitesSitdownStall::fit_to_confines(&roomy_south(), NoiseParams::default()).unwrap();
		assert!(!stall.bites_counters.is_empty());
	}

	#[test]
	fn shallow_fails_without_seating_and_kitchen() {
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("door"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(0.2, 0.0, -0.2),
				Vec3::new(5.8, 2.2, 0.2),
			)),
		);
		let confines = Confines::new(
			Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(6.0, 3.0, 2.4)),
			0.0,
			openings,
		);
		assert!(matches!(
			BitesSitdownStall::fit_to_confines(&confines, NoiseParams::default()),
			Err(FitError::TooSmall { .. })
		));
	}
}
