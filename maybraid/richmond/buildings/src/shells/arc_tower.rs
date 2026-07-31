//! Stacked circular storey shells ([`ArcFloor`]) with developer-chosen openings and slabs.

use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use richmond_building_components::floors::FloorNode;
use richmond_building_components::partitions::{PartitionNode, PartitionStyle};
use richmond_building_components::{BuildingComponents, Layers};

use crate::portals::MustAssignPortal;
use crate::shells::arc_floor::{ArcFloor, ArcFloorParams, ArcFloorSlab};
use crate::shells::connecting_hall::ConnectingHallEndpoint;

/// Authored parameters for an [`ArcTower`] shell.
#[derive(Debug, Clone, PartialEq)]
pub struct ArcTowerParams {
	/// Ground-plan center; `y` is the base floor elevation.
	pub center_xz: Vec3,
	pub radius: f32,
	/// Number of stacked storeys (developer-chosen; no noise mapping).
	pub floor_count: u32,
	pub storey_height: f32,
	/// World yaw (radians) of each ring sweep start (\(t = 0\)).
	pub start_yaw: f32,
	/// Same openings on every storey.
	pub openings: Vec<MustAssignPortal>,
	/// Floor slab on storey 0.
	pub base_floor: ArcFloorSlab,
	/// Floor slabs on storeys \(1..n-1\).
	pub intermediate_floors: ArcFloorSlab,
	/// Ceiling slab on the top storey only.
	pub top_ceiling: ArcFloorSlab,
	pub style: PartitionStyle,
}

impl Default for ArcTowerParams {
	fn default() -> Self {
		Self {
			center_xz: Vec3::ZERO,
			radius: 4.0,
			floor_count: 3,
			storey_height: 3.0,
			start_yaw: 0.0,
			openings: Vec::new(),
			base_floor: ArcFloorSlab::Solid,
			intermediate_floors: ArcFloorSlab::SquareHole { size: 2.24 },
			top_ceiling: ArcFloorSlab::Solid,
			style: PartitionStyle::RoughStonework,
		}
	}
}

/// Vertical stack of [`ArcFloor`] storeys.
#[derive(Debug, Clone, PartialEq)]
pub struct ArcTower {
	params: ArcTowerParams,
	storeys: Vec<ArcFloor>,
}

impl ArcTower {
	pub fn new(params: ArcTowerParams) -> Self {
		let radius = params.radius.max(1e-4);
		let storey_height = params.storey_height.max(1e-4);
		let floor_count = params.floor_count.max(1);
		let base_y = params.center_xz.y;
		let center_xz = Vec3::new(params.center_xz.x, base_y, params.center_xz.z);

		let storeys = (0..floor_count)
			.map(|i| {
				let y = base_y + i as f32 * storey_height;
				let floor = if i == 0 {
					params.base_floor
				} else {
					params.intermediate_floors
				};
				let ceiling = if i + 1 == floor_count {
					params.top_ceiling
				} else {
					ArcFloorSlab::None
				};
				ArcFloor::new(ArcFloorParams {
					center_xz: Vec3::new(center_xz.x, y, center_xz.z),
					radius,
					storey_height,
					start_yaw: params.start_yaw,
					openings: params.openings.clone(),
					floor,
					ceiling,
					style: params.style,
				})
			})
			.collect();

		Self {
			params: ArcTowerParams {
				center_xz,
				radius,
				floor_count,
				storey_height,
				..params
			},
			storeys,
		}
	}

	pub fn params(&self) -> &ArcTowerParams {
		&self.params
	}

	pub fn storeys(&self) -> &[ArcFloor] {
		&self.storeys
	}

	pub fn storey(&self, index: usize) -> Option<&ArcFloor> {
		self.storeys.get(index)
	}

	/// Portal opening on `storey` centered at normalized \(t\`.
	pub fn portal_endpoint(&self, storey: usize, t: f32) -> Option<ConnectingHallEndpoint> {
		self.storey(storey)?.portal_endpoint(t)
	}
}

impl BuildingComponents for ArcTower {
	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		let mut out = Layers::new();
		for storey in &self.storeys {
			out.extend(storey.partition_nodes_for_level(level));
		}
		out
	}

	fn floor_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FloorNode> {
		let mut out = Layers::new();
		for storey in &self.storeys {
			out.extend(storey.floor_nodes_for_level(level));
		}
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::portals::Portal;

	#[test]
	fn stacks_requested_floor_count() {
		let tower = ArcTower::new(ArcTowerParams {
			floor_count: 4,
			openings: vec![MustAssignPortal::at(0.0, Portal::Door)],
			..ArcTowerParams::default()
		});
		assert_eq!(tower.storeys().len(), 4);
	}

	#[test]
	fn only_top_storey_has_ceiling() {
		let tower = ArcTower::new(ArcTowerParams {
			floor_count: 3,
			base_floor: ArcFloorSlab::Solid,
			intermediate_floors: ArcFloorSlab::None,
			top_ceiling: ArcFloorSlab::Solid,
			..ArcTowerParams::default()
		});
		assert!(tower.storey(0).unwrap().ceiling_nodes().is_empty());
		assert!(tower.storey(1).unwrap().ceiling_nodes().is_empty());
		assert!(!tower.storey(2).unwrap().ceiling_nodes().is_empty());
	}

	#[test]
	fn zero_floor_count_becomes_one() {
		let tower = ArcTower::new(ArcTowerParams {
			floor_count: 0,
			..ArcTowerParams::default()
		});
		assert_eq!(tower.storeys().len(), 1);
	}
}
