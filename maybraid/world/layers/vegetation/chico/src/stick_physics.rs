//! Playable stick / trunk capsules for character physics.
//!
//! Forest plants are [`FlattenedComponentsOnly`] hosts — kit GLBs spawn as posed
//! content with no nested [`StickNode`] LOD hosts. A type-erased producer is
//! stamped when each source component is added, then one shared drain creates a
//! compound collider per host (every gated stick, no shape cap).
//!
//! Compounds live on the stick source entity (isolated `/show` host, or a posed
//! plant under a grove). Band flicker does not rebuild them. The first **High
//! or Medium** realization — on this entity or an ancestor [`LodSceneHost`] —
//! stamps High-IR capsules. Grove High/Medium pose plants without nested hosts;
//! the grove band is what makes the compound playable. Sources with no
//! collideable sticks still take [`StickPhysicsAttached`] so empty tuft / frond
//! plants do not starve the drain. Later Low / UltraLow leaves the compound in
//! place until the source is culled.

use std::collections::{HashSet, VecDeque};

use avian3d::prelude::{Collider, RigidBody};
use bevy::prelude::*;
use lod::LodSceneLevel;
use lod_avian::PhysicsInteractionLayer;
use vegetation_components::{
	Placement, StickMember, StickNode, VegetationComponents, STICK_KIT_HALF,
};

/// Four inches. Gate and floor use world-space girth after plant [`Placement`] scale.
pub const MIN_STICK_COLLIDER_RADIUS_M: f32 = 4.0 * 0.0254;
/// 10cm. Gates the min length of the stick capsule.
pub const MIN_STICK_COLLIDER_LENGTH_M: f32 = 0.1;

/// How many pending High/Medium hosts may build compounds in one frame.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StickPhysicsBudget {
	pub hosts_per_frame: u32,
	/// Unattached producers probed for a playable band when the queue needs work.
	///
	/// The drain only builds [`Self::hosts_per_frame`] compounds; a full producer
	/// walk every Update is wasted while thousands of Low / UltraLow plants wait.
	/// `u32::MAX` restores the old walk.
	pub inspect_per_frame: u32,
}

impl Default for StickPhysicsBudget {
	fn default() -> Self {
		Self { hosts_per_frame: 8, inspect_per_frame: 256 }
	}
}

type ProduceColliderPoses = fn(&World, Entity, LodSceneLevel) -> Vec<(Transform, f32, f32)>;

/// Type-erased source callback; all vegetation types share one runtime drain.
#[derive(Component, Clone, Copy)]
struct StickPhysicsProducer(ProduceColliderPoses);

#[derive(Component)]
pub(crate) struct StickPhysicsCompound;

/// This host was considered for a stick compound (empty or stamped).
///
/// Persists across LOD bands so we do not rebuild or re-queue.
#[derive(Component, Clone, Copy)]
pub(crate) struct StickPhysicsAttached;

#[derive(Resource, Default)]
struct StickPhysicsQueue {
	pending: VecDeque<Entity>,
	pending_entities: HashSet<Entity>,
	inspect_cursor: u32,
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

fn attach_vegetation_producer<T>(insert: On<Insert<T>>, mut commands: Commands)
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
				.collect()
		}));
	}
}

fn attach_stick_node_producer(insert: On<Insert<StickNode>>, mut commands: Commands) {
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
	matches!(level, LodSceneLevel::High | LodSceneLevel::Medium)
}

/// Playable band on this entity, or an ancestor [`LodSceneHost`] (posed grove plants).
fn playable_level_for(world: &World, entity: Entity) -> Option<LodSceneLevel> {
	let mut current = Some(entity);
	while let Some(entity) = current {
		if let Some(level) = world.get::<LodSceneLevel>(entity).copied() {
			return wants_playable_colliders(level).then_some(level);
		}
		current = world.get::<ChildOf>(entity).map(|child| child.parent());
	}
	None
}

fn refill_pending_producers(world: &mut World, inspect_limit: u32) {
	if inspect_limit == 0 {
		return;
	}
	let start = world.resource::<StickPhysicsQueue>().inspect_cursor;
	let entities: Vec<Entity> = {
		let mut sources = world
			.query_filtered::<Entity, (With<StickPhysicsProducer>, Without<StickPhysicsAttached>)>(
			);
		sources.iter(world).collect()
	};
	let len = entities.len() as u32;
	if len == 0 {
		world.resource_mut::<StickPhysicsQueue>().inspect_cursor = 0;
		return;
	}
	let start = start % len;
	let take = inspect_limit.min(len);
	let mut found = Vec::new();
	for offset in 0..take {
		let entity = entities[((start + offset) % len) as usize];
		if playable_level_for(world, entity).is_some() {
			found.push(entity);
		}
	}
	let mut queue = world.resource_mut::<StickPhysicsQueue>();
	queue.inspect_cursor = if take < len { (start + take) % len } else { 0 };
	for entity in found {
		queue.enqueue(entity);
	}
}

