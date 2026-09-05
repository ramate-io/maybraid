//! Move / jump from [`CharacterIntent`]. Look and use-item belong to other crates.

use bevy::prelude::*;
use maybraid_character_controller::CharacterIntent;

use crate::body::{CharacterController, JumpWish, MoveWish};
use crate::identity::{Player, PlayerLook};

pub(crate) fn apply_move_intents(
	mut commands: Commands,
	mut intents: MessageReader<CharacterIntent>,
	mut wishes: Query<
		(Entity, &mut MoveWish, &PlayerLook),
		(With<CharacterController>, With<Player>),
	>,
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

	for (entity, mut wish, look) in &mut wishes {
		wish.0 = look_wish(look.yaw, move_stick);
		if jump {
			commands.entity(entity).insert(JumpWish);
		}
	}
}

fn look_wish(yaw: f32, stick: Vec2) -> Vec3 {
	if stick == Vec2::ZERO {
		return Vec3::ZERO;
	}
	let yaw = Quat::from_axis_angle(Vec3::Y, yaw);
	let forward = yaw * -Vec3::Z;
	let right = yaw * Vec3::X;
	(right * stick.x + forward * stick.y).normalize_or_zero()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn look_wish_is_camera_relative_xz() {
		let forward = look_wish(0.0, Vec2::Y);
		assert!((forward.z + 1.0).abs() < 1e-4, "{forward}");
		assert!(forward.y.abs() < 1e-6);
		let right = look_wish(0.0, Vec2::X);
		assert!((right.x - 1.0).abs() < 1e-4, "{right}");
		assert!(right.y.abs() < 1e-6);
	}
}
