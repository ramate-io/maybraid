//! Default follow + optional [`PlayerCameraAim`] blend.

use bevy::prelude::*;
use bevy::transform::helper::TransformHelper;
use crozon_characters::{
	hide_socketed_parts, BoneMap, CharacterMembers, CharacterPartSlot, PartNode,
};
use player::{CameraFollow, PlayerCameraAim, PlayerCameraPose, PlayerLook, PlayerVisual};

use crate::look::{CameraController, CameraPov};
use crate::FollowCamera;

pub(crate) fn follow_character_camera(
	time: Res<Time>,
	players: Query<
		(Entity, &PlayerCameraAim, &PlayerLook),
		(With<CameraFollow>, Without<Camera3d>),
	>,
	visuals: Query<&CharacterMembers, With<PlayerVisual>>,
	maps: Query<&BoneMap>,
	mut transforms: ParamSet<(
		TransformHelper,
		Query<(&mut Transform, &mut CameraController, &FollowCamera), With<Camera3d>>,
	)>,
) {
	let Ok((player, aim, look)) = players.single() else {
		return;
	};
	let current = transforms.p0();
	let Ok(player) = current.compute_global_transform(player) else {
		return;
	};
	let head = head_camera_translation(&visuals, &maps, &current);
	drop(current);
	let mut cameras = transforms.p1();
	let Ok((mut camera_transform, mut controller, follow)) = cameras.single_mut() else {
		return;
	};

	let yaw = Quat::from_axis_angle(Vec3::Y, controller.yaw);
	let pitch = Quat::from_axis_angle(Vec3::X, controller.pitch);
	let look_rotation = yaw * pitch;

	let focus_target = if look.first_person { aim.focus.max(controller.focus) } else { 0.0 };
	controller.focus_blend = focus_blend_toward(
		controller.focus_blend,
		focus_target,
		follow.focus_blend_speed,
		time.delta_secs(),
	);

	let mut pose = match controller.pov {
		CameraPov::ThirdPerson => {
			third_person_pose(player.translation(), yaw, look_rotation, follow)
		}
		CameraPov::FirstPerson => {
			first_person_pose(player.translation(), head, look_rotation, follow)
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
	player: Vec3,
	yaw: Quat,
	look_rotation: Quat,
	follow: &FollowCamera,
) -> PlayerCameraPose {
	let offset = look_rotation * Vec3::new(0.0, 0.0, follow.distance) + Vec3::Y * follow.height;
	let target = player + Vec3::Y * follow.look_height + yaw * Vec3::X * follow.shoulder_offset;
	let mut pose = Transform::from_translation(target + offset);
	pose.look_at(target, Vec3::Y);
	PlayerCameraPose { translation: pose.translation, rotation: pose.rotation }
}

fn first_person_pose(
	player: Vec3,
	head: Option<Vec3>,
	look_rotation: Quat,
	follow: &FollowCamera,
) -> PlayerCameraPose {
	let head_translation = head.unwrap_or(player + Vec3::Y * (follow.height + follow.look_height));
	PlayerCameraPose {
		translation: head_translation + look_rotation * -Vec3::Z * follow.eye_forward,
		rotation: look_rotation,
	}
}

pub fn sync_camera_fov(
	followers: Query<(), With<CameraFollow>>,
	mut cameras: Query<(&CameraController, &FollowCamera, &mut Projection), With<Camera3d>>,
) {
	if followers.is_empty() {
		return;
	}
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

fn focus_blend_toward(current: f32, target: f32, speed: f32, dt: f32) -> f32 {
	let step = 1.0 - (-speed * dt.max(0.0)).exp();
	current + (target - current) * step
}

pub fn sync_first_person_head_visibility(
	followers: Query<(), With<CameraFollow>>,
	cameras: Query<&CameraController, With<Camera3d>>,
	visuals: Query<&CharacterMembers, With<PlayerVisual>>,
	parts: Query<&PartNode>,
	mut visibilities: Query<&mut Visibility>,
) {
	let Ok(controller) = cameras.single() else {
		return;
	};
	let hidden = !followers.is_empty() && controller.pov == CameraPov::FirstPerson;
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
	transforms: &TransformHelper,
) -> Option<Vec3> {
	let members = visuals.single().ok()?;
	let left = member_landmark_translation(members.iter(), maps, transforms, "eye_socket.L")?;
	let right = member_landmark_translation(members.iter(), maps, transforms, "eye_socket.R")?;
	Some((left + right) * 0.5)
}

fn member_landmark_translation(
	members: impl Iterator<Item = Entity>,
	maps: &Query<&BoneMap>,
	transforms: &TransformHelper,
	name: &str,
) -> Option<Vec3> {
	for member in members {
		let Ok(map) = maps.get(member) else {
			continue;
		};
		let Some(&entity) = map.by_name.get(name) else {
			continue;
		};
		if let Ok(global) = transforms.compute_global_transform(entity) {
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
		let midpoint = vertical_fov(CameraPov::FirstPerson, 0.5, &follow);
		assert!((midpoint - (follow.first_person_fov + follow.sight_fov) * 0.5).abs() < 1e-5);
	}

	#[test]
	fn focus_blend_moves_smoothly_to_and_from_sight() {
		let speed = FollowCamera::default().focus_blend_speed;
		let focused = focus_blend_toward(0.0, 1.0, speed, 1.0 / 60.0);
		assert!(focused > 0.0 && focused < 1.0);
		let released = focus_blend_toward(focused, 0.0, speed, 1.0 / 60.0);
		assert!(released >= 0.0 && released < focused);
	}
}
