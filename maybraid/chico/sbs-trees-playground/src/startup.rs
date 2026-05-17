//! Optional process-argv command run on the first frame.

use bevy::prelude::*;

use crate::commands::root::react_playground_command_root;
use crate::commands::PlaygroundCommand;
use crate::input::CommandConsoleOutput;

#[derive(Resource)]
pub struct PendingStartupCommand(pub Option<PlaygroundCommand>);

impl Default for PendingStartupCommand {
	fn default() -> Self {
		Self(None)
	}
}

pub fn run_pending_startup_command(
	mut pending: ResMut<PendingStartupCommand>,
	mut commands: Commands,
	mut console: ResMut<CommandConsoleOutput>,
) {
	let Some(cmd) = pending.0.take() else {
		return;
	};
	cmd.react(&mut commands, &mut console.0);
}

pub struct StartupPlugin;

impl Plugin for StartupPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, run_pending_startup_command.before(react_playground_command_root));
	}
}
