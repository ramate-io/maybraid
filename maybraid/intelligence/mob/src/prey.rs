//! Host brain that travels onto a live subject, locks members, browses, then re-acquires.
//!
//! Hunt-vs-herd and player targeting are two installs of the same user. Callers
//! stamp [`PreySubject`] on the live prey; this crate does not know `PackKind`
//! or `Player`.

use bevy::prelude::*;
use poi_intelligence::{LocalPoi, Poi, PoiGoal, PoiId, PoiKind};

use crate::lock::MobTetherLock;
use crate::Mob;

/// Live subject a [`PreyTargetingIntelligence`] host may chase.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PreySubject;

/// Installed hunt loop. Presence is the grant; runtime memory lives beside it.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PreyTargetingIntelligence {
	pub kind: PoiKind,
	pub poi_id: PoiId,
	/// Arrival disk for the host [`PoiGoal`] (hunt: 12 m).
	pub arrival_radius: f32,
	/// Lock linger after arrival — `UserTargetingIntelligence.duration`.
	pub duration: f32,
	/// Browse after lock expiry before re-acquire.
	pub browse_secs: f32,
	/// Max XZ from [`PreyTargetMemory::home`] to the prey. Farther: do not write
	/// / drop the goal. `UserTargetingIntelligence.off_course_distance`.
	pub off_course_distance: f32,
}

impl PreyTargetingIntelligence {
	pub fn hunt_herd() -> Self {
		Self {
			kind: PoiKind::new("mob-brain/prey"),
			poi_id: PoiId(10_000),
			arrival_radius: 12.0,
			duration: 45.0,
			browse_secs: 8.0,
			off_course_distance: f32::INFINITY,
		}
	}

	pub fn player(duration: f32, off_course_distance: f32) -> Self {
		Self {
			kind: PoiKind::new("world/player"),
			poi_id: PoiId(10_001),
			arrival_radius: 8.0,
			duration,
			browse_secs: 8.0,
			off_course_distance: off_course_distance.max(0.0),
		}
	}

	pub fn in_range(self, home: Vec3, prey: Vec3) -> bool {
		home.xz().distance(prey.xz()) <= self.off_course_distance
	}

	#[allow(clippy::too_many_arguments)]
	pub fn write_goal(
		self,
		commands: &mut Commands,
		host: Entity,
		memory: &PreyTargetMemory,
		prey: Entity,
		prey_at: Vec3,
		now: f32,
		goals: &mut Query<&mut PoiGoal>,
	) {
		let generation = memory.chase_generation.max(1);
		let at = Vec3::new(prey_at.x, prey_at.y, prey_at.z);
		commands.entity(prey).insert((
			Poi::new(self.poi_id, self.kind).with_arrival_radius(self.arrival_radius),
			LocalPoi,
		));
		if let Ok(mut goal) = goals.get_mut(host) {
			if goal.kind != self.kind {
				return;
			}
			goal.generation = generation;
			goal.target = self.poi_id;
			goal.kind = self.kind;
			goal.poi_entity = Some(prey);
			goal.location.point = at;
			goal.location.radius = self.arrival_radius;
			goal.linger_secs = self.duration;
			return;
		}
		commands.entity(host).insert(PoiGoal::new(
			generation,
			self.poi_id,
			Some(prey),
			self.kind,
			at,
			self.arrival_radius,
			now,
			self.duration,
		));
	}
}

/// Browse / generation bookkeeping. `home` is the host pose at install.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PreyTargetMemory {
	pub home: Vec3,
	pub browse_until: f32,
	pub chase_generation: u64,
}

impl PreyTargetMemory {
	pub fn new(home: Vec3) -> Self {
		Self { home, browse_until: 0.0, chase_generation: 1 }
	}

	pub fn browsing(self, now: f32) -> bool {
		self.browse_until > now
	}
}

