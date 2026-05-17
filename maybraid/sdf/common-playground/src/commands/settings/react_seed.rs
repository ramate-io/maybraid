//! Reacts to [`super::SettingsSeed`] leaf commands.

use bevy::prelude::*;

use crate::ground::PlaygroundSettings;
use game_commands::command::CommandConsoleOutput;

#[derive(Component, Clone, Copy, Debug)]
pub struct SettingsSeed {
	pub value: u32,
}

pub fn react_settings_seed(
	mut commands: Commands,
	q: Query<(Entity, &SettingsSeed), Added<SettingsSeed>>,
	mut playground: ResMut<PlaygroundSettings>,
	mut console: ResMut<CommandConsoleOutput>,
) {
	for (entity, cmd) in &q {
		playground.seed = cmd.value;
		log::info!("playground seed set to {}", cmd.value);
		console.0.clear();
		commands.entity(entity).despawn();
	}
}
