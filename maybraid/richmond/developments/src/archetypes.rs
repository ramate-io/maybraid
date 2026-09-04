//! Reusable development archetypes beyond the original courtyard and village families.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use lod::gen::LodSceneLevel;
use material_ref::MaterialRef;
use procedural_common::NoiseParams;
use richmond_building_components::{
	BuildingComponents, DoorNode, FloorNode, FurnitureNode, JointNode, LabelNode, Layers,
	PanelNode, PartitionNode, RoofNode, StairNode,
};
use richmond_buildings::wizards_tower::WizardsTower;
use richmond_buildings::{
	CellConstraints, Confines, FillableRegions, Fit, FitError, Openings, RectFloor,
	RectFloorParams, RectFloorSlab,
};

use crate::keep::TOWER_STOREY_HEIGHT;
use crate::{BuildingFootprint, PlacedBuilding, RingFortKeep, ShepherdsVillageBuilding};

/// A solitary vertical building sharing the proven keep shell and circulation kit.
#[derive(Debug, Clone, PartialEq)]
pub struct SingleHighrise {
	pub bounds: Aabb3d,
	pub tower: RingFortKeep,
}

impl SingleHighrise {
	pub fn storey_count(&self) -> usize {
		self.tower.storey_count()
	}

	pub fn with_wall_material(mut self, wall: MaterialRef) -> Self {
		self.tower = self.tower.with_wall_material(wall);
		self
	}
}

impl Fit for SingleHighrise {
	fn fit_to_confines(
		confines: &Confines,
		_noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let footprint = confines.footprint();
		let radius = footprint.x.min(footprint.y) * 0.42;
		if radius < 6.0 {
			return Err(FitError::TooSmall { reason: "single_highrise_footprint" });
		}
		let height = confines.bounds.max.y - confines.bounds.min.y;
		let storey_count = (height / TOWER_STOREY_HEIGHT).floor() as usize;
		if storey_count < 8 {
			return Err(FitError::TooSmall { reason: "single_highrise_height" });
		}
		let center = confines.center();
		let tower = RingFortKeep::circular(
			Vec3::new(center.x, confines.bounds.min.y, center.z),
			radius,
			storey_count,
		);
		Ok((
			Self { bounds: confines.bounds, tower },
			FillableRegions { within: Vec::new(), atop: Vec::new() },
		))
	}
}

impl BuildingFootprint for SingleHighrise {
	fn footprint_rects(&self) -> Vec<Aabb2d> {
		let center = self.tower.center_xz();
		let half = self.tower.plan_half_extent();
		vec![Aabb2d {
			min: Vec2::new(center.x - half, center.z - half),
			max: Vec2::new(center.x + half, center.z + half),
		}]
	}
}

/// Solitary integration wrapper for the existing procedural Wizard's Tower.
#[derive(Debug, Clone, PartialEq)]
pub struct SolitaryWizardsTower {
	pub bounds: Aabb3d,
	pub tower: WizardsTower,
}

impl Fit for SolitaryWizardsTower {
	fn fit_to_confines(
		confines: &Confines,
		_noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let footprint = confines.footprint();
		let side = footprint.x.min(footprint.y) * 0.72;
		let height = confines.bounds.max.y - confines.bounds.min.y;
		let storey_height = 4.0;
		let available_floors = (height / storey_height).floor() as u32;
		let available_floors = available_floors.saturating_sub(1);
		if side < 12.0 || available_floors < 10 {
			return Err(FitError::TooSmall { reason: "solitary_wizards_tower" });
		}
		let floor_count = available_floors.min(30);
		let center = confines.center();
		let half = side * 0.5;
		let bounds = Aabb3d::from_min_max(
			Vec3::new(center.x - half, confines.bounds.min.y, center.z - half),
			Vec3::new(
				center.x + half,
				confines.bounds.min.y + (floor_count + 1) as f32 * storey_height,
				center.z + half,
			),
		);
		let constraints = CellConstraints::cell_owned(bounds);
		let floor_noise = (floor_count - 10) as f32 / 20.0;
		let tower = WizardsTower::new(&constraints, floor_noise);
		Ok((Self { bounds, tower }, FillableRegions { within: Vec::new(), atop: Vec::new() }))
	}
}

