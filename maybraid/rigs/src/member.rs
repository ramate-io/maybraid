//! Assembly membership: which nested hosts belong to which [`AssemblyRoot`].
//!
//! [`MemberOf`] cannot be stamped at recipe build ([`Entity::PLACEHOLDER`]).
//! Walk [`ChildOf`] up to [`AssemblyRoot`] **before** socket fulfill. After
//! sockets, [`ChildOf`] is the bone and [`MemberOf`] still means “this assembly.”

use bevy::ecs::relationship::RelationshipTarget;
use bevy::prelude::*;

use crate::bone_map::{BoneMap, RigKey, RigRoot};

/// Marker on the assembly host (character, firearm, …).
#[derive(Component, Clone, Copy, Default)]
pub struct AssemblyRoot;

/// Nested lod/part/rig host that should join the nearest [`AssemblyRoot`].
#[derive(Component, Clone, Copy, Default)]
pub struct AssemblyHost;

/// Source of truth: this nested host belongs to the assembly at `0`.
#[derive(Component, Debug, Clone, Copy)]
#[relationship(relationship_target = AssemblyMembers)]
pub struct MemberOf(pub Entity);

/// All [`AssemblyHost`]s that belong to an [`AssemblyRoot`].
#[derive(Component, Debug)]
#[relationship_target(relationship = MemberOf)]
pub struct AssemblyMembers(Vec<Entity>);

/// Stamp [`MemberOf`] on nested hosts that do not have it yet.
pub fn stamp_assembly_members(
	mut commands: Commands,
	hosts: Query<Entity, (With<AssemblyHost>, Without<MemberOf>)>,
	child_of: Query<&ChildOf>,
	roots: Query<(), With<AssemblyRoot>>,
) {
	for entity in &hosts {
		let Some(root) = assembly_root_of(entity, &child_of, &roots) else {
			continue;
		};
		commands.entity(entity).insert(MemberOf(root));
	}
}

fn assembly_root_of(
	mut entity: Entity,
	child_of: &Query<&ChildOf>,
	roots: &Query<(), With<AssemblyRoot>>,
) -> Option<Entity> {
	loop {
		if roots.contains(entity) {
			return Some(entity);
		}
		entity = child_of.get(entity).ok()?.parent();
	}
}

/// The rig among `members` whose [`RigRoot::key`] matches `key`.
pub fn find_member_rig<'a>(
	members: &AssemblyMembers,
	key: RigKey,
	rigs: &'a Query<(Entity, &RigRoot, &BoneMap)>,
) -> Option<(Entity, &'a BoneMap)> {
	members.iter().find_map(|member| {
		let (entity, root, map) = rigs.get(member).ok()?;
		(root.key == key).then_some((entity, map))
	})
}

/// First [`RigRoot`] among `members` (single-rig assemblies).
pub fn find_any_member_rig<'a>(
	members: &AssemblyMembers,
	rigs: &'a Query<(Entity, &RigRoot, &BoneMap)>,
) -> Option<(Entity, &'a BoneMap)> {
	members.iter().find_map(|member| {
		let (entity, _, map) = rigs.get(member).ok()?;
		Some((entity, map))
	})
}
