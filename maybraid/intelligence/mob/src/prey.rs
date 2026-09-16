//! Grant policy that preempts a host [`PoiGoal`] onto a live [`PreySubject`].
//!
//! Hunt does not travel. It observes the subject, calls [`begin_poi_goal`], and
//! drops that goal when the grant ends. [`drive_poi_goals`](poi_intelligence::drive_poi_goals)
//! and [`MobTravel`](crate::MobTravel) own the corridor. Journeying is
//! `Without<PoiGoal>` and resumes after the drop; the interrupted hop stays in
//! knowledge and is not marked visited.
//!
//! Opportunity: subject inside `off_course_distance` of the **host**. Alert:
//! first-hand [`MobKnowledge`] bits ∩ `engage`. Callers stamp [`PreySubject`].

use bevy::prelude::*;
use poi_intelligence::{
	begin_poi_goal, mix_seed, KnownPoi, LocalPoi, Poi, PoiGoal, PoiGoalState, PoiId, PoiKind,
	PoiKnowledge, PoiObservation, PoiSource,
};
use threat_intelligence::{ThreatRegistry, ThreatSource};

use crate::share::MobKnowledge;
use crate::Mob;

/// Live subject a [`PreyTargetingIntelligence`] host may chase.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PreySubject;

/// Installed grant policy. Presence is the assignment; [`PreyTargetMemory::grant_until`]
/// is the grant.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PreyTargetingIntelligence {
	pub kind: PoiKind,
	pub poi_id: PoiId,
	/// Arrival disk for the host [`PoiGoal`] (hunt: 12 m).
	pub arrival_radius: f32,
	/// Nominal lock / chase linger. Noisy installs jitter this.
	pub duration: f32,
	/// Cooldown after a grant ends before Opportunity may re-acquire.
	pub browse_secs: f32,
	/// Max XZ from the host to the prey.
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

	pub fn in_range(self, host: Vec3, prey: Vec3) -> bool {
		host.xz().distance(prey.xz()) <= self.off_course_distance
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
}

/// Grant / browse bookkeeping. No course pin; range is host-to-prey.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PreyTargetMemory {
	pub grant_until: f32,
	pub browse_until: f32,
	pub chase_generation: u64,
}

impl PreyTargetMemory {
	pub fn new() -> Self {
		Self { grant_until: 0.0, browse_until: 0.0, chase_generation: 1 }
	}

	pub fn browsing(self, now: f32) -> bool {
		self.browse_until > now
	}

	pub fn granted(self, now: f32) -> bool {
		self.grant_until > now
	}

	fn raise_grant(&mut self, host: Entity, user: PreyTargetingIntelligence, now: f32) {
		if !self.granted(now) {
			self.chase_generation = self.chase_generation.saturating_add(1).max(1);
		}
		self.grant_until = now + user.noisy_duration(host, self.chase_generation);
		self.browse_until = 0.0;
	}
}

impl Default for PreyTargetMemory {
	fn default() -> Self {
		Self::new()
	}
}

/// Stamp the grant policy. Journeying / travel already live on the host.
pub fn install_prey_targeting(
	commands: &mut Commands,
	host: Entity,
	intelligence: PreyTargetingIntelligence,
) {
	commands.entity(host).insert((intelligence, PreyTargetMemory::new()));
}

