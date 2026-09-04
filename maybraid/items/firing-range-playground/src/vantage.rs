//! Register live combatants as semantic spotting subjects.

use bevy::prelude::*;
use combat_targeting::{CombatTargeting, TargetSource};
use damage::Downed;
use player::{LocomotionCapsule, Npc, Player};
use spotting_intelligence::{InterestLayers, SpotBounds, SpotSubject, SpottingHint, SpottingUser};

const ENEMY_SPOTTING_PRIORITY: i32 = 2;

type LiveCombatants<'w, 's> = Query<
	'w,
	's,
	(Entity, &'static LocomotionCapsule, Option<&'static SpotSubject>),
	(Or<(With<Player>, With<Npc>)>, Without<Downed>),
>;

/// Keep one semantic proxy per live combatant; spotting performs the broadphase.
pub(crate) fn sync_combat_spot_subjects(mut commands: Commands, combatants: LiveCombatants) {
	for (entity, hull, current) in &combatants {
		let next = SpotSubject::new(
			InterestLayers::CHARACTER,
			SpotBounds::capsule(hull.radius, hull.half_height()),
		);
		if current != Some(&next) {
			commands.entity(entity).insert(next);
		}
	}
}

/// Seed every live opponent into semantic target membership and spotting hints.
pub(crate) fn sync_combat_rosters(
	combatants: Query<Entity, (Or<(With<Player>, With<Npc>)>, Without<Downed>)>,
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
				&& subject.bounds == SpotBounds::capsule(0.4, 0.9)
		}));
		Ok(())
	}

	#[test]
	fn combat_roster_seeds_opponents_but_not_self() -> Result<(), bevy::ecs::system::RunSystemError>
	{
		let mut world = World::new();
		let observer = world.spawn((Npc, SpottingUser::default(), CombatTargeting::default())).id();
		let opponent = world.spawn(Npc).id();
		let player = world.spawn(Player).id();

		world.run_system_once(sync_combat_rosters)?;

		let spotting = world.get::<SpottingUser>(observer);
		assert!(spotting.is_some_and(|spotting| {
			!spotting.hints.contains_key(&observer)
				&& spotting.hints.contains_key(&opponent)
				&& spotting.hints.contains_key(&player)
		}));
		let targeting = world.get::<CombatTargeting>(observer);
		assert!(targeting.is_some_and(|targeting| {
			targeting
				.active_target(opponent)
				.is_some_and(|target| target.has_source(TargetSource::ENEMYSHIP))
				&& targeting
					.active_target(player)
					.is_some_and(|target| target.has_source(TargetSource::ENEMYSHIP))
		}));
		Ok(())
	}
}
