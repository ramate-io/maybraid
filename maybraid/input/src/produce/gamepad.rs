//! `Gamepad` → analog sticks / triggers and digital [`PadButton`]s.

use bevy::prelude::*;

use crate::button::PadButton;
use crate::config::VirtualPadConfig;
use crate::pad::VirtualPad;

pub fn produce_gamepad(
	gamepads: Query<&Gamepad>,
	config: Res<VirtualPadConfig>,
	time: Res<Time>,
	mut pad: ResMut<VirtualPad>,
) {
	let dt = time.delta_secs();
	for gamepad in &gamepads {
		let move_stick = config.stick_deadzone.apply_vec2(gamepad.left_stick());
		pad.add_move(move_stick);

		let look = config.stick_deadzone.apply_vec2(gamepad.right_stick());
		// Camera-space: +X look right, +Y look down (matches mouse delta).
		pad.add_look(Vec2::new(look.x, -look.y) * config.gamepad_look_pixels_per_sec * dt);

		let focus = gamepad.get(GamepadButton::LeftTrigger2).unwrap_or(0.0).clamp(0.0, 1.0);
		let fire = gamepad.get(GamepadButton::RightTrigger2).unwrap_or(0.0).clamp(0.0, 1.0);
		pad.max_triggers(focus, fire);

		pad.add_dpad(config.stick_deadzone.apply_vec2(gamepad.dpad()));

		for button in gamepad.get_pressed().copied() {
			if let Some(mapped) = PadButton::from_gamepad(button) {
				pad.hold_digital(mapped);
			}
		}
	}
}
