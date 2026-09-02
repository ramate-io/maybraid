//! Deferred socket identities: parent a host under a named bone once the
//! target [`crate::BoneMap`] is ready.

use bevy::prelude::*;

use crate::bone_map::{BoneMap, RigKey, RigRoot};
use crate::member::{
	find_any_member_rig, find_member_rig, AssemblyMembers, AssemblyRoot, MemberOf,
};

/// Deferred bone attachment: which rig, which bone, local pose on that bone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SocketRef {
	/// Target armature. `None` means the assembly's only / first rig.
	pub rig: Option<RigKey>,
	/// Bone name on the target rig. `None` parents under the [`AssemblyRoot`].
	pub bone: Option<&'static str>,
	pub local: Transform,
}

impl SocketRef {
	/// Socket onto `bone` of `rig`.
	pub fn on(rig: impl Into<RigKey>, bone: &'static str) -> Self {
		Self { rig: Some(rig.into()), bone: Some(bone), local: Transform::IDENTITY }
	}

	/// Socket onto `bone` of the assembly's default / only rig.
	pub fn bone(bone: &'static str) -> Self {
		Self { rig: None, bone: Some(bone), local: Transform::IDENTITY }
	}

	/// Parent under the assembly root (no armature, or identity kit pose).
	pub fn root() -> Self {
		Self { rig: None, bone: None, local: Transform::IDENTITY }
	}

	pub fn with_local(mut self, local: Transform) -> Self {
		self.local = local;
		self
	}
}

impl Default for SocketRef {
	fn default() -> Self {
		Self::root()
	}
}

/// BSN / ECS marker: this entity should be parented under [`SocketRef`].
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct SocketRefRoot(pub SocketRef);

/// Marker: [`SocketRefRoot`] has been parented.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SocketRefApplied;

pub fn invalidate_changed_socket_ref_roots(
	mut commands: Commands,
	changed: Query<Entity, (Changed<SocketRefRoot>, With<SocketRefApplied>)>,
) {
	for entity in &changed {
		commands.entity(entity).remove::<SocketRefApplied>();
	}
}

pub fn fulfill_socket_ref_roots(
	mut commands: Commands,
	mut pending: Query<(Entity, &SocketRefRoot, &mut Transform), Without<SocketRefApplied>>,
	member_of: Query<&MemberOf>,
	members: Query<&AssemblyMembers>,
	rigs: Query<(Entity, &RigRoot, &BoneMap)>,
	roots: Query<(), With<AssemblyRoot>>,
) {
	for (entity, SocketRefRoot(socket), mut transform) in &mut pending {
		let Ok(MemberOf(root)) = member_of.get(entity) else {
			continue;
		};
		if !roots.contains(*root) {
			continue;
		}
		let Ok(assembly_members) = members.get(*root) else {
			continue;
		};
		if attach_to_socket(
			&mut commands,
			entity,
			&mut transform,
			socket,
			*root,
			assembly_members,
			&rigs,
		) {
			commands.entity(entity).insert(SocketRefApplied);
		}
	}
}

fn attach_to_socket(
	commands: &mut Commands,
	entity: Entity,
	transform: &mut Transform,
	socket: &SocketRef,
	root: Entity,
	members: &AssemblyMembers,
	rigs: &Query<(Entity, &RigRoot, &BoneMap)>,
) -> bool {
	let Some(parent) = resolve_socket_parent(socket, root, members, rigs) else {
		return false;
	};

	let authored_scale = transform.scale;
	let authored_rotation = transform.rotation;
	*transform = socket.local;
	transform.scale *= authored_scale;
	transform.rotation *= authored_rotation;
	commands.entity(entity).try_insert(ChildOf(parent));
	true
}

/// `None` means retry next frame (rig present but bone not ready, or keyed rig missing).
pub fn resolve_socket_parent(
	socket: &SocketRef,
	assembly_root: Entity,
	members: &AssemblyMembers,
	rigs: &Query<(Entity, &RigRoot, &BoneMap)>,
) -> Option<Entity> {
	let Some(bone) = socket.bone.filter(|b| !b.is_empty()) else {
		return Some(assembly_root);
	};

	let found = match socket.rig {
		None => find_any_member_rig(members, rigs),
		Some(key) => find_member_rig(members, key, rigs),
	};

	match found {
		Some((_, map)) => map.by_name.get(bone).copied(),
		// Keyed rig is missing but another rig exists — wait for it.
		None if socket.rig.is_some() && find_any_member_rig(members, rigs).is_some() => None,
		// No armature in this assembly: kit parts sit on the root at authored pose.
		None => Some(assembly_root),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn keyed_socket_carries_the_caller_name() {
		let socket = SocketRef::on(RigKey::named("body"), "head_socket");
		assert_eq!(socket.rig, Some(RigKey::named("body")));
		assert_eq!(socket.bone, Some("head_socket"));
	}

	#[test]
	fn unkeyed_socket_does_not_invent_a_rig_name() {
		assert_eq!(SocketRef::bone("grip").rig, None);
		assert_eq!(SocketRef::root().bone, None);
	}
}
