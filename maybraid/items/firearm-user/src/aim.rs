//! Write [`PlayerCameraAim`] from the held firearm's sight socket.

use bevy::prelude::*;
use crozon_characters::BoneMap;
use firearms::{FirearmMembers, FirearmRoot};
use player::{PlayerCameraAim, PlayerCameraPose, PlayerLook};

use crate::pose::HeldFirearm;
use crate::FirearmUser;

pub(crate) fn write_sight_aim(
	mut users: Query<(&FirearmUser, &PlayerLook, &mut PlayerCameraAim)>,
	guns: Query<
		(&FirearmMembers, &Transform, &GlobalTransform),
		(With<HeldFirearm>, With<FirearmRoot>),
	>,
	maps: Query<&BoneMap>,
	globals: Query<&GlobalTransform, Without<Camera3d>>,
) {
	for (user, look, mut aim) in &mut users {
		if !look.first_person {
			aim.pose = None;
			aim.focus = 0.0;
			continue;
		}
		aim.focus = look.focus;
		aim.pose =
			sight_camera_pose(user.held, user.settings.sight_camera_back, &guns, &maps, &globals);
	}
}

fn sight_camera_pose(
	held: Entity,
	sight_camera_back: f32,
	guns: &Query<
		(&FirearmMembers, &Transform, &GlobalTransform),
		(With<HeldFirearm>, With<FirearmRoot>),
	>,
	maps: &Query<&BoneMap>,
	globals: &Query<&GlobalTransform, Without<Camera3d>>,
) -> Option<PlayerCameraPose> {
	let (members, current_root, previous_root) = guns.get(held).ok()?;
	let previous_socket =
		member_landmark_global(members.iter(), maps, globals, "sight_camera_socket")?;
	let socket_local = previous_root.affine().inverse() * previous_socket.affine();
	let socket_current = current_root.compute_affine() * socket_local;
	let (_, _, translation) = socket_current.to_scale_rotation_translation();
	let bore = (current_root.rotation * Vec3::Z).normalize_or(Vec3::Z);
	let look = bore.normalize_or(Vec3::Z);
	let mut aimed = Transform::from_translation(translation - look * sight_camera_back);
	aimed.look_to(look, Vec3::Y);
	Some(PlayerCameraPose { translation: aimed.translation, rotation: aimed.rotation })
}

fn member_landmark_global(
	members: impl Iterator<Item = Entity>,
	maps: &Query<&BoneMap>,
	globals: &Query<&GlobalTransform, Without<Camera3d>>,
	name: &str,
) -> Option<GlobalTransform> {
	for member in members {
		let Ok(map) = maps.get(member) else {
			continue;
		};
		let Some(&entity) = map.by_name.get(name) else {
			continue;
		};
		if let Ok(global) = globals.get(entity) {
			return Some(*global);
		}
	}
	None
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::FirearmUserSettings;

	#[test]
	fn sight_camera_sits_behind_socket() {
		let back = FirearmUserSettings::default().sight_camera_back;
		let bore = Vec3::X;
		let socket = Vec3::new(1.0, 1.5, 0.0);
		let camera = socket - bore * back;
		assert!((camera.x - (1.0 - back)).abs() < 1e-4, "{camera}");
	}
}
