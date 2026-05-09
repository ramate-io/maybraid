use bevy::prelude::*;

use crate::commands::PlaygroundCommand;
use crate::input::CommandConsoleOutput;

pub fn react_playground_command_root(
	mut commands: Commands,
	q: Query<(Entity, &PlaygroundCommand), Added<PlaygroundCommand>>,
	mut console: ResMut<CommandConsoleOutput>,
) {
	for (entity, cmd) in &q {
		if matches!(cmd, PlaygroundCommand::Help) {
			console.0 = PlaygroundCommand::long_help_string();
		}
		commands.entity(entity).despawn();
	}
}
