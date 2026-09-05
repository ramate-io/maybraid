//! Write live pose/health back onto the roster and request respawns.

use bevy::prelude::*;
use damage::{DespawnAfter, Downed, Health};

use crate::host::{Mob, MobId};
use crate::member::MemberOf;
use crate::roster::{MobMemberNeeded, MobRespawn, MobRespawnAt, MobRoster, RosterMember};

pub(crate) fn policy(respawn: Option<&MobRespawn>) -> MobRespawn {
	respawn.copied().unwrap_or_else(MobRespawn::never)
}

/// Death starts the replacement clock and queues the corpse. Cull does not.
pub(crate) fn queue_downed_member_deaths(
	time: Res<Time>,
	mut commands: Commands,
	downed: Query<
		(Entity, &MemberOf, &Transform, &GlobalTransform, Has<ChildOf>, Option<&Health>),
		Added<Downed>,
	>,
	mut rosters: Query<(&mut MobRoster, Option<&MobRespawn>), With<Mob>>,
) {
	let now = time.elapsed_secs();
	for (entity, membership, transform, global, parented, health) in &downed {
		let Ok((mut roster, respawn)) = rosters.get_mut(membership.mob) else {
			continue;
		};
		let policy = policy(respawn);
		let Some(member) = roster.get_mut(membership.slot) else {
			continue;
		};
		if member.entity != Some(entity) {
			continue;
		}
		member.pose = member_translation(transform, global, parented);
		if let Some(health) = health {
			member.health = *health;
		}
		schedule_respawn(member, policy, now);
		commands.entity(entity).try_insert(DespawnAfter::seconds(policy.corpse_secs));
	}
}

pub(crate) fn write_back_mob_roster(
	members: Query<(
		Entity,
		&MemberOf,
		&Transform,
		&GlobalTransform,
		Has<ChildOf>,
		Option<&Health>,
	)>,
	mut rosters: Query<(Entity, &mut MobRoster), With<Mob>>,
) {
	let live: Vec<_> = members
		.iter()
		.map(|(entity, membership, transform, global, parented, health)| {
			(
				entity,
				membership.mob,
				membership.slot,
				member_translation(transform, global, parented),
				health.copied(),
			)
		})
		.collect();

	for (host, mut roster) in &mut rosters {
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
		}
	}
}

fn member_translation(transform: &Transform, global: &GlobalTransform, parented: bool) -> Vec3 {
	if parented {
		global.translation()
	} else {
		transform.translation
	}
}

pub(crate) fn respawn_mob_members(
	time: Res<Time>,
	mut needed: MessageWriter<MobMemberNeeded>,
	mut rosters: Query<
		(Entity, &MobId, &Transform, &mut MobRoster, Option<&MobRespawn>),
		With<Mob>,
	>,
) {
	let now = time.elapsed_secs();
	for (host, id, transform, mut roster, respawn) in &mut rosters {
		let policy = policy(respawn);
		for (slot, member) in roster.iter_mut() {
			if member.entity.is_some() || member.spawn_requested {
				continue;
			}
			let Some(at) = member.respawn_at else {
				continue;
			};
			if now < at {
				continue;
			}
			member.spawn_requested = true;
			member.health.current = member.health.max;
			needed.write(MobMemberNeeded {
				mob: host,
				id: *id,
				slot,
				pose: spawn_pose(policy.at, transform.translation, member.pose),
			});
		}
	}
}

pub(crate) fn spawn_pose(at: MobRespawnAt, host: Vec3, last: Vec3) -> Vec3 {
	match at {
		MobRespawnAt::Host => {
			let y = if last.y.abs() > 1e-3 { last.y } else { host.y };
			Vec3::new(host.x, y, host.z)
		}
		MobRespawnAt::LastPose => last,
	}
}

fn schedule_respawn(member: &mut RosterMember, policy: MobRespawn, now: f32) {
	if member.respawn_at.is_some() || member.spawn_requested {
		return;
	}
	if !policy.allows(member.replacements_used) {
		return;
	}
	member.replacements_used = member.replacements_used.saturating_add(1);
	member.respawn_at = Some(now + policy.delay_secs.max(0.0));
}
