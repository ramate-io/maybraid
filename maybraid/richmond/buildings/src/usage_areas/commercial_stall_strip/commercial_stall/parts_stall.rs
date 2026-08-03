//! Parts stall: PartsOffice + PartsRacks Labels.

use lod::gen::LodSceneLevel;
use procedural_common::NoiseParams;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};

use crate::fit::{Confines, FillableRegions, Fit, FitError};
use crate::paneling::Rectangle;
use super::label_util::label_filling_aabb;
use super::stall_layout::{
	back_third, inset_band, office_divider_wall, primary_facade, sales_minus_office,
};

#[derive(Debug, Clone, PartialEq)]
pub struct PartsStall {
	pub stall_type: LabelNode,
	pub office_wall: Option<Rectangle>,
	pub parts_office: LabelNode,
	pub parts_racks: LabelNode,
}

impl Fit for PartsStall {
	fn fit_to_confines(
		confines: &Confines,
		_noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let (side, _) = primary_facade(confines);
		let office = back_third(&confines.bounds, side);
		let sales = sales_minus_office(&confines.bounds, &office, side);
		let racks = inset_band(&sales, side, 0.8, 2.0);
		Ok((
			Self {
				stall_type: label_filling_aabb(
					LabelStyle::Blue,
					"PartsStall",
					&confines.bounds,
					confines.roll,
				),
				office_wall: office_divider_wall(&confines.bounds, &office, side),
				parts_office: label_filling_aabb(
					LabelStyle::Blue,
					"PartsOffice",
					&office,
					confines.roll,
				),
				parts_racks: label_filling_aabb(
					LabelStyle::Gray,
					"PartsRacks",
					&racks,
					confines.roll,
				),
			},
			FillableRegions::empty(),
		))
	}
}

impl BuildingComponents for PartsStall {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		if let Some(wall) = &self.office_wall {
			out.extend(wall.panel_nodes_for_level(level));
		}
		out
	}

	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		Layers::from_free(vec![
			self.stall_type.clone(),
			self.parts_office.clone(),
			self.parts_racks.clone(),
		])
	}
}
