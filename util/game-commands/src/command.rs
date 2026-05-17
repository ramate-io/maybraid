//! Generic in-game command parsing, text input, scripts, and startup dispatch.

use std::collections::VecDeque;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;
use bevy::prelude::*;
use clap::Parser;

use crate::ui::{GameCommandUiConfig, GameCommandUiPlugin};

pub const COMMAND_HISTORY_MAX: usize = 1024;

#[derive(Resource, Default)]
pub struct TypedCommandLine(pub String);

#[derive(Resource, Default)]
pub struct CommandConsoleOutput(pub String);

#[derive(Resource, Default)]
pub struct TextEntryFocus(pub bool);

#[derive(Resource, Default)]
pub struct CommandHistory {
	pub entries: VecDeque<String>,
	pub browse: Option<usize>,
	pub draft: String,
}

impl CommandHistory {
	fn push_submitted(&mut self, line: String) {
		self.entries.push_back(line);
		while self.entries.len() > COMMAND_HISTORY_MAX {
			self.entries.pop_front();
			match self.browse {
				None => {}
				Some(0) => self.browse = None,
				Some(i) => self.browse = Some(i - 1),
			}
		}
	}

	fn navigate_up(&mut self, buffer: &mut String) {
		if self.entries.is_empty() {
			return;
		}
		if self.browse.is_none() {
			self.draft.clone_from(buffer);
			self.browse = Some(self.entries.len() - 1);
		} else if let Some(i) = self.browse {
			if i > 0 {
				self.browse = Some(i - 1);
			}
		}
		if let Some(i) = self.browse {
			buffer.clone_from(&self.entries[i]);
		}
	}

	fn navigate_down(&mut self, buffer: &mut String) {
		match self.browse {
			None => {}
			Some(i) => {
				if i + 1 < self.entries.len() {
					self.browse = Some(i + 1);
					buffer.clone_from(&self.entries[i + 1]);
				} else {
					self.browse = None;
					buffer.clone_from(&self.draft);
				}
			}
		}
	}
}

#[derive(Resource)]
pub struct PendingStartupCommand<T>(pub Option<T>);

impl<T> Default for PendingStartupCommand<T> {
	fn default() -> Self {
		Self(None)
	}
}

#[derive(Debug, Clone, clap::Args)]
#[command(rename_all = "kebab-case")]
pub struct CommandScript<T> {
	#[arg(long)]
	pub path: PathBuf,
	#[arg(skip)]
	_marker: PhantomData<fn() -> T>,
}

impl<T> CommandScript<T> {
	pub fn new(path: PathBuf) -> Self {
		Self { path, _marker: PhantomData }
	}
}

pub trait GameCommand: Parser + Clone + Send + Sync + 'static {
	const CLI_NAME: &'static str;

	fn react(self, commands: &mut Commands, console: &mut String);

	fn long_help_string() -> String {
		Self::command().render_long_help().to_string()
	}

	fn parse_startup_command() -> Result<Option<Self>, String> {
		let tail: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
		Self::parse_startup_from_argv_tail(tail)
	}

	fn parse_startup_from_argv_tail(tail: Vec<std::ffi::OsString>) -> Result<Option<Self>, String> {
		if tail.is_empty() {
			return Ok(None);
		}
		let mut args = vec![std::ffi::OsString::from(Self::CLI_NAME)];
		args.extend(tail);
		Self::try_parse_from(args).map(Some).map_err(|e| e.to_string())
	}

	fn parse_line(line: &str) -> Result<Self, String> {
		let line = line.trim().trim_start_matches('/').trim();
		let mut args: Vec<String> = vec![Self::CLI_NAME.to_string()];
		for w in line.split_whitespace() {
			if !w.is_empty() {
				args.push(w.to_string());
			}
		}
		if args.len() <= 1 {
			return Err("empty command".into());
		}
		Self::try_parse_from(args).map_err(|e| e.to_string())
	}
}

impl<T: GameCommand> CommandScript<T> {
	pub fn run(&self, commands: &mut Commands, console: &mut String) {
		run_script_file::<T>(&self.path, commands, console);
	}
}

pub fn read_script_lines(path: &Path) -> Result<Vec<String>, String> {
	let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
	Ok(text
		.lines()
		.map(str::trim)
		.filter(|line| !line.is_empty() && !line.starts_with('#'))
		.map(str::to_string)
		.collect())
}

