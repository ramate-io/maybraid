//! Bites sit-down: counters + passage-connected seating + kitchen remainder.
//!
//! Constraints:
//! - Same counter rules as [`super::bites_stall::BitesStall`].
//! - [`BitesSeatingArea`] ≥1×1, may sit against counters (behind them), must
//!   **touch a Passage**, and may abut the kitchen.
//! - Kitchen ≥1×1, ≥1m from counters, may abut seating.
//! Soft-fail ([`FitError::TooSmall`]) if any region cannot be reserved.

use lod::gen::LodSceneLevel;
use procedural_common::{NoiseConfig, NoiseParams};
use richmond_building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};

use crate::fit::{Confines, FillableRegions, Fit, FitError};

use super::label_util::label_filling_aabb;
use super::stall_layout::{
	pack_bites_counters, pack_bites_kitchen, pack_passage_connected_region, BITES_REGION_MIN_PLAN,
};

#[derive(Debug, Clone, PartialEq)]
pub struct BitesSitdownStall {
	pub stall_type: LabelNode,
	pub bites_counters: Vec<LabelNode>,
	pub bites_kitchen: LabelNode,
	pub bites_seating_area: LabelNode,
}

impl Fit for BitesSitdownStall {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let counter_depth = cfg.sample_range_f32_4d(0.65, 1.0, c.x, c.y, c.z, 42.0);
		let packed = pack_bites_counters(confines, counter_depth)?;

		// Reserve seating first: passage-touching, may press against counters.
		let seating = pack_passage_connected_region(
			&confines.bounds,
			&packed.counters,
			&packed.passages,
			BITES_REGION_MIN_PLAN,
		)
		.ok_or(FitError::TooSmall {
			reason: "bites seating",
		})?;

		// Kitchen: 1m from counters, may abut seating.
		let kitchen = pack_bites_kitchen(
			&confines.bounds,
			&packed.counters,
			&[seating],
			BITES_REGION_MIN_PLAN,
		)
		.ok_or(FitError::TooSmall {
			reason: "bites kitchen",
		})?;

		let style = LabelStyle::from_unit(cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, 43.0));
		let bites_counters = packed
			.counters
			.iter()
			.map(|aabb| label_filling_aabb(style, "BitesCounter", aabb, confines.roll))
			.collect();

		Ok((
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
					&kitchen,
					confines.roll,
				),
				bites_seating_area: label_filling_aabb(
					LabelStyle::Green,
					"BitesSeatingArea",
					&seating,
					confines.roll,
				),
			},
			FillableRegions::empty(),
		))
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
	use crate::openings::{Opening, OpeningId, Openings};
	use procedural_common::{aabb3_to_plan, touches_aabb2, PlanAxes};

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

	#[test]
	fn sitdown_emits_counters_seating_kitchen() {
		let (stall, _) =
			BitesSitdownStall::fit_to_confines(&roomy_south(), NoiseParams::default()).unwrap();
		assert!(!stall.bites_counters.is_empty());
		assert_eq!(stall.stall_type.text, "BitesSitdownStall");
		assert_eq!(stall.bites_seating_area.text, "BitesSeatingArea");
		assert!(stall.bites_seating_area.placement.scale.x >= 1.0);
		assert!(stall.bites_seating_area.placement.scale.z >= 1.0);
		assert!(stall.bites_kitchen.placement.scale.x >= 1.0);
		assert!(stall.bites_kitchen.placement.scale.z >= 1.0);
	}

	#[test]
	fn seating_touches_a_passage() {
		let confines = roomy_south();
		let (stall, _) =
			BitesSitdownStall::fit_to_confines(&confines, NoiseParams::default()).unwrap();
		let seating = {
			let p = &stall.bites_seating_area.placement;
			let half = p.scale * 0.5;
			Aabb3d::from_min_max(p.translation - half, p.translation + half)
		};
		let seat2 = aabb3_to_plan(&seating, PlanAxes::XZ);
		let touches = confines.openings.iter().any(|(_, o)| {
			if !matches!(o.label, crate::openings::OpeningLabel::Passage) {
				return false;
			}
			touches_aabb2(seat2, aabb3_to_plan(&o.bounds, PlanAxes::XZ))
		});
		assert!(touches, "seating must connect to a passage");
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
