//! Contact deduplication for chain capsule colliders ([#1127](https://github.com/ramate-io/maybraid/issues/1127)).
//!
//! Coalesces overlapping chain-capsule contacts BEFORE `apply_hits` to prevent double-damage
//! from a single attack or projectile hitting multiple adjacent capsules.
//!
//! ## Deduplication contract
//!
//! - **Projectile hits**: keyed by `ProjectileContact.projectile` (the Flight entity).
//! - **Attack hits**: keyed by `AttackInstanceId` (resolved from attack hazards).
//! - **NOT** keyed by `(source, chain_root)` — multiple distinct projectiles or attacks
//!   from the same source in one frame are separate interactions and must both apply.
//!
//! ## Cross-frame deduplication
//!
//! Penetrating projectiles stay in contact across multiple frames. To prevent applying
//! damage every frame, we track (projectile, chain_root) pairs that have already applied
//! damage. When a projectile despawns, we clean up its records.
//!
//! Runs in `DamageSystems::Collect`, before `contacts_to_hits` and `apply_hits`.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use projectiles::ProjectileContact;

use super::collision::ChainLinkId;

/// Tracks which (projectile, target) pairs have already been processed.
///
/// Prevents contacts from being re-emitted every frame (both chain and non-chain).
/// For chain contacts, the target is the chain_root. For non-chain contacts, it's the actual target.
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
/// One projectile or attack instance hitting multiple chain capsules is deduplicated to
/// a single hit. Distinct projectiles or attacks in the same frame are separate.
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

