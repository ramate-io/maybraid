//! Typed primitive selection (`from_name`) via [`KeyboardInput`](bevy::input::keyboard::KeyboardInput) text.

use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;
use bevy::prelude::*;
use sdf_common::SdfCommonPrimitive;

use crate::preview::PreviewConfig;

#[derive(Resource, Default)]
pub struct TypedSdfName(pub String);

/// When true, letter keys append to [`TypedSdfName`] instead of only moving the camera (press `/` to toggle).
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

pub fn capture_sdf_name_input(
	mut buffer: ResMut<TypedSdfName>,
	mut reader: MessageReader<KeyboardInput>,
	keyboard: Res<ButtonInput<KeyCode>>,
	mut config: ResMut<PreviewConfig>,
	focus: Res<TextEntryFocus>,
) {
	if !focus.0 {
		return;
	}
	if keyboard.just_pressed(KeyCode::Enter) {
		if let Some(p) = SdfCommonPrimitive::from_name(&buffer.0) {
			config.primitive = p;
			log::debug!("typed SDF selection: {}", buffer.0);
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
			if ch.is_ascii_graphic() || ch == '_' {
				buffer.0.push(ch);
			}
		}
	}
}
