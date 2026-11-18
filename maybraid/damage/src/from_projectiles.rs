//! [`ProjectileContact`] → [`Hit`] using the payload stamped on the flight.

use bevy::prelude::*;
use projectiles::ProjectileContact;

use crate::{ChainSegment, Hit, HitPayload};

pub fn contacts_to_hits(
	mut contacts: MessageReader<ProjectileContact>,
	payloads: Query<&HitPayload>,
	chain_segments: Query<(), With<ChainSegment>>,
	mut hits: MessageWriter<Hit>,
) {
	for contact in contacts.read() {
		if chain_segments.contains(contact.target) {
			continue;
		}
		let Ok(payload) = payloads.get(contact.projectile) else {
			continue;
		};
		hits.write(Hit {
			target: contact.target,
			source: contact.source,
			amount: payload.amount,
			point: contact.point,
			allow_self: false,
		});
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn missing_payload_is_not_a_hit() {
		assert_eq!(HitPayload::default().amount, crate::DEFAULT_HIT);
	}

	#[test]
	fn ordinary_contact_becomes_a_hit() {
		let mut app = App::new();
		app.add_message::<ProjectileContact>()
			.add_message::<Hit>()
			.add_systems(Update, contacts_to_hits);

		let target = app.world_mut().spawn_empty().id();
		let projectile = app.world_mut().spawn(HitPayload { amount: 25.0 }).id();
		app.world_mut().write_message(ProjectileContact {
			projectile,
			source: None,
			target,
			point: Vec3::ZERO,
			normal: Vec3::Y,
		});
		app.update();

		let hits: Vec<_> = app.world_mut().resource_mut::<Messages<Hit>>().drain().collect();
		assert_eq!(hits.len(), 1);
		assert_eq!(hits[0].target, target);
		assert_eq!(hits[0].amount, 25.0);
	}

	#[test]
	fn chain_segment_contacts_are_skipped() {
		let mut app = App::new();
		app.add_message::<ProjectileContact>()
			.add_message::<Hit>()
			.add_systems(Update, contacts_to_hits);

		let target = app.world_mut().spawn(ChainSegment).id();
		let projectile = app.world_mut().spawn(HitPayload { amount: 25.0 }).id();
		app.world_mut().write_message(ProjectileContact {
			projectile,
			source: None,
			target,
			point: Vec3::ZERO,
			normal: Vec3::Y,
		});
		app.update();

		let hits: Vec<_> = app.world_mut().resource_mut::<Messages<Hit>>().drain().collect();
		assert!(hits.is_empty(), "chain-segment contacts must be left for the coalescer");
	}
}
