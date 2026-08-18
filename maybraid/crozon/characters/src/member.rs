//! Character membership: which nested hosts belong to which character root.
//!
//! [`MemberOf`] cannot be stamped from [`lod::LodScene::host`] because
//! [`lod::LodRef::entity`] is [`Entity::PLACEHOLDER`] at recipe build. Walk
//! [`ChildOf`] up to [`CharacterRoot`] **before** socket fulfill (and retries
//! until the parent chain is ready); after sockets, [`ChildOf`] is the bone and
//! [`MemberOf`] still means “this character.”

use bevy::ecs::relationship::RelationshipTarget;
use bevy::prelude::CommandsSceneExt;
use bevy::prelude::*;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::assembly::CharacterPartSlot;
use crate::nodes::{PartNode, RigNode};
use crate::rig::{BoneMap, CharacterRig, CharacterRigRole};

/// Marker on the [`crate::ComponentsOnly`] character host (see [`lod::LodScene::host`]).
#[derive(Component, Clone, Copy, Default)]
pub struct CharacterRoot;

/// Source of truth: this nested host belongs to the character at `0`.
#[derive(Component, Debug, Clone, Copy)]
#[relationship(relationship_target = CharacterMembers)]
pub struct MemberOf(pub Entity);

/// All [`PartNode`] / [`RigNode`] hosts that belong to a [`CharacterRoot`].
#[derive(Component, Debug)]
#[relationship_target(relationship = MemberOf)]
pub struct CharacterMembers(Vec<Entity>);

/// Stamp [`MemberOf`] on nested hosts that do not have it yet, by walking
/// [`ChildOf`] to [`CharacterRoot`]. Retries until the parent chain reaches the
/// character (covers spawn after this system in the same frame).
pub fn stamp_character_members(
	mut commands: Commands,
	hosts: Query<Entity, (Or<(With<PartNode>, With<RigNode>)>, Without<MemberOf>)>,
	child_of: Query<&ChildOf>,
	roots: Query<(), With<CharacterRoot>>,
) {
	for entity in &hosts {
		let Some(root) = character_root_of(entity, &child_of, &roots) else {
			continue;
		};
		commands.entity(entity).insert(MemberOf(root));
	}
}

fn character_root_of(
	mut entity: Entity,
	child_of: &Query<&ChildOf>,
	roots: &Query<(), With<CharacterRoot>>,
) -> Option<Entity> {
	loop {
		if roots.contains(entity) {
			return Some(entity);
		}
		entity = child_of.get(entity).ok()?.parent();
	}
}

/// The rig among `members` whose [`CharacterRig::role`] matches `role`.
pub fn find_member_rig<'a>(
	members: &CharacterMembers,
	role: CharacterRigRole,
	rigs: &'a Query<(Entity, &CharacterRig, &BoneMap)>,
) -> Option<(Entity, &'a BoneMap)> {
	members.iter().find_map(|member| {
		let (entity, rig, map) = rigs.get(member).ok()?;
		(rig.role == role).then_some((entity, map))
	})
}

/// First part member under `root` matching `slot`, preferring `label` when set.
pub fn find_part_member(
	root: Entity,
	slot: CharacterPartSlot,
	label: Option<&str>,
	members: &Query<&CharacterMembers>,
	parts: &Query<&PartNode>,
) -> Option<Entity> {
	let list = members.get(root).ok()?;
	let mut fallback = None;
	for member in list.iter() {
		let Ok(part) = parts.get(member) else {
			continue;
		};
		if part.slot != slot {
			continue;
		}
		match label {
			Some(label) if part.label == label => return Some(member),
			Some(_) => {
				if fallback.is_none() {
					fallback = Some(member);
				}
			}
			None => return Some(member),
		}
	}
	fallback
}

/// Spawn `part` as a nested LodScene host parented to `root`.
///
/// [`stamp_character_members`] records [`MemberOf`]; socket fulfill then reparents
/// to the named bone.
pub fn attach_part_node(commands: &mut Commands, root: Entity, part: PartNode) -> Entity {
	let identity = Transform::IDENTITY;
	let bounds = part.scene_bounds();
	let lod_ref = LodRef {
		entity: root,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &bounds,
	};
	let entity = commands.spawn_scene(part.host(&lod_ref)).id();
	commands.entity(entity).insert(ChildOf(root));
	entity
}
