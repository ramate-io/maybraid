//! Deferred socket identities, parallel to [`scene_ref::SceneRef`].
//!
//! [`SocketRef`] names a bone on the firearm's receiver rig. Fulfill parents the
//! host under that bone once the target [`crate::BoneMap`] is ready. If the
//! firearm has no rig yet, or the bone is omitted, the host is parented under
//! [`crate::FirearmRoot`].

use bevy::prelude::*;

use crate::member::{find_member_rig, FirearmMembers, MemberOf};
use crate::nodes::BoneMap;

/// Deferred bone attachment: which receiver bone, local pose on that bone.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SocketRef {
	/// Bone name on the receiver rig. `None` parents under the firearm root.
	pub bone: Option<&'static str>,
	pub local: Transform,
}

impl SocketRef {
	pub fn root() -> Self {
		Self { bone: None, local: Transform::IDENTITY }
	}

	pub fn on(bone: &'static str) -> Self {
		Self { bone: Some(bone), local: Transform::IDENTITY }
	}

	pub fn with_local(mut self, local: Transform) -> Self {
		self.local = local;
		self
	}
}

/// BSN / ECS marker: this entity should be parented under [`SocketRef`].
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct SocketRefRoot(pub SocketRef);

/// Marker: [`SocketRefRoot`] has been parented.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SocketRefApplied;

/// Drop [`SocketRefApplied`] when the identity changes so fulfill re-parents.
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
	members: Query<&FirearmMembers>,
	rigs: Query<(Entity, &BoneMap), With<crate::FirearmRig>>,
	roots: Query<Entity, With<crate::FirearmRoot>>,
) {
	for (entity, SocketRefRoot(socket), mut transform) in &mut pending {
		let Ok(MemberOf(root)) = member_of.get(entity) else {
			continue;
		};
		if !roots.contains(*root) {
			continue;
		}
		let Ok(firearm_members) = members.get(*root) else {
			continue;
		};
		if attach_to_socket(
			&mut commands,
			entity,
			&mut transform,
			socket,
			*root,
			firearm_members,
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
	members: &FirearmMembers,
	rigs: &Query<(Entity, &BoneMap), With<crate::FirearmRig>>,
) -> bool {
	let parent = match socket.bone {
		Some(bone) => match find_member_rig(members, rigs) {
			Some((_, map)) => match map.by_name.get(bone).copied() {
				Some(bone_entity) => bone_entity,
				// Rig is present but this bone has not appeared yet — retry.
				None => return false,
			},
			// No receiver rig: kit parts sit on the firearm root at authored pose.
			None => root,
		},
		None => root,
	};

	let authored_scale = transform.scale;
	let authored_rotation = transform.rotation;
	*transform = socket.local;
	transform.scale *= authored_scale;
	transform.rotation *= authored_rotation;
	commands.entity(entity).try_insert(ChildOf(parent));
	true
}