fn sync_stick_colliders(world: &mut World) {
	let budget = *world.resource::<StickPhysicsBudget>();
	// Skip the producer walk while the drain already has a full frame of work.
	// Discovery resumes as soon as the queue drops below the budget; compounds
	// and attach stamps stay the same, only later hosts wait a few extra frames.
	if (world.resource::<StickPhysicsQueue>().pending.len() as u32) < budget.hosts_per_frame {
		refill_pending_producers(world, budget.inspect_per_frame);
	}
	for _ in 0..budget.hosts_per_frame {
		let Some(entity) = world.resource_mut::<StickPhysicsQueue>().pop_front() else {
			break;
		};
		if world.get_entity(entity).is_err() {
			continue;
		}
		if playable_level_for(world, entity).is_none() {
			continue;
		}
		if !crate::packed::PackMode::current().is_off()
			&& handoff_ordinary_packed_physics(world, entity)
		{
			continue;
		}
		if world.get::<StickPhysicsAttached>(entity).is_some() && has_compound(world, entity) {
			continue;
		}
		let Some(producer) = world.get::<StickPhysicsProducer>(entity).copied() else {
			continue;
		};
		// High stick IR even when the host is still Medium — same capsules as High visuals.
		let poses = (producer.0)(world, entity, LodSceneLevel::High);
		despawn_compound(world, entity);
		if world.get_entity(entity).is_err() {
			continue;
		}
		if poses.is_empty() {
			world.entity_mut(entity).insert(StickPhysicsAttached);
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
		world.entity_mut(entity).insert(StickPhysicsAttached);
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

/// Trunks always; branches at least four inches in world space. Thinner High twigs stay visual-only.
fn should_collide_member(is_trunk: bool, placement: Placement) -> bool {
	is_trunk || authored_radius(placement) + 1e-5 >= MIN_STICK_COLLIDER_RADIUS_M
}

fn collider_poses(node: &StickNode, level: LodSceneLevel) -> Vec<(Transform, f32, f32)> {
	if let Some(collection) = &node.collection {
		let members = collection.members_for_level(level);
		let mut poses: Vec<_> = members
			.iter()
			.filter_map(|member| gated_member_pose(node.placement, member))
			.collect();
		if poses.is_empty() {
			poses = members
				.iter()
				.max_by(|a, b| {
					world_radius(node.placement, a)
						.partial_cmp(&world_radius(node.placement, b))
						.unwrap_or(std::cmp::Ordering::Equal)
				})
				.and_then(|member| {
					capsule_from_placement(world_member_placement(node.placement, member))
				})
				.into_iter()
				.collect();
		}
		return poses;
	}
	capsule_from_placement(node.placement).into_iter().collect()
}

fn world_member_placement(parent: Placement, member: &StickMember) -> Placement {
	parent.compose_child(member.placement)
}

fn world_radius(parent: Placement, member: &StickMember) -> f32 {
	authored_radius(world_member_placement(parent, member))
}

fn gated_member_pose(parent: Placement, member: &StickMember) -> Option<(Transform, f32, f32)> {
	let placed = world_member_placement(parent, member);
	if !should_collide_member(member.is_trunk(), placed) {
		return None;
	}
	capsule_from_placement(placed)
}

fn capsule_from_placement(placement: Placement) -> Option<(Transform, f32, f32)> {
	let length = placement.scale.y;
	if length < MIN_STICK_COLLIDER_LENGTH_M {
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
	use lod::LodSceneHost;
	use vegetation_components::{StickCollection, StickGeometry, StickMember};

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
	fn high_and_medium_want_playable_colliders() {
		assert!(wants_playable_colliders(LodSceneLevel::High));
		assert!(wants_playable_colliders(LodSceneLevel::Medium));
		assert!(!wants_playable_colliders(LodSceneLevel::Low));
		assert!(!wants_playable_colliders(LodSceneLevel::UltraLow));
	}

	#[test]
	fn collection_keeps_every_gated_member() {
		let count = 84;
		// World girth `scale.x * STICK_KIT_HALF` must meet the four-inch branch gate.
		let girth = MIN_STICK_COLLIDER_RADIUS_M / STICK_KIT_HALF + 0.01;
		let members = (0..count)
			.map(|index| StickMember {
				geometry: StickGeometry::Segment,
				placement: Placement::new(Vec3::new(index as f32, 0.0, 0.0), 0.0)
					.with_scale(Vec3::new(girth, 2.0, girth)),
			})
			.collect::<Vec<_>>();
		let node = StickNode::collection(
			StickCollection::new(members).bake_bounds_from_members(),
			Placement::IDENTITY,
		);
		assert_eq!(collider_poses(&node, LodSceneLevel::High).len(), count);
	}

	#[test]
	fn ungated_segments_keep_the_thickest_member() {
		let members = (0..8)
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
		assert_eq!(collider_poses(&node, LodSceneLevel::High).len(), 1);
	}

	fn playable_trunk() -> StickNode {
		StickNode::collection(
			StickCollection::new([StickMember {
				geometry: StickGeometry::Trunk,
				placement: Placement::IDENTITY.with_scale(Vec3::new(0.4, 4.0, 0.4)),
			}])
			.bake_bounds_from_members(),
			Placement::IDENTITY,
		)
	}

	fn empty_sticks() -> StickNode {
		StickNode::collection(StickCollection::new([]), Placement::IDENTITY)
	}

	#[test]
	fn high_compound_attaches_after_playable_band_arrives() {
		let mut app = App::new();
		app.add_plugins(StickPhysicsPlugin);
		let host = app.world_mut().spawn(playable_trunk()).id();
		app.update();
		assert!(!has_compound(app.world(), host));

		app.world_mut().entity_mut(host).insert(LodSceneLevel::High);
		app.update();
		assert!(has_compound(app.world(), host));
	}

	#[test]
	fn posed_plant_uses_ancestor_grove_band() {
		let mut app = App::new();
		app.add_plugins(StickPhysicsPlugin);
		let grove = app.world_mut().spawn((LodSceneHost, LodSceneLevel::High)).id();
		let plant = app.world_mut().spawn((playable_trunk(), ChildOf(grove))).id();
		app.update();
		assert!(has_compound(app.world(), plant));
		assert!(!has_compound(app.world(), grove));
	}

	#[test]
	fn medium_host_stamps_high_ir_compound() {
		let mut app = App::new();
		app.add_plugins(StickPhysicsPlugin);
		let host = app
			.world_mut()
			.spawn((playable_trunk(), LodSceneHost, LodSceneLevel::Medium))
			.id();
		app.update();
		assert!(has_compound(app.world(), host));
	}

	#[test]
	fn compound_stays_on_the_host_after_leaving_playable_bands() {
		let mut app = App::new();
		app.add_plugins(StickPhysicsPlugin);
		let host = app
			.world_mut()
			.spawn((playable_trunk(), LodSceneHost, LodSceneLevel::High))
			.id();
		app.update();
		assert!(has_compound(app.world(), host));

		*app.world_mut().get_mut::<LodSceneLevel>(host).expect("level") = LodSceneLevel::Low;
		app.update();
		assert!(has_compound(app.world(), host));
		assert!(app.world().get::<StickPhysicsAttached>(host).is_some());
	}

	#[test]
	fn empty_hosts_do_not_starve_later_playable_hosts() {
		let mut app = App::new();
		app.add_plugins(StickPhysicsPlugin);
		app.insert_resource(StickPhysicsBudget { hosts_per_frame: 1, ..Default::default() });
		for _ in 0..3 {
			app.world_mut().spawn((empty_sticks(), LodSceneHost, LodSceneLevel::High));
		}
		app.update();
		app.update();
		app.update();

		let tree = app
			.world_mut()
			.spawn((playable_trunk(), LodSceneHost, LodSceneLevel::High))
			.id();
		app.update();
		assert!(has_compound(app.world(), tree));
		assert!(app.world().get::<StickPhysicsAttached>(tree).is_some());
	}

	#[test]
	fn backlog_skips_rescan_until_queue_drops_below_budget() {
		let mut app = App::new();
		app.add_plugins(StickPhysicsPlugin);
		app.insert_resource(StickPhysicsBudget { hosts_per_frame: 1, ..Default::default() });
		let first: Vec<_> = (0..3)
			.map(|_| {
				app.world_mut()
					.spawn((playable_trunk(), LodSceneHost, LodSceneLevel::High))
					.id()
			})
			.collect();
		app.update();
		let attached = first
			.iter()
			.filter(|entity| app.world().get::<StickPhysicsAttached>(**entity).is_some())
			.count();
		assert_eq!(attached, 1);

		let late = app
			.world_mut()
			.spawn((playable_trunk(), LodSceneHost, LodSceneLevel::High))
			.id();
		// Two first-batch hosts remain queued (>= budget), so the late host waits.
		app.update();
		assert!(app.world().get::<StickPhysicsAttached>(late).is_none());
		app.update();
		app.update();
		// Queue is empty; the next scan discovers the late host.
		app.update();
		assert!(has_compound(app.world(), late));
	}

	#[test]
	fn inspect_budget_still_attaches_every_playable_host() {
		let mut app = App::new();
		app.add_plugins(StickPhysicsPlugin);
		app.insert_resource(StickPhysicsBudget { hosts_per_frame: 8, inspect_per_frame: 2 });
		let hosts: Vec<_> = (0..5)
			.map(|_| {
				app.world_mut()
					.spawn((playable_trunk(), LodSceneHost, LodSceneLevel::High))
					.id()
			})
			.collect();
		app.update();
		let attached = hosts
			.iter()
			.filter(|entity| app.world().get::<StickPhysicsAttached>(**entity).is_some())
			.count();
		assert_eq!(attached, 2);

		app.update();
		app.update();
		for host in hosts {
			assert!(has_compound(app.world(), host));
		}
	}
}

/// Type-erased ownership recipe retains the exact typed posed plant and callbacks.
/// No mesh/entity scan is used to reconstruct its visual IR.
pub(crate) trait PackedPlantRecipe: VegetationComponents + Send + Sync {
	fn origin(&self) -> Vec3;
	fn matches(&self, world: &World, entity: Entity) -> bool;
	fn attach(
		&self,
		recipe: std::sync::Arc<dyn PackedPlantRecipe>,
		world: &mut World,
		cell: Entity,
		existing: &[Entity],
		groups: &mut std::collections::HashMap<std::any::TypeId, Vec<Entity>>,
	) -> Entity;
}
struct TypedPlantRecipe<T: Send + Sync + 'static>(
	vegetation_components::PlacedVegetation<std::sync::Arc<T>>,
);
impl<T: VegetationComponents + Send + Sync + 'static> VegetationComponents for TypedPlantRecipe<T> {
	fn stick_nodes_for_level(
		&self,
		level: LodSceneLevel,
	) -> vegetation_components::Layers<StickNode> {
		self.0.stick_nodes_for_level(level)
	}
	fn foliage_nodes_for_level(
		&self,
		level: LodSceneLevel,
	) -> vegetation_components::Layers<vegetation_components::FoliageNode> {
		self.0.foliage_nodes_for_level(level)
	}
	fn structural_lod(&self) -> Option<vegetation_components::StructuralLod> {
		self.0.structural_lod()
	}
}
impl<T: VegetationComponents + Send + Sync + 'static> PackedPlantRecipe for TypedPlantRecipe<T> {
	fn origin(&self) -> Vec3 {
		self.0.placement.translation
	}
	fn matches(&self, world: &World, entity: Entity) -> bool {
		use vegetation_components::{FlattenedComponentsOnly, PlacedVegetation};
		world
			.get::<FlattenedComponentsOnly<PlacedVegetation<std::sync::Arc<T>>>>(entity)
			.is_some_and(|source| {
				source.0.placement == self.0.placement
					&& std::sync::Arc::ptr_eq(&source.0.vegetation, &self.0.vegetation)
			})
	}
	fn attach(
		&self,
		recipe: std::sync::Arc<dyn PackedPlantRecipe>,
		world: &mut World,
		cell: Entity,
		candidates: &[Entity],
		groups: &mut std::collections::HashMap<std::any::TypeId, Vec<Entity>>,
	) -> Entity {
		use lod::LodScene;
		use std::sync::Arc;
		use vegetation_components::{
			FlattenedComponentsOnly, PlacedVegetation, VegetationInstance,
		};
		type Source<T> = FlattenedComponentsOnly<PlacedVegetation<Arc<T>>>;
		let existing = groups.entry(std::any::TypeId::of::<T>()).or_insert_with(|| {
			candidates
				.iter()
				.copied()
				.filter(|&entity| world.get::<Source<T>>(entity).is_some())
				.collect()
		});
		let placed = self.0.clone();
		let retained = existing
			.iter()
			.enumerate()
			.filter(|(_, entity)| self.matches(world, **entity))
			.min_by_key(|(_, entity)| {
				(
					world.get::<bevy::ecs::entity_disabling::Disabled>(**entity).is_some(),
					!has_compound(world, **entity),
				)
			})
			.map(|(index, _)| index);

		let entity = if let Some(index) = retained {
			let entity = existing.swap_remove(index);
			// Reparent before the ordinary visual root bag is retired. Existing compound
			// children and their bodies remain the exact same entities.
			world
				.entity_mut(entity)
				.remove::<Source<T>>()
				.remove::<lod::LodSceneHost>()
				.remove::<LodSceneLevel>();
			let visuals = world
				.get::<Children>(entity)
				.map(|children| {
					children
						.iter()
						.filter(|child| world.get::<StickPhysicsCompound>(*child).is_none())
						.collect::<Vec<_>>()
				})
				.unwrap_or_default();
			for visual in visuals {
				let _ = world.despawn(visual);
			}
			entity
		} else {
			let bounds = FlattenedComponentsOnly(placed.clone()).scene_bounds();
			let center = (bounds.min + bounds.max) * 0.5;
			let radius =
				((bounds.max.x - bounds.min.x).max(bounds.max.z - bounds.min.z) * 0.5).max(0.05);
			world
				.spawn((
					ChildOf(cell),
					Transform::IDENTITY,
					Visibility::Inherited,
					VegetationInstance::new(center.into(), radius),
				))
				.id()
		};
		world.entity_mut(entity).insert((
			PackedTreePhysics(recipe),
			StickPhysicsProducer(|world, entity, level| {
				world
					.get::<PackedTreePhysics>(entity)
					.into_iter()
					.flat_map(|source| source.0.stick_nodes_for_level(level).flatten())
					.flat_map(|node| collider_poses(&node, level))
					.collect()
			}),
		));
		entity
	}
}
#[derive(Component, Clone)]
struct PackedTreePhysics(std::sync::Arc<dyn PackedPlantRecipe>);

#[derive(Default)]
pub(crate) struct PackedPlantCollector(pub Vec<std::sync::Arc<dyn PackedPlantRecipe>>);
impl vegetation_groves::grove::PosedPlantVisitor for PackedPlantCollector {
	type Output = ();
	fn plant<T: VegetationComponents + Send + Sync + 'static>(
		&mut self,
		plant: std::sync::Arc<T>,
		placement: Placement,
		stick: &material_ref::MaterialRef,
		ball: &material_ref::MaterialRef,
		frond: &material_ref::MaterialRef,
	) {
		self.0
			.push(std::sync::Arc::new(TypedPlantRecipe(vegetation_groves::grove::placed(
				plant, placement, stick, ball, frond,
			))));
	}
}
pub(crate) fn packed_plants(
	tile: &crate::ForestGroveTile,
) -> Option<Vec<std::sync::Arc<dyn PackedPlantRecipe>>> {
	let mut visitor = PackedPlantCollector::default();
	tile.visit_posed_plants(&mut visitor).then_some(visitor.0)
}
pub(crate) fn attach_packed_source(world: &mut World, cell: Entity, host: crate::ChicoGroveHost) {
	let mut pending = vec![cell];
	let mut existing = Vec::new();
	while let Some(entity) = pending.pop() {
		existing.push(entity);
		if let Some(children) = world.get::<Children>(entity) {
			pending.extend(children.iter());
		}
	}
	let mut groups = std::collections::HashMap::new();
	let mut transferred = Vec::new();
	for recipe in packed_plants(&host.tile).into_iter().flatten() {
		transferred.push(recipe.attach(
			std::sync::Arc::clone(&recipe),
			world,
			cell,
			&existing,
			&mut groups,
		));
	}
	lod::scene::host::transfer_lod_tree_ownership(world, &transferred, cell);
}

