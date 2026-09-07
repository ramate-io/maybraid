//! Cuboids from panel / floor / partition placement, triangle prisms, and stair ramps.
//!
//! Spawn queues [`BuildingWalkShapes`] on the host. [`attach_building_walk_colliders`]
//! then stamps **one** Fixed compound child after [`lod::LodSceneHost`] exists, so
//! `spawn_scene` / LOD cull are not racing hundreds of RigidBody children.

use avian3d::prelude::{Collider, Friction, RigidBody};
use bevy::prelude::*;
use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use lod::LodSceneHost;
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

/// Marks the single Fixed compound spawned from building IR (not a LOD Host volume).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct BuildingWalkCollider;

/// Queued High-LOD walk shapes; consumed when the host's LOD scene is live.
#[derive(Component, Clone, Debug)]
pub struct BuildingWalkShapes {
	pub shapes: Vec<(Vec3, Quat, Collider)>,
	pub friction: Friction,
}

#[derive(Component, Debug, Clone, Copy, Default)]
pub(crate) struct BuildingWalkColliderAttached;

/// Panel GLBs: \(X \in [0, 1]\), \(Y \in [-0.2, 0.2]\), \(Z \in [-1, 0]\) (eave at \(Z = 0\)).
const KIT_MIN: Vec3 = PANEL_KIT_MIN;
const KIT_MAX: Vec3 = PANEL_KIT_MAX;

/// Queue walk colliders on `parent` from High-LOD domain nodes.
pub fn spawn_building_walk_colliders(
	commands: &mut Commands,
	parent: Entity,
	building: &impl BuildingComponents,
	friction: Friction,
) {
	let shapes = walk_shapes(building);
	if shapes.is_empty() {
		return;
	}
	commands.entity(parent).insert(BuildingWalkShapes { shapes, friction });
}

/// Stamp one compound child once [`LodSceneHost`] is on the parent.
pub(crate) fn attach_building_walk_colliders(
	mut commands: Commands,
	pending: Query<
		(Entity, &BuildingWalkShapes),
		(With<LodSceneHost>, Without<BuildingWalkColliderAttached>),
	>,
) {
	for (entity, spec) in &pending {
		let Ok(mut host) = commands.get_entity(entity) else {
			continue;
		};
		let shapes = spec.shapes.clone();
		let friction = spec.friction;
		host.insert(BuildingWalkColliderAttached);
		host.remove::<BuildingWalkShapes>();
		if shapes.is_empty() {
			continue;
		}
		commands.spawn((
			Name::new("building-walk-collider"),
			BuildingWalkCollider,
			ChildOf(entity),
			Transform::IDENTITY,
			Visibility::Hidden,
			RigidBody::Static,
			Collider::compound(shapes),
			PhysicsInteractionLayer::fixed_layers(),
			friction,
		));
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

fn walk_shapes(building: &impl BuildingComponents) -> Vec<(Vec3, Quat, Collider)> {
	let level = LodSceneLevel::High;
	let mut shapes = Vec::new();
	for node in building.panel_nodes_for_level(level).flatten() {
		if let Some(hull) = panel_collider(&node) {
			if let Some(shape) = hull_shape(hull) {
				shapes.push(shape);
			}
		}
	}
	for node in building.floor_nodes_for_level(level).flatten() {
		if let Some(pose) = floor_cuboid(&node) {
			shapes.push(cuboid_shape(pose));
		}
		for (translation, rotation, points) in node.triangle_walk_hulls() {
			if let Some(shape) = hull_shape(WalkHull { translation, rotation, points }) {
				shapes.push(shape);
			}
		}
	}
	for node in building.partition_nodes_for_level(level).flatten() {
		if let Some(pose) = partition_cuboid(&node) {
			shapes.push(cuboid_shape(pose));
		}
	}
	for node in building.stair_nodes_for_level(level).flatten() {
		for (translation, rotation, size) in node.walk_ramps() {
			shapes.push(cuboid_shape(CuboidPose { translation, rotation, size }));
		}
	}
	shapes
}

fn cuboid_shape(pose: CuboidPose) -> (Vec3, Quat, Collider) {
	let size = pose.size.max(Vec3::splat(0.05));
	(pose.translation, pose.rotation, Collider::cuboid(size.x, size.y, size.z))
}

fn hull_shape(hull: WalkHull) -> Option<(Vec3, Quat, Collider)> {
	if let Some(collider) = Collider::convex_hull(hull.points.clone()) {
		return Some((hull.translation, hull.rotation, collider));
	}
	// Thin / large kits can fail convex hull; keep an AABB so floors still collide.
	let mut min = Vec3::splat(f32::MAX);
	let mut max = Vec3::splat(f32::MIN);
	for p in &hull.points {
		min = min.min(*p);
		max = max.max(*p);
	}
	if !min.is_finite() || !max.is_finite() {
		return None;
	}
	let size = (max - min).max(Vec3::splat(0.05));
	let center = (min + max) * 0.5;
	Some(cuboid_shape(CuboidPose {
		translation: hull.translation + hull.rotation * center,
		rotation: hull.rotation,
		size,
	}))
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

	#[test]
	fn attach_spawns_one_compound_after_lod_host_exists() {
		use crate::BUILDING_FRICTION;
		let mut app = App::new();
		app.add_systems(Update, attach_building_walk_colliders);
		let host = app
			.world_mut()
			.spawn((
				LodSceneHost,
				BuildingWalkShapes {
					shapes: vec![(Vec3::ZERO, Quat::IDENTITY, Collider::cuboid(1.0, 1.0, 1.0))],
					friction: BUILDING_FRICTION,
				},
			))
			.id();
		app.update();
		let world = app.world_mut();
		assert_eq!(world.query::<&BuildingWalkCollider>().iter(world).count(), 1);
		assert!(world.get::<BuildingWalkShapes>(host).is_none());
	}
}
