//! Default follow + optional [`PlayerCameraAim`] blend.

use bevy::prelude::*;
use crozon_characters::{
	hide_socketed_parts, BoneMap, CharacterMembers, CharacterPartSlot, PartNode,
};
use player::{CameraFollow, PlayerCameraAim, PlayerCameraPose, PlayerLook, PlayerVisual};

use crate::look::{CameraController, CameraPov};
use crate::FollowCamera;

pub(crate) fn follow_character_camera(
	time: Res<Time>,
	players: Query<
		(&Transform, &PlayerCameraAim, &PlayerLook),
		(With<CameraFollow>, Without<Camera3d>),
	>,
	visuals: Query<&CharacterMembers, With<PlayerVisual>>,
	maps: Query<&BoneMap>,
	globals: Query<&GlobalTransform, Without<Camera3d>>,
	mut cameras: Query<(&mut Transform, &mut CameraController, &FollowCamera), With<Camera3d>>,
) {
	let Ok((player, aim, look)) = players.single() else {
		return;
	};
	let Ok((mut camera_transform, mut controller, follow)) = cameras.single_mut() else {
		return;
	};

	let yaw = Quat::from_axis_angle(Vec3::Y, controller.yaw);
	let pitch = Quat::from_axis_angle(Vec3::X, controller.pitch);
	let look_rotation = yaw * pitch;

	let focus_target = if look.first_person { aim.focus.max(controller.focus) } else { 0.0 };
	let blend_step = 1.0 - (-follow.focus_blend_speed * time.delta_secs()).exp();
	controller.focus_blend += (focus_target - controller.focus_blend) * blend_step;

	let mut pose = match controller.pov {
		CameraPov::ThirdPerson => third_person_pose(player, yaw, look_rotation, follow),
		CameraPov::FirstPerson => {
			first_person_pose(player, look_rotation, follow, &visuals, &maps, &globals)
		}
	};
	if controller.pov == CameraPov::FirstPerson {
		if let Some(sight) = aim.pose {
			pose = pose.interpolate(sight, controller.focus_blend);
		}
	}
	*camera_transform = pose.transform();
}

fn third_person_pose(
	player: &Transform,
	yaw: Quat,
	look_rotation: Quat,
	follow: &FollowCamera,
) -> PlayerCameraPose {
	let offset = look_rotation * Vec3::new(0.0, 0.0, follow.distance) + Vec3::Y * follow.height;
	let target =
		player.translation + Vec3::Y * follow.look_height + yaw * Vec3::X * follow.shoulder_offset;
	let mut pose = Transform::from_translation(target + offset);
	pose.look_at(target, Vec3::Y);
	PlayerCameraPose { translation: pose.translation, rotation: pose.rotation }
}

fn first_person_pose(
	player: &Transform,
	look_rotation: Quat,
	follow: &FollowCamera,
	visuals: &Query<&CharacterMembers, With<PlayerVisual>>,
	maps: &Query<&BoneMap>,
	globals: &Query<&GlobalTransform, Without<Camera3d>>,
) -> PlayerCameraPose {
	let head_translation = head_camera_translation(visuals, maps, globals)
		.unwrap_or(player.translation + Vec3::Y * (follow.height + follow.look_height));
	PlayerCameraPose {
		translation: head_translation + look_rotation * -Vec3::Z * follow.eye_forward,
		rotation: look_rotation,
	}
}

pub fn sync_camera_fov(
	mut cameras: Query<(&CameraController, &FollowCamera, &mut Projection), With<Camera3d>>,
) {
	let Ok((controller, follow, mut projection)) = cameras.single_mut() else {
		return;
	};
	let Projection::Perspective(perspective) = projection.as_mut() else {
		return;
	};
	perspective.fov = vertical_fov(controller.pov, controller.focus_blend, follow);
}

fn vertical_fov(pov: CameraPov, focus_blend: f32, follow: &FollowCamera) -> f32 {
	match pov {
		CameraPov::ThirdPerson => follow.third_person_fov,
		CameraPov::FirstPerson => {
			follow.first_person_fov + (follow.sight_fov - follow.first_person_fov) * focus_blend
		}
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
	let hidden = controller.pov == CameraPov::FirstPerson;
	for members in &visuals {
		hide_socketed_parts(
			members,
			&parts,
			&mut visibilities,
			CharacterPartSlot::hides_in_first_person,
			hidden,
		);
	}
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
		assert!(CharacterPartSlot::HeadMesh.hides_in_first_person());
		assert!(!CharacterPartSlot::BodyMesh.hides_in_first_person());
	}

	#[test]
	fn first_person_hipfire_is_wider_than_orbit() {
		let follow = FollowCamera::default();
		assert!(
			vertical_fov(CameraPov::FirstPerson, 0.0, &follow)
				> vertical_fov(CameraPov::ThirdPerson, 0.0, &follow)
		);
		assert!(
			(vertical_fov(CameraPov::FirstPerson, 1.0, &follow) - follow.sight_fov).abs() < 1e-5
		);
	}
}
