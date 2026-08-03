//! Supermarket stall: office divider + aisles / register / shelves Labels.

use lod::gen::LodSceneLevel;
use procedural_common::NoiseParams;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};

use crate::fit::{Confines, FillableRegions, Fit, FitError};
use crate::paneling::Rectangle;
use super::label_util::label_filling_aabb;
use super::stall_layout::{
	back_third, facade_band, inset_band, office_divider_wall, primary_facade, sales_minus_office,
};

#[derive(Debug, Clone, PartialEq)]
pub struct SupermarketStall {
	pub office_wall: Option<Rectangle>,
	pub supermarket_stall_office: LabelNode,
	pub stall_aisles: LabelNode,
	pub register: LabelNode,
	pub grocery_shelves: LabelNode,
}

impl Fit for SupermarketStall {
	fn fit_to_confines(
		confines: &Confines,
		_noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let (side, _) = primary_facade(confines);
		let office = back_third(&confines.bounds, side);
		let sales = sales_minus_office(&confines.bounds, &office, side);
		let office_wall = office_divider_wall(&confines.bounds, &office, side);
		let register = facade_band(&sales, side, 1.0, 0.45);
		let shelves = inset_band(&sales, side, 1.2, 1.2);
		let aisle_depth = 1.8_f32;
		let aisles = inset_band(&sales, side, 2.5, aisle_depth);
		Ok((
			Self {
				office_wall,
				supermarket_stall_office: label_filling_aabb(
					LabelStyle::Blue,
					"SupermarketStallOffice",
					&office,
					confines.roll,
				),
				stall_aisles: label_filling_aabb(
					LabelStyle::Cyan,
					"StallAisles",
					&aisles,
					confines.roll,
				),
				register: label_filling_aabb(LabelStyle::Magenta, "Register", &register, confines.roll),
				grocery_shelves: label_filling_aabb(
					LabelStyle::Gray,
					"GroceryShelves",
					&shelves,
					confines.roll,
				),
			},
			FillableRegions::empty(),
		))
	}
}

impl BuildingComponents for SupermarketStall {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		if let Some(wall) = &self.office_wall {
			out.extend(wall.panel_nodes_for_level(level));
		}
		out
	}

	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		Layers::from_free(vec![
			self.supermarket_stall_office.clone(),
			self.stall_aisles.clone(),
			self.register.clone(),
			self.grocery_shelves.clone(),
		])
	}
}
