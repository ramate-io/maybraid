//! Write live pose/health back onto the roster and request respawns.

use bevy::prelude::*;
use damage::Health;

use crate::host::{Mob, MobId};
use crate::member::MemberOf;
use crate::roster::{MobMemberNeeded, MobRespawn, MobRoster, RosterMember};

pub(crate) fn write_back_mob_roster(
	time: Res<Time>,
	members: Query<(Entity, &MemberOf, &Transform, Option<&Health>)>,
	mut rosters: Query<(Entity, &mut MobRoster, Option<&MobRespawn>), With<Mob>>,
) {
	let now = time.elapsed_secs();
	let live: Vec<_> = members
		.iter()
		.map(|(entity, membership, transform, health)| {
			(entity, membership.mob, membership.slot, transform.translation, health.copied())
		})
		.collect();

	for (host, mut roster, respawn) in &mut rosters {
		for (slot, member) in roster.iter_mut() {
			if let Some((_, _, _, pose, health)) =
				live.iter().find(|(entity, mob, live_slot, ..)| {
					*mob == host && *live_slot == slot && member.entity == Some(*entity)
				}) {
				member.pose = *pose;
				if let Some(health) = health {
					member.health = *health;
				}
				continue;
			}
			if member.entity.is_none() {
				continue;
			}
			member.entity = None;
			schedule_respawn(member, respawn.copied().unwrap_or_default(), now);
		}
	}
}

pub(crate) fn respawn_mob_members(
	time: Res<Time>,
	mut needed: MessageWriter<MobMemberNeeded>,
	mut rosters: Query<(Entity, &MobId, &mut MobRoster, Option<&MobRespawn>), With<Mob>>,
) {
	let now = time.elapsed_secs();
	for (host, id, mut roster, respawn) in &mut rosters {
		let policy = respawn.copied().unwrap_or_else(MobRespawn::never);
		for (slot, member) in roster.iter_mut() {
			if member.entity.is_some() || member.spawn_requested {
				continue;
			}
			let Some(at) = member.respawn_at else {
				continue;
			};
			if now < at || !policy.allows(member.lives_used.saturating_sub(1)) {
				continue;
			}
			member.spawn_requested = true;
			needed.write(MobMemberNeeded { mob: host, id: *id, slot, pose: member.pose });
		}
	}
}

fn schedule_respawn(member: &mut RosterMember, policy: MobRespawn, now: f32) {
	if member.respawn_at.is_some() || member.spawn_requested {
		return;
	}
	if !policy.allows(member.lives_used) {
		return;
	}
	member.lives_used = member.lives_used.saturating_add(1);
	member.respawn_at = Some(now + policy.delay_secs.max(0.0));
}
