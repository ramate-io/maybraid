//! Demo: [`ArcTower`] joined to a [`Trazaloid`] by a [`ConnectingHall`].
//!
//! # Joinery note
//!
//! Start from the two openings and connect **backwards**. Prefer endpoints that
//! **overrun** the door a little on either side — a hall that reads slightly wide
//! of the jambs looks better than one that stops short or goes too narrow.

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

/// Door facing the trazaloid: kit sweep \(t = 0.5 → +X\) (`start_yaw = 0`).
const DOOR_T: f32 = 0.5;

/// Extra meters past each jamb on the arc-tower hall end (15° door is narrow).
const TOWER_OVERRUN_M: f32 = 0.35;
/// Extra meters past each jamb on the trazaloid hall end (reads wider than the door).
const TRAZALOID_OVERRUN_M: f32 = 1.1;

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
				MustAssignPortal::at(0.0, Portal::Window),
				MustAssignPortal::at(0.25, Portal::Window),
				MustAssignPortal::at(DOOR_T, Portal::Door),
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

		// Hall tops on the trazaloid end follow the footprint→waist pitch (door clip
		// points lie on that face; Tube stations keep authored `top_middle`).
		let end_tower = tower
			.portal_endpoint(0, DOOR_T)
			.expect("arc tower ground door")
			.widened(TOWER_OVERRUN_M);
		let end_traz = trazaloid
			.door_endpoint(TrazaloidSide::West)
			.expect("trazaloid west door")
			.widened(TRAZALOID_OVERRUN_M);
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

	#[test]
	fn trazaloid_hall_end_carries_face_pitch() {
		let demo = ConnectingShells::new();
		let (_a, end_b) = demo.hall().endpoints();
		// West door: top should be inset (larger x) relative to bottom.
		let bottom_x = 0.5 * (end_b.targets.0.x + end_b.targets.1.x);
		let top_x = 0.5 * (end_b.targets.2.x + end_b.targets.3.x);
		assert!(top_x > bottom_x + 1e-3);
		let stations = demo.hall().stations();
		let traz_station = stations[2];
		let top = traz_station.top_middle.expect("pitched top_middle");
		assert!(
			(top.x - traz_station.bottom_middle.x).abs() > 1e-3,
			"tube station should keep non-vertical top lift"
		);
	}

	#[test]
	fn tower_hall_end_on_next_clockwise_segment() {
		let demo = ConnectingShells::new();
		let (end_a, _) = demo.hall().endpoints();
		let mid = (end_a.targets.0 + end_a.targets.1) * 0.5;
		// Clockwise of t=0.5 is decreasing t → mid toward +Z of +X.
		assert!(mid.z > 0.5, "mid={mid:?}");
		assert!(end_a.orientation.normalize().x > 0.7, "orient={:?}", end_a.orientation);
	}

	#[test]
	fn both_ends_widen_past_door_jambs() {
		let demo = ConnectingShells::new();
		let stations = demo.hall().stations();
		// Tower: ~R sin(7.5°) + overrun ≈ 0.52 + 0.35
		assert!(stations[0].bottom_left_width > 0.8, "{}", stations[0].bottom_left_width);
		assert!(
			(stations[0].bottom_left_width - stations[0].bottom_right_width).abs() < 1e-3
		);
		// Trazaloid: door half 0.6 + overrun 1.1 = 1.7 (must not shrink from L/R swap)
		assert!(
			stations[2].bottom_left_width > 1.5,
			"traz width {} — outward corner order / widen bug?",
			stations[2].bottom_left_width
		);
		assert!(
			(stations[2].bottom_left_width - stations[2].bottom_right_width).abs() < 1e-3
		);
	}
}
