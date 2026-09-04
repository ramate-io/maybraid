#![allow(clippy::expect_used)]

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use damage::Health;
use npc_intelligence::{NpcInstall, NpcIntelligence, Personality};
use poi_intelligence::{PoiGoal, PoiId, PoiInterest, PoiInterests, PoiKind};
use tether_intelligence::{Tether, TetherIntelligenceUser, TetherObjective};
use threat_intelligence::{AffiliationStrength, Affiliations, ThreatGroupId, ThreatSubject};

use crate::bind::bind_mob_members;
use crate::lifecycle::write_back_mob_roster;
use crate::lock::{
	apply_mob_tether_subjects, expire_mob_tether_locks, forget_mob_tether_lock_when_leaving,
	lock_mobs_on_poi_arrival,
};
use crate::{
	spawn_mob, MemberOf, Mob, MobAffiliations, MobId, MobInstall, MobIntelligencePlugin,
	MobMemberNeeded, MobRespawn, MobRoster, MobSlot, MobTetherLock, RosterMember,
};

const PACK: ThreatGroupId = ThreatGroupId::group(9);
const PUBLIC: ThreatGroupId = ThreatGroupId::group(1);

fn pack_affiliations() -> MobAffiliations {
	let mut affiliations = Affiliations::default();
	affiliations.join(PACK, AffiliationStrength::permanent(1.0));
	affiliations.antagonize(PUBLIC, AffiliationStrength::permanent(1.0));
	affiliations.mitigate(PACK, AffiliationStrength::permanent(1.0));
	MobAffiliations::new(affiliations)
}

fn interests() -> PoiInterests {
	PoiInterests::new([PoiInterest::new(PoiKind::new("mob/camp"), 1.0)])
}

fn spawn_host(world: &mut World, id: MobId, members: Vec<RosterMember>) -> Entity {
	let mut commands = world.commands();
	let host = spawn_mob(
		&mut commands,
		Transform::from_xyz(3.0, 0.0, 1.0),
		MobInstall::new(id, 12.0, members)
			.with_interests(interests())
			.with_affiliations(pack_affiliations()),
	);
	world.flush();
	host
}

#[test]
fn host_is_a_tether_anchor() {
	let mut world = World::new();
	let host =
		spawn_host(&mut world, MobId(1), vec![RosterMember::new(Personality::Grazer, Vec3::X)]);
	assert!(world.get::<Tether>(host).is_some());
	assert!(world.get::<Mob>(host).is_some_and(|mob| (mob.leash - 12.0).abs() < 1e-4));
}

#[test]
fn bind_by_mob_id_stamps_membership_and_installs() {
	let mut world = World::new();
	let pose = Vec3::new(4.0, 0.9, 2.0);
	let host = spawn_host(
		&mut world,
		MobId(7),
		vec![RosterMember::new(Personality::Grazer, pose).with_armed(false)],
	);
	let plant = world.spawn((Transform::from_translation(pose), MobSlot(0), MobId(7))).id();
	world.run_system_once(bind_mob_members).expect("bind");
	world.flush();

	assert_eq!(world.get::<MemberOf>(plant), Some(&MemberOf { mob: host, slot: 0 }));
	assert_eq!(world.get::<MobRoster>(host).and_then(|roster| roster.get(0)?.entity), Some(plant));
	assert!(world.get::<NpcIntelligence>(plant).is_some());
	assert!(matches!(
		world.get::<TetherIntelligenceUser>(plant).map(|user| user.objective),
		Some(TetherObjective::Tether(subject, _)) if subject == host
	));
	let affiliations = world.get::<threat_intelligence::Affiliations>(plant).expect("affiliations");
	assert!(affiliations.memberships.contains_key(&PACK));
	assert!(world.get::<ThreatSubject>(plant).is_some());
}

#[test]
fn bind_by_ancestor_when_the_plant_has_no_mob_id() {
	let mut world = World::new();
	let host = spawn_host(
		&mut world,
		MobId(2),
		vec![RosterMember::new(Personality::Civilian, Vec3::ZERO).with_armed(false)],
	);
	let root = world.spawn((MobLevelRoot, ChildOf(host))).id();
	let plant = world.spawn((Transform::default(), MobSlot(0), ChildOf(root))).id();
	world.run_system_once(bind_mob_members).expect("bind");
	world.flush();
	assert_eq!(world.get::<MemberOf>(plant).map(|membership| membership.mob), Some(host));
}

