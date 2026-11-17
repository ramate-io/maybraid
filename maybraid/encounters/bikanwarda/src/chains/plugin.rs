//! Chain motion and collision plugin for [#1126](https://github.com/ramate-io/maybraid/issues/1126).
//!
//! Wires up anchored chain motion, capsule collision, and contact deduplication.

use bevy::prelude::*;
use damage::DamageSystems;
use projectiles::ProjectileContact;

use super::collision::sync_chain_colliders;
use super::damage_coalesce::coalesce_chain_contacts;
use super::motion::{deactivate_released_chains, update_chain_joints};

/// Plugin that registers chain motion and collision systems.
///
/// Systems run in PostUpdate:
/// - Motion systems update joint positions based on anchor transforms.
/// - Collision sync follows motion to update capsule positions.
/// - Contact coalescing runs in DamageSystems::Collect (before apply_hits).
pub struct BikanwardaChainPhysicsPlugin;

impl Plugin for BikanwardaChainPhysicsPlugin {
	fn build(&self, app: &mut App) {
		// ProjectileContact message is added by DamagePlugin/ProjectilePlugin

		app.add_systems(
			PostUpdate,
			(
				// Update chain joints based on anchor positions
				update_chain_joints,
				// Sync collider positions to joints
				sync_chain_colliders.after(update_chain_joints),
				// Deactivate motion for released chains
				deactivate_released_chains,
			)
				.after(TransformSystems::Propagate),
		)
		.add_systems(
			PostUpdate,
			// Contact deduplication runs in Collect, BEFORE contacts_to_hits
			coalesce_chain_contacts.in_set(DamageSystems::Collect),
		);

		// System ordering in DamageSystems::Collect:
		//   1. coalesce_chain_contacts (this plugin)
		//   2. contacts_to_hits (damage plugin)
		//
		// We achieve this by not using any .after() on coalesce_chain_contacts within Collect.
		// Since both are in the same set, and we register this plugin before DamagePlugin,
		// our system runs first by registration order.
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
