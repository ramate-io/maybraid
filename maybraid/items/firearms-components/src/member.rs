//! Firearm membership: which nested hosts belong to which firearm root.
//!
//! [`MemberOf`] cannot be stamped from [`lod::LodScene::host`] because
//! [`lod::LodRef::entity`] is [`Entity::PLACEHOLDER`] at recipe build. Walk
//! [`ChildOf`] up to [`FirearmRoot`] **before** socket fulfill.

use bevy::ecs::relationship::RelationshipTarget;
use bevy::prelude::*;

use crate::nodes::{BoneMap, FirearmRig, PartNode, RigNode};

/// Marker on the [`crate::ComponentsOnly`] firearm host (see [`lod::LodScene::host`]).
#[derive(Component, Clone, Copy, Default)]
pub struct FirearmRoot;

/// Source of truth: this nested host belongs to the firearm at `0`.
#[derive(Component, Debug, Clone, Copy)]
#[relationship(relationship_target = FirearmMembers)]
pub struct MemberOf(pub Entity);

/// All [`PartNode`] / [`RigNode`] hosts that belong to a [`FirearmRoot`].
#[derive(Component, Debug)]
#[relationship_target(relationship = MemberOf)]
pub struct FirearmMembers(Vec<Entity>);

/// Stamp [`MemberOf`] on nested hosts that do not have it yet.
pub fn stamp_firearm_members(
	mut commands: Commands,
	hosts: Query<Entity, (Or<(With<PartNode>, With<RigNode>)>, Without<MemberOf>)>,
	child_of: Query<&ChildOf>,
	roots: Query<(), With<FirearmRoot>>,
) {
	for entity in &hosts {
		let Some(root) = firearm_root_of(entity, &child_of, &roots) else {
			continue;
		};
		commands.entity(entity).insert(MemberOf(root));
	}
}

fn firearm_root_of(
	mut entity: Entity,
	child_of: &Query<&ChildOf>,
	roots: &Query<(), With<FirearmRoot>>,
) -> Option<Entity> {
	loop {
		if roots.contains(entity) {
			return Some(entity);
		}
		entity = child_of.get(entity).ok()?.parent();
	}
}

/// The receiver rig among `members`, if any.
pub fn find_member_rig<'a>(
	members: &FirearmMembers,
	rigs: &'a Query<(Entity, &BoneMap), With<FirearmRig>>,
) -> Option<(Entity, &'a BoneMap)> {
	members.iter().find_map(|member| rigs.get(member).ok())
}

/// Rebuild each rig's [`BoneMap`] from named descendants, stopping at nested
/// [`FirearmRig`] / [`PartNode`] boundaries.
pub fn build_rig_bone_map(
	mut rig_roots: Query<(Entity, &Children, &mut BoneMap), With<FirearmRig>>,
	children_q: Query<&Children>,
	names_q: Query<&Name>,
	boundaries: Query<(), Or<(With<FirearmRig>, With<PartNode>)>>,
) {
	for (_rig_root, children, mut map) in &mut rig_roots {
		map.by_name.clear();
		let mut stack: Vec<Entity> = children.iter().collect();
		while let Some(entity) = stack.pop() {
			if boundaries.contains(entity) {
				continue;
			}
			if let Ok(name) = names_q.get(entity) {
				map.by_name.insert(name.to_string(), entity);
			}
			if let Ok(children) = children_q.get(entity) {
				stack.extend(children.iter());
			}
		}
	}
}