#[derive(Component)]
struct MobLevelRoot;

#[test]
fn occupied_slot_does_not_steal_a_live_member() {
	let mut world = World::new();
	let host = spawn_host(
		&mut world,
		MobId(3),
		vec![RosterMember::new(Personality::Grazer, Vec3::ZERO).with_armed(false)],
	);
	let first = world.spawn((Transform::default(), MobSlot(0), MobId(3))).id();
	world.run_system_once(bind_mob_members).expect("bind");
	world.flush();
	let second = world.spawn((Transform::default(), MobSlot(0), MobId(3))).id();
	world.run_system_once(bind_mob_members).expect("bind");
	world.flush();
	assert_eq!(world.get::<MemberOf>(first).map(|membership| membership.mob), Some(host));
	assert!(world.get::<MemberOf>(second).is_none());
}

#[test]
fn writeback_clears_the_pointer_and_schedules_respawn() {
	let mut world = World::new();
	world.init_resource::<Time>();
	let pose = Vec3::new(8.0, 0.9, -2.0);
	let host = spawn_host(
		&mut world,
		MobId(4),
		vec![RosterMember::new(Personality::Grazer, Vec3::ZERO).with_armed(false)],
	);
	world
		.entity_mut(host)
		.insert(MobRespawn { delay_secs: 1.0, max_lives: Some(1) });
	let plant = world.spawn((Transform::from_translation(pose), MobSlot(0), MobId(4))).id();
	world.run_system_once(bind_mob_members).expect("bind");
	world.flush();
	world.entity_mut(plant).insert(Health::from_max(40.0));
	world.run_system_once(write_back_mob_roster).expect("writeback");
	assert!(world.get::<MobRoster>(host).is_some_and(|roster| roster.get(0).is_some_and(
		|member| {
			(member.pose - pose).length() < 1e-4 && (member.health.current - 40.0).abs() < 1e-4
		}
	)));

	world.entity_mut(plant).despawn();
	world.run_system_once(write_back_mob_roster).expect("writeback");
	let member = world.get::<MobRoster>(host).and_then(|roster| roster.get(0)).expect("slot");
	assert!(member.entity.is_none());
	assert!(member.respawn_at.is_some());
	assert_eq!(member.lives_used, 1);
}

#[test]
fn respawn_emits_one_needed_message() {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins).add_plugins(MobIntelligencePlugin);
	let pose = Vec3::new(1.0, 0.9, 0.0);
	let host = spawn_mob(
		&mut app.world_mut().commands(),
		Transform::default(),
		MobInstall::new(MobId(5), 8.0, vec![RosterMember::new(Personality::Grazer, pose)])
			.with_respawn(MobRespawn { delay_secs: 0.0, max_lives: Some(1) }),
	);
	app.world_mut().flush();
	{
		let mut roster = app.world_mut().get_mut::<MobRoster>(host).expect("roster");
		let member = roster.get_mut(0).expect("slot");
		member.entity = Some(Entity::from_bits(99));
	}
	app.world_mut().run_system_once(write_back_mob_roster).expect("writeback");
	app.update();
	let events: Vec<_> =
		app.world_mut().resource_mut::<Messages<MobMemberNeeded>>().drain().collect();
	assert_eq!(events, vec![MobMemberNeeded { mob: host, id: MobId(5), slot: 0, pose }]);
	app.update();
	let again: Vec<_> =
		app.world_mut().resource_mut::<Messages<MobMemberNeeded>>().drain().collect();
	assert!(again.is_empty());
}

#[test]
fn travel_steps_the_host_across_the_ground() {
	let at = crate::travel::step_xz(Vec3::new(3.0, 1.0, 1.0), Vec3::new(10.0, 0.0, 1.0), 5.0);
	assert!((at.x - 8.0).abs() < 1e-3);
	assert!((at.y - 1.0).abs() < 1e-3);
	assert!((at.z - 1.0).abs() < 1e-3);
	let arrived = crate::travel::step_xz(Vec3::X * 9.0, Vec3::X * 10.0, 4.0);
	assert!((arrived.x - 10.0).abs() < 1e-4);
}

