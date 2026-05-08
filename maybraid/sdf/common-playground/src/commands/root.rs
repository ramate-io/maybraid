//! Root [`super::PlaygroundCommand`] announcement entities (`help`, and per-branch spawns).

use bevy::prelude::*;

use crate::commands::PlaygroundCommand;
use crate::input::CommandConsoleOutput;

/// Handles `help` text and despawns root command entities after this frame’s routing.
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
