//! Write [`PlayerCameraAim`] from the held firearm's sight socket.

use bevy::prelude::*;
use bevy::transform::helper::TransformHelper;
use characters::BoneMap;
use firearms::{FirearmMembers, FirearmRoot};
use player::{PlayerCameraAim, PlayerCameraPose, PlayerLook};

use crate::pose::HeldFirearm;
use crate::weapon::LiveWeapon;
use crate::FirearmUser;

pub(crate) fn write_sight_aim(
	mut cameras: ParamSet<(
		Query<(&FirearmUser, &PlayerLook, &mut PlayerCameraAim)>,
		Query<&mut PlayerCameraAim>,
	)>,
	mut removed_users: RemovedComponents<FirearmUser>,
	guns: Query<(&FirearmMembers, Option<&LiveWeapon>), (With<HeldFirearm>, With<FirearmRoot>)>,
	maps: Query<&BoneMap>,
	transforms: TransformHelper,
) {
	for entity in removed_users.read() {
		if let Ok(mut aim) = cameras.p1().get_mut(entity) {
			*aim = PlayerCameraAim::default();
		}
	}
	for (user, look, mut aim) in &mut cameras.p0() {
		if !look.first_person {
			aim.pose = None;
			aim.focus = 0.0;
			aim.sight_fov = None;
			continue;
		}
		aim.focus = look.focus;
		aim.sight_fov =
			guns.get(user.held).ok().and_then(|(_, live)| live.map(|live| live.sight_fov));
		aim.pose = sight_camera_pose(
			user.held,
			user.settings.sight_camera_back,
			&guns,
			&maps,
			&transforms,
		);
	}
}

fn sight_camera_pose(
	held: Entity,
	sight_camera_back: f32,
	guns: &Query<(&FirearmMembers, Option<&LiveWeapon>), (With<HeldFirearm>, With<FirearmRoot>)>,
	maps: &Query<&BoneMap>,
	transforms: &TransformHelper,
) -> Option<PlayerCameraPose> {
	let (members, _) = guns.get(held).ok()?;
	let socket = member_landmark_global(members.iter(), maps, transforms, "sight_camera_socket")?;
	Some(sight_camera_pose_from_socket(socket, sight_camera_back))
}

fn sight_camera_pose_from_socket(
	socket: GlobalTransform,
	sight_camera_back: f32,
) -> PlayerCameraPose {
	let look = (socket.rotation() * Vec3::Z).normalize_or(Vec3::Z);
	let mut aimed = Transform::from_translation(socket.translation() - look * sight_camera_back);
	aimed.look_to(look, Vec3::Y);
	PlayerCameraPose { translation: aimed.translation, rotation: aimed.rotation }
}

fn member_landmark_global(
	members: impl Iterator<Item = Entity>,
	maps: &Query<&BoneMap>,
	transforms: &TransformHelper,
	name: &str,
) -> Option<GlobalTransform> {
	for member in members {
		let Ok(map) = maps.get(member) else {
			continue;
		};
		let Some(&entity) = map.by_name.get(name) else {
			continue;
		};
		if let Ok(global) = transforms.compute_global_transform(entity) {
			return Some(global);
		}
	}
	None
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{FirearmUser, FirearmUserSettings};

	#[test]
	fn sight_camera_sits_behind_socket() {
		let back = FirearmUserSettings::default().sight_camera_back;
		let socket = GlobalTransform::from(
			Transform::from_translation(Vec3::new(1.0, 1.5, 0.0))
				.with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
		);
		let camera = sight_camera_pose_from_socket(socket, back);
		assert!((camera.translation - Vec3::new(1.0 - back, 1.5, 0.0)).length() < 1e-4);
		assert!((camera.rotation * -Vec3::Z - Vec3::X).length() < 1e-4);
	}

	#[test]
	fn removing_firearm_user_clears_cached_sight_override() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_systems(Update, write_sight_aim);
		let held = app.world_mut().spawn_empty().id();
		let actor = app
			.world_mut()
			.spawn((
				FirearmUser::holding(held),
				PlayerLook::default(),
				PlayerCameraAim {
					pose: Some(PlayerCameraPose {
						translation: Vec3::new(1.0, 2.0, 3.0),
						rotation: Quat::IDENTITY,
					}),
					focus: 1.0,
					sight_fov: Some(0.2),
				},
			))
			.id();
		app.world_mut().entity_mut(actor).remove::<FirearmUser>();
		app.update();
		assert!(app.world().get::<PlayerCameraAim>(actor).is_some_and(|aim| {
			aim.pose.is_none() && aim.focus == 0.0 && aim.sight_fov.is_none()
		}));
	}
}
