//! Adapt firearm-specific settings to the generic spotting→targeting bridge.

use bevy::prelude::*;
use combat_targeting::SpottingTargetingSettings;

use crate::combat::FirearmIntelligence;

/// Updates SpottingTargetingSettings based on FirearmIntelligence.
///
/// Runs before the generic sync_spotted_to_combat_targeting (in combat-targeting)
/// to convert firearm-specific settings into the generic settings component.
pub(crate) fn update_firearm_spotting_settings(
	mut combatants: Query<(&FirearmIntelligence, &mut SpottingTargetingSettings)>,
) {
	for (firearm, mut settings) in &mut combatants {
		let memory_secs = firearm.settings.target_spotting_memory.max(0.0);
		let continuity = firearm.settings.focus.clamp(0.0, 1.0) * 8.0;
		if settings.memory_secs != memory_secs || settings.continuity != continuity {
			settings.memory_secs = memory_secs;
			settings.continuity = continuity;
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use spotting_intelligence::{InterestLayers, SpotDirective, SpottedContact, SpottingSettings};

	#[test]
	fn spotted_contacts_enter_weighted_combat_memory()
	-> Result<(), bevy::ecs::system::RunSystemError> {
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
		targeting.include(target, TargetSource::ENEMYSHIP);
		world.spawn((Transform::default(), spotting, FirearmIntelligence::new(), targeting));

		world.run_system_once(sync_spotted_combat_targets)?;
		let targeting = world.query::<&CombatTargeting>().single(&world)?;
		assert_eq!(targeting.contact(target).map(|contact| contact.position), Some(Vec3::X * 4.0));
		assert_eq!(targeting.active_target(target).map(|target| target.factors.bias), Some(-4.0));
		world.query::<&mut SpottingUser>().single_mut(&mut world)?.contacts.clear();
		world.run_system_once(sync_spotted_combat_targets)?;
		let targeting = world.query::<&CombatTargeting>().single(&world)?;
		assert!(targeting.contact(target).is_none());
		assert!(
			targeting
				.active_target(target)
				.is_some_and(|target| target.has_source(TargetSource::ENEMYSHIP))
		);
		Ok(())
	}

	#[test]
	fn disabled_targeting_does_not_admit_spotted_contacts()
	-> Result<(), bevy::ecs::system::RunSystemError> {
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
		targeting.include(target, TargetSource::ENEMYSHIP);
		targeting.enabled = false;
		world.spawn((Transform::default(), spotting, FirearmIntelligence::new(), targeting));
		world.run_system_once(sync_spotted_combat_targets)?;
		let targeting = world.query::<&CombatTargeting>().single(&world)?;
		assert!(targeting.contact(target).is_none());
		assert!(
			targeting
				.active_target(target)
				.is_some_and(|target| target.has_source(TargetSource::ENEMYSHIP))
		);
		Ok(())
	}

	#[test]
	fn visible_allies_do_not_enter_the_actionable_target_set()
	-> Result<(), bevy::ecs::system::RunSystemError> {
		let enemy = Entity::from_bits(7);
		let ally = Entity::from_bits(8);
		let mut world = World::new();
		let mut spotting =
			SpottingUser::new(Vec3::Y, [SpotDirective::new(InterestLayers::CHARACTER, 20.0)])
				.with_settings(SpottingSettings::new(4, 4, 2.5));
		for (index, subject) in [enemy, ally].into_iter().enumerate() {
			let position = Vec3::X * (index + 1) as f32 * 4.0;
			spotting.contacts.insert(
				subject,
				SpottedContact::new(subject, position, Vec3::ZERO, position, None, 1.0, 0.1),
			);
		}
		let mut targeting = CombatTargeting::default();
		targeting.include(enemy, TargetSource::ENEMYSHIP);
		world.spawn((Transform::default(), spotting, FirearmIntelligence::new(), targeting));

		world.run_system_once(sync_spotted_combat_targets)?;

		let targeting = world.query::<&CombatTargeting>().single(&world)?;
		assert!(targeting.contact(enemy).is_some());
		assert!(targeting.contact(ally).is_none());
		assert!(targeting.active_target(ally).is_none());
		Ok(())
	}

	#[test]
	fn shared_membership_can_admit_a_later_sighting()
	-> Result<(), bevy::ecs::system::RunSystemError> {
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
		targeting.include(target, TargetSource::SHARED);
		world.spawn((Transform::default(), spotting, FirearmIntelligence::new(), targeting));

		world.run_system_once(sync_spotted_combat_targets)?;
		let targeting = world.query::<&CombatTargeting>().single(&world)?;
		assert!(targeting.contact(target).is_some());
		assert!(
			targeting
				.active_target(target)
				.is_some_and(|target| target.has_source(TargetSource::SHARED)
					&& target.has_source(TargetSource::SPOTTING))
		);
		Ok(())
	}
	#[test]
	fn unchanged_contact_keeps_membership_and_updates_observer_factors_and_settings()
	-> anyhow::Result<()> {
		let target = Entity::from_bits(7);
		let mut world = World::new();
		let mut spotting = SpottingUser::default();
		spotting.contacts.insert(
			target,
			SpottedContact::new(target, Vec3::X * 4.0, Vec3::ZERO, Vec3::X * 4.0, None, 1.0, 0.125),
		);
		let mut targeting = CombatTargeting::default();
		targeting.include(target, TargetSource::ENEMYSHIP);
		let observer = world
			.spawn((Transform::default(), spotting, FirearmIntelligence::default(), targeting))
			.id();
		world
			.run_system_once(sync_spotted_combat_targets)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		if let Some(mut targeting) = world.get_mut::<CombatTargeting>(observer) {
			targeting.rebalance(1.0);
		}
		world
			.run_system_once(sync_spotted_combat_targets)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.get::<CombatTargeting>(observer).is_some_and(|targeting| !targeting.dirty));
		if let Some(mut brain) = world.get_mut::<FirearmIntelligence>(observer) {
			brain.settings.focus = 1.0;
			brain.settings.target_spotting_memory = 7.0;
		}
		world
			.run_system_once(sync_spotted_combat_targets)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.get::<CombatTargeting>(observer).is_some_and(|targeting| targeting.dirty));
		if let Some(mut targeting) = world.get_mut::<CombatTargeting>(observer) {
			targeting.rebalance(1.0);
		}
		if let Some(mut transform) = world.get_mut::<Transform>(observer) {
			transform.translation = Vec3::X * 2.0;
		}
		if let Some(mut targeting) = world.get_mut::<CombatTargeting>(observer) {
			targeting.remove_source(target, TargetSource::SPOTTING);
			targeting.rebalance(1.0);
		}
		world
			.run_system_once(sync_spotted_combat_targets)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let targeting = world
			.get::<CombatTargeting>(observer)
			.ok_or_else(|| anyhow::anyhow!("missing targeting"))?;
		assert_eq!(targeting.memory_secs, 7.0);
		assert_eq!(targeting.algebra.continuity, 8.0);
		assert_eq!(targeting.contact(target).map(|contact| contact.last_spotted_at), Some(1.0));
		assert!(targeting.active_target(target).is_some_and(|active| {
			active.has_source(TargetSource::SPOTTING) && active.factors.bias == -2.0
		}));
		assert!(targeting.dirty);
		Ok(())
	}
}
