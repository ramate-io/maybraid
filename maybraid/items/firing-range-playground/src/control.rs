//! [`CharacterIntent`] → wish, look, jump, and trigger fire.

use bevy::prelude::*;
use firearms::TriggerFire;
use game_commands::command::TextEntryFocus;
use maybraid_character_controller::CharacterIntent;
use maybraid_input::PadGameplayEnabled;
use std::f32::consts::FRAC_PI_2;

use crate::camera::CameraController;
use crate::character::PlayerVisual;
use crate::player::{CharacterController, MoveWish, MovementAction, Player};

pub(crate) fn gate_pad(focus: Res<TextEntryFocus>, mut enabled: ResMut<PadGameplayEnabled>) {
	enabled.0 = !focus.0;
}

pub(crate) fn apply_intents(
	text_focus: Res<TextEntryFocus>,
	mouse: Res<ButtonInput<MouseButton>>,
	mut intents: MessageReader<CharacterIntent>,
	mut cameras: Query<&mut CameraController, With<Camera3d>>,
	mut wishes: Query<&mut MoveWish, With<CharacterController>>,
	mut movement: MessageWriter<MovementAction>,
	mut trigger: ResMut<TriggerFire>,
) {
	if text_focus.0 {
		for _ in intents.read() {}
		for mut camera in &mut cameras {
			camera.focus = 0.0;
		}
		for mut wish in &mut wishes {
			wish.0 = Vec3::ZERO;
		}
		trigger.0 = false;
		return;
	}

	let mut move_stick = Vec2::ZERO;
	let mut jump = false;
	let mut fire = mouse.pressed(MouseButton::Left);
	let mut focus = f32::from(mouse.pressed(MouseButton::Right));
	let mut swap_pov = false;
	for intent in intents.read() {
		match *intent {
			CharacterIntent::Move(value) => move_stick = value,
			CharacterIntent::Look(value) => {
				if let Ok(mut controller) = cameras.single_mut() {
					controller.yaw -= value.x * controller.sensitivity;
					controller.pitch -= value.y * controller.sensitivity;
					controller.pitch = controller.pitch.clamp(-FRAC_PI_2 + 0.1, FRAC_PI_2 - 0.1);
				}
			}
			CharacterIntent::Jump => jump = true,
			CharacterIntent::Focus(value) => focus = focus.max(value),
			CharacterIntent::UseItem(_) => fire = true,
			CharacterIntent::SwapPov => swap_pov = true,
			_ => {}
		}
	}
	if let Ok(mut controller) = cameras.single_mut() {
		controller.focus = focus.clamp(0.0, 1.0);
		if swap_pov {
			controller.pov.toggle();
		}
	}
	trigger.0 = fire;

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

pub(crate) fn face_player(
	time: Res<Time>,
	wishes: Query<&MoveWish, With<Player>>,
	mut visuals: Query<&mut Transform, With<PlayerVisual>>,
) {
	let Ok(wish) = wishes.single() else {
		return;
	};
	let Ok(mut visual) = visuals.single_mut() else {
		return;
	};
	face_wish(&mut visual, wish.0, time.delta_secs());
}

fn face_wish(visual: &mut Transform, wish: Vec3, dt: f32) {
	const FACE_DEADZONE: f32 = 0.05;
	const TURN_RATE: f32 = 5.5;
	let target = Vec3::new(wish.x, 0.0, wish.z);
	if target.length_squared() < 1e-4 {
		return;
	}
	let target = target.normalize();
	let current = {
		let facing = -visual.forward();
		let xz = Vec3::new(facing.x, 0.0, facing.z);
		if xz.length_squared() < 1e-4 {
			visual.look_to(-target, Vec3::Y);
			return;
		}
		xz.normalize()
	};
	let angle = current.angle_between(target);
	if angle < FACE_DEADZONE {
		return;
	}
	let t = (TURN_RATE * dt / angle).min(1.0);
	visual.look_to(-current.slerp(target, t), Vec3::Y);
}
