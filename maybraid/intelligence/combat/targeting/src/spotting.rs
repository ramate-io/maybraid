//! Bridge from spotting-intelligence contacts to combat targeting.
//!
//! Generic adapter that reads `SpottingUser` contacts (with real visibility/occlusion)
//! and writes them to `CombatTargeting` for ranking. Used by both firearm and non-firearm
//! attackers.

use bevy::prelude::*;
use spotting_intelligence::SpottingUser;

use crate::{CombatContact, CombatTargeting, TargetFactors, TargetSource};

/// Settings for spotted contact memory and ranking.
#[derive(Component, Clone, Copy, Debug)]
pub struct SpottingTargetingSettings {
	/// How long to remember spotted contacts (seconds)
	pub memory_secs: f32,
	/// Continuity factor for target ranking (0..=8, higher = more sticky)
	pub continuity: f32,
}

impl Default for SpottingTargetingSettings {
	fn default() -> Self {
		Self {
			memory_secs: 3.0,
			continuity: 0.0,
		}
	}
}

const ACTIONABLE_SOURCES: TargetSource = TargetSource::from_bits(
	TargetSource::OBJECTIVE.bits()
		| TargetSource::RECEIVED_FIRE.bits()
		| TargetSource::ENEMYSHIP.bits()
		| TargetSource::FIREARM.bits()
		| TargetSource::SHARED.bits(),
);

/// Syncs SpottingUser contacts to CombatTargeting.
///
/// Reads spotted contacts (with real occlusion/visibility from spotting systems)
/// and writes them to CombatTargeting for ranking. Removes forgotten contacts.
///
/// Used by both firearm combatants and non-firearm attackers (e.g. boss encounters).
pub fn sync_spotted_to_combat_targeting(
	mut combatants: Query<(
		&Transform,
		&SpottingUser,
		&SpottingTargetingSettings,
		&mut CombatTargeting,
	)>,
) {
	for (transform, spotting, settings, mut targeting) in &mut combatants {
		// Configure targeting memory and continuity
		if targeting.memory_secs != settings.memory_secs
			|| targeting.algebra.continuity != settings.continuity
		{
			targeting.memory_secs = settings.memory_secs;
			targeting.algebra.continuity = settings.continuity;
			targeting.dirty = true;
		}

		// Remove forgotten contacts (no longer in spotting)
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

		// Transfer spotted contacts to targeting (with real visibility from spotting)
		for contact in spotting.contacts.values() {
			// Only admit targets that have actionable sources (to avoid admitting neutral/ally sightings)
			let actionable = targeting
				.active_target(contact.subject)
				.is_some_and(|target| target.active_sources().intersects(ACTIONABLE_SOURCES));
			if !actionable {
				targeting.memory.remove(&contact.subject);
				targeting.remove_source(contact.subject, TargetSource::SPOTTING);
				continue;
			}

			let observed = CombatContact {
				subject: contact.subject,
				position: contact.position,
				movement_vector: contact.velocity,
				visible_point: contact.visible_point,
				visible_head: contact.visible_head,
				last_spotted_at: contact.last_success_at,
			};

			if targeting.contact(contact.subject) != Some(&observed) {
				targeting.upsert_contact(observed);
			} else {
				// Restore SPOTTING membership without dirtying unchanged target
				targeting.include(contact.subject, TargetSource::SPOTTING);
			}

			// Set distance-based factors
			let delta = contact.position - transform.translation;
			let distance = Vec2::new(delta.x, delta.z).length();
			let previous = targeting
				.active_target(contact.subject)
				.map_or(TargetFactors::default(), |target| target.factors);
			targeting.set_factors(
				contact.subject,
				TargetFactors {
					hostility: 1.0,
					bias: -distance,
					..previous
				},
			);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use spotting_intelligence::{InterestLayers, SpotDirective, SpottedContact, SpottingSettings, SpottingUser};

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
		world.spawn((
			Transform::default(),
			spotting,
			SpottingTargetingSettings::default(),
			targeting,
		));

		world.run_system_once(sync_spotted_to_combat_targeting)?;
		let targeting = world.query::<&CombatTargeting>().single(&world)?;
		assert_eq!(targeting.contact(target).map(|contact| contact.position), Some(Vec3::X * 4.0));
		assert_eq!(targeting.active_target(target).map(|target| target.factors.bias), Some(-4.0));
		world.query::<&mut SpottingUser>().single_mut(&mut world)?.contacts.clear();
		world.run_system_once(sync_spotted_to_combat_targeting)?;
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
		world.spawn((
			Transform::default(),
			spotting,
			SpottingTargetingSettings::default(),
			targeting,
		));
		world.run_system_once(sync_spotted_to_combat_targeting)?;
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
		world.spawn((
			Transform::default(),
			spotting,
			SpottingTargetingSettings::default(),
			targeting,
		));

		world.run_system_once(sync_spotted_to_combat_targeting)?;

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
		world.spawn((
			Transform::default(),
			spotting,
			SpottingTargetingSettings::default(),
			targeting,
		));

		world.run_system_once(sync_spotted_to_combat_targeting)?;
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
			.spawn((
				Transform::default(),
				spotting,
				SpottingTargetingSettings::default(),
				targeting,
			))
			.id();
		world
			.run_system_once(sync_spotted_to_combat_targeting)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		if let Some(mut targeting) = world.get_mut::<CombatTargeting>(observer) {
			targeting.rebalance(1.0);
		}
		world
			.run_system_once(sync_spotted_to_combat_targeting)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.get::<CombatTargeting>(observer).is_some_and(|targeting| !targeting.dirty));
		if let Some(mut settings) = world.get_mut::<SpottingTargetingSettings>(observer) {
			settings.continuity = 8.0;
			settings.memory_secs = 7.0;
		}
		world
			.run_system_once(sync_spotted_to_combat_targeting)
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
			.run_system_once(sync_spotted_to_combat_targeting)
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
