//! Cuboids from panel / floor / partition placement and stair tread IR.

use avian3d::prelude::{Collider, Friction, RigidBody};
use bevy::prelude::*;
use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use lod_avian::PhysicsInteractionLayer;
use richmond_building_components::floors::FloorGeometry;
use richmond_building_components::panels::{to_centered_rect_placement, PanelGeometry};
use richmond_building_components::partitions::{PartitionGeometry, PANEL_Y_HALF};
use richmond_building_components::placed::Placement;
use richmond_building_components::{BuildingComponents, FloorNode, PanelNode, PartitionNode};

use crate::BuildingFrictionConfig;

/// Marks a Fixed cuboid spawned from building IR (not a LOD Host volume).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct BuildingWalkCollider;

const KIT_MIN: Vec3 = Vec3::new(0.0, -PANEL_Y_HALF, 0.0);
const KIT_MAX: Vec3 = Vec3::new(1.0, PANEL_Y_HALF, 1.0);

/// Stamp walk colliders as children of `parent` from High-LOD domain nodes.
pub fn spawn_building_walk_colliders(
	commands: &mut Commands,
	parent: Entity,
	building: &impl BuildingComponents,
	friction: Friction,
) {
	let level = LodSceneLevel::High;
	for node in building.panel_nodes_for_level(level).flatten() {
		if let Some(pose) = panel_cuboid(&node) {
			spawn_cuboid(commands, parent, pose, friction);
		}
	}
	for node in building.floor_nodes_for_level(level).flatten() {
		if let Some(pose) = floor_cuboid(&node) {
			spawn_cuboid(commands, parent, pose, friction);
		}
	}
	for node in building.partition_nodes_for_level(level).flatten() {
		if let Some(pose) = partition_cuboid(&node) {
			spawn_cuboid(commands, parent, pose, friction);
		}
	}
	for node in building.stair_nodes_for_level(level).flatten() {
		for (translation, rotation, size) in node.tread_cuboids() {
			spawn_cuboid(
				commands,
				parent,
				CuboidPose { translation, rotation, size },
				friction,
			);
		}
	}
}

/// Convenience when the app has [`BuildingFrictionConfig`].
impl BuildingFrictionConfig {
	pub fn spawn_walk_colliders(
		self,
		commands: &mut Commands,
		parent: Entity,
		building: &impl BuildingComponents,
	) {
		spawn_building_walk_colliders(commands, parent, building, self.0);
	}
}

struct CuboidPose {
	translation: Vec3,
	rotation: Quat,
	size: Vec3,
}

fn spawn_cuboid(commands: &mut Commands, parent: Entity, pose: CuboidPose, friction: Friction) {
	let size = pose.size.max(Vec3::splat(0.05));
	commands.spawn((
		Name::new("building-walk-collider"),
		BuildingWalkCollider,
		ChildOf(parent),
		Transform::from_translation(pose.translation).with_rotation(pose.rotation),
		Visibility::Hidden,
		RigidBody::Static,
		Collider::cuboid(size.x, size.y, size.z),
		PhysicsInteractionLayer::fixed_layers(),
		friction,
	));
}

fn oriented_kit_cuboid(placement: Placement, kit_min: Vec3, kit_max: Vec3) -> CuboidPose {
	let size = (kit_max - kit_min).abs().max(Vec3::splat(1e-3));
	let kit_center = (kit_min + kit_max) * 0.5;
	let scaled = kit_center * placement.scale;
	CuboidPose {
		translation: placement.translation + placement.rotation() * scaled,
		rotation: placement.rotation(),
		size: size * placement.scale.abs(),
	}
}

fn panel_cuboid(node: &PanelNode) -> Option<CuboidPose> {
	match node.geometry {
		PanelGeometry::Rectangle(_) | PanelGeometry::RightTriangle(_) => {
			Some(oriented_kit_cuboid(node.placement, KIT_MIN, KIT_MAX))
		}
		_ => None,
	}
}

fn floor_cuboid(node: &FloorNode) -> Option<CuboidPose> {
	match node.geometry {
		FloorGeometry::Rectangle(_) => {
			let centered = to_centered_rect_placement(node.placement);
			let size = Vec3::new(
				(centered.scale.x * 2.0).abs().max(0.2),
				(centered.scale.y.abs() * PANEL_Y_HALF * 2.0).max(0.08),
				(centered.scale.z * 2.0).abs().max(0.2),
			);
			Some(CuboidPose {
				translation: centered.translation,
				rotation: centered.rotation(),
				size,
			})
		}
		_ => None,
	}
}

fn partition_cuboid(node: &PartitionNode) -> Option<CuboidPose> {
	match node.geometry {
		PartitionGeometry::Linear(_) => Some(oriented_kit_cuboid(node.placement, KIT_MIN, KIT_MAX)),
		_ => None,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use richmond_building_components::placed::Placement;

	#[test]
	fn kit_cuboid_centers_a_unit_panel() -> anyhow::Result<()> {
		let pose = oriented_kit_cuboid(Placement::IDENTITY, KIT_MIN, KIT_MAX);
		assert!((pose.translation - Vec3::new(0.5, 0.0, 0.5)).length() < 1e-4);
		assert!((pose.size.x - 1.0).abs() < 1e-4);
		assert!((pose.size.z - 1.0).abs() < 1e-4);
		assert!((pose.size.y - PANEL_Y_HALF * 2.0).abs() < 1e-4);
		Ok(())
	}
}
