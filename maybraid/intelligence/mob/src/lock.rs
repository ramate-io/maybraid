//! After the host arrives, member tethers sit on the destination for a linger,
//! then restore to the host.

use bevy::prelude::*;
use npc_intelligence::NpcIntelligence;
use poi_intelligence::PoiGoal;
use tether_intelligence::TetherIntelligenceUser;

use crate::Mob;
use crate::bind::retarget_member_tether;
use crate::member::MemberOf;

type ArrivalHost<'a> = (
	Entity,
	&'a GlobalTransform,
	&'a PoiGoal,
	Option<&'a MobTetherLock>,
	Option<&'a MobTetherLockMemory>,
);
type ReleasedHost<'a> = (Entity, &'a GlobalTransform, &'a PoiGoal, &'a MobTetherLockMemory);

/// Pack is sitting on `subject` (a POI or another host). Ignore-only member
/// tethers use this instead of the mob host until [`Self::until`].
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct MobTetherLock {
	pub subject: Entity,
	pub generation: u64,
	pub until: f32,
}

/// Last released lock generation so the same arrival does not re-lock.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(crate) struct MobTetherLockMemory {
	pub generation: u64,
}

pub(crate) fn expire_mob_tether_locks(
	time: Res<Time>,
	mut commands: Commands,
	locks: Query<(Entity, &MobTetherLock), With<Mob>>,
) {
	let now = time.elapsed_secs();
	for (host, lock) in &locks {
		if now < lock.until {
			continue;
		}
		commands
			.entity(host)
			.remove::<MobTetherLock>()
			.insert(MobTetherLockMemory { generation: lock.generation });
	}
}

pub(crate) fn lock_mobs_on_poi_arrival(
	time: Res<Time>,
	mut commands: Commands,
	hosts: Query<ArrivalHost<'_>, With<Mob>>,
) {
	let now = time.elapsed_secs();
	for (host, transform, goal, lock, memory) in &hosts {
		if lock.is_some() {
			continue;
		}
		let Some(subject) = goal.poi_entity else {
			continue;
		};
		if goal.linger_secs <= 0.0 {
			continue;
		}
		if !goal.location.contains_xz(transform.translation()) {
			continue;
		}
		if memory.is_some_and(|memory| memory.generation == goal.generation) {
			continue;
		}
		commands.entity(host).insert(MobTetherLock {
			subject,
			generation: goal.generation,
			until: now + goal.linger_secs,
		});
	}
}

/// After a lock is released, leaving the arrival disk allows the next visit.
pub(crate) fn forget_mob_tether_lock_when_leaving(
	mut commands: Commands,
	hosts: Query<ReleasedHost<'_>, (With<Mob>, Without<MobTetherLock>)>,
) {
	for (host, transform, goal, memory) in &hosts {
		if memory.generation != goal.generation {
			continue;
		}
		if goal.location.contains_xz(transform.translation()) {
			continue;
		}
		commands.entity(host).remove::<MobTetherLockMemory>();
	}
}

pub(crate) fn apply_mob_tether_subjects(
	locks: Query<&MobTetherLock, With<Mob>>,
	members: Query<(Entity, &MemberOf)>,
	mut mixers: Query<&mut NpcIntelligence>,
	mut tethers: Query<&mut TetherIntelligenceUser>,
) {
	for (plant, membership) in &members {
		let subject = locks.get(membership.mob).map(|lock| lock.subject).unwrap_or(membership.mob);
		retarget_member_tether(subject, plant, &mut mixers, &mut tethers);
	}
}
