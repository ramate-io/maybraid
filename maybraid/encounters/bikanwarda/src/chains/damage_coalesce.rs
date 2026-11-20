//! Contact coalescing for chain capsule colliders ([#1127](https://github.com/ramate-io/maybraid/issues/1127)).
//!
//! Reads the shared [`ProjectileContact`] queue **non-destructively** and emits
//! one [`Hit`] per interaction into the generic damage pipeline. Ordinary
//! contacts stay on the queue for [`damage::contacts_to_hits`]; chain-capsule
//! targets carry [`damage::ChainSegment`] so that path skips them.
//!
//! ## Deduplication contract
//!
//! - **Projectile hits**: keyed by `ProjectileContact.projectile` (the Flight entity)
//!   plus the chain root.
//! - **Attack hits**: keyed by `AttackInstanceId` plus the chain root once attack
//!   integration is wired.
//! - **NOT** keyed by `(source, chain_root)` — multiple distinct projectiles or
//!   attacks from the same source in one frame are separate interactions and
//!   must both apply.
//!
//! History entries are dropped when the projectile despawns so the set stays
//! bounded. This system never drains or rewrites `ProjectileContact`.
//!
//! Runs in `DamageSystems::Collect`, before `contacts_to_hits` and `apply_hits`.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use damage::{ChainSegment, Hit, HitPayload};
use projectiles::ProjectileContact;

use super::collision::ChainLinkId;

/// Tracks which (projectile, chain_root) pairs have already emitted a [`Hit`].
///
/// Prevents penetrating chain contacts from applying damage every frame they
/// remain in contact. Ordinary (non-chain) contacts are not recorded here.
#[derive(Resource, Default)]
pub struct ChainDamageHistory {
	processed: HashSet<(Entity, Entity)>,
}

/// Component on chain roots tracking which projectiles have already hit this chain.
///
/// Used for cross-frame damage deduplication. Once a projectile hits a chain,
/// it's recorded here and cannot hit again.
#[derive(Component, Default, Clone, Debug)]
pub struct ChainHitProjectiles {
	pub hits: HashSet<Entity>,
}

/// Interaction identity for deduplication.
///
/// One projectile or attack instance hitting multiple chain capsules is
/// deduplicated to a single hit. Distinct projectiles or attacks in the same
/// frame are separate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum InteractionId {
	/// Projectile flight entity.
	Projectile(Entity),
	/// Attack instance id (from attack hazards).
	/// TODO: Wire this when attack integration is complete.
	#[allow(dead_code)]
	Attack(u64),
}

/// Contact coalescer state for one chain root.
#[derive(Default)]
struct ChainContactState {
	/// Map from interaction id to the chosen contact for that interaction.
	interactions: HashMap<InteractionId, ProjectileContact>,
}

impl ChainContactState {
	fn note_contact(&mut self, id: InteractionId, contact: ProjectileContact) {
		// Keep the first contact for each interaction (arbitrary but stable).
		self.interactions.entry(id).or_insert(contact);
	}

	fn drain_coalesced(&mut self) -> impl Iterator<Item = ProjectileContact> + '_ {
		self.interactions.drain().map(|(_, contact)| contact)
	}
}