/// System that coalesces chain capsule contacts before they become hits.
///
/// Runs in `DamageSystems::Collect`, before `contacts_to_hits`.
///
/// Deduplic ates in two ways:
/// 1. Within a frame: multiple capsule hits from the same projectile → one contact  
/// 2. Cross-frame: once a (projectile, chain) pair hits, it never hits again
///
/// NOTE: This approach is incomplete and needs reworking. The fundamental issue is that
/// we cannot easily filter messages from the queue for other consumers. A better approach
/// would be to have contacts_to_hits skip chain capsule targets, or use a separate
/// message type for coalesced contacts.
pub fn coalesce_chain_contacts(
	link_ids: Query<&ChainLinkId>,
	mut messages: ResMut<Messages<ProjectileContact>>,
	mut history: ResMut<ChainDamageHistory>,
) {
	// TODO: This system needs a complete redesign. The current approach of drain-and-rewrite
	// creates a cycle where contacts are re-emitted every frame. Using MessageReader alone
	// doesn't solve it because contacts_to_hits will see both original and coalesced contacts.
	//
	// Possible solutions:
	// 1. Have contacts_to_hits skip chain capsule targets (requires modifying damage crate)
	// 2. Use a separate CoalescedProjectileContact message type
	// 3. Use a Local resource to track which message IDs have been processed
	//
	// For now, keeping the drain approach but with proper history tracking.
	
	let all_contacts: Vec<ProjectileContact> = messages.drain().collect();
	let mut chains: HashMap<Entity, ChainContactState> = HashMap::new();
	
	for contact in all_contacts {
		let (is_chain, chain_root) = link_ids
			.get(contact.target)
			.map(|link_id| (true, link_id.chain_root))
			.unwrap_or((false, contact.target));
		
		let history_key = (contact.projectile, if is_chain { chain_root } else { contact.target });
		
		if history.processed.contains(&history_key) {
			// Already processed - don't re-emit
			continue;
		}
		
		if is_chain {
			let interaction_id = InteractionId::Projectile(contact.projectile);
			chains.entry(chain_root).or_default().note_contact(interaction_id, contact);
		} else {
			history.processed.insert(history_key);
			messages.write(contact);
		}
	}
	
	for (root, mut state) in chains {
		for contact in state.drain_coalesced() {
			history.processed.insert((contact.projectile, root));
			messages.write(contact);
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
	// Collect all live projectile entities
	let live_projectiles: HashSet<Entity> = projectiles.iter().collect();

	// Remove history entries for despawned projectiles
	history.processed.retain(|(projectile, _)| live_projectiles.contains(projectile));
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn projectile_hitting_multiple_capsules_coalesces_to_one_contact() {
		let mut app = App::new();
		app.init_resource::<ChainDamageHistory>()
			.add_message::<ProjectileContact>()
			.add_systems(Update, coalesce_chain_contacts);

		let chain_root = app.world_mut().spawn_empty().id();
		let capsule_a =
			app.world_mut().spawn(ChainLinkId { link_id: 0, chain_root }).id();
		let capsule_b =
			app.world_mut().spawn(ChainLinkId { link_id: 1, chain_root }).id();

		let projectile = Entity::from_bits(999);

		// Same projectile hits two adjacent capsules
		app.world_mut().write_message(ProjectileContact {
			projectile,
			source: Some(Entity::from_bits(100)),
			target: capsule_a,
			point: Vec3::new(1.0, 0.0, 0.0),
			normal: Vec3::Y,
		});
		app.world_mut().write_message(ProjectileContact {
			projectile,
			source: Some(Entity::from_bits(100)),
			target: capsule_b,
			point: Vec3::new(1.1, 0.0, 0.0),
			normal: Vec3::Y,
		});

		app.update();

		let contacts: Vec<_> =
			app.world_mut().resource_mut::<Messages<ProjectileContact>>().drain().collect();

		// Should coalesce to one contact
		assert_eq!(contacts.len(), 1);
		assert_eq!(contacts[0].projectile, projectile);
	}

	#[test]
	fn different_projectiles_do_not_coalesce() {
		let mut app = App::new();
		app.init_resource::<ChainDamageHistory>()
			.add_message::<ProjectileContact>()
			.add_systems(Update, coalesce_chain_contacts);

		let chain_root = app.world_mut().spawn_empty().id();
		let capsule = app.world_mut().spawn(ChainLinkId { link_id: 0, chain_root }).id();

		let projectile_a = Entity::from_bits(999);
		let projectile_b = Entity::from_bits(1000);

		// Two different projectiles hit the same capsule
		app.world_mut().write_message(ProjectileContact {
			projectile: projectile_a,
			source: Some(Entity::from_bits(100)),
			target: capsule,
			point: Vec3::new(1.0, 0.0, 0.0),
			normal: Vec3::Y,
		});
		app.world_mut().write_message(ProjectileContact {
			projectile: projectile_b,
			source: Some(Entity::from_bits(100)),
			target: capsule,
			point: Vec3::new(1.0, 0.0, 0.0),
			normal: Vec3::Y,
		});

		app.update();

		let contacts: Vec<_> =
			app.world_mut().resource_mut::<Messages<ProjectileContact>>().drain().collect();

		// Should keep both contacts (different projectiles)
		assert_eq!(contacts.len(), 2);
	}

	#[test]
	fn non_chain_contacts_pass_through_unchanged() {
		let mut app = App::new();
		app.init_resource::<ChainDamageHistory>()
			.add_message::<ProjectileContact>()
			.add_systems(Update, coalesce_chain_contacts);

		let regular_target = app.world_mut().spawn_empty().id();
		let projectile = Entity::from_bits(999);

		app.world_mut().write_message(ProjectileContact {
			projectile,
			source: Some(Entity::from_bits(100)),
			target: regular_target,
			point: Vec3::new(1.0, 0.0, 0.0),
			normal: Vec3::Y,
		});

		app.update();

		let contacts: Vec<_> =
			app.world_mut().resource_mut::<Messages<ProjectileContact>>().drain().collect();

		// Non-chain contact should pass through
		assert_eq!(contacts.len(), 1);
		assert_eq!(contacts[0].target, regular_target);
	}

	#[test]
	fn multiple_chains_are_coalesced_independently() {
		let mut app = App::new();
		app.init_resource::<ChainDamageHistory>()
			.add_message::<ProjectileContact>()
			.add_systems(Update, coalesce_chain_contacts);

		let chain_a = app.world_mut().spawn_empty().id();
		let chain_b = app.world_mut().spawn_empty().id();
		let capsule_a1 = app.world_mut().spawn(ChainLinkId { link_id: 0, chain_root: chain_a }).id();
		let capsule_a2 = app.world_mut().spawn(ChainLinkId { link_id: 1, chain_root: chain_a }).id();
		let capsule_b1 = app.world_mut().spawn(ChainLinkId { link_id: 0, chain_root: chain_b }).id();

		let projectile = Entity::from_bits(999);

		// Same projectile hits both chains
		app.world_mut().write_message(ProjectileContact {
			projectile,
			source: Some(Entity::from_bits(100)),
			target: capsule_a1,
			point: Vec3::new(1.0, 0.0, 0.0),
			normal: Vec3::Y,
		});
		app.world_mut().write_message(ProjectileContact {
			projectile,
			source: Some(Entity::from_bits(100)),
			target: capsule_a2,
			point: Vec3::new(1.1, 0.0, 0.0),
			normal: Vec3::Y,
		});
		app.world_mut().write_message(ProjectileContact {
			projectile,
			source: Some(Entity::from_bits(100)),
			target: capsule_b1,
			point: Vec3::new(2.0, 0.0, 0.0),
			normal: Vec3::Y,
		});

		app.update();

		let contacts: Vec<_> =
			app.world_mut().resource_mut::<Messages<ProjectileContact>>().drain().collect();

		// Should coalesce to two contacts (one per chain)
		assert_eq!(contacts.len(), 2);
	}
}
