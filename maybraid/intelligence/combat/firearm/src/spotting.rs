//! Adapt generic visual contacts into combat target memory and factors.

use bevy::prelude::*;
use combat_targeting::{CombatContact, CombatTargeting, TargetFactors, TargetSource};
use spotting_intelligence::SpottingUser;

use crate::combat::FirearmIntelligence;

/// Refresh combat memory from successful generic spotting contacts.
pub(crate) fn sync_spotted_combat_targets(
	mut combatants: Query<(&Transform, &SpottingUser, &FirearmIntelligence, &mut CombatTargeting)>,
) {
	for (transform, spotting, firearm, mut targeting) in &mut combatants {
		targeting.memory_secs = firearm.settings.target_spotting_memory.max(0.0);
		targeting.algebra.continuity = firearm.settings.focus.clamp(0.0, 1.0) * 8.0;
		let forgotten: Vec<Entity> = targeting
			.memory
			.keys()
			.filter(|entity| !spotting.contacts.contains_key(entity))
			.copied()
			.collect();
		for entity in forgotten {
			targeting.memory.remove(&entity);
			targeting.remove_source(entity, TargetSource::SPOTTING);
		}

		if !targeting.enabled {
			targeting.clear_source(TargetSource::SPOTTING);
			continue;
		}

		for contact in spotting.contacts.values() {
			targeting.upsert_contact(CombatContact {
				subject: contact.subject,
				position: contact.position,
				movement_vector: contact.velocity,
				visible_point: contact.visible_point,
				visible_head: contact.visible_head,
				last_spotted_at: contact.last_success_at,
			});
			let delta = contact.position - transform.translation;
			let distance = Vec2::new(delta.x, delta.z).length();
			let previous = targeting
				.active_target(contact.subject)
				.map_or(TargetFactors::default(), |target| target.factors);
			targeting.set_factors(
				contact.subject,
				TargetFactors { hostility: 1.0, bias: -distance, ..previous },
			);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use spotting_intelligence::{InterestLayers, SpotDirective, SpottedContact, SpottingSettings};

	#[test]
	fn spotted_contacts_enter_weighted_combat_memory(
	) -> Result<(), bevy::ecs::system::RunSystemError> {
		let target = Entity::from_bits(7);
		let mut world = World::new();
		let mut spotting =
			SpottingUser::new(Vec3::Y, [SpotDirective::new(InterestLayers::CHARACTER, 20.0)])
				.with_settings(SpottingSettings::new(4, 4, 2.5));
		spotting.contacts.insert(
			target,
			SpottedContact::new(target, Vec3::X * 4.0, Vec3::ZERO, Vec3::X * 4.0, None, 1.0, 0.1),
		);
		world.spawn((
			Transform::default(),
			spotting,
			FirearmIntelligence::new(),
			CombatTargeting::default(),
		));

		world.run_system_once(sync_spotted_combat_targets)?;
		let targeting = world.query::<&CombatTargeting>().single(&world)?;
		assert_eq!(targeting.contact(target).map(|contact| contact.position), Some(Vec3::X * 4.0));
		assert_eq!(targeting.active_target(target).map(|target| target.factors.bias), Some(-4.0));
		world.query::<&mut SpottingUser>().single_mut(&mut world)?.contacts.clear();
		world.run_system_once(sync_spotted_combat_targets)?;
		let targeting = world.query::<&CombatTargeting>().single(&world)?;
		assert!(targeting.contact(target).is_none());
		assert!(targeting.active_target(target).is_none());
		Ok(())
	}

	#[test]
	fn disabled_targeting_does_not_admit_spotted_contacts(
	) -> Result<(), bevy::ecs::system::RunSystemError> {
		let target = Entity::from_bits(7);
		let mut world = World::new();
		let mut spotting =
			SpottingUser::new(Vec3::Y, [SpotDirective::new(InterestLayers::CHARACTER, 20.0)])
				.with_settings(SpottingSettings::new(4, 4, 2.5));
		spotting.contacts.insert(
			target,
			SpottedContact::new(target, Vec3::X * 4.0, Vec3::ZERO, Vec3::X * 4.0, None, 1.0, 0.1),
		);
		let mut targeting = CombatTargeting::default();
		targeting.enabled = false;
		world.spawn((Transform::default(), spotting, FirearmIntelligence::new(), targeting));
		world.run_system_once(sync_spotted_combat_targets)?;
		let targeting = world.query::<&CombatTargeting>().single(&world)?;
		assert!(targeting.contact(target).is_none());
		assert!(targeting.active_target(target).is_none());
		Ok(())
	}
}
