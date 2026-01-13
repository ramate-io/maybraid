use bevy::prelude::*;
use skill_map::{viewport::SkillMapViewportId, SkillMapId};

pub fn skill_map_playground(mut commands: Commands) {
	log::info!("Spawning skill map playground");

	commands.spawn((SkillMapId(0), SkillMapViewportId(0)));
}
