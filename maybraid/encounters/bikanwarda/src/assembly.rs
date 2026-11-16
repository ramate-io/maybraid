//! Boss assembly spawn system for HCSG hosts.

use bevy::prelude::*;
use bikanwarda_character::{spawn_bikanwarda_encounter, BikanwardaRoot};

use crate::presentation::RequestBossAssembly;

/// System to spawn actual boss assemblies for HCSG hosts requesting them.
pub fn spawn_boss_assemblies(
	mut commands: Commands,
	requests: Query<(Entity, &RequestBossAssembly, &Transform), Without<BikanwardaRoot>>,
) {
	for (host_entity, request, transform) in &requests {
		let boss_entity = spawn_bikanwarda_encounter(&mut commands, &request.spec, *transform);
		
		// Parent the spawned boss to the host (use ChildOf component)
		commands.entity(boss_entity).insert(ChildOf(host_entity));
		
		// Remove the request marker
		commands.entity(host_entity).remove::<RequestBossAssembly>();
	}
}

/// Plugin to handle boss assembly spawning.
pub struct BossAssemblyPlugin;

impl Plugin for BossAssemblyPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, spawn_boss_assemblies);
	}
}
