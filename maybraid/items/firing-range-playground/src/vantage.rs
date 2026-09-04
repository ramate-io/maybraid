//! Register live combatants as semantic spotting subjects.

use bevy::prelude::*;
use combat_targeting::{CombatTargeting, TargetSource};
use damage::Downed;
use evasion_intelligence::{
	AssailantContact, AssailantFactor, AssailantSource, EvasionIntelligenceUser, TimedInfluence,
};
use firearms::WeaponFired;
use player::{LocomotionCapsule, Npc, Player};
use spotting_intelligence::{InterestLayers, SpotBounds, SpotSubject, SpottingHint, SpottingUser};

use crate::session::Civilian;

const ENEMY_SPOTTING_PRIORITY: i32 = 2;
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

/// Seed every live armed opponent into combat membership and spotting hints.
pub(crate) fn sync_combat_rosters(
	combatants: Query<
		Entity,
		(Or<(With<Player>, With<CombatTargeting>)>, Without<Downed>, Without<Civilian>),
	>,
	mut users: Query<(Entity, &mut SpottingUser, &mut CombatTargeting), With<Npc>>,
) {
	let live: Vec<Entity> = combatants.iter().collect();
	for (user_entity, mut spotting, mut targeting) in &mut users {
		spotting
			.hints
			.retain(|subject, _| *subject != user_entity && live.contains(subject));
		for &subject in &live {
			if subject == user_entity {
				continue;
			}
			spotting.hint(subject, SpottingHint::new(ENEMY_SPOTTING_PRIORITY));
			targeting.include(subject, TargetSource::ENEMYSHIP);
		}

		let retired: Vec<Entity> = targeting
			.active
			.iter()
			.filter_map(|(subject, target)| {
				(target.has_source(TargetSource::ENEMYSHIP)
					&& (*subject == user_entity || !live.contains(subject)))
				.then_some(*subject)
			})
			.collect();
		for subject in retired {
			targeting.remove_source(subject, TargetSource::ENEMYSHIP);
		}
	}
}

/// Seed armed opponents into civilian spotting hints and enemyship membership.
pub(crate) fn sync_evasion_rosters(
	threats: Query<
		Entity,
		(Or<(With<Player>, With<CombatTargeting>)>, Without<Downed>, Without<Civilian>),
	>,
	mut civilians: Query<(Entity, &mut SpottingUser, &mut EvasionIntelligenceUser), With<Civilian>>,
) {
	let live: Vec<Entity> = threats.iter().collect();
	for (user_entity, mut spotting, mut evasion) in &mut civilians {
		spotting
			.hints
			.retain(|subject, _| *subject != user_entity && live.contains(subject));
		for &subject in &live {
			if subject == user_entity {
				continue;
			}
			spotting.hint(subject, SpottingHint::new(ENEMY_SPOTTING_PRIORITY));
			evasion.include(subject, AssailantSource::ENEMYSHIP);
		}

		let retired: Vec<Entity> = evasion
			.active
			.iter()
			.filter_map(|(subject, assailant)| {
				(assailant.has_source(AssailantSource::ENEMYSHIP)
					&& (*subject == user_entity || !live.contains(subject)))
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
	shooters: Query<&Transform>,
	mut civilians: Query<(&Transform, &mut EvasionIntelligenceUser), With<Civilian>>,
) {
	let now = time.elapsed_secs();
	for event in fired.read() {
		let Ok(origin) = shooters.get(event.shooter) else {
			continue;
		};
		let position = origin.translation;
		for (transform, mut evasion) in &mut civilians {
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
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;

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
	fn combat_roster_seeds_opponents_but_not_self() -> Result<(), bevy::ecs::system::RunSystemError>
	{
		let mut world = World::new();
		let observer = world.spawn((Npc, SpottingUser::default(), CombatTargeting::default())).id();
		let opponent = world.spawn((Npc, CombatTargeting::default())).id();
		let civilian = world.spawn((Npc, Civilian)).id();
		let player = world.spawn(Player).id();

		world.run_system_once(sync_combat_rosters)?;

		let spotting = world.get::<SpottingUser>(observer);
		assert!(spotting.is_some_and(|spotting| {
			!spotting.hints.contains_key(&observer)
				&& spotting.hints.contains_key(&opponent)
				&& spotting.hints.contains_key(&player)
				&& !spotting.hints.contains_key(&civilian)
		}));
		let targeting = world.get::<CombatTargeting>(observer);
		assert!(targeting.is_some_and(|targeting| {
			targeting
				.active_target(opponent)
				.is_some_and(|target| target.has_source(TargetSource::ENEMYSHIP))
				&& targeting
					.active_target(player)
					.is_some_and(|target| target.has_source(TargetSource::ENEMYSHIP))
				&& targeting.active_target(civilian).is_none()
		}));
		Ok(())
	}

	#[test]
	fn evasion_roster_seeds_armed_threats_but_not_other_civilians(
	) -> Result<(), bevy::ecs::system::RunSystemError> {
		let mut world = World::new();
		let civilian = world
			.spawn((Npc, Civilian, SpottingUser::default(), EvasionIntelligenceUser::default()))
			.id();
		let other_civilian = world.spawn((Npc, Civilian)).id();
		let combatant = world.spawn((Npc, CombatTargeting::default())).id();
		let player = world.spawn(Player).id();

		world.run_system_once(sync_evasion_rosters)?;

		let spotting = world.get::<SpottingUser>(civilian);
		assert!(spotting.is_some_and(|spotting| {
			spotting.hints.contains_key(&combatant)
				&& spotting.hints.contains_key(&player)
				&& !spotting.hints.contains_key(&other_civilian)
		}));
		let evasion = world.get::<EvasionIntelligenceUser>(civilian);
		assert!(evasion.is_some_and(|evasion| {
			evasion
				.active_assailant(combatant)
				.is_some_and(|assailant| assailant.has_source(AssailantSource::ENEMYSHIP))
				&& evasion.active_assailant(other_civilian).is_none()
		}));
		Ok(())
	}
}
