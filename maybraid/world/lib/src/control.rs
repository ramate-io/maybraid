//! Apply [`CharacterIntent`] to the vegetation capsule / camera-relative wish.

use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::{
	CameraController, MoveWish, MovementAction, Player, PlaygroundMode,
};
use game_commands::command::{CommandConsoleOutput, TextEntryFocus};
use maybraid_character_controller::CharacterIntent;

use crate::camera::CameraPov;

pub(crate) fn apply_intents_to_movement(
	mode: Res<PlaygroundMode>,
	text_focus: Res<TextEntryFocus>,
	mut intents: MessageReader<CharacterIntent>,
	cameras: Query<&CameraController, With<Camera3d>>,
	mut wishes: Query<&mut MoveWish, With<Player>>,
	mut movement: MessageWriter<MovementAction>,
	mut pov: ResMut<CameraPov>,
) {
	let mut move_stick = Vec2::ZERO;
	let mut jump = false;
	for intent in intents.read() {
		match *intent {
			CharacterIntent::Move(value) => move_stick = value,
			CharacterIntent::Jump => jump = true,
			CharacterIntent::SwapPov => {
				*pov = (*pov).toggle();
			}
			_ => {}
		}
	}

	if *mode != PlaygroundMode::Character || text_focus.0 {
		for mut wish in &mut wishes {
			wish.0 = Vec3::ZERO;
		}
		return;
	}

	let wish_dir = if move_stick != Vec2::ZERO {
		if let Ok(camera) = cameras.single() {
			let yaw = Quat::from_axis_angle(Vec3::Y, camera.yaw);
			let forward = yaw * -Vec3::Z;
			let right_dir = yaw * Vec3::X;
			(right_dir * move_stick.x + forward * move_stick.y).normalize_or_zero()
		} else {
			Vec3::ZERO
		}
	} else {
		Vec3::ZERO
	};
	for mut wish in &mut wishes {
		wish.0 = wish_dir;
	}

	if move_stick != Vec2::ZERO {
		movement.write(MovementAction::Move(move_stick));
	}
	if jump {
		movement.write(MovementAction::Jump);
	}
}

pub(crate) fn echo_character_intents(
	mut intents: MessageReader<CharacterIntent>,
	mut console: ResMut<CommandConsoleOutput>,
) {
	let mut parts = Vec::new();
	for intent in intents.read() {
		parts.push(match *intent {
			CharacterIntent::Move(value) => format!("move=({:.2},{:.2})", value.x, value.y),
			CharacterIntent::Look(value) => format!("look=({:.2},{:.2})", value.x, value.y),
			CharacterIntent::Focus(value) => format!("focus={value:.2}"),
			CharacterIntent::UseItem(value) => format!("use={value:.2}"),
			other => other.label().to_string(),
		});
	}
	if !parts.is_empty() {
		console.0 = parts.join(" ");
	}
}
