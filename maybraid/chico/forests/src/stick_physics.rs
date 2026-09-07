//! Playable stick / trunk capsules for character physics.
//!
//! Forest plants are [`FlattenedComponentsOnly`] hosts — kit GLBs spawn as posed
//! content with no nested [`StickNode`] LOD hosts. A type-erased producer is
//! stamped when each source component is added, then one shared change-driven
//! drain creates a bounded compound collider per host.
//!
//! Only structural **High** plants get colliders (the walk-into ring). Medium /
//! Low / UltraLow drop them so the far present ring does not pay contacts.
//! Leftover nested [`StickNode`] hosts still get the same High-only treatment.

use std::collections::{HashSet, VecDeque};

use avian3d::prelude::{Collider, RigidBody};
use bevy::prelude::*;
use chico_vegetation_components::{
	Placement, StickMember, StickNode, VegetationComponents, STICK_KIT_HALF,
};
use lod::LodSceneHost;
use lod::LodSceneLevel;
use lod_avian::PhysicsInteractionLayer;

/// Two inches. Gate and floor use world-space girth after plant [`Placement`] scale.
pub const MIN_STICK_COLLIDER_RADIUS_M: f32 = 2.0 * 0.0254;
/// Hard fan-out bound within one plant compound.
pub const MAX_STICK_COLLIDER_SHAPES: usize = 64;

/// How many changed High-band hosts may build compounds in one frame.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StickPhysicsBudget {
	pub hosts_per_frame: u32,
}

impl Default for StickPhysicsBudget {
	fn default() -> Self {
		Self { hosts_per_frame: 8 }
	}
}

type ProduceColliderPoses = fn(&World, Entity, LodSceneLevel) -> Vec<(Transform, f32, f32)>;

/// Type-erased source callback; all vegetation types share one runtime drain.
#[derive(Component, Clone, Copy)]
struct StickPhysicsProducer(ProduceColliderPoses);

#[derive(Component)]
pub(crate) struct StickPhysicsCompound;

/// Last band we spawned a compound for.
#[derive(Component, Clone, Copy)]
pub(crate) struct StickPhysicsAttached {
	level: LodSceneLevel,
}

#[derive(Resource, Default)]
struct StickPhysicsQueue {
	pending: VecDeque<Entity>,
	pending_entities: HashSet<Entity>,
}

impl StickPhysicsQueue {
	fn enqueue(&mut self, entity: Entity) {
		if self.pending_entities.insert(entity) {
			self.pending.push_back(entity);
		}
	}

	fn pop_front(&mut self) -> Option<Entity> {
		let entity = self.pending.pop_front()?;
		self.pending_entities.remove(&entity);
		Some(entity)
	}
}

pub struct StickPhysicsPlugin;

impl Plugin for StickPhysicsPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<StickPhysicsBudget>()
			.init_resource::<StickPhysicsQueue>()
			.add_observer(attach_stick_node_producer)
			.add_systems(Update, sync_stick_colliders);
	}
}

/// Register a flattened vegetation source without adding another update system.
pub(crate) fn register_vegetation_stick_colliders<T>(app: &mut App)
where
	T: VegetationComponents + Component,
{
	app.add_observer(attach_vegetation_producer::<T>);
}

fn attach_vegetation_producer<T>(insert: On<Insert, T>, mut commands: Commands)
where
	T: VegetationComponents + Component,
{
	if let Ok(mut entity) = commands.get_entity(insert.entity) {
		entity.insert(StickPhysicsProducer(|world, entity, level| {
			world
				.get::<T>(entity)
				.into_iter()
				.flat_map(|vegetation| vegetation.stick_nodes_for_level(level).flatten())
				.flat_map(|node| collider_poses(&node, level))
				.take(MAX_STICK_COLLIDER_SHAPES)
				.collect()
		}));
	}
}

fn attach_stick_node_producer(insert: On<Insert, StickNode>, mut commands: Commands) {
	if let Ok(mut entity) = commands.get_entity(insert.entity) {
		entity.insert(StickPhysicsProducer(|world, entity, level| {
			world
				.get::<StickNode>(entity)
				.map(|node| collider_poses(node, level))
				.unwrap_or_default()
		}));
	}
}

fn wants_playable_colliders(level: LodSceneLevel) -> bool {
	matches!(level, LodSceneLevel::High)
}

fn sync_stick_colliders(world: &mut World) {
	let changed: Vec<_> = {
		let mut hosts = world
			.query_filtered::<(Entity, &LodSceneLevel, Option<&StickPhysicsAttached>), (
				With<LodSceneHost>,
				With<StickPhysicsProducer>,
				Or<(Added<StickPhysicsProducer>, Changed<LodSceneLevel>)>,
			)>();
		hosts
			.iter(world)
			.map(|(entity, level, attached)| (entity, *level, attached.copied()))
			.collect()
	};

	for (entity, level, attached) in changed {
		if wants_playable_colliders(level) {
			world.resource_mut::<StickPhysicsQueue>().enqueue(entity);
		} else if attached.is_some() {
			despawn_compound(world, entity);
			world.entity_mut(entity).remove::<StickPhysicsAttached>();
		}
	}

	let limit = world.resource::<StickPhysicsBudget>().hosts_per_frame;
	for _ in 0..limit {
		let Some(entity) = world.resource_mut::<StickPhysicsQueue>().pop_front() else {
			break;
		};
		let Some(level) = world.get::<LodSceneLevel>(entity).copied() else {
			continue;
		};
		if !wants_playable_colliders(level) {
			continue;
		}
		let already_current = world
			.get::<StickPhysicsAttached>(entity)
			.is_some_and(|attached| attached.level == level)
			&& has_compound(world, entity);
		if already_current {
			continue;
		}
		let Some(producer) = world.get::<StickPhysicsProducer>(entity).copied() else {
			continue;
		};
		let poses = (producer.0)(world, entity, level);
		despawn_compound(world, entity);
		if poses.is_empty() {
			continue;
		}
		let shapes = poses
			.into_iter()
			.map(|(transform, radius, cylinder)| {
				(transform.translation, transform.rotation, Collider::capsule(radius, cylinder))
			})
			.collect();
		world.spawn((
			StickPhysicsCompound,
			ChildOf(entity),
			Transform::IDENTITY,
			Visibility::Hidden,
			RigidBody::Static,
			Collider::compound(shapes),
			PhysicsInteractionLayer::fixed_layers(),
		));
		world.entity_mut(entity).insert(StickPhysicsAttached { level });
	}
}

