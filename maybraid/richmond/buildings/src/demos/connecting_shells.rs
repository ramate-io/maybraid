//! Demo: [`ArcTower`] joined to a [`Trazaloid`] by a [`ConnectingHall`].

use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use richmond_building_components::floors::FloorNode;
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::partitions::{PartitionNode, PartitionStyle};
use richmond_building_components::{BuildingComponents, Layers};

use crate::portals::{MustAssignPortal, Portal};
use crate::shells::arc_floor::ArcFloorSlab;
use crate::shells::arc_tower::{ArcTower, ArcTowerParams};
use crate::shells::connecting_hall::ConnectingHall;
use crate::shells::trazaloid::{
	Trazaloid, TrazaloidDoors, TrazaloidParams, TrazaloidSide, TrazaloidSlab,
};

/// Door at \(t = 0\) faces \(+X\) with default `start_yaw = 0`.
const DOOR_T: f32 = 0.0;

/// Fixed composition: circular tower west of a trazaloid, linked by a one-kink hall.
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectingShells {
	tower: ArcTower,
	hall: ConnectingHall,
	trazaloid: Trazaloid,
}

impl ConnectingShells {
	pub fn new() -> Self {
		let tower = ArcTower::new(ArcTowerParams {
			// Door on +X sits near x ≈ -10; trazaloid west face at x = -4.
			center_xz: Vec3::new(-14.0, 0.0, 0.0),
			radius: 4.0,
			floor_count: 3,
			storey_height: 3.0,
			start_yaw: 0.0,
			openings: vec![
				MustAssignPortal::at(DOOR_T, Portal::Door),
				MustAssignPortal::at(0.25, Portal::Window),
				MustAssignPortal::at(0.5, Portal::Window),
				MustAssignPortal::at(0.75, Portal::Window),
			],
			base_floor: ArcFloorSlab::Solid,
			intermediate_floors: ArcFloorSlab::SquareHole { size: 2.24 },
			top_ceiling: ArcFloorSlab::Solid,
			style: PartitionStyle::RoughStonework,
		});

		let trazaloid = Trazaloid::new(TrazaloidParams {
			footprint: bevy_math::Vec2::new(8.0, 6.0),
			ridge: bevy_math::Vec2::new(4.0, 3.0),
			lower_height: 3.0,
			upper_height: 2.5,
			band_vertical_offset: 0.35,
			waist_horizontal_offset: 0.25,
			doors: TrazaloidDoors {
				west: true,
				..TrazaloidDoors::NONE
			},
			door_width_frac: 0.28,
			door_thickness: 1.2,
			door_height_frac: 0.7,
			floor: TrazaloidSlab::None,
			ceiling: TrazaloidSlab::Solid,
			style: richmond_building_components::panels::PanelStyle::RoughStonework,
			joint_thickness: crate::paneling::panel_complex::DEFAULT_PANEL_THICKNESS,
			face_post_count: 2,
		});

		let end_tower = tower
			.portal_endpoint(0, DOOR_T)
			.expect("arc tower ground door");
		let end_traz = trazaloid
			.door_endpoint(TrazaloidSide::West)
			.expect("trazaloid west door");
		let hall = ConnectingHall::rough_stone(end_tower, end_traz);

		Self {
			tower,
			hall,
			trazaloid,
		}
	}

	pub fn tower(&self) -> &ArcTower {
		&self.tower
	}

	pub fn hall(&self) -> &ConnectingHall {
		&self.hall
	}

	pub fn trazaloid(&self) -> &Trazaloid {
		&self.trazaloid
	}
}

impl Default for ConnectingShells {
	fn default() -> Self {
		Self::new()
	}
}

impl BuildingComponents for ConnectingShells {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = self.trazaloid.panel_nodes_for_level(level);
		out.extend(self.hall.panel_nodes_for_level(level));
		out
	}

	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		self.tower.partition_nodes_for_level(level)
	}

	fn floor_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FloorNode> {
		self.tower.floor_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = self.trazaloid.joint_nodes_for_level(level);
		out.extend(self.hall.joint_nodes_for_level(level));
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn hall_stations_built_between_shells() {
		let demo = ConnectingShells::new();
		let stations = demo.hall().stations();
		assert!(stations[0].bottom_middle.x < stations[2].bottom_middle.x);
		assert!(demo.hall().midpoint().x.is_finite());
		assert!(!demo
			.hall()
			.panel_nodes_for_level(LodSceneLevel::High)
			.flatten()
			.is_empty());
		assert_eq!(demo.tower().storeys().len(), 3);
	}
}
