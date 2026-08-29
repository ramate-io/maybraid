//! Playable stick / trunk capsules for character physics.
//!
//! Forest plants are [`FlattenedComponentsOnly`] hosts — kit GLBs spawn as posed
//! content with no nested [`StickNode`] LOD hosts. Capsules therefore attach to
//! the plant host from [`VegetationComponents::stick_nodes_for_level`], not from
//! a `StickNode` + `LodSceneHost` query.
//!
//! Only structural **High** plants get colliders (the walk-into ring). Medium /
//! Low / UltraLow drop them so the far present ring does not pay contacts.
//! Leftover nested [`StickNode`] hosts still get the same High-only treatment.

use avian3d::prelude::{Collider, RigidBody};
use bevy::prelude::*;
use chico_vegetation_components::{Placement, StickNode, VegetationComponents, STICK_KIT_HALF};
use lod::LodSceneHost;
use lod::LodSceneLevel;
use lod_avian::PhysicsInteractionLayer;

/// One inch. Collider girth is `max(authored radius, this)` for sticks we emit.
pub const MIN_STICK_COLLIDER_RADIUS_M: f32 = 1.0 * 0.01;

#[derive(Component)]
pub(crate) struct StickPhysicsCapsule;

/// Last band we spawned capsules for. Avoids rebuilding from IR every frame.
#[derive(Component)]
pub(crate) struct StickPhysicsAttached {
	level: LodSceneLevel,
}

pub struct StickPhysicsPlugin;

impl Plugin for StickPhysicsPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, sync_stick_node_colliders);
	}
}

/// Attach High-band capsules from a [`VegetationComponents`] host (flattened plants).
pub(crate) fn sync_vegetation_stick_colliders<T: VegetationComponents + Component>(
	mut commands: Commands,
	hosts: Query<(Entity, &T, &LodSceneLevel, Option<&StickPhysicsAttached>), With<LodSceneHost>>,
	children: Query<&Children>,
	capsules: Query<Entity, With<StickPhysicsCapsule>>,
) {
	for (entity, vegetation, level, attached) in &hosts {
		sync_host_colliders(
			&mut commands,
			entity,
			*level,
			attached,
			|| {
				vegetation
					.stick_nodes_for_level(*level)
					.flatten()
					.into_iter()
					.flat_map(|node| collider_poses(&node, *level))
					.collect()
			},
			&children,
			&capsules,
		);
	}
}

fn sync_stick_node_colliders(
	mut commands: Commands,
	hosts: Query<
		(Entity, &StickNode, &LodSceneLevel, Option<&StickPhysicsAttached>),
		With<LodSceneHost>,
	>,
	children: Query<&Children>,
	capsules: Query<Entity, With<StickPhysicsCapsule>>,
) {
	for (entity, node, level, attached) in &hosts {
		sync_host_colliders(
			&mut commands,
			entity,
			*level,
			attached,
			|| collider_poses(node, *level),
			&children,
			&capsules,
		);
	}
}

fn wants_playable_colliders(level: LodSceneLevel) -> bool {
	matches!(level, LodSceneLevel::High)
}

fn sync_host_colliders(
	commands: &mut Commands,
	entity: Entity,
	level: LodSceneLevel,
	attached: Option<&StickPhysicsAttached>,
	poses: impl FnOnce() -> Vec<(Transform, f32, f32)>,
	children: &Query<&Children>,
	capsules: &Query<Entity, With<StickPhysicsCapsule>>,
) {
	if !wants_playable_colliders(level) {
		despawn_capsules(commands, entity, children, capsules);
		if attached.is_some() {
			commands.entity(entity).remove::<StickPhysicsAttached>();
		}
		return;
	}
	if attached.is_some_and(|a| a.level == level) && has_capsules(entity, children, capsules) {
		return;
	}
	despawn_capsules(commands, entity, children, capsules);
	for (transform, radius, cylinder) in poses() {
		commands.spawn((
			StickPhysicsCapsule,
			ChildOf(entity),
			transform,
			Visibility::Hidden,
			RigidBody::Static,
			Collider::capsule(radius, cylinder),
			PhysicsInteractionLayer::fixed_layers(),
		));
	}
	commands.entity(entity).insert(StickPhysicsAttached { level });
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

fn authored_radius(placement: Placement) -> f32 {
	(placement.scale.x.abs() * STICK_KIT_HALF).max(placement.scale.z.abs() * STICK_KIT_HALF)
}

/// Trunks always; branches at least one inch too. Thinner High twigs stay visual-only.
fn should_collide_member(is_trunk: bool, placement: Placement) -> bool {
	is_trunk || authored_radius(placement) + 1e-5 >= MIN_STICK_COLLIDER_RADIUS_M
}

fn collider_poses(node: &StickNode, level: LodSceneLevel) -> Vec<(Transform, f32, f32)> {
	if let Some(collection) = &node.collection {
		let members = collection.members_for_level(level);
		let mut poses: Vec<_> = members
			.iter()
			.filter(|member| should_collide_member(member.is_trunk(), member.placement))
			.filter_map(|member| {
				let placed = node.placement.compose_child(member.placement);
				capsule_from_placement(placed)
			})
			.collect();
		if poses.is_empty() {
			poses = members
				.iter()
				.max_by(|a, b| {
					authored_radius(a.placement)
						.partial_cmp(&authored_radius(b.placement))
						.unwrap_or(std::cmp::Ordering::Equal)
				})
				.and_then(|member| {
					capsule_from_placement(node.placement.compose_child(member.placement))
				})
				.into_iter()
				.collect();
		}
		return poses;
	}
	capsule_from_placement(node.placement).into_iter().collect()
}

fn capsule_from_placement(placement: Placement) -> Option<(Transform, f32, f32)> {
	let length = placement.scale.y;
	if length < 0.05 {
		return None;
	}
	let radius = authored_radius(placement).max(MIN_STICK_COLLIDER_RADIUS_M);
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
	use chico_vegetation_components::{StickCollection, StickGeometry, StickMember};

	#[test]
	fn collection_keeps_trunks_and_drops_thin_twigs() {
		let trunk = StickMember {
			geometry: StickGeometry::Trunk,
			placement: Placement::IDENTITY.with_scale(Vec3::new(0.4, 4.0, 0.4)),
		};
		let twig = StickMember {
			geometry: StickGeometry::Segment,
			placement: Placement::new(Vec3::new(1.0, 2.0, 0.0), 0.0)
				.with_scale(Vec3::new(0.04, 1.0, 0.04)),
		};
		let node = StickNode::collection(
			StickCollection::new([trunk, twig]).bake_bounds_from_members(),
			Placement::IDENTITY,
		);
		let poses = collider_poses(&node, LodSceneLevel::High);
		assert_eq!(poses.len(), 1);
	}

	#[test]
	fn only_high_band_wants_playable_colliders() {
		assert!(wants_playable_colliders(LodSceneLevel::High));
		assert!(!wants_playable_colliders(LodSceneLevel::Medium));
		assert!(!wants_playable_colliders(LodSceneLevel::Low));
		assert!(!wants_playable_colliders(LodSceneLevel::UltraLow));
	}
}
