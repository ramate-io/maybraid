//! Look intents, POV, body cone, and copy onto [`PlayerLook`].

use crate::FollowCamera;
use bevy::prelude::*;
use crozon_characters::CharacterHeading;
use maybraid_character_controller::CharacterIntent;
use player::{CameraFollow, PlayerLook, PlayerVisual, PlayerYawOwner};
use std::f32::consts::{FRAC_PI_2, PI};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CameraPov {
	#[default]
	ThirdPerson,
	FirstPerson,
}

impl CameraPov {
	pub fn toggle(&mut self) {
		*self = match self {
			Self::ThirdPerson => Self::FirstPerson,
			Self::FirstPerson => Self::ThirdPerson,
		};
	}
}

#[derive(Component)]
pub struct CameraController {
	pub yaw: f32,
	pub pitch: f32,
	pub pov: CameraPov,
	pub focus: f32,
	pub focus_blend: f32,
}

pub(crate) fn apply_look_intents(
	mouse: Res<ButtonInput<MouseButton>>,
	mut intents: MessageReader<CharacterIntent>,
	mut cameras: Query<(&mut CameraController, &FollowCamera), With<Camera3d>>,
) {
	let mut focus = f32::from(mouse.pressed(MouseButton::Right));
	let mut swap_pov = false;
	for intent in intents.read() {
		match *intent {
			CharacterIntent::Look(value) => {
				if let Ok((mut controller, follow)) = cameras.single_mut() {
					controller.yaw -= value.x * follow.sensitivity;
					controller.pitch -= value.y * follow.sensitivity;
					controller.pitch = controller.pitch.clamp(-FRAC_PI_2 + 0.1, FRAC_PI_2 - 0.1);
				}
			}
			CharacterIntent::Focus(value) => focus = focus.max(value),
			CharacterIntent::SwapPov => swap_pov = true,
			_ => {}
		}
	}
	if let Ok((mut controller, _)) = cameras.single_mut() {
		controller.focus = focus.clamp(0.0, 1.0);
		if swap_pov {
			controller.pov.toggle();
		}
	}
}

pub(crate) fn sync_player_look(
	cameras: Query<&CameraController, With<Camera3d>>,
	mut looks: Query<&mut PlayerLook, With<CameraFollow>>,
) {
	let Ok(controller) = cameras.single() else {
		return;
	};
	for mut look in &mut looks {
		look.yaw = controller.yaw;
		look.pitch = controller.pitch;
		look.first_person = controller.pov == CameraPov::FirstPerson;
		look.focus = controller.focus;
	}
}

pub(crate) fn sync_yaw_owner(
	cameras: Query<&CameraController, With<Camera3d>>,
	followers: Query<Entity, With<CameraFollow>>,
	children: Query<&Children>,
	mut owners: Query<&mut PlayerYawOwner>,
) {
	let Ok(controller) = cameras.single() else {
		return;
	};
	let owner = match controller.pov {
		CameraPov::FirstPerson => PlayerYawOwner::Look,
		CameraPov::ThirdPerson => PlayerYawOwner::Wish,
	};
	for follower in &followers {
		set_yaw_owner(&mut owners, follower, owner);
		if let Ok(children) = children.get(follower) {
			for child in children.iter() {
				set_yaw_owner(&mut owners, child, owner);
			}
		}
	}
}

fn set_yaw_owner(owners: &mut Query<&mut PlayerYawOwner>, entity: Entity, owner: PlayerYawOwner) {
	if let Ok(mut yaw) = owners.get_mut(entity) {
		*yaw = owner;
	}
}

pub(crate) fn turn_body_with_look(
	time: Res<Time>,
	mut cameras: Query<(&mut CameraController, &FollowCamera), With<Camera3d>>,
	mut visuals: Query<
		(Entity, &mut Transform, &mut CharacterHeading),
		(With<PlayerVisual>, Without<Camera3d>),
	>,
	owners: Query<&PlayerYawOwner>,
) {
	let Ok((mut controller, follow)) = cameras.single_mut() else {
		return;
	};
	if controller.pov != CameraPov::FirstPerson {
		return;
	}
	let Ok((entity, mut visual, mut heading)) = visuals.single_mut() else {
		return;
	};
	if owners.get(entity).ok().copied().unwrap_or(PlayerYawOwner::Look) != PlayerYawOwner::Look {
		return;
	}

	let body = body_yaw(&mut heading, &visual);
	let target = follow_body_yaw(controller.yaw, body, follow.max_look_yaw);
	let step = wrap_to_pi(target - body);
	let max_step = follow.body_turn_rate * time.delta_secs();
	let applied = step.abs().min(max_step).copysign(step);
	if applied.abs() > 1e-5 {
		set_body_yaw(&mut heading, &mut visual, body + applied);
	}
	controller.yaw =
		clamp_look_yaw(controller.yaw, body_yaw(&mut heading, &visual), follow.max_look_yaw);
}

fn body_yaw(heading: &mut CharacterHeading, visual: &Transform) -> f32 {
	camera_yaw_of_forward(heading.resolve(visual))
}

fn set_body_yaw(heading: &mut CharacterHeading, visual: &mut Transform, yaw: f32) {
	let forward = Quat::from_axis_angle(Vec3::Y, yaw) * -Vec3::Z;
	heading.set(visual, forward);
}

fn camera_yaw_of_forward(dir: Vec3) -> f32 {
	let xz = Vec3::new(dir.x, 0.0, dir.z);
	if xz.length_squared() < 1e-8 {
		0.0
	} else {
		let n = xz.normalize();
		(-n.x).atan2(-n.z)
	}
}

fn wrap_to_pi(angle: f32) -> f32 {
	(angle + PI).rem_euclid(2.0 * PI) - PI
}

fn follow_body_yaw(look_yaw: f32, body_yaw: f32, max_delta: f32) -> f32 {
	let delta = wrap_to_pi(look_yaw - body_yaw);
	look_yaw - delta.clamp(-max_delta, max_delta)
}

fn clamp_look_yaw(look_yaw: f32, body_yaw: f32, max_delta: f32) -> f32 {
	let delta = wrap_to_pi(look_yaw - body_yaw);
	body_yaw + delta.clamp(-max_delta, max_delta)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn pov_toggle_round_trips() {
		let mut pov = CameraPov::ThirdPerson;
		pov.toggle();
		assert_eq!(pov, CameraPov::FirstPerson);
		pov.toggle();
		assert_eq!(pov, CameraPov::ThirdPerson);
	}

	#[test]
	fn body_stays_put_inside_look_cone() {
		let max = FollowCamera::default().max_look_yaw;
		let look = -FRAC_PI_2 + max * 0.5;
		let body = -FRAC_PI_2;
		assert!((follow_body_yaw(look, body, max) - body).abs() < 1e-5);
	}

	#[test]
	fn body_follows_when_look_exceeds_cone() {
		let max = FollowCamera::default().max_look_yaw;
		let look = -FRAC_PI_2 + max + 0.4;
		let body = -FRAC_PI_2;
		let target = follow_body_yaw(look, body, max);
		assert!(target < look);
		assert!((clamp_look_yaw(look, target, max) - look).abs() < 1e-4);
	}
}