impl BuildingFootprint for SolitaryWizardsTower {
	fn footprint_rects(&self) -> Vec<Aabb2d> {
		vec![Aabb2d {
			min: Vec2::new(self.bounds.min.x, self.bounds.min.z),
			max: Vec2::new(self.bounds.max.x, self.bounds.max.z),
		}]
	}
}

/// Enclosed elevated lane joining two bazaar towers.
#[derive(Debug, Clone, PartialEq)]
pub struct Skybridge {
	pub bounds: Aabb3d,
	shell: RectFloor,
}

impl Skybridge {
	pub fn new(bounds: Aabb3d) -> Self {
		let center = Vec3::from((bounds.min + bounds.max) * 0.5);
		let footprint = Vec2::new(bounds.max.x - bounds.min.x, bounds.max.z - bounds.min.z);
		let shell = RectFloorParams::new(
			Vec3::new(center.x, bounds.min.y, center.z),
			footprint,
			bounds.max.y - bounds.min.y,
		)
		.floor(RectFloorSlab::Solid)
		.ceiling(RectFloorSlab::Solid)
		.openings(Openings::new())
		.build();
		Self { bounds, shell }
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

/// Axial halls around a taller central shrine.
#[derive(Debug, Clone, PartialEq)]
pub struct TempleComplex {
	pub bounds: Aabb3d,
	pub halls: Vec<ShepherdsVillageBuilding>,
	pub sanctum: PlacedBuilding<SingleHighrise>,
}

impl TempleComplex {
	pub fn with_landmark_material(mut self, wall: MaterialRef) -> Self {
		self.sanctum.building = self.sanctum.building.with_wall_material(wall);
		self
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
}

/// Detached homes with broad spacing and street-like rows.
#[derive(Debug, Clone, PartialEq)]
pub struct SuburbanHomes {
	pub bounds: Aabb3d,
	pub homes: Vec<ShepherdsVillageBuilding>,
}

/// Tight, irregular lanes of mostly small hut-like shops with occasional larger halls.
#[derive(Debug, Clone, PartialEq)]
pub struct OldCityMarket {
	pub bounds: Aabb3d,
	pub buildings: Vec<ShepherdsVillageBuilding>,
}

macro_rules! delegate_components {
	($ty:ty, $field:ident) => {
		impl BuildingComponents for $ty {
			fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
				self.$field.panel_nodes_for_level(level)
			}
			fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
				self.$field.partition_nodes_for_level(level)
			}
			fn floor_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FloorNode> {
				self.$field.floor_nodes_for_level(level)
			}
			fn roof_nodes_for_level(&self, level: LodSceneLevel) -> Layers<RoofNode> {
				self.$field.roof_nodes_for_level(level)
			}
			fn stair_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StairNode> {
				self.$field.stair_nodes_for_level(level)
			}
			fn door_nodes_for_level(&self, level: LodSceneLevel) -> Layers<DoorNode> {
				self.$field.door_nodes_for_level(level)
			}
			fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
				self.$field.joint_nodes_for_level(level)
			}
			fn furniture_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureNode> {
				self.$field.furniture_nodes_for_level(level)
			}
			fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
				self.$field.label_nodes_for_level(level)
			}
		}
	};
}

delegate_components!(SingleHighrise, tower);
delegate_components!(SolitaryWizardsTower, tower);
delegate_components!(Skybridge, shell);

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn solitary_archetypes_use_requested_vertical_envelope() -> anyhow::Result<()> {
		let confines = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(-20.0, 0.0, -20.0),
			Vec3::new(20.0, 80.0, 20.0),
		));
		let (highrise, _) = SingleHighrise::fit_to_confines(&confines, NoiseParams::default())?;
		let (wizard, _) = SolitaryWizardsTower::fit_to_confines(&confines, NoiseParams::default())?;
		assert!(highrise.storey_count() >= 20);
		assert!(wizard.bounds.max.y <= confines.bounds.max.y + 1e-3);
		Ok(())
	}

	#[test]
	fn skybridge_is_a_floor_and_ceiling_shell() {
		let bridge = Skybridge::new(Aabb3d::from_min_max(
			Vec3::new(-12.0, 20.0, -2.0),
			Vec3::new(12.0, 24.0, 2.0),
		));
		assert!(bridge.shell.has_floor());
		assert!(bridge.shell.has_ceiling());
	}
}
