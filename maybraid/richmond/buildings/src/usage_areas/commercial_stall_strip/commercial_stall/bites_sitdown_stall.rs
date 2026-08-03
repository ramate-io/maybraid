//! Bites sit-down stall: counter, kitchen, seating Labels.

use lod::gen::LodSceneLevel;
use procedural_common::{NoiseConfig, NoiseParams};
use richmond_building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};

use crate::fit::{Confines, FillableRegions, Fit, FitError};
use super::label_util::label_filling_aabb;
use super::stall_layout::{facade_band, inset_band, primary_facade};

#[derive(Debug, Clone, PartialEq)]
pub struct BitesSitdownStall {
	pub stall_type: LabelNode,
	pub food_counter: LabelNode,
	pub stall_kitchen: LabelNode,
	pub bites_seating_area: LabelNode,
}

impl Fit for BitesSitdownStall {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let (side, _) = primary_facade(confines);
		let cover = cfg.sample_range_f32_4d(0.5, 0.75, c.x, c.y, c.z, 41.0);
		let counter_depth = cfg.sample_range_f32_4d(0.55, 0.9, c.x, c.y, c.z, 42.0);
		let counter = facade_band(&confines.bounds, side, counter_depth, cover);
		let kitchen = inset_band(&confines.bounds, side, counter_depth + 0.3, 1.1);
		let seating = inset_band(&confines.bounds, side, counter_depth + 1.5, 1.6);
		Ok((
			Self {
				stall_type: label_filling_aabb(
					LabelStyle::Yellow,
					"BitesSitdownStall",
					&confines.bounds,
					confines.roll,
				),
				food_counter: label_filling_aabb(
					LabelStyle::Yellow,
					"FoodCounter",
					&counter,
					confines.roll,
				),
				stall_kitchen: label_filling_aabb(
					LabelStyle::Orange,
					"StallKitchen",
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
		Layers::from_free(vec![
			self.stall_type.clone(),
			self.food_counter.clone(),
			self.stall_kitchen.clone(),
			self.bites_seating_area.clone(),
		])
	}
}