/// Observe the subject and [`begin_poi_goal`] while granted. After Select so
/// a live hop can be preempted; Drive then copies the goal onto routing.
pub(crate) fn grant_prey_objective(
	time: Res<Time>,
	registry: Res<ThreatRegistry>,
	mut commands: Commands,
	subjects: Query<(Entity, &GlobalTransform), With<PreySubject>>,
	mut hosts: Query<
		(
			Entity,
			&GlobalTransform,
			&PreyTargetingIntelligence,
			Option<&MobKnowledge>,
			Option<&mut PoiKnowledge>,
			Option<&mut PoiGoalState>,
			Option<&PoiGoal>,
			&mut PreyTargetMemory,
		),
		With<Mob>,
	>,
) {
	let now = time.elapsed_secs();
	let prey = subjects
		.iter()
		.next()
		.map(|(entity, transform)| (entity, transform.translation()));
	for (host, host_at, user, board, mut knowledge, mut state, goal, mut memory) in &mut hosts {
		let Some((prey, prey_at)) = prey else {
			memory.grant_until = 0.0;
			drop_kind_goal(&mut commands, host, user.kind, goal);
			continue;
		};
		let on_host = user.in_range(host_at.translation(), prey_at);
		let alert = board.is_some_and(|board| board.alerts(&registry, prey, user.engage));
		refresh_grant(host, *user, &mut memory, on_host, alert, now);

		if memory.browsing(now) || !memory.granted(now) {
			drop_kind_goal(&mut commands, host, user.kind, goal);
			continue;
		}
		if !on_host {
			drop_kind_goal(&mut commands, host, user.kind, goal);
			continue;
		}

		commands.entity(prey).insert((
			Poi::new(user.poi_id, user.kind).with_arrival_radius(user.arrival_radius),
			LocalPoi,
		));
		let known = remember_subject(host, *user, prey, prey_at, now, knowledge.as_deref_mut());
		if goal.is_some_and(|goal| goal.kind == user.kind && goal.target == user.poi_id) {
			continue;
		}
		begin_poi_goal(
			&mut commands,
			host,
			known,
			now,
			memory.grant_until - now,
			state.as_deref_mut(),
			0,
		);
	}
}

fn remember_subject(
	host: Entity,
	user: PreyTargetingIntelligence,
	prey: Entity,
	prey_at: Vec3,
	now: f32,
	knowledge: Option<&mut PoiKnowledge>,
) -> KnownPoi {
	let observation = PoiObservation {
		user: host,
		id: user.poi_id,
		entity: Some(prey),
		kind: user.kind,
		position: prey_at,
		arrival_radius: user.arrival_radius,
		salience: 1.0,
		confidence: 1.0,
		source: PoiSource::EXTERNAL | PoiSource::OBJECTIVE,
	};
	if let Some(knowledge) = knowledge {
		if let Some(known) = knowledge.observe(observation, now) {
			return known;
		}
	}
	KnownPoi {
		id: user.poi_id,
		entity: Some(prey),
		kind: user.kind,
		position: prey_at,
		arrival_radius: user.arrival_radius,
		salience: 1.0,
		confidence: 1.0,
		sources: observation.source,
		first_observed_at: now,
		last_observed_at: now,
	}
}