fn has_compound(world: &World, entity: Entity) -> bool {
	world.get::<Children>(entity).is_some_and(|children| {
		children.iter().any(|child| world.get::<StickPhysicsCompound>(child).is_some())
	})
}

fn despawn_compound(world: &mut World, entity: Entity) {
	let Some(children) = world.get::<Children>(entity) else {
		return;
	};
	let compounds: Vec<_> = children
		.iter()
		.filter(|child| world.get::<StickPhysicsCompound>(*child).is_some())
		.collect();
	for child in compounds {
		world.despawn(child);
	}
}

fn authored_radius(placement: Placement) -> f32 {
	(placement.scale.x.abs() * STICK_KIT_HALF).max(placement.scale.z.abs() * STICK_KIT_HALF)
}

/// Trunks always; branches at least two inches in world space. Thinner High twigs stay visual-only.
fn should_collide_member(is_trunk: bool, placement: Placement) -> bool {
	is_trunk || authored_radius(placement) + 1e-5 >= MIN_STICK_COLLIDER_RADIUS_M
}

fn collider_poses(node: &StickNode, level: LodSceneLevel) -> Vec<(Transform, f32, f32)> {
	if let Some(collection) = &node.collection {
		let members = collection.members_for_level(level);
		let mut ranked: Vec<_> = members
			.iter()
			.filter_map(|member| ranked_member_pose(node.placement, member))
			.collect();
		if ranked.is_empty() {
			ranked = members
				.iter()
				.max_by(|a, b| {
					world_radius(node.placement, a)
						.partial_cmp(&world_radius(node.placement, b))
						.unwrap_or(std::cmp::Ordering::Equal)
				})
				.and_then(|member| {
					let placed = world_member_placement(node.placement, member);
					capsule_from_placement(placed)
						.map(|pose| (member.is_trunk(), authored_radius(placed), pose))
				})
				.into_iter()
				.collect();
		}
		ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.total_cmp(&a.1)));
		ranked.truncate(MAX_STICK_COLLIDER_SHAPES);
		return ranked.into_iter().map(|(_, _, pose)| pose).collect();
	}
	capsule_from_placement(node.placement).into_iter().collect()
}

fn world_member_placement(parent: Placement, member: &StickMember) -> Placement {
	parent.compose_child(member.placement)
}

fn world_radius(parent: Placement, member: &StickMember) -> f32 {
	authored_radius(world_member_placement(parent, member))
}

fn ranked_member_pose(
	parent: Placement,
	member: &StickMember,
) -> Option<(bool, f32, (Transform, f32, f32))> {
	let placed = world_member_placement(parent, member);
	if !should_collide_member(member.is_trunk(), placed) {
		return None;
	}
	let radius = authored_radius(placed);
	capsule_from_placement(placed).map(|pose| (member.is_trunk(), radius, pose))
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
	fn collection_keeps_scaled_limbs_that_are_thin_in_unit_space() {
		let trunk = StickMember {
			geometry: StickGeometry::Trunk,
			placement: Placement::IDENTITY.with_scale(Vec3::new(0.4, 4.0, 0.4)),
		};
		let limb = StickMember {
			geometry: StickGeometry::Segment,
			placement: Placement::new(Vec3::new(1.0, 2.0, 0.0), 0.0)
				.with_scale(Vec3::new(0.04, 1.0, 0.04)),
		};
		let node = StickNode::collection(
			StickCollection::new([trunk, limb]).bake_bounds_from_members(),
			Placement::IDENTITY.with_scale(Vec3::splat(30.0)),
		);
		let poses = collider_poses(&node, LodSceneLevel::High);
		assert_eq!(poses.len(), 2);
		let expected = 0.04 * STICK_KIT_HALF * 30.0;
		let limb_radius = poses.iter().map(|(_, radius, _)| *radius).fold(f32::INFINITY, f32::min);
		assert!((limb_radius - expected).abs() < 1e-4);
	}

	#[test]
	fn only_high_band_wants_playable_colliders() {
		assert!(wants_playable_colliders(LodSceneLevel::High));
		assert!(!wants_playable_colliders(LodSceneLevel::Medium));
		assert!(!wants_playable_colliders(LodSceneLevel::Low));
		assert!(!wants_playable_colliders(LodSceneLevel::UltraLow));
	}

	#[test]
	fn collection_compound_has_bounded_shape_count() {
		let members = (0..(MAX_STICK_COLLIDER_SHAPES + 20))
			.map(|index| StickMember {
				geometry: StickGeometry::Segment,
				placement: Placement::new(Vec3::new(index as f32, 0.0, 0.0), 0.0)
					.with_scale(Vec3::new(0.3, 2.0, 0.3)),
			})
			.collect::<Vec<_>>();
		let node = StickNode::collection(
			StickCollection::new(members).bake_bounds_from_members(),
			Placement::IDENTITY,
		);
		assert_eq!(collider_poses(&node, LodSceneLevel::High).len(), MAX_STICK_COLLIDER_SHAPES);
	}
}
