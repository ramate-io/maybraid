//! Register live combatants as semantic spotting subjects.

use std::collections::HashSet;

use bevy::prelude::*;
use combat_targeting::{CombatTargeting, TargetSource};
use damage::Downed;
use evasion_intelligence::{
	AssailantContact, AssailantFactor, AssailantSource, EvasionIntelligenceUser, TimedInfluence,
};
use firearms::WeaponFired;
use player::{LocomotionCapsule, Npc, Player};
use spotting_intelligence::{InterestLayers, SpotBounds, SpotSubject, SpottingUser};
use threat_intelligence::{
	AffiliationStrength, Affiliations, ThreatDiscoveryPolicy, ThreatGroupId, ThreatId,
	ThreatIntelligenceUser, ThreatKnowledge, ThreatObservation, ThreatSource, ThreatSubject,
};

use crate::session::Civilian;

const ARENA_COMBATANTS: ThreatGroupId = ThreatGroupId::group(1);
const ARENA_CIVILIANS: ThreatGroupId = ThreatGroupId::group(2);
const SHOT_HEARING_RANGE: f32 = 80.0;
const RECEIVED_FIRE_THREAT: f32 = 6.0;
const RECEIVED_FIRE_HALF_LIFE: f32 = 2.0;

type LiveCombatants<'w, 's> = Query<
	'w,
	's,
	(Entity, &'static LocomotionCapsule, Option<&'static SpotSubject>, Has<Civilian>),
	(Or<(With<Player>, With<Npc>)>, Without<Downed>),
>;

/// Keep one semantic proxy per live body; spotting performs the broadphase.
pub(crate) fn sync_combat_spot_subjects(mut commands: Commands, combatants: LiveCombatants) {
	for (entity, hull, current, is_civilian) in &combatants {
		let layers = if is_civilian { InterestLayers::CIVILIAN } else { InterestLayers::CHARACTER };
		let next = SpotSubject::new(layers, SpotBounds::capsule(hull.radius, hull.half_height()));
		if current != Some(&next) {
			commands.entity(entity).insert(next);
		}
	}
}

type LiveThreatActors<'w, 's> = Query<
	'w,
	's,
	(
		Entity,
		Has<Civilian>,
		Has<SpottingUser>,
		Option<&'static ThreatSubject>,
		Option<&'static Affiliations>,
		Has<ThreatIntelligenceUser>,
		Has<ThreatKnowledge>,
	),
	(Or<(With<Player>, With<Npc>)>, Without<Downed>),
>;

/// Install stable identities and arena affiliations on each live actor.
pub(crate) fn sync_range_threat_actors(
	mut commands: Commands,
	actors: LiveThreatActors,
	downed: Query<Entity, (With<Downed>, With<ThreatSubject>)>,
) {
	for (
		entity,
		is_civilian,
		has_spotting,
		current_subject,
		current_affiliations,
		has_user,
		has_knowledge,
	) in &actors
	{
		let id = ThreatId(entity.to_bits());
		let subject = ThreatSubject::new(id);
		let affiliations = arena_affiliations(id, is_civilian);
		let mut entity_commands = commands.entity(entity);
		if current_subject != Some(&subject) {
			entity_commands.insert(subject);
		}
		if current_affiliations.is_none() {
			entity_commands.insert(affiliations);
		}
		if has_spotting && !has_user {
			entity_commands.insert(ThreatIntelligenceUser::new(ThreatDiscoveryPolicy {
				radius: SHOT_HEARING_RANGE,
				scan_interval_secs: 0.125,
				retained_scan_interval_secs: 2.0,
				retention_secs: 8.0,
				desired_threats: 8,
				candidates_per_scan: 16,
				max_known: 32,
				threat_threshold: 0.2,
			}));
		}
		if has_spotting && !has_knowledge {
			entity_commands.insert(ThreatKnowledge::default());
		}
	}
	for entity in &downed {
		commands
			.entity(entity)
			.remove::<(ThreatSubject, ThreatIntelligenceUser, ThreatKnowledge)>();
	}
}

fn arena_affiliations(id: ThreatId, is_civilian: bool) -> Affiliations {
	let mut affiliations = Affiliations::with_self(id);
	let membership = if is_civilian { ARENA_CIVILIANS } else { ARENA_COMBATANTS };
	affiliations.join(membership, AffiliationStrength::permanent(1.0));
	affiliations.antagonize(ARENA_COMBATANTS, AffiliationStrength::permanent(1.0));
	affiliations
}

