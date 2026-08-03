//! Bites stall: BitesCounter(s) on long passages + BitesKitchen in the remainder.

use bevy_math::bounding::Aabb3d;
use lod::gen::LodSceneLevel;
use procedural_common::{NoiseConfig, NoiseParams};
use richmond_building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};

use crate::fit::{aabb_xz_extent, Confines, FillableRegions, Fit, FitError};
use crate::openings::OpeningLabel;

use super::label_util::label_filling_aabb;
use super::stall_layout::{
	counter_on_opening, largest_remainder_away_from, opening_along_len, side_for_opening,
};

/// Passages must be at least this long (along-wall) to host a BitesCounter.
pub const LONG_PASSAGE_MIN: f32 = 2.0;
/// Counter along-length floor; the rest of the passage (≥1m) stays clear.
pub const COUNTER_ALONG_MIN: f32 = 1.0;
/// Clear passage length left beside each counter.
pub const PASSAGE_REMAIN_MIN: f32 = 1.0;
/// Kitchen stays at least this far (XZ) from every counter.
pub const KITCHEN_COUNTER_CLEARANCE: f32 = 1.0;
/// Kitchen plan minimum (width and depth).
pub const KITCHEN_MIN_PLAN: f32 = 1.0;

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

		let mut counter_aabbs: Vec<Aabb3d> = Vec::new();
		for (_id, opening) in confines.openings.iter() {
			if !matches!(opening.label, OpeningLabel::Passage) {
				continue;
			}
			let Some(side) = side_for_opening(&confines.bounds, opening) else {
				continue;
			};
			let passage_len = opening_along_len(opening, side);
			if passage_len + 1e-3 < LONG_PASSAGE_MIN {
				continue;
			}
			// Counter ≥1m along, leaving ≥1m of the passage clear.
			let along = (passage_len - PASSAGE_REMAIN_MIN).max(COUNTER_ALONG_MIN);
			if along + 1e-3 < COUNTER_ALONG_MIN || passage_len - along + 1e-3 < PASSAGE_REMAIN_MIN
			{
				continue;
			}
			counter_aabbs.push(counter_on_opening(
				&confines.bounds,
				opening,
				side,
				counter_depth,
				along,
			));
		}

		if counter_aabbs.is_empty() {
			return Err(FitError::TooSmall {
				reason: "bites counter passage",
			});
		}

		let kitchen_aabb = largest_remainder_away_from(
			&confines.bounds,
			&counter_aabbs,
			KITCHEN_COUNTER_CLEARANCE,
		)
		.filter(|k| {
			let e = aabb_xz_extent(k);
			e.x + 1e-3 >= KITCHEN_MIN_PLAN && e.y + 1e-3 >= KITCHEN_MIN_PLAN
		})
		.ok_or(FitError::TooSmall {
			reason: "bites kitchen",
		})?;

		let style = LabelStyle::from_unit(cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, 33.0));
		let bites_counters = counter_aabbs
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
		// With clearance, the max empty rect should still be wide (behind + between).
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
		// Door spans nearly full width so no lateral kitchen pocket remains.
		openings.insert(
			OpeningId::new("door"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(0.2, 0.0, -0.2),
				Vec3::new(5.8, 2.2, 0.2),
			)),
		);
		// Depth 2.2: counter ≥0.65 + clearance 1 leaves <1m behind.
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
