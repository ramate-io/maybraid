//! In-game command line after **`/`**: parses [`crate::PlaygroundCommand`] on Enter.

use std::collections::VecDeque;

use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;
use bevy::prelude::*;

use crate::commands::PlaygroundCommand;

/// Maximum submitted lines kept for ↑ / ↓ recall (oldest dropped first).
pub const COMMAND_HISTORY_MAX: usize = 1024;

#[derive(Resource, Default)]
pub struct TypedCommandLine(pub String);

/// Last command result / `help` output for the in-game console (HUD).
#[derive(Resource, Default)]
pub struct CommandConsoleOutput(pub String);

/// When true, keys append to [`TypedCommandLine`] instead of only moving the camera (press `/` to toggle).
#[derive(Resource, Default)]
pub struct TextEntryFocus(pub bool);

#[derive(Resource, Default)]
pub struct CommandHistory {
	/// Oldest at front, newest at back.
	pub entries: VecDeque<String>,
	/// `None` = editing a fresh line; `Some(i)` = viewing `entries[i]`.
	pub browse: Option<usize>,
	/// Line being edited before first ↑ in this “session”.
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

pub fn toggle_text_entry_focus(
	keyboard: Res<ButtonInput<KeyCode>>,
	mut focus: ResMut<TextEntryFocus>,
) {
	if keyboard.just_pressed(KeyCode::Slash) {
		focus.0 = !focus.0;
		log::debug!("text entry focus: {}", focus.0);
	}
}

pub fn capture_command_line_input(
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
			match PlaygroundCommand::parse_line(&line) {
				Ok(cmd) => {
					cmd.react(&mut commands, &mut console.0);
				}
				Err(e) => {
					log::debug!("command parse error (HUD): {e}");
					console.0 = e;
				}
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
			if ch == '\r' || ch == '\n' {
				continue;
			}
			// `/` toggles focus; do not treat it as part of the command.
			if ch == '/' {
				continue;
			}
			if ch.is_ascii_graphic() || ch == '_' || ch == '-' || ch == ' ' || ch == ',' || ch == '.' {
				// Typing clears “browsing” mode so we edit a new line derived from history.
				if history.browse.is_some() {
					history.browse = None;
					history.draft.clear();
				}
				buffer.0.push(ch);
			}
		}
	}
}
