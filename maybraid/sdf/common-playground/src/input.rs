//! In-game command line after **`/`**: parses [`crate::PlaygroundCommand`] on Enter.

use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;
use bevy::prelude::*;

use crate::commands::PlaygroundCommand;

#[derive(Resource, Default)]
pub struct TypedCommandLine(pub String);

/// Last command result / `help` output for the in-game console (HUD).
#[derive(Resource, Default)]
pub struct CommandConsoleOutput(pub String);

/// When true, keys append to [`TypedCommandLine`] instead of only moving the camera (press `/` to toggle).
#[derive(Resource, Default)]
pub struct TextEntryFocus(pub bool);

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
	mut reader: MessageReader<KeyboardInput>,
	keyboard: Res<ButtonInput<KeyCode>>,
	mut console: ResMut<CommandConsoleOutput>,
	focus: Res<TextEntryFocus>,
) {
	if !focus.0 {
		return;
	}
	if keyboard.just_pressed(KeyCode::Enter) {
		let line = buffer.0.trim();
		if !line.is_empty() {
			match PlaygroundCommand::parse_line(line) {
				Ok(cmd) => {
					cmd.react(&mut commands);
				}
				Err(e) => {
					log::debug!("command parse error (HUD): {e}");
					console.0 = e;
				}
			}
		}
		buffer.0.clear();
		return;
	}
	if keyboard.just_pressed(KeyCode::Backspace) {
		buffer.0.pop();
		return;
	}
	if keyboard.just_pressed(KeyCode::Escape) {
		buffer.0.clear();
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
			if ch.is_ascii_graphic() || ch == '_' || ch == '-' || ch == ' ' {
				buffer.0.push(ch);
			}
		}
	}
}
