//! Bites stall: FoodCounter + StallKitchen Labels.

use lod::gen::LodSceneLevel;
use procedural_common::{NoiseConfig, NoiseParams};
use richmond_building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};

use crate::fit::{Confines, FillableRegions, Fit, FitError};
use super::label_util::label_filling_aabb;
use super::stall_layout::{facade_band, inset_band, primary_facade};

#[derive(Debug, Clone, PartialEq)]
pub struct BitesStall {
	pub food_counter: LabelNode,
	pub stall_kitchen: LabelNode,
}

impl Fit for BitesStall {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let (side, _) = primary_facade(confines);
		let cover = cfg.sample_range_f32_4d(0.5, 0.75, c.x, c.y, c.z, 31.0);
		let counter_depth = cfg.sample_range_f32_4d(0.6, 1.0, c.x, c.y, c.z, 32.0);
		let counter = facade_band(&confines.bounds, side, counter_depth, cover);
		let kitchen = inset_band(&confines.bounds, side, counter_depth + 0.35, 1.4);
		let style = LabelStyle::from_unit(cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, 33.0));
		Ok((
			Self {
				food_counter: label_filling_aabb(style, "FoodCounter", &counter, confines.roll),
				stall_kitchen: label_filling_aabb(
					LabelStyle::Orange,
					"StallKitchen",
					&kitchen,
					confines.roll,
				),
			},
			FillableRegions::empty(),
		))
	}
}

impl BuildingComponents for BitesStall {
	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		Layers::from_free(vec![self.food_counter.clone(), self.stall_kitchen.clone()])
	}
}