/// Read chain-capsule contacts and emit one [`Hit`] per interaction.
///
/// Runs in `DamageSystems::Collect`, before `contacts_to_hits`.
///
/// Deduplicates in two ways:
/// 1. Within a frame: multiple capsule hits from the same projectile → one Hit
/// 2. Cross-frame: once a (projectile, chain_root) pair hits, it never hits again
///
/// The shared [`ProjectileContact`] queue is never drained or rewritten.
pub fn coalesce_chain_contacts(
	mut contacts: MessageReader<ProjectileContact>,
	link_ids: Query<&ChainLinkId>,
	segments: Query<(), With<ChainSegment>>,
	payloads: Query<&HitPayload>,
	mut history: ResMut<ChainDamageHistory>,
	mut hits: MessageWriter<Hit>,
) {
	let mut chains: HashMap<Entity, ChainContactState> = HashMap::new();

	for contact in contacts.read() {
		if !segments.contains(contact.target) {
			continue;
		}
		let Ok(link_id) = link_ids.get(contact.target) else {
			continue;
		};

		let chain_root = link_id.chain_root;
		let history_key = (contact.projectile, chain_root);
		if history.processed.contains(&history_key) {
			continue;
		}

		let interaction_id = InteractionId::Projectile(contact.projectile);
		chains.entry(chain_root).or_default().note_contact(interaction_id, *contact);
	}

	for (root, mut state) in chains {
		for contact in state.drain_coalesced() {
			history.processed.insert((contact.projectile, root));
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
}

/// Clean up damage history when projectiles despawn.
///
/// Runs after projectiles are despawned to free memory.
pub fn cleanup_despawned_projectiles(
	mut history: ResMut<ChainDamageHistory>,
	projectiles: Query<Entity, With<projectiles::Flight>>,
) {
	let live_projectiles: HashSet<Entity> = projectiles.iter().collect();
	history
		.processed
		.retain(|(projectile, _)| live_projectiles.contains(projectile));
}

#[cfg(test)]
mod tests {
	use super::*;

	fn chain_app() -> App {
		let mut app = App::new();
		app.init_resource::<ChainDamageHistory>()
			.add_message::<ProjectileContact>()
			.add_message::<Hit>()
			.add_systems(Update, coalesce_chain_contacts);
		app
	}

	fn drain_hits(app: &mut App) -> Vec<Hit> {
		app.world_mut().resource_mut::<Messages<Hit>>().drain().collect()
	}

	fn drain_contacts(app: &mut App) -> Vec<ProjectileContact> {
		app.world_mut().resource_mut::<Messages<ProjectileContact>>().drain().collect()
	}

	#[test]
	fn projectile_hitting_multiple_capsules_coalesces_to_one_hit() {
		let mut app = chain_app();

		let chain_root = app.world_mut().spawn_empty().id();
		let capsule_a = app
			.world_mut()
			.spawn((ChainLinkId { link_id: 0, chain_root }, ChainSegment))
			.id();
		let capsule_b = app
			.world_mut()
			.spawn((ChainLinkId { link_id: 1, chain_root }, ChainSegment))
			.id();
		let projectile = app.world_mut().spawn(HitPayload { amount: 10.0 }).id();
		let source = Some(Entity::from_bits(100));

		app.world_mut().write_message(ProjectileContact {
			projectile,
			source,
			target: capsule_a,
			point: Vec3::new(1.0, 0.0, 0.0),
			normal: Vec3::Y,
		});
		app.world_mut().write_message(ProjectileContact {
			projectile,
			source,
			target: capsule_b,
			point: Vec3::new(1.1, 0.0, 0.0),
			normal: Vec3::Y,
		});

		app.update();

		let hits = drain_hits(&mut app);
		assert_eq!(hits.len(), 1);
		assert_eq!(hits[0].amount, 10.0);
		assert!(hits[0].target == capsule_a || hits[0].target == capsule_b);

		let contacts = drain_contacts(&mut app);
		assert_eq!(contacts.len(), 2, "coalescer must not drain the shared contact queue");
	}

	#[test]
	fn different_projectiles_do_not_coalesce() {
		let mut app = chain_app();

		let chain_root = app.world_mut().spawn_empty().id();
		let capsule = app
			.world_mut()
			.spawn((ChainLinkId { link_id: 0, chain_root }, ChainSegment))
			.id();
		let projectile_a = app.world_mut().spawn(HitPayload { amount: 10.0 }).id();
		let projectile_b = app.world_mut().spawn(HitPayload { amount: 10.0 }).id();
		let source = Some(Entity::from_bits(100));

		app.world_mut().write_message(ProjectileContact {
			projectile: projectile_a,
			source,
			target: capsule,
			point: Vec3::new(1.0, 0.0, 0.0),
			normal: Vec3::Y,
		});
		app.world_mut().write_message(ProjectileContact {
			projectile: projectile_b,
			source,
			target: capsule,
			point: Vec3::new(1.0, 0.0, 0.0),
			normal: Vec3::Y,
		});

		app.update();

		let hits = drain_hits(&mut app);
		assert_eq!(hits.len(), 2);
	}

	#[test]
	fn non_chain_contacts_are_ignored() {
		let mut app = chain_app();

		let regular_target = app.world_mut().spawn_empty().id();
		let projectile = app.world_mut().spawn(HitPayload { amount: 25.0 }).id();

		app.world_mut().write_message(ProjectileContact {
			projectile,
			source: Some(Entity::from_bits(100)),
			target: regular_target,
			point: Vec3::new(1.0, 0.0, 0.0),
			normal: Vec3::Y,
		});

		app.update();

		assert!(drain_hits(&mut app).is_empty());
		let contacts = drain_contacts(&mut app);
		assert_eq!(contacts.len(), 1);
		assert_eq!(contacts[0].target, regular_target);
	}

	#[test]
	fn multiple_chains_are_coalesced_independently() {
		let mut app = chain_app();

		let chain_a = app.world_mut().spawn_empty().id();
		let chain_b = app.world_mut().spawn_empty().id();
		let capsule_a1 = app
			.world_mut()
			.spawn((ChainLinkId { link_id: 0, chain_root: chain_a }, ChainSegment))
			.id();
		let capsule_a2 = app
			.world_mut()
			.spawn((ChainLinkId { link_id: 1, chain_root: chain_a }, ChainSegment))
			.id();
		let capsule_b1 = app
			.world_mut()
			.spawn((ChainLinkId { link_id: 0, chain_root: chain_b }, ChainSegment))
			.id();
		let projectile = app.world_mut().spawn(HitPayload { amount: 10.0 }).id();
		let source = Some(Entity::from_bits(100));

		app.world_mut().write_message(ProjectileContact {
			projectile,
			source,
			target: capsule_a1,
			point: Vec3::new(1.0, 0.0, 0.0),
			normal: Vec3::Y,
		});
		app.world_mut().write_message(ProjectileContact {
			projectile,
			source,
			target: capsule_a2,
			point: Vec3::new(1.1, 0.0, 0.0),
			normal: Vec3::Y,
		});
		app.world_mut().write_message(ProjectileContact {
			projectile,
			source,
			target: capsule_b1,
			point: Vec3::new(2.0, 0.0, 0.0),
			normal: Vec3::Y,
		});

		app.update();

		assert_eq!(drain_hits(&mut app).len(), 2);
		assert_eq!(drain_contacts(&mut app).len(), 3);
	}
}
