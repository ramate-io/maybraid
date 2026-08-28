//! High-band stick / trunk capsules for character physics.
//!
//! Radius is at least four inches. Medium / Low / UltraLow hosts drop the
//! colliders so only the near High ring costs contacts.

use avian3d::prelude::{Collider, RigidBody};
use bevy::prelude::*;
use chico_vegetation_components::{Placement, StickNode, STICK_KIT_HALF};
use lod::LodSceneHost;
use lod::LodSceneLevel;
use lod_avian::PhysicsInteractionLayer;

/// Four inches. Collider girth is `max(authored radius, this)`.
pub const MIN_STICK_COLLIDER_RADIUS_M: f32 = 4.0 * 0.0254;

#[derive(Component)]
struct StickPhysicsCapsule;

pub struct StickPhysicsPlugin;

impl Plugin for StickPhysicsPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, sync_high_stick_colliders);
	}
}

fn sync_high_stick_colliders(
	mut commands: Commands,
	hosts: Query<(Entity, &StickNode, &LodSceneLevel), With<LodSceneHost>>,
	children: Query<&Children>,
	capsules: Query<Entity, With<StickPhysicsCapsule>>,
) {
	for (entity, node, level) in &hosts {
		if *level != LodSceneLevel::High {
			despawn_capsules(&mut commands, entity, &children, &capsules);
			continue;
		}
		if has_capsules(entity, &children, &capsules) {
			continue;
		}
		for (transform, radius, cylinder) in collider_poses(node) {
			let child = commands
				.spawn((
					StickPhysicsCapsule,
					transform,
					Visibility::Hidden,
					RigidBody::Static,
					Collider::capsule(radius, cylinder),
					PhysicsInteractionLayer::fixed_layers(),
				))
				.id();
			commands.entity(entity).add_child(child);
		}
	}
}

fn has_capsules(
	entity: Entity,
	children: &Query<&Children>,
	capsules: &Query<Entity, With<StickPhysicsCapsule>>,
) -> bool {
	children
		.get(entity)
		.ok()
		.is_some_and(|kids| kids.iter().any(|c| capsules.contains(c)))
}

fn despawn_capsules(
	commands: &mut Commands,
	entity: Entity,
	children: &Query<&Children>,
	capsules: &Query<Entity, With<StickPhysicsCapsule>>,
) {
	let Ok(kids) = children.get(entity) else {
		return;
	};
	for child in kids.iter() {
		if capsules.contains(child) {
			commands.entity(child).despawn();
		}
	}
}

fn collider_poses(node: &StickNode) -> Vec<(Transform, f32, f32)> {
	if let Some(collection) = &node.collection {
		return collection
			.members_for_level(LodSceneLevel::High)
			.iter()
			.filter_map(|member| {
				let placed = node.placement.compose_child(member.placement);
				capsule_from_placement(placed)
			})
			.collect();
	}
	capsule_from_placement(node.placement).into_iter().collect()
}

fn capsule_from_placement(placement: Placement) -> Option<(Transform, f32, f32)> {
	let length = placement.scale.y;
	if length < 0.05 {
		return None;
	}
	let authored =
		(placement.scale.x.abs() * STICK_KIT_HALF).max(placement.scale.z.abs() * STICK_KIT_HALF);
	let radius = authored.max(MIN_STICK_COLLIDER_RADIUS_M);
	let cylinder = (length - 2.0 * radius).max(0.02);
	let center = placement.translation + placement.rotation() * Vec3::new(0.0, length * 0.5, 0.0);
	Some((
		Transform::from_translation(center).with_rotation(placement.rotation()),
		radius,
		cylinder,
	))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn collider_radius_is_at_least_four_inches() {
		let thin = Placement::IDENTITY.with_scale(Vec3::new(0.1, 2.0, 0.1));
		let (_, radius, _) = capsule_from_placement(thin).expect("capsule");
		assert!(radius + 1e-5 >= MIN_STICK_COLLIDER_RADIUS_M);
	}
}
