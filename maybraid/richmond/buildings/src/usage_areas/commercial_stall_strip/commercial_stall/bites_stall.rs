//! Bites stall: BitesCounter(s) on long passages + BitesKitchen in the remainder.

use bevy_math::bounding::Aabb3d;
use lod::gen::LodSceneLevel;
use procedural_common::{NoiseConfig, NoiseParams};
use richmond_building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};

use crate::fit::{Confines, FillableRegions, Fit, FitError};
use crate::openings::OpeningLabel;

use super::label_util::label_filling_aabb;
use super::stall_layout::{
	counter_on_opening, facade_band, largest_remainder_away_from, opening_along_len, primary_facade,
	side_for_opening,
};

/// Passages at least this long (world units) each get a BitesCounter.
pub const LONG_PASSAGE_MIN: f32 = 2.0;
/// Kitchen stays at least this far (XZ) from every counter.
pub const KITCHEN_COUNTER_CLEARANCE: f32 = 1.0;

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
			let len = opening_along_len(opening, side);
			if len + 1e-3 < LONG_PASSAGE_MIN {
				continue;
			}
			counter_aabbs.push(counter_on_opening(
				&confines.bounds,
				opening,
				side,
				counter_depth,
			));
		}

		// No long passage → one counter on the primary façade (opening or edge).
		if counter_aabbs.is_empty() {
			let (side, _) = primary_facade(confines);
			counter_aabbs.push(facade_band(&confines.bounds, side, counter_depth, 0.7));
		}

		let kitchen_aabb = largest_remainder_away_from(
			&confines.bounds,
			&counter_aabbs,
			KITCHEN_COUNTER_CLEARANCE,
		)
		.filter(|k| {
			let e = crate::fit::aabb_xz_extent(k);
			e.x >= 0.8 && e.y >= 0.8
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

	#[test]
	fn long_passages_each_get_a_counter() {
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
		let confines = Confines::new(
			Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(10.0, 3.0, 6.0)),
			0.0,
			openings,
		);
		let (stall, _) = BitesStall::fit_to_confines(&confines, NoiseParams::default()).unwrap();
		assert_eq!(stall.bites_counters.len(), 2);
		assert_eq!(stall.stall_type.text, "BitesStall");
		assert_eq!(stall.bites_kitchen.text, "BitesKitchen");
	}

	#[test]
	fn kitchen_spans_full_width_behind_both_counters() {
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
		let confines = Confines::new(
			Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(10.0, 3.0, 6.0)),
			0.0,
			openings,
		);
		let (stall, _) = BitesStall::fit_to_confines(&confines, NoiseParams::default()).unwrap();
		let place = &stall.bites_kitchen.placement;
		// Placement scale is full extents; translation is center.
		assert!(
			place.scale.x >= 9.0,
			"kitchen width {} should span nearly full stall",
			place.scale.x
		);
	}
}
