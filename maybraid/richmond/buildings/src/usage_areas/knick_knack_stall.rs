//! Knick-knack stall: rack aisle Labels.

use lod::gen::LodSceneLevel;
use procedural_common::NoiseParams;
use richmond_building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};

use crate::fit::{Confines, FillableRegions, Fit, FitError};
use crate::usage_areas::label_util::{inset_xz, label_filling_aabb};
use crate::usage_areas::stall_layout::{facade_band, inset_band, primary_facade};

#[derive(Debug, Clone, PartialEq)]
pub struct KnickKnackStall {
	pub knick_knack_racks: LabelNode,
	pub front_display: LabelNode,
}

impl Fit for KnickKnackStall {
	fn fit_to_confines(
		confines: &Confines,
		_noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let (side, _) = primary_facade(confines);
		let front = facade_band(&confines.bounds, side, 0.8, 0.7);
		let racks = inset_band(&confines.bounds, side, 1.0, 2.2);
		let racks = inset_xz(&racks, 0.15);
		Ok((
			Self {
				knick_knack_racks: label_filling_aabb(
					LabelStyle::Magenta,
					"KnickKnackRacks",
					&racks,
					confines.roll,
				),
				front_display: label_filling_aabb(
					LabelStyle::Cyan,
					"KnickKnackDisplay",
					&front,
					confines.roll,
				),
			},
			FillableRegions::empty(),
		))
	}
}

impl BuildingComponents for KnickKnackStall {
	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		Layers::from_free(vec![
			self.knick_knack_racks.clone(),
			self.front_display.clone(),
		])
	}
}