/// One-time session inbox seed; normal refreshes use retained local discovery.
pub(crate) fn seed_range_threat_observations(
	time: Res<Time>,
	recipients: Query<
		(Entity, &Affiliations, &ThreatIntelligenceUser),
		Added<ThreatIntelligenceUser>,
	>,
	subjects: Query<(Entity, &ThreatSubject, &Affiliations)>,
	mut observations: MessageWriter<ThreatObservation>,
) {
	let now = time.elapsed_secs();
	for (recipient, recipient_affiliations, user) in &recipients {
		for (entity, subject, subject_affiliations) in &subjects {
			if entity == recipient
				|| recipient_affiliations.threat_weight(subject_affiliations, now)
					< user.policy.threat_threshold
			{
				continue;
			}
			observations.write(ThreatObservation::new(
				recipient,
				subject.id,
				ThreatSource::SESSION,
				1.0,
			));
		}
	}
}

/// Project retained semantic threats into combat-target membership.
pub(crate) fn sync_threat_combat_membership(
	mut users: Query<(&ThreatKnowledge, &mut CombatTargeting), With<Npc>>,
) {
	for (knowledge, mut targeting) in &mut users {
		let active: HashSet<Entity> = knowledge.iter().filter_map(|known| known.entity).collect();
		for subject in active.iter().copied() {
			targeting.include(subject, TargetSource::ENEMYSHIP);
		}
		let retired: Vec<_> = targeting
			.active
			.iter()
			.filter_map(|(subject, target)| {
				(target.has_source(TargetSource::ENEMYSHIP) && !active.contains(subject))
					.then_some(*subject)
			})
			.collect();
		for subject in retired {
			targeting.remove_source(subject, TargetSource::ENEMYSHIP);
		}
	}
}

/// Project retained semantic threats into civilian evasion membership.
pub(crate) fn sync_threat_evasion_membership(
	mut civilians: Query<(&ThreatKnowledge, &mut EvasionIntelligenceUser), With<Civilian>>,
) {
	for (knowledge, mut evasion) in &mut civilians {
		let active: HashSet<Entity> = knowledge.iter().filter_map(|known| known.entity).collect();
		for subject in active.iter().copied() {
			evasion.include(subject, AssailantSource::ENEMYSHIP);
		}
		let retired: Vec<_> = evasion
			.active
			.iter()
			.filter_map(|(subject, assailant)| {
				(assailant.has_source(AssailantSource::ENEMYSHIP) && !active.contains(subject))
					.then_some(*subject)
			})
			.collect();
		for subject in retired {
			evasion.remove_source(subject, AssailantSource::ENEMYSHIP);
		}
	}
}

