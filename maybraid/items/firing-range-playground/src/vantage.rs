//! Register live combatants as semantic spotting subjects.

use bevy::prelude::*;
use damage::Downed;
use player::{LocomotionCapsule, Npc, Player};
use spotting_intelligence::{InterestLayers, SpotBounds, SpotSubject};

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
}
