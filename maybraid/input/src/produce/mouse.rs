//! Mouse motion → [`VirtualPad::look_stick`] in pixel-delta units.

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

use crate::pad::VirtualPad;

fn command_held(keys: &ButtonInput<KeyCode>) -> bool {
	keys.pressed(KeyCode::SuperLeft) || keys.pressed(KeyCode::SuperRight)
}

pub fn produce_mouse(
	keyboard: Res<ButtonInput<KeyCode>>,
	mut mouse_motion: MessageReader<MouseMotion>,
	mut pad: ResMut<VirtualPad>,
) {
	if command_held(&keyboard) {
		mouse_motion.clear();
		return;
	}
	let mut delta = Vec2::ZERO;
	for event in mouse_motion.read() {
		delta += event.delta;
	}
	pad.add_look(delta);
}