/// A shot is a typed stimulus: last-known position plus threat, not a sighting.
pub(crate) fn note_civilian_received_fire(
	time: Res<Time>,
	mut fired: MessageReader<WeaponFired>,
	shooters: Query<(&Transform, &ThreatSubject)>,
	mut observations: MessageWriter<ThreatObservation>,
	mut civilians: Query<(Entity, &Transform, &mut EvasionIntelligenceUser), With<Civilian>>,
) {
	let now = time.elapsed_secs();
	for event in fired.read() {
		let Ok((origin, subject)) = shooters.get(event.shooter) else {
			continue;
		};
		let position = origin.translation;
		for (recipient, transform, mut evasion) in &mut civilians {
			if recipient == event.shooter {
				continue;
			}
			let distance = Vec2::new(
				position.x - transform.translation.x,
				position.z - transform.translation.z,
			)
			.length();
			if distance > SHOT_HEARING_RANGE {
				continue;
			}
			evasion.note_stimulus(
				AssailantContact {
					subject: event.shooter,
					position,
					movement_vector: Vec3::ZERO,
					last_known_at: now,
				},
				AssailantSource::RECEIVED_FIRE,
			);
			evasion.add_influence(
				event.shooter,
				TimedInfluence {
					factor: AssailantFactor::Threat,
					magnitude: RECEIVED_FIRE_THREAT,
					applied_at: now,
					half_life: RECEIVED_FIRE_HALF_LIFE,
				},
			);
			observations.write(ThreatObservation::new(
				recipient,
				subject.id,
				ThreatSource::RECEIVED_FIRE,
				1.0,
			));
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use threat_intelligence::{ThreatIntelligencePlugin, ThreatSystems};

	#[test]
	fn live_combatant_gets_one_character_subject() -> Result<(), bevy::ecs::system::RunSystemError>
	{
		let mut world = World::new();
		let entity = world.spawn((Npc, LocomotionCapsule { radius: 0.4, length: 1.0 })).id();
		world.run_system_once(sync_combat_spot_subjects)?;
		let subject = world.get::<SpotSubject>(entity);
		assert!(subject.is_some_and(|subject| {
			subject.layers.intersects(InterestLayers::CHARACTER)
				&& !subject.layers.intersects(InterestLayers::CIVILIAN)
				&& subject.bounds == SpotBounds::capsule(0.4, 0.9)
		}));
		Ok(())
	}

	#[test]
	fn civilian_subject_is_not_a_combat_character() -> Result<(), bevy::ecs::system::RunSystemError>
	{
		let mut world = World::new();
		let entity = world
			.spawn((Npc, Civilian, LocomotionCapsule { radius: 0.4, length: 1.0 }))
			.id();
		world.run_system_once(sync_combat_spot_subjects)?;
		let subject = world.get::<SpotSubject>(entity);
		assert!(subject.is_some_and(|subject| {
			subject.layers.intersects(InterestLayers::CIVILIAN)
				&& !subject.layers.intersects(InterestLayers::CHARACTER)
		}));
		Ok(())
	}

	#[test]
	fn range_actor_setup_installs_self_affiliation_and_threat_memory(
	) -> Result<(), bevy::ecs::system::RunSystemError> {
		let mut world = World::new();
		let observer = world.spawn((Npc, SpottingUser::default())).id();
		world.run_system_once(sync_range_threat_actors)?;
		let subject = world.get::<ThreatSubject>(observer).copied();
		let affiliations = world.get::<Affiliations>(observer);
		assert!(subject.zip(affiliations).is_some_and(|(subject, affiliations)| {
			affiliations.memberships.contains_key(&ThreatGroupId::individual(subject.id))
				&& affiliations.memberships.contains_key(&ARENA_COMBATANTS)
				&& affiliations.known_antagonists.contains_key(&ARENA_COMBATANTS)
		}));
		assert!(world.get::<ThreatIntelligenceUser>(observer).is_some());
		assert!(world.get::<ThreatKnowledge>(observer).is_some());
		Ok(())
	}

	#[test]
	fn threat_membership_drives_combat_and_evasion_adapters(
	) -> Result<(), bevy::ecs::system::RunSystemError> {
		let mut world = World::new();
		let threat = world.spawn_empty().id();
		let threat_id = ThreatId(threat.to_bits());
		let record = threat_intelligence::ThreatRecord {
			id: threat_id,
			entity: threat,
			position: Vec3::X,
			salience: 1.0,
			affiliations: arena_affiliations(threat_id, false),
		};
		let recipient_id = ThreatId(77);
		let recipient_affiliations = arena_affiliations(recipient_id, true);
		let mut knowledge = ThreatKnowledge::default();
		knowledge.observe(
			&record,
			&recipient_affiliations,
			ThreatSource::LOCAL_SCAN,
			1.0,
			0.0,
			0.2,
		);
		let combatant = world.spawn((Npc, knowledge.clone(), CombatTargeting::default())).id();
		let civilian =
			world.spawn((Npc, Civilian, knowledge, EvasionIntelligenceUser::default())).id();
		world.run_system_once(sync_threat_combat_membership)?;
		world.run_system_once(sync_threat_evasion_membership)?;
		let targeting = world.get::<CombatTargeting>(combatant);
		assert!(targeting.is_some_and(|targeting| {
			targeting
				.active_target(threat)
				.is_some_and(|target| target.has_source(TargetSource::ENEMYSHIP))
		}));
		let evasion = world.get::<EvasionIntelligenceUser>(civilian);
		assert!(evasion.is_some_and(|evasion| {
			evasion
				.active_assailant(threat)
				.is_some_and(|assailant| assailant.has_source(AssailantSource::ENEMYSHIP))
		}));
		Ok(())
	}

	#[test]
	fn startup_inbox_seeds_ffa_opponent_but_not_self() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ThreatIntelligencePlugin))
			.add_systems(Update, sync_range_threat_actors.in_set(ThreatSystems::Prepare))
			.add_systems(
				Update,
				seed_range_threat_observations
					.in_set(ThreatSystems::Ingest)
					.before(threat_intelligence::ingest_threat_observations),
			)
			.add_systems(Update, sync_threat_combat_membership.after(ThreatSystems::Discover));
		let observer = app
			.world_mut()
			.spawn((
				Npc,
				SpottingUser::default(),
				CombatTargeting::default(),
				GlobalTransform::default(),
			))
			.id();
		let opponent =
			app.world_mut().spawn((Player, GlobalTransform::from_translation(Vec3::X))).id();
		app.update();
		let observer_id = ThreatId(observer.to_bits());
		let opponent_id = ThreatId(opponent.to_bits());
		assert!(app.world().get::<ThreatKnowledge>(observer).is_some_and(|knowledge| {
			knowledge.get(observer_id).is_none() && knowledge.get(opponent_id).is_some()
		}));
		assert!(app
			.world()
			.get::<CombatTargeting>(observer)
			.and_then(|targeting| targeting.active_target(opponent))
			.is_some_and(|target| target.has_source(TargetSource::ENEMYSHIP)));
	}
}