/// Stamp the user and a home pin. Do not move `home` as the host steps.
pub fn install_prey_targeting(
	commands: &mut Commands,
	host: Entity,
	intelligence: PreyTargetingIntelligence,
	home: Vec3,
) {
	commands.entity(host).insert((intelligence, PreyTargetMemory::new(home)));
}

/// Hunt lock expiry starts a browse window. Waypoint locks during browse
/// must not reset it.
pub(crate) fn start_prey_browse(
	time: Res<Time>,
	mut released: RemovedComponents<MobTetherLock>,
	mut memories: Query<(&PreyTargetingIntelligence, &mut PreyTargetMemory)>,
) {
	let now = time.elapsed_secs();
	for entity in released.read() {
		let Ok((user, mut memory)) = memories.get_mut(entity) else {
			continue;
		};
		if memory.browsing(now) {
			continue;
		}
		memory.browse_until = now + user.browse_secs;
	}
}

/// Write or drop a host prey [`PoiGoal`]. Other kinds are left alone.
pub(crate) fn prey_tracks_subject(
	time: Res<Time>,
	mut commands: Commands,
	subjects: Query<(Entity, &GlobalTransform), With<PreySubject>>,
	mut hosts: Query<
		(Entity, &PreyTargetingIntelligence, Option<&MobTetherLock>, &mut PreyTargetMemory),
		With<Mob>,
	>,
	mut goals: Query<&mut PoiGoal>,
) {
	let now = time.elapsed_secs();
	let prey = subjects
		.iter()
		.next()
		.map(|(entity, transform)| (entity, transform.translation()));
	for (host, user, lock, mut memory) in &mut hosts {
		let Some((prey, prey_at)) = prey else {
			drop_kind_goal(&mut commands, host, user.kind, &goals);
			continue;
		};
		if lock.is_none() && memory.browsing(now) {
			drop_kind_goal(&mut commands, host, user.kind, &goals);
			continue;
		}
		if !user.in_range(memory.home, prey_at) {
			drop_kind_goal(&mut commands, host, user.kind, &goals);
			continue;
		}
		if lock.is_none() && memory.browse_until > 0.0 {
			memory.chase_generation = memory.chase_generation.saturating_add(1).max(1);
			memory.browse_until = 0.0;
		}
		user.write_goal(&mut commands, host, &memory, prey, prey_at, now, &mut goals);
	}
}

