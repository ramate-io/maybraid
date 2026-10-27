//! Pure mouse mapping for an explicitly assigned controller.
use crate::VirtualPad;
use bevy::prelude::*;

pub struct MousePad;
impl MousePad {
	pub fn apply(
		keyboard: &ButtonInput<KeyCode>,
		mouse: &ButtonInput<MouseButton>,
		delta: Vec2,
		pad: &mut VirtualPad,
	) {
		if keyboard.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]) {
			return;
		}
		pad.add_look(delta);
		pad.max_triggers(
			if mouse.pressed(MouseButton::Right) { 1.0 } else { 0.0 },
			if mouse.pressed(MouseButton::Left) { 1.0 } else { 0.0 },
		);
	}
}
#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn left_click_sets_trigger_fire() {
		let mut mouse = ButtonInput::default();
		mouse.press(MouseButton::Left);
		let mut pad = VirtualPad::default();
		MousePad::apply(&ButtonInput::default(), &mouse, Vec2::ZERO, &mut pad);
		assert_eq!(pad.trigger_fire, 1.0);
	}
	#[test]
	fn right_click_sets_focus_and_command_chord_suppresses_mouse() {
		let mut mouse = ButtonInput::default();
		mouse.press(MouseButton::Right);
		let mut pad = VirtualPad::default();
		MousePad::apply(&ButtonInput::default(), &mouse, Vec2::X, &mut pad);
		assert_eq!(pad.trigger_focus, 1.0);
		assert_eq!(pad.look_stick, Vec2::X);
		pad.begin_frame();
		let mut keys = ButtonInput::default();
		keys.press(KeyCode::SuperLeft);
		MousePad::apply(&keys, &mouse, Vec2::X, &mut pad);
		assert_eq!(pad.trigger_focus, 0.0);
		assert_eq!(pad.look_stick, Vec2::ZERO);
	}
}
