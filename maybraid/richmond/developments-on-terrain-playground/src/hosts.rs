//! Spawn complete building hosts from generated developments.

use bevy::prelude::*;
use richmond_development_models::DevelopmentHosts;

#[derive(Component)]
pub struct DevelopmentHostRoot;

pub fn spawn_development_hosts(
	commands: &mut Commands,
	development: &impl DevelopmentHosts,
) -> usize {
	let mut count = 0;
	for host in development.hosts() {
		let entities = host.spawn(commands);
		count += tag_hosts(commands, entities);
	}
	count
}

fn tag_hosts(commands: &mut Commands, entities: Vec<Entity>) -> usize {
	let n = entities.len();
	for entity in entities {
		commands.entity(entity).insert(DevelopmentHostRoot);
	}
	n
}
