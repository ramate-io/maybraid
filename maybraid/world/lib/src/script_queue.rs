//! Script command queue with delays and readiness gates (debug-commands feature only).

use std::collections::VecDeque;
use std::time::Duration;

use bevy::prelude::*;
use game_commands::command::CommandConsoleOutput;
use player::Player as MaybraidPlayer;
use world_player::Player as WorldPlayer;

use crate::commands::PlaygroundCommand;

/// Queued script commands awaiting execution.
#[derive(Resource, Default)]
pub struct ScriptQueue {
	pub commands: VecDeque<PlaygroundCommand>,
	pub wait_until: Option<Duration>,
	pub wait_for_world_ready: bool,
}

impl ScriptQueue {
	pub fn enqueue(&mut self, cmd: PlaygroundCommand) {
		self.commands.push_back(cmd);
	}

	pub fn is_waiting(&self) -> bool {
		self.wait_until.is_some() || self.wait_for_world_ready
	}
}

/// Load startup script into queue if it's a Script command.
pub fn load_startup_script(
	mut pending: ResMut<crate::PendingStartupCommand<PlaygroundCommand>>,
	mut queue: ResMut<ScriptQueue>,
	time: Res<Time>,
	mut console: ResMut<CommandConsoleOutput>,
	mut commands: Commands,
) {
	if let Some(cmd) = pending.0.take() {
		if let crate::commands::PlaygroundCommand::Script(script) = &cmd {
			info!("Loading startup script: {}", script.path.display());
			crate::commands::PlaygroundCommand::run_script_with_queue(
				&script.path,
				&mut queue,
				&time,
				&mut console.0,
			);
		} else {
			// Not a script, execute immediately
			cmd.react(&mut commands, &mut console.0);
		}
	}
}

/// Process queued script commands with delays and readiness gates.
pub fn process_script_queue(
	time: Res<Time>,
	mut queue: ResMut<ScriptQueue>,
	player: Query<Entity, (With<MaybraidPlayer>, With<WorldPlayer>)>,
	mut commands: Commands,
	mut console: ResMut<CommandConsoleOutput>,
) {
	// Check if we're waiting for world ready
	if queue.wait_for_world_ready {
		if player.single().is_ok() {
			info!("World ready: player spawned");
			queue.wait_for_world_ready = false;
		} else {
			return;
		}
	}

	// Check if we're waiting for a timer
	if let Some(wait_until) = queue.wait_until {
		if time.elapsed() < wait_until {
			return;
		}
		info!("Wait complete, resuming script");
		queue.wait_until = None;
	}

	// Process next command if not waiting
	if let Some(cmd) = queue.commands.pop_front() {
		// Handle ScriptCtrl commands specially
		if let crate::commands::PlaygroundCommand::ScriptCtrl(script_cmd) = cmd {
			script_cmd.react(&mut queue, &time, &mut console.0);
			// Don't process more commands this frame if we just set a wait state
			if queue.is_waiting() {
				return;
			}
		} else {
			cmd.react(&mut commands, &mut console.0);
		}
	}
}