pub(crate) fn remove_packed_sources(world: &mut World, cell: Entity) {
	let sources = world
		.get::<Children>(cell)
		.map(|children| {
			children
				.iter()
				.filter(|child| world.get::<PackedTreePhysics>(*child).is_some())
				.collect::<Vec<_>>()
		})
		.unwrap_or_default();
	for source in sources {
		let _ = world.despawn(source);
	}
}

/// During reverse handoff reuse exact existing bodies before the normal drain
/// attempts a replacement. No extra queue or collider grouping is introduced.
fn handoff_ordinary_packed_physics(world: &mut World, entity: Entity) -> bool {
	let mut ancestor = world.get::<ChildOf>(entity).map(ChildOf::parent);
	let mut cell = None;
	while let Some(parent) = ancestor {
		if world.get::<crate::packed::lifecycle::RestoringOrdinary>(parent).is_some() {
			cell = Some(parent);
			break;
		}
		ancestor = world.get::<ChildOf>(parent).map(ChildOf::parent);
	}
	let Some(cell) = cell else {
		return false;
	};
	let retained = world.get::<Children>(cell).and_then(|children| {
		children.iter().find(|child| {
			world
				.get::<PackedTreePhysics>(*child)
				.is_some_and(|old| old.0.matches(world, entity))
		})
	});
	let Some(retained) = retained else {
		return false;
	};
	if world.get::<StickPhysicsAttached>(retained).is_none() {
		return false;
	}
	let compounds = world
		.get::<Children>(retained)
		.map(|children| {
			children
				.iter()
				.filter(|child| world.get::<StickPhysicsCompound>(*child).is_some())
				.collect::<Vec<_>>()
		})
		.unwrap_or_default();
	lod::scene::host::transfer_lod_tree_ownership(world, &compounds, entity);
	for compound in compounds {
		world.entity_mut(compound).insert(Visibility::Hidden);
	}
	world.entity_mut(entity).insert(StickPhysicsAttached);
	let _ = world.despawn(retained);
	true
}

