use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::Vec2;
use lod::gen::LodSceneLevel;
use material_ref::MaterialRef;
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{building_bounds, BuildingComponents, Layers};
use richmond_buildings::{CardinalFace, ConnectingHall, MappedOpening};

use crate::{BuildingFootprint, PlacedBuilding, ShepherdsVillageBuilding, SingleHighrise};

/// Enclosed elevated lane joining two bazaar towers.
#[derive(Debug, Clone, PartialEq)]
pub struct Skybridge {
	pub bounds: Aabb3d,
	pub hall: ConnectingHall,
	pub storey: usize,
	pub directions: [CardinalFace; 2],
	material: Option<MaterialRef>,
}

impl Skybridge {
	pub fn new(hall: ConnectingHall, storey: usize, directions: [CardinalFace; 2]) -> Self {
		let bounds = building_bounds(&hall);
		Self { bounds, hall, storey, directions, material: None }
	}

	pub fn with_material(mut self, material: MaterialRef) -> Self {
		self.material = Some(material);
		self
	}

	pub fn endpoints(&self) -> (MappedOpening, MappedOpening) {
		self.hall.endpoints()
	}

	pub fn material(&self) -> Option<&MaterialRef> {
		self.material.as_ref()
	}
}

impl BuildingFootprint for Skybridge {
	fn footprint_rects(&self) -> Vec<Aabb2d> {
		vec![Aabb2d {
			min: Vec2::new(self.bounds.min.x, self.bounds.min.z),
			max: Vec2::new(self.bounds.max.x, self.bounds.max.z),
		}]
	}
}

impl BuildingComponents for Skybridge {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let panels = self.hall.panel_nodes_for_level(level);
		match &self.material {
			Some(material) => panels.with_material(material.clone()),
			None => panels,
		}
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.hall.joint_nodes_for_level(level)
	}
}

/// Towers, occupied skybridges, and a market at ground level.
#[derive(Debug, Clone, PartialEq)]
pub struct SkybridgeBazaar {
	pub bounds: Aabb3d,
	pub towers: Vec<PlacedBuilding<SingleHighrise>>,
	pub bridges: Vec<PlacedBuilding<Skybridge>>,
	pub market: Vec<ShepherdsVillageBuilding>,
}

impl SkybridgeBazaar {
	pub fn with_tower_material(mut self, wall: MaterialRef) -> Self {
		for tower in &mut self.towers {
			tower.building = tower.building.clone().with_wall_material(wall.clone());
		}
		self
	}

	pub fn with_bridge_material(mut self, material: MaterialRef) -> Self {
		for bridge in &mut self.bridges {
			bridge.building = bridge.building.clone().with_material(material.clone());
		}
		self
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::{Vec2, Vec3};
	use richmond_building_components::BuildingComponents;
	use richmond_buildings::{MappedOpening, MappedOpeningQuad};

	#[test]
	fn skybridge_uses_connecting_hall_geometry_without_floor_nodes() {
		let a = MappedOpening::new(
			MappedOpeningQuad::new(
				Vec3::new(-8.0, 20.0, -1.5),
				Vec3::new(-8.0, 20.0, 1.5),
				Vec3::new(-8.0, 23.0, -1.5),
				Vec3::new(-8.0, 23.0, 1.5),
			),
			Vec2::X,
		);
		let b = MappedOpening::new(
			MappedOpeningQuad::new(
				Vec3::new(8.0, 20.0, 1.5),
				Vec3::new(8.0, 20.0, -1.5),
				Vec3::new(8.0, 23.0, 1.5),
				Vec3::new(8.0, 23.0, -1.5),
			),
			-Vec2::X,
		);
		let bridge = Skybridge::new(
			ConnectingHall::rough_stone(a, b),
			6,
			[CardinalFace::East, CardinalFace::West],
		);
		assert!(!bridge.panel_nodes_for_level(LodSceneLevel::High).is_empty());
		assert!(bridge.floor_nodes_for_level(LodSceneLevel::High).is_empty());
		assert_eq!(bridge.endpoints(), (a, b));
		assert_eq!(bridge.storey, 6);
	}
}
