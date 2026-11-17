//! Chain motion and collision plugin for [#1126](https://github.com/ramate-io/maybraid/issues/1126)
//! and [#1127](https://github.com/ramate-io/maybraid/issues/1127).
//!
//! Wires up anchored chain motion, capsule collision, contact deduplication, and durability/break logic.

use bevy::prelude::*;
use damage::{contacts_to_hits, DamageSystems};

use super::collision::{retire_released_chain_colliders, sync_chain_colliders};
use super::damage::{detect_and_signal_broken_chains, update_chain_durability_fraction, ChainBrokenHistory};
use super::damage_coalesce::{cleanup_despawned_projectiles, coalesce_chain_contacts, ChainDamageHistory};
use super::motion::{deactivate_released_chains, update_chain_joints};

/// Plugin that registers chain motion, collision, and damage systems.
///
/// Systems run in PostUpdate:
/// - Motion systems update joint positions based on anchor transforms.
/// - Collision sync follows motion to update capsule positions.
/// - Contact coalescing runs in DamageSystems::Collect (before apply_hits).
/// - Break detection runs after DamageSystems::Apply to check Health and emit ChainBroken signals.
pub struct BikanwardaChainPhysicsPlugin;

impl Plugin for BikanwardaChainPhysicsPlugin {
	fn build(&self, app: &mut App) {
		// ProjectileContact message is added by DamagePlugin/ProjectilePlugin

		app.init_resource::<ChainDamageHistory>()
			.init_resource::<ChainBrokenHistory>()
			.add_systems(
				PostUpdate,
				(
					// Update chain joints based on anchor positions
					update_chain_joints,
					// Sync collider positions to joints
					sync_chain_colliders.after(update_chain_joints),
					// Deactivate motion for released chains
					deactivate_released_chains,
					// Retire colliders for released chains
					retire_released_chain_colliders.after(deactivate_released_chains),
					// Clean up despawned projectiles from history
					cleanup_despawned_projectiles,
				)
					.after(TransformSystems::Propagate),
			)
		.add_systems(
			PostUpdate,
			// Contact deduplication runs in Collect, explicitly BEFORE contacts_to_hits
			coalesce_chain_contacts
				.in_set(DamageSystems::Collect)
				.before(contacts_to_hits),
		)
		.add_systems(
			PostUpdate,
			(
				// Update durability fraction for material systems
				update_chain_durability_fraction,
				// Break detection runs after damage is applied
				detect_and_signal_broken_chains,
			)
				.after(DamageSystems::Apply),
		);

		// System ordering:
		//   DamageSystems::Collect: coalesce_chain_contacts → contacts_to_hits
		//   DamageSystems::Apply: apply_hits (routes hits from ChildOf link colliders to chain roots)
		//   After DamageSystems::Apply: detect_and_signal_broken_chains (checks Health)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn plugin_builds_successfully() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, TransformPlugin, BikanwardaChainPhysicsPlugin));
	}
}
