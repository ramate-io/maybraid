//! Spawn flattened Les Halles hosts from generated developments.

use bevy::prelude::*;
use richmond_building_components::{building_bounds, spawn_building_components};
use richmond_development_models::LesHallesDevelopment;
use richmond_developments::MixedUseLesHallesHost;

#[derive(Component)]
pub struct DevelopmentHostRoot;

pub fn spawn_les_halles_hosts(
	commands: &mut Commands,
	development: &LesHallesDevelopment,
) -> usize {
	let mut n = 0usize;
	for host in development.development.hosts() {
		n += match host {
			MixedUseLesHallesHost::Storey(storey) => {
				let bounds = building_bounds(&storey);
				let entities =
					spawn_building_components(commands, &storey, Transform::IDENTITY, bounds);
				tag_hosts(commands, entities)
			}
			MixedUseLesHallesHost::Stairwell(stairwell) => {
				let bounds = building_bounds(&stairwell);
				let entities =
					spawn_building_components(commands, &stairwell, Transform::IDENTITY, bounds);
				tag_hosts(commands, entities)
			}
			MixedUseLesHallesHost::Roof(roof) => {
				let bounds = building_bounds(&roof);
				let entities =
					spawn_building_components(commands, &roof, Transform::IDENTITY, bounds);
				tag_hosts(commands, entities)
			}
		};
	}
	n
}

fn tag_hosts(commands: &mut Commands, entities: Vec<Entity>) -> usize {
	let n = entities.len();
	for entity in entities {
		commands.entity(entity).insert(DevelopmentHostRoot);
	}
	n
}
