//! Move / jump from [`CharacterIntent`]. Look and use-item belong to other crates.

use bevy::prelude::*;
use maybraid_character_controller::CharacterIntent;

use crate::body::{CharacterController, MoveWish, MovementAction};
use crate::identity::{Player, PlayerLook};

pub(crate) fn apply_move_intents(
	mut intents: MessageReader<CharacterIntent>,
	mut wishes: Query<(&mut MoveWish, &PlayerLook), (With<CharacterController>, With<Player>)>,
	mut movement: MessageWriter<MovementAction>,
) {
	let mut move_stick = Vec2::ZERO;
	let mut jump = false;
	for intent in intents.read() {
		match *intent {
			CharacterIntent::Move(value) => move_stick = value,
			CharacterIntent::Jump => jump = true,
			_ => {}
		}
	}

	for (mut wish, look) in &mut wishes {
		wish.0 = if move_stick == Vec2::ZERO {
			Vec3::ZERO
		} else {
			let yaw = Quat::from_axis_angle(Vec3::Y, look.yaw);
			let forward = yaw * -Vec3::Z;
			let right = yaw * Vec3::X;
			(right * move_stick.x + forward * move_stick.y).normalize_or_zero()
		};
	}

	if move_stick != Vec2::ZERO {
		movement.write(MovementAction::Move(move_stick));
	}
	if jump {
		movement.write(MovementAction::Jump);
	}
}