fn drop_kind_goal(
	commands: &mut Commands,
	host: Entity,
	kind: PoiKind,
	goals: &Query<&mut PoiGoal>,
) {
	if goals.get(host).is_ok_and(|goal| goal.kind == kind) {
		commands.entity(host).remove::<PoiGoal>();
	}
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
	use anyhow::Result;
	use bevy::ecs::system::RunSystemOnce;
	use npc_intelligence::Personality;

	use super::*;
	use crate::lock::lock_mobs_on_poi_arrival;
	use crate::travel::travel_mobs;
	use crate::{spawn_mob, MobId, MobInstall, MobTravel, RosterMember};

	const CAMP: PoiKind = PoiKind::new("mob/camp");

	fn spawn_chaser(
		world: &mut World,
		home: Vec3,
		user: PreyTargetingIntelligence,
		travel: Option<f32>,
	) -> Entity {
		let mut install =
			MobInstall::new(MobId(1), 12.0, vec![RosterMember::new(Personality::Predator, home)]);
		if let Some(speed) = travel {
			install = install.with_travel(MobTravel::new(speed));
		}
		let host = spawn_mob(&mut world.commands(), Transform::from_translation(home), install);
		install_prey_targeting(&mut world.commands(), host, user, home);
		world.flush();
		world.entity_mut(host).insert(GlobalTransform::from_translation(home));
		host
	}

	fn spawn_subject(world: &mut World, at: Vec3) -> Entity {
		let prey = world
			.spawn((
				Transform::from_translation(at),
				GlobalTransform::from_translation(at),
				PreySubject,
			))
			.id();
		world.flush();
		prey
	}

	fn set_time(world: &mut World, secs: f32) {
		world
			.resource_mut::<Time>()
			.advance_by(std::time::Duration::from_secs_f32(secs));
	}

	fn track(world: &mut World) -> Result<()> {
		world
			.run_system_once(prey_tracks_subject)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.flush();
		Ok(())
	}

	#[test]
	fn hunt_extract_is_behavior_preserving() -> Result<()> {
		let mut world = World::new();
		world.init_resource::<Time>();
		let home = Vec3::ZERO;
		let herd_at = Vec3::new(20.0, 0.0, 0.0);
		let host =
			spawn_chaser(&mut world, home, PreyTargetingIntelligence::hunt_herd(), Some(4.0));
		let herd = spawn_subject(&mut world, herd_at);

		track(&mut world)?;
		let goal =
			world.get::<PoiGoal>(host).ok_or_else(|| anyhow::anyhow!("missing prey goal"))?;
		assert_eq!(goal.kind, PoiKind::new("mob-brain/prey"));
		assert_eq!(goal.poi_entity, Some(herd));
		assert!((goal.linger_secs - 45.0).abs() < 1e-4);
		assert!((goal.location.radius - 12.0).abs() < 1e-4);

		world.entity_mut(host).insert((
			Transform::from_translation(herd_at),
			GlobalTransform::from_translation(herd_at),
		));
		world
			.run_system_once(lock_mobs_on_poi_arrival)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.flush();
		assert_eq!(world.get::<MobTetherLock>(host).map(|lock| lock.subject), Some(herd));

		set_time(&mut world, 45.0);
		world
			.run_system_once(crate::lock::expire_mob_tether_locks)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.flush();
		world
			.run_system_once(start_prey_browse)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		track(&mut world)?;
		assert!(world.get::<PoiGoal>(host).is_none());

		set_time(&mut world, 7.0);
		track(&mut world)?;
		assert!(world.get::<PoiGoal>(host).is_none(), "browse must not re-write early");

		set_time(&mut world, 2.0);
		track(&mut world)?;
		let goal = world
			.get::<PoiGoal>(host)
			.ok_or_else(|| anyhow::anyhow!("missing re-acquire"))?;
		assert_eq!(goal.kind, PoiKind::new("mob-brain/prey"));
		assert!(goal.generation > 1);
		Ok(())
	}

	#[test]
	fn player_install_uses_the_same_write() -> Result<()> {
		let mut world = World::new();
		world.init_resource::<Time>();
		let home = Vec3::ZERO;
		let host = spawn_chaser(
			&mut world,
			home,
			PreyTargetingIntelligence::player(10.0, 40.0),
			Some(8.0),
		);
		spawn_subject(&mut world, Vec3::new(20.0, 0.0, 0.0));

		track(&mut world)?;
		let goal = world
			.get::<PoiGoal>(host)
			.ok_or_else(|| anyhow::anyhow!("missing player goal"))?;
		assert_eq!(goal.kind, PoiKind::new("world/player"));
		assert!((goal.linger_secs - 10.0).abs() < 1e-4);

		set_time(&mut world, 0.5);
		world
			.run_system_once(travel_mobs)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let at = world
			.get::<Transform>(host)
			.ok_or_else(|| anyhow::anyhow!("missing transform"))?
			.translation;
		assert!(at.x > 0.5, "travel_mobs should step the host toward the player, got {at}");
		Ok(())
	}

	#[test]
	fn off_course_does_not_write() -> Result<()> {
		let mut world = World::new();
		world.init_resource::<Time>();
		let home = Vec3::ZERO;
		let host = spawn_chaser(
			&mut world,
			home,
			PreyTargetingIntelligence::player(10.0, 40.0),
			Some(8.0),
		);
		spawn_subject(&mut world, Vec3::new(80.0, 0.0, 0.0));

		track(&mut world)?;
		assert!(world.get::<PoiGoal>(host).is_none());
		assert_eq!(world.get::<Transform>(host).map(|transform| transform.translation), Some(home));
		Ok(())
	}

	#[test]
	fn off_course_drops_a_live_goal() -> Result<()> {
		let mut world = World::new();
		world.init_resource::<Time>();
		let home = Vec3::ZERO;
		let host = spawn_chaser(
			&mut world,
			home,
			PreyTargetingIntelligence::player(10.0, 40.0),
			Some(8.0),
		);
		let prey = spawn_subject(&mut world, Vec3::new(20.0, 0.0, 0.0));

		track(&mut world)?;
		assert!(world.get::<PoiGoal>(host).is_some());
		world
			.entity_mut(host)
			.insert(MobTetherLock { subject: prey, generation: 1, until: 30.0 });
		world.entity_mut(prey).insert((
			Transform::from_xyz(80.0, 0.0, 0.0),
			GlobalTransform::from_xyz(80.0, 0.0, 0.0),
		));

		track(&mut world)?;
		assert!(world.get::<PoiGoal>(host).is_none());
		assert!(
			world.get::<MobTetherLock>(host).is_some(),
			"off-course drops the goal, not the lock"
		);
		Ok(())
	}

	#[test]
	fn browse_does_not_cancel_waypoints() -> Result<()> {
		let mut world = World::new();
		world.init_resource::<Time>();
		let home = Vec3::ZERO;
		let host =
			spawn_chaser(&mut world, home, PreyTargetingIntelligence::player(10.0, 40.0), None);
		spawn_subject(&mut world, Vec3::new(20.0, 0.0, 0.0));
		world.entity_mut(host).insert((
			PoiGoal::new(4, PoiId(3), None, CAMP, Vec3::new(6.0, 0.0, 0.0), 4.0, 0.0, 2.5),
			PreyTargetMemory { home, browse_until: 8.0, chase_generation: 1 },
		));

		track(&mut world)?;
		assert!(world.get::<PoiGoal>(host).is_some_and(|goal| goal.kind == CAMP));
		Ok(())
	}

	#[test]
	fn high_stays_plausible_after_an_in_range_sally() -> Result<()> {
		let mut world = World::new();
		world.init_resource::<Time>();
		let home = Vec3::ZERO;
		let off_course = 40.0;
		let host = spawn_chaser(
			&mut world,
			home,
			PreyTargetingIntelligence::player(10.0, off_course),
			Some(20.0),
		);
		spawn_subject(&mut world, Vec3::new(20.0, 0.0, 0.0));

		track(&mut world)?;
		set_time(&mut world, 2.0);
		world
			.run_system_once(travel_mobs)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let at = world
			.get::<Transform>(host)
			.ok_or_else(|| anyhow::anyhow!("missing transform"))?
			.translation;
		let memory = world
			.get::<PreyTargetMemory>(host)
			.ok_or_else(|| anyhow::anyhow!("missing memory"))?;
		assert!(at.distance(memory.home) <= off_course + 1e-3);
		assert!(at.x > 0.0);
		Ok(())
	}

	#[test]
	fn constructors_match_the_issue_knobs() -> Result<()> {
		let hunt = PreyTargetingIntelligence::hunt_herd();
		assert_eq!(hunt.kind, PoiKind::new("mob-brain/prey"));
		assert_eq!(hunt.poi_id, PoiId(10_000));
		assert!((hunt.arrival_radius - 12.0).abs() < 1e-4);
		assert!((hunt.duration - 45.0).abs() < 1e-4);
		assert!(!hunt.off_course_distance.is_finite());
		assert!(hunt.in_range(Vec3::ZERO, Vec3::new(10_000.0, 0.0, 0.0)));

		let player = PreyTargetingIntelligence::player(-1.0, -4.0);
		assert_eq!(player.kind, PoiKind::new("world/player"));
		assert_eq!(player.poi_id, PoiId(10_001));
		assert!((player.arrival_radius - 8.0).abs() < 1e-4);
		assert!((player.off_course_distance - 0.0).abs() < 1e-4);
		Ok(())
	}
}
