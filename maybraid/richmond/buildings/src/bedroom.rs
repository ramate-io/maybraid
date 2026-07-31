//! Hierarchical bedroom: allocate cells, children fill them.

pub mod bed;
pub mod closet;
pub mod ensuite;
pub mod layout;
pub mod nightstand;
pub mod shell;

pub use bed::Bed;
pub use closet::Closet;
pub use ensuite::EnsuiteBathroom;
pub use layout::{BedroomFillParams, BedroomLayout, PartitionSlot};
pub use nightstand::Nightstand;
pub use shell::ShellWall;

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use richmond_building_components::floors::{Floor, FloorNode};
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{
	BuildingComponents, FurnitureNode, Layers, Placement,
};

use crate::bedroom::shell::face_rectangle;
use crate::constraints::{BoundaryOwnershipEntry, BoundaryOwnershipStatus, FaceKind};
use crate::wizards_tower::floor_fill::{FLOOR_SLAB_Y_SCALE, RECT_HALF_EXTENT};
use crate::CellConstraints;
use procedural_common::NoiseParams;

const OUTER_WALL_THICK: f32 = 0.15;

/// Bedroom cell: outer shell + allocated closet / bed / nightstand / ensuite fills.
#[derive(Debug, Clone, PartialEq)]
pub struct Bedroom {
	pub constraints: CellConstraints,
	pub floor: FloorNode,
	pub walls: Vec<ShellWall>,
	pub closets: Vec<Closet>,
	pub beds: Vec<Bed>,
	pub nightstands: Vec<Nightstand>,
	pub ensuites: Vec<EnsuiteBathroom>,
}

impl Bedroom {
	/// Allocate child cells inside `constraints` via noise-fitted layout.
	pub fn new(constraints: CellConstraints, noise: f32) -> Self {
		Self::with_fill(constraints, noise, BedroomFillParams::default())
	}

	/// Same as [`Self::new`] with explicit spaciousness / occupancy budgets.
	pub fn with_fill(constraints: CellConstraints, noise: f32, fill: BedroomFillParams) -> Self {
		let noise = NoiseParams {
			seed: (noise.clamp(0.0, 1.0) * 1_000_000.0) as i32,
			..NoiseParams::default()
		};
		let layout = BedroomLayout::fit(&constraints, noise, fill);
		let closets = layout
			.closets
			.into_iter()
			.map(|slot| Closet::new(subset_or_owned(&constraints, slot.aabb), slot.open_face))
			.collect();
		let beds = layout
			.beds
			.into_iter()
			.map(|aabb| Bed::new(subset_or_owned(&constraints, aabb)))
			.collect();
		let nightstands = layout
			.nightstands
			.into_iter()
			.map(|aabb| Nightstand::new(subset_or_owned(&constraints, aabb)))
			.collect();
		let ensuites = layout
			.ensuites
			.into_iter()
			.map(|slot| {
				EnsuiteBathroom::new(subset_or_owned(&constraints, slot.aabb), slot.open_face)
			})
			.collect();

		let floor = room_floor(&constraints);
		let walls = room_outer_walls(&constraints);

		Self { constraints, floor, walls, closets, beds, nightstands, ensuites }
	}
}

impl BuildingComponents for Bedroom {
	fn floor_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FloorNode> {
		Layers::from_free(vec![self.floor.clone()])
	}

	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for w in &self.walls {
			out.extend(w.panel_nodes_for_level(level));
		}
		for c in &self.closets {
			out.extend(c.panel_nodes_for_level(level));
		}
		for e in &self.ensuites {
			out.extend(e.panel_nodes_for_level(level));
		}
		out
	}

	fn furniture_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureNode> {
		let mut out = Layers::new();
		for c in &self.closets {
			out.extend(c.furniture_nodes_for_level(level));
		}
		for b in &self.beds {
			out.extend(b.furniture_nodes_for_level(level));
		}
		for n in &self.nightstands {
			out.extend(n.furniture_nodes_for_level(level));
		}
		for e in &self.ensuites {
			out.extend(e.furniture_nodes_for_level(level));
		}
		out
	}
}

fn subset_or_owned(parent: &CellConstraints, aabb: Aabb3d) -> CellConstraints {
	parent.subset(aabb).unwrap_or_else(|_| CellConstraints::cell_owned(aabb))
}

fn room_floor(constraints: &CellConstraints) -> FloorNode {
	let aabb = &constraints.aabb;
	let center = (aabb.min + aabb.max) * 0.5;
	let center_xz = Vec3::new(center.x, aabb.min.y, center.z);
	let size = aabb.max - aabb.min;
	let floor_scale = Vec3::new(
		size.x.max(1e-4) / (2.0 * RECT_HALF_EXTENT),
		FLOOR_SLAB_Y_SCALE,
		size.z.max(1e-4) / (2.0 * RECT_HALF_EXTENT),
	);
	FloorNode::rough_stone(
		Floor::rectangle(),
		Placement::new(center_xz, 0.0).with_scale(floor_scale),
	)
}

/// True when this face is wholly cell-owned (or absent, which implies Cell).
pub(crate) fn owns_face_as_cell(constraints: &CellConstraints, face: FaceKind) -> bool {
	match constraints.boundary_ownership.get(face) {
		None => true,
		Some(BoundaryOwnershipEntry::Whole(BoundaryOwnershipStatus::Cell)) => true,
		Some(_) => false,
	}
}

fn room_outer_walls(constraints: &CellConstraints) -> Vec<ShellWall> {
	let aabb = &constraints.aabb;
	let mut walls = Vec::new();
	for face in [FaceKind::Front, FaceKind::Back, FaceKind::Left, FaceKind::Right] {
		if !owns_face_as_cell(constraints, face) {
			continue;
		}
		if let Some(r) = face_rectangle(aabb, face, OUTER_WALL_THICK) {
			walls.push(ShellWall(r));
		}
	}
	walls
}

/// Placement that fills `aabb` with a unit cube centered in the volume.
pub(crate) fn placement_filling_aabb(aabb: &Aabb3d) -> Placement {
	let center = Vec3::from((aabb.min + aabb.max) * 0.5);
	let extent = Vec3::from(aabb.max - aabb.min).max(Vec3::splat(1e-4));
	Placement::new(center, 0.0).with_scale(extent)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::constraints::{BoundaryOwnershipEntry, BoundaryOwnershipStatus, CellConstraints};

	#[test]
	fn room_outer_walls_skip_parent_owned_faces() -> anyhow::Result<()> {
		let mut constraints =
			CellConstraints::cell_owned(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(4.0, 3.0, 3.5)));
		constraints.boundary_ownership.front =
			Some(BoundaryOwnershipEntry::Whole(BoundaryOwnershipStatus::Parent));
		constraints.boundary_ownership.left =
			Some(BoundaryOwnershipEntry::Whole(BoundaryOwnershipStatus::Sibling));

		let walls = room_outer_walls(&constraints);
		assert_eq!(walls.len(), 2);
		Ok(())
	}

	#[test]
	fn cell_owned_room_emits_all_four_walls() -> anyhow::Result<()> {
		let constraints =
			CellConstraints::cell_owned(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(4.0, 3.0, 3.5)));
		assert_eq!(room_outer_walls(&constraints).len(), 4);
		Ok(())
	}
}
