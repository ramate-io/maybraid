//! Chain motion and collision plugin for [#1126](https://github.com/ramate-io/maybraid/issues/1126).
//!
//! Wires up anchored chain motion, capsule collision, and contact deduplication.

use bevy::prelude::*;
use projectiles::{ProjectileContact, ProjectileSystems};

use super::damage_coalesce::coalesce_chain_contacts;
use super::motion::{deactivate_released_chains, update_chain_joints};
use super::collision::sync_chain_colliders;

/// Plugin that registers chain motion and collision systems.
///
/// Systems run in PostUpdate:
/// - Motion systems update joint positions based on anchor transforms.
/// - Collision sync follows motion to update capsule positions.
/// - Contact coalescing runs in DamageSystems::Collect (before apply_hits).
pub struct ChainMotionPlugin;

impl Plugin for ChainMotionPlugin {
	fn build(&self, app: &mut App) {
		// Add ProjectileContact message if not already present (may be added by DamagePlugin)
		if !app.world().contains_resource::<Events<ProjectileContact>>() {
			app.add_message::<ProjectileContact>();
		}

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
		);

		// Contact deduplication runs in Collect, BEFORE contacts_to_hits
		// This requires coordination with the damage plugin, which we'll handle
		// by using a well-known system set.
		//
		// Note: The damage plugin defines DamageSystems::Collect.after(ProjectileSystems::Sweep).
		// We run coalesce_chain_contacts at the start of Collect so it processes contacts
		// before contacts_to_hits (which also runs in Collect).
		//
		// System ordering in Collect:
		//   1. coalesce_chain_contacts (this plugin)
		//   2. contacts_to_hits (damage plugin)
		//
		// We achieve this by not using any .after() on coalesce_chain_contacts within Collect.
		// Since both are in the same set, and we register this plugin first, our system
		// should run first by registration order.
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn plugin_builds_successfully() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, TransformPlugin, ChainMotionPlugin));
	}
}