#[test]
fn preinstalled_mixer_is_retargeted_to_the_host() {
	let mut world = World::new();
	let host = spawn_host(
		&mut world,
		MobId(8),
		vec![RosterMember::new(Personality::Grazer, Vec3::ZERO).with_armed(false)],
	);
	let decoy = world.spawn_empty().id();
	let plant = world.spawn((Transform::default(), MobSlot(0), MobId(8))).id();
	Personality::Grazer.install(
		&mut world.commands(),
		plant,
		NpcInstall { tether: Some(decoy), armed: false, ..NpcInstall::default() },
	);
	world.flush();
	world.run_system_once(bind_mob_members).expect("bind");
	world.flush();
	assert!(matches!(
		world.get::<TetherIntelligenceUser>(plant).map(|user| user.objective.subject()),
		Some(subject) if subject == host
	));
}

#[test]
fn lock_retargets_member_tethers_to_the_destination() {
	let mut world = World::new();
	world.init_resource::<Time>();
	let host = spawn_host(
		&mut world,
		MobId(10),
		vec![RosterMember::new(Personality::Grazer, Vec3::ZERO).with_armed(false)],
	);
	let dest = world.spawn((Transform::from_xyz(20.0, 0.0, 0.0), Tether)).id();
	let plant = world.spawn((Transform::default(), MobSlot(0), MobId(10))).id();
	world.run_system_once(bind_mob_members).expect("bind");
	world.flush();
	world
		.entity_mut(host)
		.insert(MobTetherLock { subject: dest, generation: 1, until: 8.0 });
	world.run_system_once(apply_mob_tether_subjects).expect("apply");
	assert!(matches!(
		world.get::<TetherIntelligenceUser>(plant).map(|user| user.objective.subject()),
		Some(subject) if subject == dest
	));
}

#[test]
fn expired_lock_restores_the_host_subject() {
	let mut world = World::new();
	world.init_resource::<Time>();
	let host = spawn_host(
		&mut world,
		MobId(11),
		vec![RosterMember::new(Personality::Grazer, Vec3::ZERO).with_armed(false)],
	);
	let dest = world.spawn((Transform::from_xyz(20.0, 0.0, 0.0), Tether)).id();
	let plant = world.spawn((Transform::default(), MobSlot(0), MobId(11))).id();
	world.run_system_once(bind_mob_members).expect("bind");
	world.flush();
	world
		.entity_mut(host)
		.insert(MobTetherLock { subject: dest, generation: 1, until: -1.0 });
	world.run_system_once(expire_mob_tether_locks).expect("expire");
	world.flush();
	world.run_system_once(apply_mob_tether_subjects).expect("apply");
	assert!(world.get::<MobTetherLock>(host).is_none());
	assert!(matches!(
		world.get::<TetherIntelligenceUser>(plant).map(|user| user.objective.subject()),
		Some(subject) if subject == host
	));
}

#[test]
fn arrival_locks_once_until_the_host_leaves() {
	let mut world = World::new();
	world.init_resource::<Time>();
	let at = Vec3::new(6.0, 0.0, 2.0);
	let host = spawn_host(
		&mut world,
		MobId(12),
		vec![RosterMember::new(Personality::Grazer, Vec3::ZERO).with_armed(false)],
	);
	let dest = world.spawn((Transform::from_translation(at), Tether)).id();
	world.entity_mut(host).insert((
		GlobalTransform::from_translation(at),
		PoiGoal::new(3, PoiId(40), Some(dest), PoiKind::new("mob/camp"), at, 4.0, 0.0, 2.5),
	));
	world.run_system_once(lock_mobs_on_poi_arrival).expect("lock");
	world.flush();
	assert!(world
		.get::<MobTetherLock>(host)
		.is_some_and(|lock| lock.subject == dest && lock.generation == 3));

	world
		.entity_mut(host)
		.insert(crate::lock::MobTetherLockMemory { generation: 3 });
	world.entity_mut(host).remove::<MobTetherLock>();
	world.run_system_once(lock_mobs_on_poi_arrival).expect("no-relock");
	world.flush();
	assert!(world.get::<MobTetherLock>(host).is_none());

	world
		.entity_mut(host)
		.insert(GlobalTransform::from_translation(Vec3::new(80.0, 0.0, 0.0)));
	world.run_system_once(forget_mob_tether_lock_when_leaving).expect("leave");
	world.flush();
	assert!(world.get::<crate::lock::MobTetherLockMemory>(host).is_none());
}
