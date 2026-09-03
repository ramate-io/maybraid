//! Spawn complete building hosts from generated developments.

use bevy::prelude::*;
use richmond_building_components::{building_bounds, spawn_building_components};
use richmond_development_models::{LesHallesDevelopment, ShepherdsVillageDevelopment};
use richmond_developments::ShepherdsBuilding;
use std::sync::Arc;

#[derive(Component)]
pub struct DevelopmentHostRoot;

pub fn spawn_les_halles_hosts(
	commands: &mut Commands,
	development: &LesHallesDevelopment,
) -> usize {
	let mut n = 0usize;
	let transform = development.host_transform();
	let dev = &development.development;
	for floor in &dev.tower.floors {
		let storey = Arc::new(floor.clone());
		let bounds = building_bounds(storey.as_ref());
		let entities = spawn_building_components(commands, &storey, transform, bounds);
		n += tag_hosts(commands, entities);
	}
	for stairwell in &dev.stairwells {
		let bounds = building_bounds(stairwell);
		let entities = spawn_building_components(commands, stairwell, transform, bounds);
		n += tag_hosts(commands, entities);
	}
	let bounds = building_bounds(&dev.roof);
	let entities = spawn_building_components(commands, &dev.roof, transform, bounds);
	n += tag_hosts(commands, entities);
	n
}

pub fn spawn_shepherds_village_hosts(
	commands: &mut Commands,
	development: &ShepherdsVillageDevelopment,
) -> usize {
	let mut n = 0usize;
	for placed in &development.village.buildings {
		let transform = ShepherdsVillageDevelopment::host_transform(placed);
		let entities = match &placed.building {
			ShepherdsBuilding::House(house) => {
				let host = Arc::new(house.clone());
				let bounds = building_bounds(host.as_ref());
				spawn_building_components(commands, &host, transform, bounds)
			}
			ShepherdsBuilding::Hut(hut) => {
				let host = Arc::new(hut.clone());
				let bounds = building_bounds(host.as_ref());
				spawn_building_components(commands, &host, transform, bounds)
			}
		};
		n += tag_hosts(commands, entities);
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
