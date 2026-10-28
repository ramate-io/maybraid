//! Pure physical-key mapping for an explicitly assigned controller.
use crate::{PadButton, PadInputPolicy, VirtualPad};
use bevy::prelude::*;

pub struct KeyboardPad;
impl KeyboardPad {
	pub fn apply(keyboard: &ButtonInput<KeyCode>, policy: PadInputPolicy, pad: &mut VirtualPad) {
		if policy.keyboard_menu() {
			Self::menu(keyboard, pad);
		} else {
			Self::gameplay(keyboard, pad);
		}
		pad.keys = keyboard.clone();
	}
	fn gameplay(keyboard: &ButtonInput<KeyCode>, pad: &mut VirtualPad) {
		let mut move_stick = Vec2::ZERO;
		if keyboard.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
			move_stick.y += 1.0;
		}
		if keyboard.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) {
			move_stick.y -= 1.0;
		}
		if keyboard.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
			move_stick.x += 1.0;
		}
		if keyboard.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
			move_stick.x -= 1.0;
		}
		pad.add_move(move_stick.clamp_length_max(1.0));
		Self::hold_arrows(keyboard, pad);

		if keyboard.pressed(KeyCode::Space) {
			pad.hold_digital(PadButton::A);
		}
		if keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
			pad.hold_digital(PadButton::StickClickMove);
		}
		if keyboard.pressed(KeyCode::Escape) {
			pad.hold_digital(PadButton::B);
		}
		if keyboard.pressed(KeyCode::Enter) {
			pad.hold_digital(PadButton::Start);
		}
		if keyboard.pressed(KeyCode::Tab) {
			pad.hold_digital(PadButton::Select);
		}
		if keyboard.pressed(KeyCode::KeyC) {
			pad.hold_digital(PadButton::BumperFocus);
		}
		// V is RB: hold to steer the skill map, then flick a stick.
		if keyboard.pressed(KeyCode::KeyV) {
			pad.hold_digital(PadButton::BumperFire);
		}
		if keyboard.pressed(KeyCode::KeyE) || keyboard.pressed(KeyCode::KeyX) {
			pad.hold_digital(PadButton::X);
		}
		if keyboard.pressed(KeyCode::KeyY) {
			pad.hold_digital(PadButton::Y);
		}
		pad.max_triggers(
			if keyboard.pressed(KeyCode::KeyF) { 1.0 } else { 0.0 },
			if keyboard.pressed(KeyCode::KeyG) { 1.0 } else { 0.0 },
		);
		// Brackets cycle the active equipped queue in character intent production.
	}
	fn menu(keyboard: &ButtonInput<KeyCode>, pad: &mut VirtualPad) {
		Self::hold_arrows(keyboard, pad);
		if keyboard.pressed(KeyCode::Escape) {
			pad.hold_digital(PadButton::B);
		}
		if keyboard.pressed(KeyCode::Enter) {
			pad.hold_digital(PadButton::Start);
		}
		if keyboard.pressed(KeyCode::KeyE) || keyboard.pressed(KeyCode::KeyX) {
			pad.hold_digital(PadButton::X);
		}
		if keyboard.pressed(KeyCode::KeyY) {
			pad.hold_digital(PadButton::Y);
		}
	}
	fn hold_arrows(keyboard: &ButtonInput<KeyCode>, pad: &mut VirtualPad) {
		let mut dpad = Vec2::ZERO;
		if keyboard.pressed(KeyCode::ArrowUp) {
			dpad.y += 1.0;
			pad.hold_digital(PadButton::DpadUp);
		}
		if keyboard.pressed(KeyCode::ArrowDown) {
			dpad.y -= 1.0;
			pad.hold_digital(PadButton::DpadDown);
		}
		if keyboard.pressed(KeyCode::ArrowRight) {
			dpad.x += 1.0;
			pad.hold_digital(PadButton::DpadRight);
		}
		if keyboard.pressed(KeyCode::ArrowLeft) {
			dpad.x -= 1.0;
			pad.hold_digital(PadButton::DpadLeft);
		}
		pad.add_dpad(dpad);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	fn sample(keys: &[KeyCode], policy: PadInputPolicy) -> VirtualPad {
		let mut keyboard = ButtonInput::default();
		for key in keys {
			keyboard.press(*key);
		}
		let mut pad = VirtualPad::default();
		KeyboardPad::apply(&keyboard, policy, &mut pad);
		pad
	}
	#[test]
	fn shift_holds_l3() {
		for key in [KeyCode::ShiftLeft, KeyCode::ShiftRight] {
			assert!(
				sample(&[key], PadInputPolicy::gameplay()).digital_held(PadButton::StickClickMove)
			);
		}
	}
	#[test]
	fn wasd_builds_unit_move() {
		let pad = sample(&[KeyCode::KeyW, KeyCode::KeyD], PadInputPolicy::gameplay());
		assert!((pad.move_stick.length() - 1.0).abs() < 1e-5);
	}
	#[test]
	fn y_holds_swap_active() {
		assert!(sample(&[KeyCode::KeyY], PadInputPolicy::gameplay()).digital_held(PadButton::Y));
	}
	#[test]
	fn e_and_x_hold_interact() {
		for key in [KeyCode::KeyE, KeyCode::KeyX] {
			assert!(sample(&[key], PadInputPolicy::gameplay()).digital_held(PadButton::X));
		}
	}
	#[test]
	fn v_holds_right_bumper() {
		assert!(sample(&[KeyCode::KeyV], PadInputPolicy::gameplay())
			.digital_held(PadButton::BumperFire));
	}
	#[test]
	fn escape_is_the_only_keyboard_back() {
		assert!(!sample(&[KeyCode::KeyB], PadInputPolicy::gameplay()).digital_held(PadButton::B));
		assert!(sample(&[KeyCode::KeyB, KeyCode::Escape], PadInputPolicy::gameplay())
			.digital_held(PadButton::B));
	}
	#[test]
	fn menu_map_uses_arrows_not_wasd() {
		let pad = sample(
			&[KeyCode::KeyW, KeyCode::KeyA, KeyCode::Space, KeyCode::KeyB],
			PadInputPolicy::menu(),
		);
		assert_eq!(pad.move_stick, Vec2::ZERO);
		assert_eq!(pad.dpad, Vec2::ZERO);
		assert!(!pad.digital_held(PadButton::A));
		assert!(!pad.digital_held(PadButton::B));
		let pad = sample(&[KeyCode::ArrowUp, KeyCode::Escape], PadInputPolicy::menu());
		assert!(pad.digital_held(PadButton::DpadUp));
		assert!(pad.digital_held(PadButton::B));
		assert!(pad.dpad.y > 0.0);
	}
	#[test]
	fn menu_map_e_and_x_hold_drop() {
		for key in [KeyCode::KeyE, KeyCode::KeyX] {
			assert!(sample(&[key], PadInputPolicy::menu()).digital_held(PadButton::X));
		}
	}
	#[test]
	fn menu_map_y_holds_unequip() {
		assert!(sample(&[KeyCode::KeyY], PadInputPolicy::menu()).digital_held(PadButton::Y));
	}
}