fn refresh_grant(
	host: Entity,
	user: PreyTargetingIntelligence,
	memory: &mut PreyTargetMemory,
	on_host: bool,
	alert: bool,
	now: f32,
) {
	if alert {
		memory.raise_grant(host, user, now);
	}
	if !on_host {
		if memory.granted(now) {
			memory.grant_until = 0.0;
			memory.browse_until = now + user.browse_secs;
		}
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

fn drop_kind_goal(commands: &mut Commands, host: Entity, kind: PoiKind, goal: Option<&PoiGoal>) {
	if goal.is_some_and(|goal| goal.kind == kind) {
		commands.entity(host).remove::<PoiGoal>();
	}
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
	use anyhow::Result;
	use bevy::ecs::system::RunSystemOnce;
	use npc_intelligence::Personality;
	use poi_intelligence::{drive_poi_goals, PoiVisitState};
	use routing_intelligence::{RoutingIntelligenceUser, RoutingSettings};
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
		install_prey_targeting(&mut world.commands(), host, user);
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

	fn grant(world: &mut World) -> Result<()> {
		world
			.run_system_once(grant_prey_objective)
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

		grant(&mut world)?;
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
		assert_eq!(
			world.get::<crate::lock::MobTetherLock>(host).map(|lock| lock.subject),
			Some(herd)
		);

		set_time(&mut world, 45.0);
		grant(&mut world)?;
		assert!(world.get::<PoiGoal>(host).is_none());

		set_time(&mut world, 7.0);
		grant(&mut world)?;
		assert!(world.get::<PoiGoal>(host).is_none(), "browse must not re-write early");

		set_time(&mut world, 2.0);
		grant(&mut world)?;
		let goal = world
			.get::<PoiGoal>(host)
			.ok_or_else(|| anyhow::anyhow!("missing re-acquire"))?;
		assert_eq!(goal.kind, PoiKind::new("mob-brain/prey"));
		assert!(goal.generation > 1);
		Ok(())
	}

	#[test]
	fn player_install_uses_begin_poi_goal() -> Result<()> {
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

		grant(&mut world)?;
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
	fn grant_drives_routing_to_the_subject() -> Result<()> {
		let mut world = World::new();
		world.init_resource::<Time>();
		let home = Vec3::ZERO;
		let prey_at = Vec3::new(20.0, 0.0, 0.0);
		let host = spawn_chaser(
			&mut world,
			home,
			PreyTargetingIntelligence::player(10.0, 250.0).without_noise(),
			Some(8.0),
		);
		spawn_subject(&mut world, prey_at);
		world.entity_mut(host).insert((
			PoiGoal::new(4, PoiId(3), None, CAMP, Vec3::new(400.0, 0.0, 0.0), 4.0, 0.0, 2.5),
			RoutingIntelligenceUser::new(RoutingSettings::from_segments([40.0])),
		));
		if let Some(mut routing) = world.get_mut::<RoutingIntelligenceUser>(host) {
			routing.set_destination(Vec3::new(400.0, 0.0, 0.0));
		}

		grant(&mut world)?;
		world
			.run_system_once(drive_poi_goals)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let dest = world
			.get::<RoutingIntelligenceUser>(host)
			.and_then(|routing| routing.destination)
			.ok_or_else(|| anyhow::anyhow!("missing routing dest"))?;
		assert!((dest.x - prey_at.x).abs() < 1e-3);
		assert_eq!(
			world.get::<PoiGoal>(host).map(|goal| goal.kind),
			Some(PoiKind::new("world/player"))
		);
		Ok(())
	}

	#[test]
	fn off_host_does_not_write() -> Result<()> {
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

		grant(&mut world)?;
		assert!(world.get::<PoiGoal>(host).is_none());
		assert_eq!(world.get::<Transform>(host).map(|transform| transform.translation), Some(home));
		Ok(())
	}

	#[test]
	fn off_host_drops_a_live_goal() -> Result<()> {
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

		grant(&mut world)?;
		assert!(world.get::<PoiGoal>(host).is_some());
		world.entity_mut(prey).insert((
			Transform::from_xyz(80.0, 0.0, 0.0),
			GlobalTransform::from_xyz(80.0, 0.0, 0.0),
		));

		grant(&mut world)?;
		assert!(world.get::<PoiGoal>(host).is_none());
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
			PreyTargetMemory { grant_until: 0.0, browse_until: 8.0, chase_generation: 1 },
		));

		grant(&mut world)?;
		assert!(world.get::<PoiGoal>(host).is_some_and(|goal| goal.kind == CAMP));
		Ok(())
	}

	#[test]
	fn preempt_keeps_the_hop_in_knowledge() -> Result<()> {
		let mut world = World::new();
		world.init_resource::<Time>();
		let home = Vec3::ZERO;
		let host = spawn_chaser(
			&mut world,
			home,
			PreyTargetingIntelligence::player(10.0, 250.0).without_noise(),
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
				position: Vec3::new(400.0, 0.0, 0.0),
				arrival_radius: 4.0,
				salience: 1.0,
				confidence: 1.0,
				source: PoiSource::LOCAL_SCAN | PoiSource::OBJECTIVE,
			},
			0.0,
		);
		world.entity_mut(host).insert((
			PoiGoal::new(4, camp_id, None, CAMP, Vec3::new(400.0, 0.0, 0.0), 4.0, 0.0, 2.5),
			knowledge,
			PoiVisitState::default(),
		));

		grant(&mut world)?;
		assert!(world
			.get::<PoiGoal>(host)
			.is_some_and(|goal| goal.kind == PoiKind::new("world/player")));
		let knowledge = world
			.get::<PoiKnowledge>(host)
			.ok_or_else(|| anyhow::anyhow!("missing knowledge"))?;
		let known = knowledge.get(camp_id).ok_or_else(|| anyhow::anyhow!("camp forgotten"))?;
		assert!(known.sources.contains(PoiSource::LOCAL_SCAN));
		assert!(world
			.get::<PoiVisitState>(host)
			.is_some_and(|visits| visits.last_visited_at(camp_id).is_none()));
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
			PreyTargetMemory { grant_until: 0.0, browse_until: 8.0, chase_generation: 1 },
		));

		grant(&mut world)?;
		assert!(world
			.get::<PoiGoal>(host)
			.is_some_and(|goal| goal.kind == PoiKind::new("world/player")));
		assert!(world.get::<PreyTargetMemory>(host).is_some_and(|memory| memory.granted(0.0)));
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
