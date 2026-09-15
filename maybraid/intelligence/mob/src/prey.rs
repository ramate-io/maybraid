//! Host brain that travels onto a live subject, locks members, browses, then re-acquires.
//!
//! Plants write first-hand findings up to [`MobKnowledge`]. This user turns
//! Opportunity (subject on the POI course) and Alert (`engage` ∩ board sources)
//! into one grant clock, then writes a kind-scoped [`PoiGoal`]. Callers stamp
//! [`PreySubject`]; this crate does not know `PackKind` or `Player`.

use bevy::prelude::*;
use poi_intelligence::{
	mix_seed, LocalPoi, Poi, PoiGoal, PoiGoalState, PoiGoalStatus, PoiId, PoiKind, PoiKnowledge,
	PoiSource,
};
use threat_intelligence::{ThreatRegistry, ThreatSource};

use crate::lock::MobTetherLock;
use crate::share::MobKnowledge;
use crate::Mob;

/// Live subject a [`PreyTargetingIntelligence`] host may chase.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PreySubject;

/// Installed hunt loop. Presence is the assignment; [`PreyTargetMemory::grant_until`]
/// is the grant.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PreyTargetingIntelligence {
	pub kind: PoiKind,
	pub poi_id: PoiId,
	/// Arrival disk for the host [`PoiGoal`] (hunt: 12 m).
	pub arrival_radius: f32,
	/// Nominal lock / chase linger. Noisy installs jitter this.
	pub duration: f32,
	/// Browse after lock or timeout before Opportunity may re-acquire.
	pub browse_secs: f32,
	/// Max XZ from [`PreyTargetMemory::course`] to the prey.
	pub off_course_distance: f32,
	/// First-hand board bits that raise Alert. Empty: Opportunity only.
	pub engage: ThreatSource,
	/// Jitter [`Self::duration`] so packs do not snap back together.
	pub noisy: bool,
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
			engage: ThreatSource::default(),
			noisy: false,
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
			engage: ThreatSource::RECEIVED_DAMAGE | ThreatSource::RECEIVED_FIRE,
			noisy: true,
		}
	}

	pub fn in_range(self, course: Vec3, prey: Vec3) -> bool {
		course.xz().distance(prey.xz()) <= self.off_course_distance
	}

	#[cfg(test)]
	fn without_noise(mut self) -> Self {
		self.noisy = false;
		self
	}

	pub fn noisy_duration(self, host: Entity, generation: u64) -> f32 {
		if !self.noisy {
			return self.duration.max(0.0);
		}
		let mixed = mix_seed(host.to_bits() ^ generation.rotate_left(17));
		let unit = ((mixed >> 40) as f32) / ((1_u32 << 24) as f32);
		self.duration.max(0.0) * (0.7 + 0.6 * unit)
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
		let linger = memory.remaining(now);
		let at = Vec3::new(prey_at.x, prey_at.y, prey_at.z);
		commands.entity(prey).insert((
			Poi::new(self.poi_id, self.kind).with_arrival_radius(self.arrival_radius),
			LocalPoi,
		));
		commands.entity(host).insert(PoiGoalState {
			generation,
			target: self.poi_id,
			status: PoiGoalStatus::Active,
		});
		if let Ok(mut goal) = goals.get_mut(host) {
			goal.generation = generation;
			goal.target = self.poi_id;
			goal.kind = self.kind;
			goal.poi_entity = Some(prey);
			goal.location.point = at;
			goal.location.radius = self.arrival_radius;
			goal.linger_secs = linger;
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
			linger,
		));
	}
}

/// Browse / grant bookkeeping. `course` is the last non-prey hop, or install pose.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PreyTargetMemory {
	pub course: Vec3,
	pub grant_until: f32,
	pub browse_until: f32,
	pub chase_generation: u64,
}

impl PreyTargetMemory {
	pub fn new(course: Vec3) -> Self {
		Self { course, grant_until: 0.0, browse_until: 0.0, chase_generation: 1 }
	}

	pub fn browsing(self, now: f32) -> bool {
		self.browse_until > now
	}

	pub fn granted(self, now: f32) -> bool {
		self.grant_until > now
	}

	pub fn remaining(self, now: f32) -> f32 {
		(self.grant_until - now).max(0.0)
	}

	fn raise_grant(&mut self, host: Entity, user: PreyTargetingIntelligence, now: f32) {
		if !self.granted(now) {
			self.chase_generation = self.chase_generation.saturating_add(1).max(1);
		}
		self.grant_until = now + user.noisy_duration(host, self.chase_generation);
		self.browse_until = 0.0;
	}
}

/// Stamp the user and a course pin (install pose until a hop is seen).
pub fn install_prey_targeting(
	commands: &mut Commands,
	host: Entity,
	intelligence: PreyTargetingIntelligence,
	course: Vec3,
) {
	commands.entity(host).insert((intelligence, PreyTargetMemory::new(course)));
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
		memory.grant_until = 0.0;
		memory.browse_until = now + user.browse_secs;
	}
}

