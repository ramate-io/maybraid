//! Spawn complete building hosts from generated developments.

use bevy::prelude::*;
use furniture_assemblies::paint_host_furniture;
use richmond_building_components::FurnitureNode;
use richmond_development_models::DevelopmentHosts;

#[derive(Component)]
pub struct DevelopmentHostRoot;

pub fn spawn_development_hosts(
	commands: &mut Commands,
	development: &impl DevelopmentHosts,
) -> usize {
	spawn_tagged_host_entities(commands, development).len()
}

/// Spawn each host, tag [`DevelopmentHostRoot`], and paint furniture kits.
pub fn spawn_tagged_host_entities(
	commands: &mut Commands,
	development: &impl DevelopmentHosts,
) -> Vec<Entity> {
	let mut spawned = Vec::new();
	for host in development.hosts() {
		let slots = host.furniture_nodes();
		for entity in host.spawn(commands) {
			tag_painted_host(commands, entity, &slots);
			spawned.push(entity);
		}
	}
	spawned
}

fn tag_painted_host(commands: &mut Commands, entity: Entity, slots: &[FurnitureNode]) {
	commands.entity(entity).insert(DevelopmentHostRoot);
	paint_host_furniture(commands, entity, slots);
}
