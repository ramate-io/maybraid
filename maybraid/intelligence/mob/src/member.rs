//! Entity-free wish and resolved membership. Not `ChildOf`.

use bevy::prelude::*;
use npc_intelligence::NpcBody;

use crate::MobId;

/// High-plant wish: which roster slot this body should occupy.
///
/// Pair with [`MobId`] when the plant is not under the host. Slot-only wishes
/// bind by walking to an ancestor [`crate::Mob`].
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MobSlot(pub u16);

/// Resolved membership. Written by bind; cleared when the plant despawns.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MemberOf {
	pub mob: Entity,
	pub slot: u16,
}

/// Optional hull override for bind-time [`npc_intelligence::Personality::install`].
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct MobMemberBody(pub NpcBody);

impl From<NpcBody> for MobMemberBody {
	fn from(body: NpcBody) -> Self {
		Self(body)
	}
}

/// Walk `ChildOf` to the nearest ancestor [`crate::Mob`] (not any LOD host).
pub(crate) fn ancestor_mob(
	entity: Entity,
	child_of: &Query<&ChildOf>,
	mobs: &Query<(), With<crate::Mob>>,
) -> Option<Entity> {
	let mut current = child_of.get(entity).ok().map(ChildOf::parent);
	while let Some(entity) = current {
		if mobs.contains(entity) {
			return Some(entity);
		}
		current = child_of.get(entity).ok().map(ChildOf::parent);
	}
	None
}

/// Prefer an explicit [`MobId`] on the plant; otherwise walk to an ancestor mob.
pub(crate) fn resolve_host(
	plant: Entity,
	wish_id: Option<MobId>,
	child_of: &Query<&ChildOf>,
	hosts: &Query<(Entity, &MobId), With<crate::Mob>>,
	mobs: &Query<(), With<crate::Mob>>,
) -> Option<Entity> {
	if let Some(id) = wish_id {
		return hosts.iter().find_map(|(entity, host_id)| (*host_id == id).then_some(entity));
	}
	ancestor_mob(plant, child_of, mobs)
}
