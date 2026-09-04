use bevy::prelude::*;
use spotting_intelligence::SpottingUser;

use crate::{AssailantContact, AssailantFactors, AssailantSource, EvasionIntelligenceUser};

/// Refresh assailant memory from successful generic spotting contacts.
pub fn sync_spotted_assailants(
	mut users: Query<(&Transform, &SpottingUser, &mut EvasionIntelligenceUser)>,
) {
	for (transform, spotting, mut evasion) in &mut users {
		let forgotten: Vec<Entity> = evasion
			.memory
			.keys()
			.filter(|entity| !spotting.contacts.contains_key(entity))
			.copied()
			.collect();
		for entity in forgotten {
			evasion.remove_source(entity, AssailantSource::SPOTTING);
			if evasion.active_assailant(entity).is_none() {
				evasion.memory.remove(&entity);
			}
		}

		for contact in spotting.contacts.values() {
			evasion.upsert_sighting(AssailantContact {
				subject: contact.subject,
				position: contact.position,
				movement_vector: contact.velocity,
				last_known_at: contact.last_success_at,
			});
			let previous = evasion
				.active_assailant(contact.subject)
				.map_or(AssailantFactors::default(), |assailant| assailant.factors);
			let distance = Vec2::new(
				contact.position.x - transform.translation.x,
				contact.position.z - transform.translation.z,
			)
			.length();
			evasion.set_factors(
				contact.subject,
				AssailantFactors {
					threat: previous.threat.max(1.0),
					proximity: 1.0 / (1.0 + distance),
					bias: -distance,
					uncertainty: previous.uncertainty,
				},
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
	fn spotted_contacts_enter_assailant_memory() -> Result<(), bevy::ecs::system::RunSystemError> {
		let target = Entity::from_bits(7);
		let mut world = World::new();
		let mut spotting =
			SpottingUser::new(Vec3::Y, [SpotDirective::new(InterestLayers::CHARACTER, 20.0)])
				.with_settings(SpottingSettings::new(4, 4, 2.5));
		spotting.contacts.insert(
			target,
			SpottedContact::new(target, Vec3::X * 4.0, Vec3::ZERO, Vec3::X * 4.0, None, 1.0, 0.1),
		);
		world.spawn((Transform::default(), spotting, EvasionIntelligenceUser::default()));

		world.run_system_once(sync_spotted_assailants)?;
		let evasion = world.query::<&EvasionIntelligenceUser>().single(&world)?;
		assert_eq!(evasion.contact(target).map(|contact| contact.position), Some(Vec3::X * 4.0));
		Ok(())
	}
}
