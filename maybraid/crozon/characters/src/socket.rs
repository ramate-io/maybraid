//! Deferred socket and skin identities, parallel to [`scene_ref::SceneRef`].
//!
//! [`SocketRef`] names a bone on a semantic [`RigId`]. Fulfill parents the host
//! under that bone once the target rig's [`crate::rig::BoneMap`] is ready.
//! [`SkinRef`] names the rig that should receive a part's joint remap.

use bevy::prelude::*;

use crate::rig::{BoneMap, CharacterRig, CharacterRigRole, LodCharacterRig};

/// Semantic rig identity used at scene-build time (entities do not exist yet).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RigId {
	#[default]
	Body,
	Neck,
	Head,
}

impl RigId {
	pub fn role(self) -> CharacterRigRole {
		match self {
			Self::Body => CharacterRigRole::Body,
			Self::Neck => CharacterRigRole::Neck,
			Self::Head => CharacterRigRole::Head,
		}
	}
}

/// Deferred bone attachment: which rig, which bone, local pose on that bone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SocketRef {
	pub rig: RigId,
	pub bone: &'static str,
	pub local: Transform,
}

impl SocketRef {
	pub fn on(rig: RigId, bone: &'static str) -> Self {
		Self { rig, bone, local: Transform::IDENTITY }
	}

	pub fn with_local(mut self, local: Transform) -> Self {
		self.local = local;
		self
	}
}

impl Default for SocketRef {
	fn default() -> Self {
		Self::on(RigId::Body, "")
	}
}

/// BSN / ECS marker: this entity should be parented under [`SocketRef`].
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct SocketRefRoot(pub SocketRef);

/// Marker: [`SocketRefRoot`] (or a node-owned socket) has been parented.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SocketRefApplied;

/// Deferred skin-remap target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SkinRef {
	pub target: RigId,
}

impl SkinRef {
	pub fn to(target: RigId) -> Self {
		Self { target }
	}
}

/// BSN / ECS marker: remap this part's `SkinnedMesh` joints onto [`SkinRef`].
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SkinRefRoot(pub SkinRef);

/// Marker: [`SkinRefRoot`] has been resolved to a concrete rig entity.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SkinRefApplied;

pub fn fulfill_socket_ref_roots(
	mut commands: Commands,
	mut pending: Query<(Entity, &SocketRefRoot, &mut Transform), Without<SocketRefApplied>>,
	rigs: Query<(&CharacterRig, &BoneMap), With<LodCharacterRig>>,
) {
	for (entity, SocketRefRoot(socket), mut transform) in &mut pending {
		if attach_to_socket(&mut commands, entity, &mut transform, socket, &rigs) {
			commands.entity(entity).insert(SocketRefApplied);
		}
	}
}

pub(crate) fn attach_to_socket(
	commands: &mut Commands,
	entity: Entity,
	transform: &mut Transform,
	socket: &SocketRef,
	rigs: &Query<(&CharacterRig, &BoneMap), With<LodCharacterRig>>,
) -> bool {
	let role = socket.rig.role();
	let Some((_, map)) = rigs.iter().find(|(rig, _)| rig.role == role) else {
		return false;
	};
	let Some(&bone_entity) = map.by_name.get(socket.bone) else {
		return false;
	};

	let authored_scale = transform.scale;
	let authored_rotation = transform.rotation;
	*transform = socket.local;
	transform.scale *= authored_scale;
	transform.rotation *= authored_rotation;
	commands.entity(entity).try_insert(ChildOf(bone_entity));
	true
}
