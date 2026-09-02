//! Default follow + optional [`PlayerCameraAim`] blend.

use bevy::prelude::*;
use crozon_characters::{BoneMap, CharacterMembers, CharacterPartSlot, PartNode};
use maybraid_player::{CameraFollow, PlayerCameraAim, PlayerCameraPose, PlayerLook, PlayerVisual};

use crate::look::{CameraController, CameraPov};

pub(crate) const CAMERA_DISTANCE: f32 = 3.6;
pub(crate) const CAMERA_HEIGHT: f32 = 1.1;
pub(crate) const CAMERA_LOOK_HEIGHT: f32 = 0.65;
const CAMERA_SHOULDER_OFFSET: f32 = 0.7;
const FIRST_PERSON_EYE_FORWARD: f32 = 0.04;
const FOCUS_BLEND_SPEED: f32 = 12.0;
pub(crate) const THIRD_PERSON_FOV: f32 = 45.0_f32.to_radians();
const FIRST_PERSON_FOV: f32 = 75.0_f32.to_radians();
const SIGHT_FOV: f32 = 50.0_f32.to_radians();

pub(crate) fn follow_character_camera(
	time: Res<Time>,
	players: Query<
		(&Transform, &PlayerCameraAim, &PlayerLook),
		(With<CameraFollow>, Without<Camera3d>),
	>,
	visuals: Query<&CharacterMembers, With<PlayerVisual>>,
	maps: Query<&BoneMap>,
	globals: Query<&GlobalTransform, Without<Camera3d>>,
	mut cameras: Query<(&mut Transform, &mut CameraController), With<Camera3d>>,
) {
	let Ok((player, aim, look)) = players.single() else {
		return;
	};
	let Ok((mut camera_transform, mut controller)) = cameras.single_mut() else {
		return;
	};

	let yaw = Quat::from_axis_angle(Vec3::Y, controller.yaw);
	let pitch = Quat::from_axis_angle(Vec3::X, controller.pitch);
	let rotation = yaw * pitch;
	let offset = rotation * Vec3::new(0.0, 0.0, CAMERA_DISTANCE) + Vec3::Y * CAMERA_HEIGHT;
	let target =
		player.translation + Vec3::Y * CAMERA_LOOK_HEIGHT + yaw * Vec3::X * CAMERA_SHOULDER_OFFSET;
	let mut third_person = Transform::from_translation(target + offset);
	third_person.look_at(target, Vec3::Y);
	let third_person =
		PlayerCameraPose { translation: third_person.translation, rotation: third_person.rotation };

	let head_translation = head_camera_translation(&visuals, &maps, &globals)
		.unwrap_or(player.translation + Vec3::Y * (CAMERA_HEIGHT + CAMERA_LOOK_HEIGHT));
	let head = PlayerCameraPose {
		translation: head_translation + rotation * -Vec3::Z * FIRST_PERSON_EYE_FORWARD,
		rotation,
	};

	let focus_target = if look.first_person { aim.focus.max(controller.focus) } else { 0.0 };
	let blend_step = 1.0 - (-FOCUS_BLEND_SPEED * time.delta_secs()).exp();
	controller.focus_blend += (focus_target - controller.focus_blend) * blend_step;

	let mut pose = match controller.pov {
		CameraPov::ThirdPerson => third_person,
		CameraPov::FirstPerson => head,
	};
	if let Some(aim_pose) = aim.pose {
		pose = pose.interpolate(aim_pose, controller.focus_blend);
	}
	*camera_transform = pose.transform();
}

pub fn sync_camera_fov(mut cameras: Query<(&CameraController, &mut Projection), With<Camera3d>>) {
	let Ok((controller, mut projection)) = cameras.single_mut() else {
		return;
	};
	let Projection::Perspective(perspective) = projection.as_mut() else {
		return;
	};
	perspective.fov = vertical_fov(controller.pov, controller.focus_blend);
}

fn vertical_fov(pov: CameraPov, focus_blend: f32) -> f32 {
	match pov {
		CameraPov::ThirdPerson => THIRD_PERSON_FOV,
		CameraPov::FirstPerson => FIRST_PERSON_FOV + (SIGHT_FOV - FIRST_PERSON_FOV) * focus_blend,
	}
}

pub fn sync_first_person_head_visibility(
	cameras: Query<&CameraController, With<Camera3d>>,
	visuals: Query<&CharacterMembers, With<PlayerVisual>>,
	parts: Query<&PartNode>,
	mut visibilities: Query<&mut Visibility>,
) {
	let Ok(controller) = cameras.single() else {
		return;
	};
	let hide = controller.pov == CameraPov::FirstPerson;
	let visibility = if hide { Visibility::Hidden } else { Visibility::Inherited };
	for members in &visuals {
		for member in members.iter() {
			let Ok(part) = parts.get(member) else {
				continue;
			};
			if !is_first_person_hidden_slot(part.slot) {
				continue;
			}
			if let Ok(mut vis) = visibilities.get_mut(member) {
				*vis = visibility;
			}
		}
	}
}

fn is_first_person_hidden_slot(slot: CharacterPartSlot) -> bool {
	matches!(
		slot,
		CharacterPartSlot::HeadMesh
			| CharacterPartSlot::Nose
			| CharacterPartSlot::Mouth
			| CharacterPartSlot::EyeLeft
			| CharacterPartSlot::EyeRight
			| CharacterPartSlot::EarLeft
			| CharacterPartSlot::EarRight
			| CharacterPartSlot::Hair
			| CharacterPartSlot::Horns
	)
}

fn head_camera_translation(
	visuals: &Query<&CharacterMembers, With<PlayerVisual>>,
	maps: &Query<&BoneMap>,
	globals: &Query<&GlobalTransform, Without<Camera3d>>,
) -> Option<Vec3> {
	let members = visuals.single().ok()?;
	let left = member_landmark_translation(members.iter(), maps, globals, "eye_socket.L")?;
	let right = member_landmark_translation(members.iter(), maps, globals, "eye_socket.R")?;
	Some((left + right) * 0.5)
}

fn member_landmark_translation(
	members: impl Iterator<Item = Entity>,
	maps: &Query<&BoneMap>,
	globals: &Query<&GlobalTransform, Without<Camera3d>>,
	name: &str,
) -> Option<Vec3> {
	for member in members {
		let Ok(map) = maps.get(member) else {
			continue;
		};
		let Some(&entity) = map.by_name.get(name) else {
			continue;
		};
		if let Ok(global) = globals.get(entity) {
			return Some(global.translation());
		}
	}
	None
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn first_person_hides_face_not_body() {
		assert!(is_first_person_hidden_slot(CharacterPartSlot::HeadMesh));
		assert!(!is_first_person_hidden_slot(CharacterPartSlot::BodyMesh));
	}

	#[test]
	fn first_person_hipfire_is_wider_than_orbit() {
		assert!(
			vertical_fov(CameraPov::FirstPerson, 0.0) > vertical_fov(CameraPov::ThirdPerson, 0.0)
		);
		assert!((vertical_fov(CameraPov::FirstPerson, 1.0) - SIGHT_FOV).abs() < 1e-5);
	}
}
