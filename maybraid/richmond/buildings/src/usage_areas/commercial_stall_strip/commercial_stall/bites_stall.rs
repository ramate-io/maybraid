//! Bites stall: BitesCounter(s) on long passages + BitesKitchen in the remainder.

use lod::gen::LodSceneLevel;
use procedural_common::{NoiseConfig, NoiseParams};
use richmond_building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};

use crate::fit::{Confines, FillableRegions, Fit, FitError};

use super::label_util::label_filling_aabb;
use super::stall_layout::{
	pack_bites_counters, pack_bites_kitchen, BITES_REGION_MIN_PLAN,
};

#[derive(Debug, Clone, PartialEq)]
pub struct BitesStall {
	/// Higher-order type label covering the whole stall.
	pub stall_type: LabelNode,
	pub bites_counters: Vec<LabelNode>,
	pub bites_kitchen: LabelNode,
}

impl Fit for BitesStall {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let counter_depth = cfg.sample_range_f32_4d(0.65, 1.0, c.x, c.y, c.z, 32.0);
		let packed = pack_bites_counters(confines, counter_depth)?;
		let kitchen_aabb = pack_bites_kitchen(
			&confines.bounds,
			&packed.counters,
			&[],
			BITES_REGION_MIN_PLAN,
		)
		.ok_or(FitError::TooSmall {
			reason: "bites kitchen",
		})?;

		let style = LabelStyle::from_unit(cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, 33.0));
		let bites_counters = packed
			.counters
			.iter()
			.map(|aabb| label_filling_aabb(style, "BitesCounter", aabb, confines.roll))
			.collect();

		Ok((
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
					&kitchen_aabb,
					confines.roll,
				),
			},
			FillableRegions::empty(),
		))
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
	use super::*;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;
	use crate::openings::{Opening, OpeningId, Openings};

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
		Confines::new(
			Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(10.0, 3.0, 6.0)),
			0.0,
			openings,
		)
	}

	#[test]
	fn long_passages_each_get_a_counter() {
		let (stall, _) =
			BitesStall::fit_to_confines(&two_south_doors(), NoiseParams::default()).unwrap();
		assert_eq!(stall.bites_counters.len(), 2);
		assert_eq!(stall.stall_type.text, "BitesStall");
		assert_eq!(stall.bites_kitchen.text, "BitesKitchen");
	}

	#[test]
	fn kitchen_claims_gap_and_full_behind_counters() {
		let (stall, _) =
			BitesStall::fit_to_confines(&two_south_doors(), NoiseParams::default()).unwrap();
		assert!(
			stall.bites_kitchen.placement.scale.x >= 8.0,
			"kitchen width {}",
			stall.bites_kitchen.placement.scale.x
		);
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
		assert_eq!(stall.bites_counters.len(), 2);
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
		assert!(matches!(
			BitesStall::fit_to_confines(&confines, NoiseParams::default()),
			Err(FitError::TooSmall { .. })
		));
	}
}