/// Write or drop a host prey [`PoiGoal`]. Grant-live writes may steal a hop;
/// the interrupted POI stays in [`PoiKnowledge`] and is not marked visited.
pub(crate) fn prey_tracks_subject(
	time: Res<Time>,
	registry: Res<ThreatRegistry>,
	mut commands: Commands,
	subjects: Query<(Entity, &GlobalTransform), With<PreySubject>>,
	mut hosts: Query<
		(
			Entity,
			&PreyTargetingIntelligence,
			Option<&MobTetherLock>,
			Option<&MobKnowledge>,
			Option<&mut PoiKnowledge>,
			&mut PreyTargetMemory,
		),
		With<Mob>,
	>,
	mut goals: Query<&mut PoiGoal>,
) {
	let now = time.elapsed_secs();
	let prey = subjects
		.iter()
		.next()
		.map(|(entity, transform)| (entity, transform.translation()));
	for (host, user, lock, board, mut knowledge, mut memory) in &mut hosts {
		remember_course(&mut memory, host, user.kind, &goals);
		let Some((prey, prey_at)) = prey else {
			memory.grant_until = 0.0;
			drop_kind_goal(&mut commands, host, user.kind, &goals);
			continue;
		};
		let on_course = user.in_range(memory.course, prey_at);
		let alert = board.is_some_and(|board| board.alerts(&registry, prey, user.engage));
		refresh_grant(host, *user, &mut memory, on_course, alert, now);

		if lock.is_none() && (memory.browsing(now) || !memory.granted(now)) {
			drop_kind_goal(&mut commands, host, user.kind, &goals);
			continue;
		}
		if !on_course {
			drop_kind_goal(&mut commands, host, user.kind, &goals);
			continue;
		}
		interrupt_course(host, user.kind, knowledge.as_deref_mut(), &goals);
		user.write_goal(&mut commands, host, &memory, prey, prey_at, now, &mut goals);
	}
}

fn remember_course(
	memory: &mut PreyTargetMemory,
	host: Entity,
	kind: PoiKind,
	goals: &Query<&mut PoiGoal>,
) {
	if let Ok(goal) = goals.get(host) {
		if goal.kind != kind {
			memory.course = goal.location.point;
		}
	}
}

fn refresh_grant(
	host: Entity,
	user: PreyTargetingIntelligence,
	memory: &mut PreyTargetMemory,
	on_course: bool,
	alert: bool,
	now: f32,
) {
	if alert {
		memory.raise_grant(host, user, now);
	}
	if !on_course {
		memory.grant_until = 0.0;
		return;
	}
	if memory.grant_until > 0.0 && now >= memory.grant_until && !memory.browsing(now) {
		memory.grant_until = 0.0;
		memory.browse_until = now + user.browse_secs;
		return;
	}
	if !memory.granted(now) && !memory.browsing(now) {
		memory.raise_grant(host, user, now);
	}
}

