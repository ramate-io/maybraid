//! [`ProjectileContact`] → [`Hit`] using the payload stamped on the flight.

use std::collections::HashSet;

use bevy::prelude::*;
use projectiles::{Flight, ProjectileContact};

use crate::{ChainSegment, Health, Hit, HitPayload, HurtboxOf};

/// One [`Hit`] per projectile per resolved [`Health`] host.
///
/// Motor capsules and query sensors are distinct colliders. A penetrating
/// flight can contact several of them; they must debit the host once.
#[derive(Resource, Default)]
pub struct ProjectileHostHits {
	processed: HashSet<(Entity, Entity)>,
}

pub fn contacts_to_hits(
	mut contacts: MessageReader<ProjectileContact>,
	payloads: Query<&HitPayload>,
	chain_segments: Query<(), With<ChainSegment>>,
	hurtboxes: Query<&HurtboxOf>,
	parents: Query<&ChildOf>,
	health: Query<(), With<Health>>,
	mut history: ResMut<ProjectileHostHits>,
	mut hits: MessageWriter<Hit>,
) {
	for contact in contacts.read() {
		if chain_segments.contains(contact.target) {
			continue;
		}
		let Ok(payload) = payloads.get(contact.projectile) else {
			continue;
		};
		let hurtbox = hurtboxes.get(contact.target).ok().map(HurtboxOf::host);
		let parent = parents.get(contact.target).ok().map(|child| child.parent());
		let host = Health::entity_or_hurtbox_or_parent(
			contact.target,
			hurtbox,
			parent,
			health.contains(contact.target),
			hurtbox.is_some_and(|host| health.contains(host)),
			parent.is_some_and(|parent| health.contains(parent)),
		);
		if !history.processed.insert((contact.projectile, host)) {
			continue;
		}
		hits.write(Hit {
			target: contact.target,
			source: contact.source,
			amount: payload.amount,
			point: contact.point,
			allow_self: false,
		});
	}
}

pub(crate) fn cleanup_projectile_host_hits(
	mut history: ResMut<ProjectileHostHits>,
	flights: Query<Entity, With<Flight>>,
) {
	let live: HashSet<Entity> = flights.iter().collect();
	history.processed.retain(|(projectile, _)| live.contains(projectile));
}

#[cfg(test)]
mod tests {
	use super::*;

	fn drain_hits(app: &mut App) -> Vec<Hit> {
		app.world_mut().resource_mut::<Messages<Hit>>().drain().collect()
	}

	#[test]
	fn missing_payload_is_not_a_hit() {
		assert_eq!(HitPayload::default().amount, crate::DEFAULT_HIT);
	}

	#[test]
	fn ordinary_contact_becomes_a_hit() {
		let mut app = App::new();
		app.init_resource::<ProjectileHostHits>()
			.add_message::<ProjectileContact>()
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

		let hits = drain_hits(&mut app);
		assert_eq!(hits.len(), 1);
		assert_eq!(hits[0].target, target);
		assert_eq!(hits[0].amount, 25.0);
	}

	#[test]
	fn chain_segment_contacts_are_skipped() {
		let mut app = App::new();
		app.init_resource::<ProjectileHostHits>()
			.add_message::<ProjectileContact>()
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

		let hits = drain_hits(&mut app);
		assert!(hits.is_empty(), "chain-segment contacts must be left for the coalescer");
	}

	#[test]
	fn multiple_hurtboxes_of_one_projectile_emit_one_hit() {
		let mut app = App::new();
		app.init_resource::<ProjectileHostHits>()
			.add_message::<ProjectileContact>()
			.add_message::<Hit>()
			.add_systems(Update, contacts_to_hits);

		let host = app.world_mut().spawn(Health::from_max(100.0)).id();
		let actor = app.world_mut().spawn(HurtboxOf(host)).id();
		let sensor = app.world_mut().spawn((ChildOf(actor), HurtboxOf(host))).id();
		let projectile = app.world_mut().spawn(HitPayload { amount: 25.0 }).id();
		for target in [sensor, actor] {
			app.world_mut().write_message(ProjectileContact {
				projectile,
				source: None,
				target,
				point: Vec3::ZERO,
				normal: Vec3::Y,
			});
		}
		app.update();

		let hits = drain_hits(&mut app);
		assert_eq!(hits.len(), 1, "one projectile must debit one host, got {hits:?}");
		assert_eq!(hits[0].amount, 25.0);
	}
}