pub fn run_script_file<T: GameCommand>(path: &Path, commands: &mut Commands, console: &mut String) {
	let lines = match read_script_lines(path) {
		Ok(l) => l,
		Err(e) => {
			*console = e;
			return;
		}
	};
	for (idx, line) in lines.iter().enumerate() {
		match T::parse_line(line) {
			Ok(cmd) => cmd.react(commands, console),
			Err(e) => {
				*console = format!("{} line {}: {e}", path.display(), idx + 1);
			}
		}
	}
}

pub fn toggle_text_entry_focus(
	keyboard: Res<ButtonInput<KeyCode>>,
	mut focus: ResMut<TextEntryFocus>,
) {
	if keyboard.just_pressed(KeyCode::Slash) {
		focus.0 = !focus.0;
	}
}

pub fn capture_command_line_input<T: GameCommand>(
	mut commands: Commands,
	mut buffer: ResMut<TypedCommandLine>,
	mut history: ResMut<CommandHistory>,
	mut reader: MessageReader<KeyboardInput>,
	keyboard: Res<ButtonInput<KeyCode>>,
	mut console: ResMut<CommandConsoleOutput>,
	focus: Res<TextEntryFocus>,
) {
	if !focus.0 {
		return;
	}

	let shift = keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);

	if !shift && keyboard.just_pressed(KeyCode::ArrowUp) {
		history.navigate_up(&mut buffer.0);
		return;
	}
	if !shift && keyboard.just_pressed(KeyCode::ArrowDown) {
		history.navigate_down(&mut buffer.0);
		return;
	}

	if keyboard.just_pressed(KeyCode::Enter) {
		let line = buffer.0.trim().to_string();
		if !line.is_empty() {
			history.push_submitted(line.clone());
			match T::parse_line(&line) {
				Ok(cmd) => cmd.react(&mut commands, &mut console.0),
				Err(e) => console.0 = e,
			}
		}
		buffer.0.clear();
		history.browse = None;
		history.draft.clear();
		return;
	}
	if keyboard.just_pressed(KeyCode::Backspace) {
		buffer.0.pop();
		return;
	}
	if keyboard.just_pressed(KeyCode::Escape) {
		buffer.0.clear();
		history.browse = None;
		history.draft.clear();
		return;
	}

	for ev in reader.read() {
		if ev.state != ButtonState::Pressed || ev.repeat {
			continue;
		}
		let Some(t) = ev.text.as_ref() else {
			continue;
		};
		for ch in t.chars() {
			if ch == '\r' || ch == '\n' || ch == '/' {
				continue;
			}
			if ch.is_ascii_graphic()
				|| ch == '_' || ch == '-'
				|| ch == ' ' || ch == ','
				|| ch == '.'
			{
				if history.browse.is_some() {
					history.browse = None;
					history.draft.clear();
				}
				buffer.0.push(ch);
			}
		}
	}
}

pub fn run_pending_startup_command<T: GameCommand>(
	mut pending: ResMut<PendingStartupCommand<T>>,
	mut commands: Commands,
	mut console: ResMut<CommandConsoleOutput>,
) {
	let Some(cmd) = pending.0.take() else {
		return;
	};
	cmd.react(&mut commands, &mut console.0);
}

pub struct GameCommandPlugin<T> {
	pub ui_config: GameCommandUiConfig,
	_marker: PhantomData<fn() -> T>,
}

impl<T> GameCommandPlugin<T> {
	pub fn with_config(ui_config: GameCommandUiConfig) -> Self {
		Self { ui_config, _marker: PhantomData }
	}
}

impl<T> Default for GameCommandPlugin<T> {
	fn default() -> Self {
		Self::with_config(GameCommandUiConfig::default())
	}
}

impl<T: GameCommand> Plugin for GameCommandPlugin<T> {
	fn build(&self, app: &mut App) {
		app.init_resource::<TypedCommandLine>()
			.init_resource::<TextEntryFocus>()
			.init_resource::<CommandConsoleOutput>()
			.init_resource::<CommandHistory>()
			.init_resource::<PendingStartupCommand<T>>()
			.add_plugins(GameCommandUiPlugin { config: self.ui_config.clone() })
			.add_systems(
				Update,
				(
					toggle_text_entry_focus,
					capture_command_line_input::<T>,
					run_pending_startup_command::<T>,
				),
			);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn read_script_lines_skips_comments_and_blank() -> anyhow::Result<()> {
		let path = std::env::temp_dir().join("game_commands_script_read_test.txt");
		std::fs::write(&path, "\n# c\n  render tapered-cylinder  \n")?;
		let lines = read_script_lines(&path).map_err(|e| anyhow::anyhow!("{e}"))?;
		std::fs::remove_file(&path).ok();
		assert_eq!(lines, vec!["render tapered-cylinder"]);
		Ok(())
	}
}