/// Called only after the replacement ordinary bag has completely streamed.
/// Inspect that bag's actual producers, never the old packed source count/type.
pub(crate) fn ordinary_packed_physics_ready(world: &World, bag: Entity) -> bool {
	let mut pending = vec![bag];
	while let Some(entity) = pending.pop() {
		if world.get::<StickPhysicsProducer>(entity).is_some()
			&& world.get::<StickPhysicsAttached>(entity).is_none()
		{
			return false;
		}
		if let Some(children) = world.get::<Children>(entity) {
			pending.extend(children.iter());
		}
	}
	true
}

#[cfg(test)]
mod packed_transfer_tests {
	use super::*;
	use bevy::ecs::entity_disabling::Disabled;
	use lod::{LodLevelRoot, LodLevelRoots, LodTreeVisQueue};
	use std::sync::Arc;
	use vegetation_components::{FlattenedComponentsOnly, PlacedVegetation};
	use vegetation_groves::grove::GroveCellVariant;
	use vegetation_groves::orchard::OrchardCell;
	use vegetation_groves::{GroveExtent, Orchard};
	type Source = FlattenedComponentsOnly<PlacedVegetation<Arc<sbs_trees::StorybookTree>>>;

	fn fixture() -> (App, Entity, crate::ChicoGroveHost) {
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::splat(10.0));
		let orchard = Orchard::from_placements(
			&[GroveCellVariant::new(OrchardCell::FruitingStorybook, Vec3::new(3.0, 0.0, 3.0), 1.0)],
			procedural_common::NoiseParams::default(),
			&extent,
			1,
		);
		assert_eq!(orchard.plants.len(), 1);
		let host = crate::ChicoGroveHost::new(
			crate::ForestGroveTile::Orchard(orchard),
			crate::ForestLayer::UpperCanopy,
		);
		let mut app = App::new();
		app.add_plugins(StickPhysicsPlugin);
		register_vegetation_stick_colliders::<Source>(&mut app);
		let cell = app
			.world_mut()
			.spawn((LodSceneLevel::High, lod::LodSceneHost, Visibility::Inherited))
			.id();
		(app, cell, host)
	}
	fn ordinary_source(
		app: &mut App,
		cell: Entity,
		host: &crate::ChicoGroveHost,
	) -> anyhow::Result<(Entity, Entity, Entity)> {
		let plant = host
			.tile
			.as_orchard()
			.and_then(|orchard| orchard.plants.first())
			.ok_or_else(|| anyhow::anyhow!("missing fixture plant"))?;
		let bag = app.world_mut().spawn((LodLevelRoots, ChildOf(cell))).id();
		let root = app
			.world_mut()
			.spawn((LodLevelRoot(LodSceneLevel::High), ChildOf(bag), Visibility::Inherited))
			.id();
		let source = app
			.world_mut()
			.spawn((
				FlattenedComponentsOnly(plant.placed()),
				ChildOf(root),
				Transform::IDENTITY,
				Visibility::Inherited,
			))
			.id();
		app.update();
		let compound = app
			.world()
			.get::<Children>(source)
			.and_then(|children| {
				children
					.iter()
					.find(|child| app.world().get::<StickPhysicsCompound>(*child).is_some())
			})
			.ok_or_else(|| anyhow::anyhow!("real tree did not produce its compound"))?;
		Ok((bag, source, compound))
	}
	fn hide(app: &mut App, entity: Entity) {
		lod::scene::host::hide_lod_tree(&mut app.world_mut().commands(), entity);
		app.world_mut().flush();
	}
	fn assert_enabled_body(app: &mut App, source: Entity, body: Entity) {
		assert!(app.world().get::<Disabled>(source).is_none());
		assert!(app.world().get::<Disabled>(body).is_none());
		assert!(app.world().get::<StickPhysicsProducer>(source).is_some());
		assert!(app.world().get::<Collider>(body).is_some());
		assert_eq!(app.world().get::<ChildOf>(body).map(ChildOf::parent), Some(source));
		assert_eq!(
			app.world_mut()
				.query_filtered::<Entity, With<StickPhysicsCompound>>()
				.iter(app.world())
				.count(),
			1
		);
	}
	#[test]
	fn hidden_real_source_survives_queued_hide_and_body_return() -> anyhow::Result<()> {
		let (mut app, cell, host) = fixture();
		let (bag, source, body) = ordinary_source(&mut app, cell, &host)?;
		hide(&mut app, bag);
		assert!(app.world().get::<Disabled>(source).is_some());
		assert!(app.world().get::<Disabled>(body).is_some());
		app.world_mut().insert_resource(LodTreeVisQueue::default());
		// Leave an old hide job with explicit surviving source/body visits pending.
		lod::scene::host::show_lod_tree(&mut app.world_mut().commands(), bag);
		app.world_mut().flush();
		hide(&mut app, bag);
		app.world_mut().insert_resource(lod::LodTreeVisBudget {
			entities_per_frame: 1,
			time_per_frame: std::time::Duration::from_secs(1),
		});
		lod::scene::host::drain_lod_tree_vis(app.world_mut());
		attach_packed_source(app.world_mut(), cell, host.clone());
		app.world_mut().despawn(bag);
		lod::scene::host::drain_lod_tree_vis(app.world_mut());
		assert_enabled_body(&mut app, source, body);
		assert_eq!(
			app.world_mut()
				.query_filtered::<Entity, With<PackedTreePhysics>>()
				.iter(app.world())
				.count(),
			1
		);
		app.world_mut()
			.entity_mut(cell)
			.insert(crate::packed::lifecycle::RestoringOrdinary);
		let placed = host
			.tile
			.as_orchard()
			.and_then(|orchard| orchard.plants.first())
			.ok_or_else(|| anyhow::anyhow!("plant disappeared"))?
			.placed();
		let ordinary = app
			.world_mut()
			.spawn((
				FlattenedComponentsOnly(placed),
				ChildOf(cell),
				Transform::IDENTITY,
				Visibility::Inherited,
			))
			.id();
		app.world_mut().flush();
		assert!(handoff_ordinary_packed_physics(app.world_mut(), ordinary));
		lod::scene::host::drain_lod_tree_vis(app.world_mut());
		assert!(app.world().get_entity(source).is_err());
		assert_enabled_body(&mut app, ordinary, body);
		app.update();
		assert_enabled_body(&mut app, ordinary, body);
		Ok(())
	}
	#[test]
	fn live_matching_source_is_preferred_to_disabled_warm_copy() -> anyhow::Result<()> {
		let (mut app, cell, host) = fixture();
		let (hidden_bag, hidden, _) = ordinary_source(&mut app, cell, &host)?;
		hide(&mut app, hidden_bag);
		let (live_bag, live, body) = ordinary_source(&mut app, cell, &host)?;
		attach_packed_source(app.world_mut(), cell, host);
		app.world_mut().despawn(hidden_bag);
		app.world_mut().despawn(live_bag);
		assert!(app.world().get_entity(hidden).is_err());
		assert_enabled_body(&mut app, live, body);
		Ok(())
	}
	#[test]
	fn transferred_real_body_preserves_disabled_cell_ownership() -> anyhow::Result<()> {
		let (mut app, cell, host) = fixture();
		let (bag, source, body) = ordinary_source(&mut app, cell, &host)?;
		hide(&mut app, cell);
		attach_packed_source(app.world_mut(), cell, host);
		app.world_mut().despawn(bag);
		assert!(app.world().get::<Disabled>(source).is_some());
		assert!(app.world().get::<Disabled>(body).is_some());
		lod::scene::host::show_lod_tree(&mut app.world_mut().commands(), cell);
		app.world_mut().flush();
		assert_enabled_body(&mut app, source, body);
		Ok(())
	}
	#[test]
	fn mixed_oak_storybook_bush_preserves_real_source_and_body_ids_both_directions(
	) -> anyhow::Result<()> {
		use vegetation_groves::grove::PosedPlantVisitor;
		use vegetation_groves::{riparian_general::RiparianGeneralCell, RiparianGeneral};
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::splat(40.0));
		let grove = RiparianGeneral::from_placements(
			&[
				GroveCellVariant::new(
					RiparianGeneralCell::RiparianBraidOak,
					Vec3::new(4.0, 0.0, 4.0),
					1.0,
				),
				GroveCellVariant::new(
					RiparianGeneralCell::RiparianStorybook,
					Vec3::new(20.0, 0.0, 4.0),
					1.0,
				),
				GroveCellVariant::new(
					RiparianGeneralCell::RareRiparianHighBush,
					Vec3::new(36.0, 0.0, 4.0),
					1.0,
				),
			],
			procedural_common::NoiseParams::default(),
			&extent,
			1,
		);
		let host = crate::ChicoGroveHost::new(
			crate::ForestGroveTile::RiparianGeneral(grove),
			crate::ForestLayer::UpperCanopy,
		);
		struct Spawn<'a> {
			world: &'a mut World,
			parent: Entity,
			sources: Vec<Entity>,
		}
		impl PosedPlantVisitor for Spawn<'_> {
			type Output = ();
			fn plant<T: VegetationComponents + Send + Sync + 'static>(
				&mut self,
				plant: Arc<T>,
				placement: Placement,
				stick: &material_ref::MaterialRef,
				ball: &material_ref::MaterialRef,
				frond: &material_ref::MaterialRef,
			) {
				let placed = vegetation_groves::grove::placed(plant, placement, stick, ball, frond);
				self.sources.push(
					self.world
						.spawn((
							FlattenedComponentsOnly(placed),
							ChildOf(self.parent),
							Transform::IDENTITY,
							Visibility::Inherited,
						))
						.id(),
				);
			}
		}
		let mut app = App::new();
		app.add_plugins(StickPhysicsPlugin);
		register_vegetation_stick_colliders::<Source>(&mut app);
		register_vegetation_stick_colliders::<
			FlattenedComponentsOnly<PlacedVegetation<Arc<sbs_trees::BraidOakTree>>>,
		>(&mut app);
		register_vegetation_stick_colliders::<
			FlattenedComponentsOnly<PlacedVegetation<Arc<sbs_trees::HighBushShoots>>>,
		>(&mut app);
		let cell = app
			.world_mut()
			.spawn((LodSceneLevel::High, lod::LodSceneHost, Visibility::Inherited))
			.id();
		let bag = app
			.world_mut()
			.spawn((LodLevelRoots, ChildOf(cell), Visibility::Inherited))
			.id();
		let mut spawn = Spawn { world: app.world_mut(), parent: bag, sources: Vec::new() };
		assert!(host.tile.visit_posed_plants(&mut spawn));
		let sources = spawn.sources;
		assert_eq!(sources.len(), 3);
		app.update();
		let bodies = sources
			.iter()
			.map(|source| {
				app.world()
					.get::<Children>(*source)
					.and_then(|children| {
						children
							.iter()
							.find(|child| app.world().get::<StickPhysicsCompound>(*child).is_some())
					})
					.ok_or_else(|| anyhow::anyhow!("mixed plant missing real body"))
			})
			.collect::<anyhow::Result<Vec<_>>>()?;
		hide(&mut app, bag);
		attach_packed_source(app.world_mut(), cell, host.clone());
		app.world_mut().despawn(bag);
		for (&source, &body) in sources.iter().zip(&bodies) {
			assert!(app.world().get_entity(source).is_ok());
			assert!(app.world().get::<Disabled>(body).is_none());
			assert_eq!(app.world().get::<ChildOf>(body).map(ChildOf::parent), Some(source));
		}
		app.world_mut()
			.entity_mut(cell)
			.insert(crate::packed::lifecycle::RestoringOrdinary);
		let new_bag = app.world_mut().spawn((LodLevelRoots, ChildOf(cell))).id();
		let mut spawn = Spawn { world: app.world_mut(), parent: new_bag, sources: Vec::new() };
		host.tile.visit_posed_plants(&mut spawn);
		let restored = spawn.sources;
		app.world_mut().flush();
		assert!(!ordinary_packed_physics_ready(app.world(), new_bag));
		for &source in &restored {
			assert!(handoff_ordinary_packed_physics(app.world_mut(), source));
		}
		assert!(ordinary_packed_physics_ready(app.world(), new_bag));
		for (&source, &body) in restored.iter().zip(&bodies) {
			assert_eq!(app.world().get::<ChildOf>(body).map(ChildOf::parent), Some(source));
		}
		for source in sources {
			assert!(app.world().get_entity(source).is_err());
		}
		assert_eq!(
			app.world_mut()
				.query_filtered::<Entity, With<StickPhysicsCompound>>()
				.iter(app.world())
				.count(),
			3
		);
		Ok(())
	}
	#[test]
	fn changed_count_replacement_waits_for_its_actual_two_producers() {
		let mut app = App::new();
		app.add_plugins(StickPhysicsPlugin);
		let cell = app.world_mut().spawn(LodSceneLevel::High).id();
		let bag = app.world_mut().spawn(ChildOf(cell)).id();
		for _ in 0..2 {
			app.world_mut()
				.spawn((ChildOf(bag), StickPhysicsProducer(|_, _, _| Vec::new())));
		}
		assert!(!ordinary_packed_physics_ready(app.world(), bag));
		app.update();
		assert!(ordinary_packed_physics_ready(app.world(), bag));
		assert_eq!(
			app.world_mut()
				.query_filtered::<Entity, With<StickPhysicsAttached>>()
				.iter(app.world())
				.count(),
			2
		);
	}
}
