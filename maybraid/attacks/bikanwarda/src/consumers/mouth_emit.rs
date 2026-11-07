//! Live Bray mouth pose for gameplay emission sockets ([#1141](https://github.com/ramate-io/maybraid/pull/1141)).
//!
//! Samples [`character_motion::sample_bray_mouth_emit`] from descendant [`BoneMap`]s and
//! drives [`crate::BikanwardaMouthSocket`] children so `BeginAttack::emission_socket`
//! tracks the animated nose during Bray.

use attacks::{AttackInstance, AttackSystems};
use bevy::prelude::*;
use character_motion::{sample_bray_mouth_emit, BoneMap};

use crate::lanes::BikanwardaPresentationSystems;
use crate::{BikanwardaMouthSocket, BRAY};

pub fn plugin(app: &mut App) {
	app.add_systems(
		Update,
		sync_bray_mouth_sockets
			.in_set(BikanwardaPresentationSystems::Adapt)
			.after(AttackSystems::Advance),
	);
}

fn is_descendant_of(entity: Entity, ancestor: Entity, parents: &Query<&ChildOf>) -> bool {
	let mut current = Some(entity);
	while let Some(entity) = current {
		if entity == ancestor {
			return true;
		}
		current = parents.get(entity).ok().map(|child| child.parent());
	}
	false
}

fn sync_bray_mouth_sockets(
	instances: Query<&AttackInstance>,
	parents: Query<&ChildOf>,
	bone_maps: Query<(Entity, &BoneMap)>,
	globals: Query<&GlobalTransform>,
	owners: Query<&GlobalTransform>,
	mut sockets: Query<(&ChildOf, &mut Transform), With<BikanwardaMouthSocket>>,
) {
	for (parent, mut socket_transform) in &mut sockets {
		let owner = parent.parent();
		let live = instances.iter().any(|instance| {
			instance.owner == owner && instance.definition == BRAY
		});
		if !live {
			continue;
		}
		let maps: Vec<&BoneMap> = bone_maps
			.iter()
			.filter(|(entity, _)| is_descendant_of(*entity, owner, &parents))
			.map(|(_, map)| map)
			.collect();
		if maps.is_empty() {
			continue;
		}
		let Some(emit) = sample_bray_mouth_emit(&maps, &globals) else {
			continue;
		};
		let Ok(owner_global) = owners.get(owner) else {
			continue;
		};
		let local = owner_global.affine().inverse().transform_point3(emit.origin);
		let forward = emit.forward;
		let rotation = if forward.length_squared() > 1e-8 {
			Quat::from_rotation_arc(Vec3::Z, forward.normalize())
		} else {
			Quat::IDENTITY
		};
		*socket_transform = Transform::from_translation(local).with_rotation(rotation);
	}
}
