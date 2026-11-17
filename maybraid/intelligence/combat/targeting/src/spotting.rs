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
