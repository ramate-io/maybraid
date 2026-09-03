//! Perception candidates: every other live combatant.

use bevy::prelude::*;
use firearm_intelligence::{CombatTarget, FirearmSpotting, TargetCapsule};
use player::{LocomotionCapsule, Npc, Player};

type CombatantEntities<'w, 's> =
	Query<'w, 's, (Entity, &'static LocomotionCapsule), Or<(With<Player>, With<Npc>)>>;

pub(crate) fn assign_combat_targets(
	combatants: CombatantEntities,
	mut spotters: Query<(Entity, &mut FirearmSpotting)>,
) {
	let everyone: Vec<(Entity, TargetCapsule)> = combatants
		.iter()
		.map(|(entity, hull)| (entity, TargetCapsule::new(hull.radius, hull.half_height())))
		.collect();
	for (spotter, mut spotting) in &mut spotters {
		let next: Vec<_> = everyone
			.iter()
			.copied()
			.filter(|(entity, _)| *entity != spotter)
			.map(|(entity, capsule)| CombatTarget::new(entity, capsule))
			.collect();
		if spotting.candidates != next {
			spotting.candidates = next;
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_spotter_does_not_list_itself() {
		let a = Entity::from_bits(1);
		let b = Entity::from_bits(2);
		let capsule = TargetCapsule::new(0.4, 0.9);
		let everyone = [a, b];
		let next: Vec<_> = everyone
			.iter()
			.copied()
			.filter(|entity| *entity != a)
			.map(|entity| CombatTarget::new(entity, capsule))
			.collect();
		assert_eq!(next.len(), 1);
		assert_eq!(next[0].entity, b);
	}
}
