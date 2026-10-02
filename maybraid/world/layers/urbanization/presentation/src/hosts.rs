//! Spawn complete building hosts from generated developments.

use bevy::prelude::*;
use richmond::DevelopmentHosts;

#[derive(Component)]
pub struct DevelopmentHostRoot;

/// Spawn each host and tag [`DevelopmentHostRoot`]. Furniture presents on its
/// own 50 m cell hosts — do not parent kits here.
pub fn spawn_tagged_host_entities(
	commands: &mut Commands,
	development: &impl DevelopmentHosts,
) -> Vec<Entity> {
	let mut spawned = Vec::new();
	for host in development.hosts() {
		for entity in host.spawn(commands) {
			commands.entity(entity).insert(DevelopmentHostRoot);
			spawned.push(entity);
		}
	}
	spawned
}

pub fn spawn_development_hosts(
	commands: &mut Commands,
	development: &impl DevelopmentHosts,
) -> usize {
	spawn_tagged_host_entities(commands, development).len()
}