fn interrupt_course(
	host: Entity,
	kind: PoiKind,
	knowledge: Option<&mut PoiKnowledge>,
	goals: &Query<&mut PoiGoal>,
) {
	let Ok(goal) = goals.get(host) else {
		return;
	};
	if goal.kind == kind {
		return;
	}
	if let Some(knowledge) = knowledge {
		knowledge.remove_source(goal.target, PoiSource::OBJECTIVE);
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
	use poi_intelligence::{PoiObservation, PoiVisitState};
	use threat_intelligence::{Affiliations, ThreatId, ThreatSubject};

	use super::*;
	use crate::lock::lock_mobs_on_poi_arrival;
	use crate::travel::travel_mobs;
	use crate::{spawn_mob, MobId, MobInstall, MobKnowledge, MobTravel, RosterMember};

	const CAMP: PoiKind = PoiKind::new("mob/camp");

	fn spawn_chaser(
		world: &mut World,
		home: Vec3,
		user: PreyTargetingIntelligence,
		travel: Option<f32>,
	) -> Entity {
		world.init_resource::<ThreatRegistry>();
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
			PreyTargetingIntelligence::player(10.0, 40.0).without_noise(),
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
			PreyTargetingIntelligence::player(10.0, 40.0).without_noise(),
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
			PreyTargetingIntelligence::player(10.0, 40.0).without_noise(),
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
		let host = spawn_chaser(
			&mut world,
			home,
			PreyTargetingIntelligence::player(10.0, 40.0).without_noise(),
			None,
		);
		spawn_subject(&mut world, Vec3::new(20.0, 0.0, 0.0));
		world.entity_mut(host).insert((
			PoiGoal::new(4, PoiId(3), None, CAMP, Vec3::new(6.0, 0.0, 0.0), 4.0, 0.0, 2.5),
			PreyTargetMemory {
				course: home,
				grant_until: 0.0,
				browse_until: 8.0,
				chase_generation: 1,
			},
		));

		track(&mut world)?;
		assert!(world.get::<PoiGoal>(host).is_some_and(|goal| goal.kind == CAMP));
		Ok(())
	}

	#[test]
	fn steal_clears_objective_but_keeps_the_hop_in_knowledge() -> Result<()> {
		let mut world = World::new();
		world.init_resource::<Time>();
		let home = Vec3::ZERO;
		let host = spawn_chaser(
			&mut world,
			home,
			PreyTargetingIntelligence::player(10.0, 40.0).without_noise(),
			Some(8.0),
		);
		spawn_subject(&mut world, Vec3::new(20.0, 0.0, 0.0));
		let camp_id = PoiId(3);
		let mut knowledge = PoiKnowledge::default();
		knowledge.observe(
			PoiObservation {
				user: host,
				id: camp_id,
				entity: None,
				kind: CAMP,
				position: Vec3::new(6.0, 0.0, 0.0),
				arrival_radius: 4.0,
				salience: 1.0,
				confidence: 1.0,
				source: PoiSource::LOCAL_SCAN | PoiSource::OBJECTIVE,
			},
			0.0,
		);
		world.entity_mut(host).insert((
			PoiGoal::new(4, camp_id, None, CAMP, Vec3::new(6.0, 0.0, 0.0), 4.0, 0.0, 2.5),
			knowledge,
			PoiVisitState::default(),
		));

		track(&mut world)?;
		assert!(world
			.get::<PoiGoal>(host)
			.is_some_and(|goal| goal.kind == PoiKind::new("world/player")));
		let knowledge = world
			.get::<PoiKnowledge>(host)
			.ok_or_else(|| anyhow::anyhow!("missing knowledge"))?;
		let known = knowledge.get(camp_id).ok_or_else(|| anyhow::anyhow!("camp forgotten"))?;
		assert!(!known.sources.contains(PoiSource::OBJECTIVE));
		assert!(known.sources.contains(PoiSource::LOCAL_SCAN));
		assert!(world
			.get::<PoiVisitState>(host)
			.is_some_and(|visits| visits.last_visited_at(camp_id).is_none()));
		assert_eq!(
			world.get::<PreyTargetMemory>(host).map(|memory| memory.course),
			Some(Vec3::new(6.0, 0.0, 0.0))
		);
		Ok(())
	}

	#[test]
	fn alert_raises_a_grant_from_board_damage() -> Result<()> {
		let mut world = World::new();
		world.init_resource::<Time>();
		let home = Vec3::ZERO;
		let host = spawn_chaser(
			&mut world,
			home,
			PreyTargetingIntelligence::player(10.0, 40.0).without_noise(),
			None,
		);
		let prey = spawn_subject(&mut world, Vec3::new(20.0, 0.0, 0.0));
		let id = ThreatId(9);
		world.entity_mut(prey).insert(ThreatSubject::new(id));
		let affiliations = Affiliations::with_self(id);
		world.resource_mut::<ThreatRegistry>().upsert(
			prey,
			ThreatSubject::new(id),
			&affiliations,
			Vec3::new(20.0, 0.0, 0.0),
		)?;
		let mut board = MobKnowledge::default();
		board.adopt_finding(id, ThreatSource::RECEIVED_DAMAGE);
		world.entity_mut(host).insert((
			board,
			PoiGoal::new(4, PoiId(3), None, CAMP, Vec3::new(6.0, 0.0, 0.0), 4.0, 0.0, 2.5),
			PreyTargetMemory {
				course: home,
				grant_until: 0.0,
				browse_until: 8.0,
				chase_generation: 1,
			},
		));

		track(&mut world)?;
		assert!(world
			.get::<PoiGoal>(host)
			.is_some_and(|goal| goal.kind == PoiKind::new("world/player")));
		assert!(world.get::<PreyTargetMemory>(host).is_some_and(|memory| memory.granted(0.0)));
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
			PreyTargetingIntelligence::player(10.0, off_course).without_noise(),
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
		assert!(at.distance(memory.course) <= off_course + 1e-3);
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
		assert!(hunt.engage.is_empty());
		assert!(!hunt.noisy);
		assert!(hunt.in_range(Vec3::ZERO, Vec3::new(10_000.0, 0.0, 0.0)));

		let player = PreyTargetingIntelligence::player(-1.0, -4.0);
		assert_eq!(player.kind, PoiKind::new("world/player"));
		assert_eq!(player.poi_id, PoiId(10_001));
		assert!((player.arrival_radius - 8.0).abs() < 1e-4);
		assert!((player.off_course_distance - 0.0).abs() < 1e-4);
		assert!(player.engage.intersects(ThreatSource::RECEIVED_DAMAGE));
		assert!(player.noisy);
		let quiet = player.without_noise();
		assert!((quiet.noisy_duration(Entity::PLACEHOLDER, 1) - 0.0).abs() < 1e-4);
		Ok(())
	}
}
