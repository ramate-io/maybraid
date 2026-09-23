//! Static walk colliders from stamped furniture slots.

use avian3d::prelude::{Collider, RigidBody};
use bevy::prelude::*;
use lod::LodSceneHost;
use lod_avian::PhysicsInteractionLayer;
use richmond_building_components::{FurnitureGeometry, FurnitureNode};
use richmond_building_physics::BUILDING_FRICTION;

use crate::host::FurnitureCell;

/// Marks the Fixed compound spawned from a [`FurnitureCell`].
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct FurnitureWalkCollider;

#[derive(Component, Debug, Clone, Copy, Default)]
struct FurnitureWalkColliderAttached;

/// Registers furniture walk-collider attach.
pub struct FurnitureWalkColliderPlugin;

impl Plugin for FurnitureWalkColliderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, attach_furniture_walk_colliders);
	}
}

fn attach_furniture_walk_colliders(
	mut commands: Commands,
	pending: Query<
		(Entity, &FurnitureCell),
		(With<LodSceneHost>, Without<FurnitureWalkColliderAttached>),
	>,
) {
	for (entity, cell) in &pending {
		let Ok(mut host) = commands.get_entity(entity) else {
			continue;
		};
		host.insert(FurnitureWalkColliderAttached);
		let shapes = walk_shapes(&cell.slots);
		if shapes.is_empty() {
			continue;
		}
		commands.spawn((
			Name::new("furniture-walk-collider"),
			FurnitureWalkCollider,
			ChildOf(entity),
			Transform::IDENTITY,
			Visibility::Hidden,
			RigidBody::Static,
			Collider::compound(shapes),
			PhysicsInteractionLayer::fixed_layers(),
			BUILDING_FRICTION,
		));
	}
}

fn walk_shapes(slots: &[FurnitureNode]) -> Vec<(Vec3, Quat, Collider)> {
	slots.iter().filter(|slot| blocks_walk(slot.geometry)).map(slot_shape).collect()
}

fn blocks_walk(geometry: FurnitureGeometry) -> bool {
	!matches!(
		geometry,
		FurnitureGeometry::Fruit
			| FurnitureGeometry::Bread
			| FurnitureGeometry::Cookware
			| FurnitureGeometry::Faucet
			| FurnitureGeometry::FoodDisplay
			| FurnitureGeometry::Basin
			| FurnitureGeometry::Range
	)
}

fn slot_shape(node: &FurnitureNode) -> (Vec3, Quat, Collider) {
	let size = node.placement.scale.max(Vec3::splat(0.08));
	(
		node.placement.translation,
		node.placement.rotation(),
		Collider::cuboid(size.x, size.y, size.z),
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use richmond_building_components::Placement;

	#[test]
	fn partitions_and_counters_block_sit_ons_do_not() {
		let partition =
			FurnitureNode::partition(Placement::IDENTITY.with_scale(Vec3::new(2.6, 3.2, 0.22)));
		let fruit = FurnitureNode::fruit(Placement::IDENTITY);
		let shapes = walk_shapes(&[partition, fruit]);
		assert_eq!(shapes.len(), 1);
	}
}
