//! Cuboids from panel / floor / partition placement, triangle prisms, and stair ramps.

use avian3d::prelude::{Collider, Friction, RigidBody};
use bevy::prelude::*;
use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use lod_avian::PhysicsInteractionLayer;
use richmond_building_components::floors::FloorGeometry;
use richmond_building_components::panels::{
	rectangle_kit_hull, right_triangle_kit_hull, tessellated_triangle_kit_hull,
	to_centered_rect_placement, PanelGeometry, PANEL_KIT_MAX, PANEL_KIT_MIN,
};
use richmond_building_components::partitions::{PartitionGeometry, PANEL_Y_HALF};
use richmond_building_components::placed::Placement;
use richmond_building_components::{BuildingComponents, FloorNode, PanelNode, PartitionNode};

use crate::BuildingFrictionConfig;

/// Marks a Fixed collider spawned from building IR (not a LOD Host volume).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct BuildingWalkCollider;

/// Panel GLBs: \(X \in [0, 1]\), \(Y \in [-0.2, 0.2]\), \(Z \in [-1, 0]\) (eave at \(Z = 0\)).
const KIT_MIN: Vec3 = PANEL_KIT_MIN;
const KIT_MAX: Vec3 = PANEL_KIT_MAX;

/// Stamp walk colliders as children of `parent` from High-LOD domain nodes.
pub fn spawn_building_walk_colliders(
	commands: &mut Commands,
	parent: Entity,
	building: &impl BuildingComponents,
	friction: Friction,
) {
	let level = LodSceneLevel::High;
	for node in building.panel_nodes_for_level(level).flatten() {
		if let Some(hull) = panel_collider(&node) {
			spawn_hull(commands, parent, hull.translation, hull.rotation, hull.points, friction);
		}
	}
	for node in building.floor_nodes_for_level(level).flatten() {
		if let Some(pose) = floor_cuboid(&node) {
			spawn_cuboid(commands, parent, pose, friction);
		}
		for (translation, rotation, points) in node.triangle_walk_hulls() {
			spawn_hull(commands, parent, translation, rotation, points, friction);
		}
	}
	for node in building.partition_nodes_for_level(level).flatten() {
		if let Some(pose) = partition_cuboid(&node) {
			spawn_cuboid(commands, parent, pose, friction);
		}
	}
	for node in building.stair_nodes_for_level(level).flatten() {
		for (translation, rotation, size) in node.walk_ramps() {
			spawn_cuboid(commands, parent, CuboidPose { translation, rotation, size }, friction);
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

struct WalkHull {
	translation: Vec3,
	rotation: Quat,
	points: Vec<Vec3>,
}

fn spawn_cuboid(commands: &mut Commands, parent: Entity, pose: CuboidPose, friction: Friction) {
	let size = pose.size.max(Vec3::splat(0.05));
	spawn_fixed(
		commands,
		parent,
		pose.translation,
		pose.rotation,
		Collider::cuboid(size.x, size.y, size.z),
		friction,
	);
}

fn spawn_hull(
	commands: &mut Commands,
	parent: Entity,
	translation: Vec3,
	rotation: Quat,
	points: Vec<Vec3>,
	friction: Friction,
) {
	if let Some(collider) = Collider::convex_hull(points.clone()) {
		spawn_fixed(commands, parent, translation, rotation, collider, friction);
		return;
	}
	// Thin / large kits can fail convex hull; keep an AABB so floors still collide.
	let mut min = Vec3::splat(f32::MAX);
	let mut max = Vec3::splat(f32::MIN);
	for p in &points {
		min = min.min(*p);
		max = max.max(*p);
	}
	if !min.is_finite() || !max.is_finite() {
		return;
	}
	let size = (max - min).max(Vec3::splat(0.05));
	let center = (min + max) * 0.5;
	spawn_cuboid(
		commands,
		parent,
		CuboidPose { translation: translation + rotation * center, rotation, size },
		friction,
	);
}

fn spawn_fixed(
	commands: &mut Commands,
	parent: Entity,
	translation: Vec3,
	rotation: Quat,
	collider: Collider,
	friction: Friction,
) {
	commands.spawn((
		Name::new("building-walk-collider"),
		BuildingWalkCollider,
		ChildOf(parent),
		Transform::from_translation(translation).with_rotation(rotation),
		Visibility::Hidden,
		RigidBody::Static,
		collider,
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

fn panel_collider(node: &PanelNode) -> Option<WalkHull> {
	// Hull points are local to the kit origin so the collider shares the mesh TRS
	// (`pose(placement)` scales the same corners). QuadPanel landings are
	// tessellated triangles, not rectangle leaves.
	match &node.geometry {
		PanelGeometry::Rectangle(_) => Some(WalkHull {
			translation: node.placement.translation,
			rotation: node.placement.rotation(),
			points: rectangle_kit_hull(node.placement.scale),
		}),
		PanelGeometry::RightTriangle(tri) => Some(WalkHull {
			translation: node.placement.translation,
			rotation: node.placement.rotation(),
			points: right_triangle_kit_hull(node.placement.scale, tri.mirror),
		}),
		PanelGeometry::TessellatedTriangle(tri) => Some(WalkHull {
			translation: node.placement.translation,
			rotation: node.placement.rotation(),
			points: tessellated_triangle_kit_hull(tri.a, tri.b, tri.c, node.placement.scale),
		}),
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
	fn kit_cuboid_centers_on_neg_z_panel_space() -> anyhow::Result<()> {
		let pose = oriented_kit_cuboid(Placement::IDENTITY, KIT_MIN, KIT_MAX);
		assert!((pose.translation - Vec3::new(0.5, 0.0, -0.5)).length() < 1e-4);
		assert!((pose.size.x - 1.0).abs() < 1e-4);
		assert!((pose.size.z - 1.0).abs() < 1e-4);
		assert!((pose.size.y - PANEL_Y_HALF * 2.0).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn right_triangle_hull_keeps_the_origin_and_far_leg() -> anyhow::Result<()> {
		let pts = right_triangle_kit_hull(Vec3::ONE, None);
		assert_eq!(pts.len(), 6);
		assert!(pts.iter().any(|p| (p - Vec3::new(0.0, PANEL_Y_HALF, 0.0)).length() < 1e-4));
		assert!(pts.iter().any(|p| (p - Vec3::new(1.0, -PANEL_Y_HALF, 0.0)).length() < 1e-4));
		assert!(pts.iter().any(|p| (p - Vec3::new(0.0, -PANEL_Y_HALF, -1.0)).length() < 1e-4));
		assert!(Collider::convex_hull(pts).is_some());
		Ok(())
	}

	#[test]
	fn rectangle_panel_hull_matches_kit_and_is_convex() -> anyhow::Result<()> {
		let node = PanelNode::rough_stone(
			PanelGeometry::rectangle(),
			Placement::IDENTITY.with_scale(Vec3::new(4.0, 0.4, 2.0)),
		);
		let Some(hull) = panel_collider(&node) else {
			anyhow::bail!("rectangle panel should stamp a hull");
		};
		assert_eq!(hull.points.len(), 8);
		assert!(hull.translation.length() < 1e-6);
		assert!(Collider::convex_hull(hull.points).is_some());
		Ok(())
	}

	#[test]
	fn tessellated_panel_stamps_a_prism_hull() -> anyhow::Result<()> {
		use bevy_math::Vec2;
		use richmond_building_components::panels::TessellatedTriangle;
		let tri = TessellatedTriangle::new(Vec2::ZERO, Vec2::new(2.0, 0.0), Vec2::new(0.5, -1.5));
		let node =
			PanelNode::rough_stone(PanelGeometry::tessellated_triangle(tri), Placement::IDENTITY);
		let Some(hull) = panel_collider(&node) else {
			anyhow::bail!("tessellated panel should stamp a prism");
		};
		assert_eq!(hull.points.len(), 6);
		assert!(hull.points.iter().any(|p| (Vec2::new(p.x, p.z) - tri.c).length() < 1e-4));
		assert!(Collider::convex_hull(hull.points).is_some());
		Ok(())
	}
}
