//! Experimental world follow-cam: first/third POV and collision vs Fixed geometry.
//!
//! This lives in the world playground on purpose. Extract the orbit, R3 POV
//! toggle, and shapecast pull-in into a shared follow-cam crate when a second
//! playground needs the same rig.
//!
//! First person sockets to `nose_socket` and hides face parts. Head rig stays
//! so the bone still updates; look rotation stays on [`CameraController`].

use avian3d::prelude::{Collider, ShapeCastConfig, SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::{
	player::{CAMERA_DISTANCE, CAMERA_HEIGHT, CAMERA_LOOK_HEIGHT},
	CameraController, Player, PlayerVisual, PlaygroundMode,
};
use crozon_characters::{
	find_member_rig, BoneMap, CharacterMembers, CharacterPartSlot, CharacterRig, CharacterRigRole,
	PartNode,
};
use lod_avian::PhysicsInteractionLayer;

const CAMERA_COLLISION_RADIUS: f32 = 0.18;
const CAMERA_COLLISION_SKIN: f32 = 0.08;
const CAMERA_COLLISION_MIN: f32 = 0.12;
/// Capsule fallback when the visual has no `nose_socket`.
const FIRST_PERSON_EYE_OFFSET: f32 = CAMERA_LOOK_HEIGHT + 0.12;
/// Sit just in front of the nose along look so the near plane misses the neck.
const FIRST_PERSON_FORWARD: f32 = 0.12;
const NOSE_SOCKET: &str = "nose_socket";

/// World-playground camera POV. Default matches the vegetation third-person orbit.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CameraPov {
	#[default]
	ThirdPerson,
	FirstPerson,
}

impl CameraPov {
	pub fn toggle(self) -> Self {
		match self {
			Self::ThirdPerson => Self::FirstPerson,
			Self::FirstPerson => Self::ThirdPerson,
		}
	}
}

/// Follow after vegetation's third-person rig so this overwrites the same frame.
pub(crate) fn follow_world_camera(
	mode: Res<PlaygroundMode>,
	pov: Res<CameraPov>,
	spatial: SpatialQuery,
	players: Query<(Entity, &Transform), (With<Player>, Without<Camera3d>)>,
	visuals: Query<&CharacterMembers, With<PlayerVisual>>,
	rigs: Query<(Entity, &CharacterRig, &BoneMap)>,
	globals: Query<&GlobalTransform>,
	mut cameras: Query<(&mut Transform, &CameraController), With<Camera3d>>,
) {
	if *mode != PlaygroundMode::Character {
		return;
	}
	let Ok((player_entity, player)) = players.single() else {
		return;
	};
	let Ok((mut camera_transform, controller)) = cameras.single_mut() else {
		return;
	};

	let yaw = Quat::from_axis_angle(Vec3::Y, controller.yaw);
	let pitch = Quat::from_axis_angle(Vec3::X, controller.pitch);
	let rotation = yaw * pitch;

	match *pov {
		CameraPov::FirstPerson => {
			let translation = first_person_translation(player, rotation, &visuals, &rigs, &globals);
			camera_transform.translation = translation;
			camera_transform.rotation = rotation;
		}
		CameraPov::ThirdPerson => {
			let offset = rotation * Vec3::new(0.0, 0.0, CAMERA_DISTANCE) + Vec3::Y * CAMERA_HEIGHT;
			let target = player.translation + Vec3::Y * CAMERA_LOOK_HEIGHT;
			let desired = target + offset;
			camera_transform.translation =
				obstructed_camera_translation(&spatial, target, desired, player_entity);
			camera_transform.look_at(target, Vec3::Y);
		}
	}
}

/// Hide face meshes in first person; leave body / neck / clothing.
pub(crate) fn sync_first_person_head_visibility(
	mode: Res<PlaygroundMode>,
	pov: Res<CameraPov>,
	visuals: Query<&CharacterMembers, With<PlayerVisual>>,
	parts: Query<&PartNode>,
	mut visibilities: Query<&mut Visibility>,
) {
	let hide = *mode == PlaygroundMode::Character && *pov == CameraPov::FirstPerson;
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

fn first_person_translation(
	player: &Transform,
	look_rotation: Quat,
	visuals: &Query<&CharacterMembers, With<PlayerVisual>>,
	rigs: &Query<(Entity, &CharacterRig, &BoneMap)>,
	globals: &Query<&GlobalTransform>,
) -> Vec3 {
	let nose = visuals.iter().find_map(|members| nose_socket_world(members, rigs, globals));
	match nose {
		Some(origin) => first_person_camera_origin(origin, look_rotation, FIRST_PERSON_FORWARD),
		None => player.translation + Vec3::Y * FIRST_PERSON_EYE_OFFSET,
	}
}

fn nose_socket_world(
	members: &CharacterMembers,
	rigs: &Query<(Entity, &CharacterRig, &BoneMap)>,
	globals: &Query<&GlobalTransform>,
) -> Option<Vec3> {
	for role in [CharacterRigRole::Head, CharacterRigRole::Body] {
		let Some((_, map)) = find_member_rig(members, role, rigs) else {
			continue;
		};
		let Some(&bone) = map.by_name.get(NOSE_SOCKET) else {
			continue;
		};
		if let Ok(global) = globals.get(bone) {
			return Some(global.translation());
		}
	}
	None
}

pub(crate) fn first_person_camera_origin(nose: Vec3, look_rotation: Quat, forward_m: f32) -> Vec3 {
	nose + look_rotation * Vec3::new(0.0, 0.0, -forward_m)
}

pub(crate) fn is_first_person_hidden_slot(slot: CharacterPartSlot) -> bool {
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

fn obstructed_camera_translation(
	spatial: &SpatialQuery,
	origin: Vec3,
	desired: Vec3,
	exclude: Entity,
) -> Vec3 {
	let delta = desired - origin;
	let Ok(direction) = Dir3::new(delta) else {
		return desired;
	};
	let distance = delta.length();
	let filter = SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed)
		.with_excluded_entities([exclude]);
	let shape = Collider::sphere(CAMERA_COLLISION_RADIUS);
	let config = ShapeCastConfig::from_max_distance(distance);
	let hit_distance = spatial
		.cast_shape(&shape, origin, Quat::IDENTITY, direction, &config, &filter)
		.map(|hit| hit.distance);
	let travel =
		camera_cast_travel(distance, hit_distance, CAMERA_COLLISION_SKIN, CAMERA_COLLISION_MIN);
	origin + *direction * travel
}

/// Pull the camera in along the look-at → desired ray when Fixed geometry is hit.
pub(crate) fn camera_cast_travel(
	desired_distance: f32,
	hit_distance: Option<f32>,
	skin: f32,
	min_distance: f32,
) -> f32 {
	match hit_distance {
		Some(distance) => (distance - skin).clamp(min_distance, desired_distance),
		None => desired_distance,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn r3_toggles_first_and_third() {
		assert_eq!(CameraPov::ThirdPerson.toggle(), CameraPov::FirstPerson);
		assert_eq!(CameraPov::FirstPerson.toggle(), CameraPov::ThirdPerson);
	}

	#[test]
	fn miss_keeps_desired_distance() {
		assert_eq!(camera_cast_travel(3.6, None, 0.08, 0.12), 3.6);
	}

	#[test]
	fn hit_pulls_in_by_skin() {
		assert!((camera_cast_travel(3.6, Some(1.0), 0.08, 0.12) - 0.92).abs() < 1e-5);
	}

	#[test]
	fn near_hit_does_not_go_inside_look_at() {
		assert_eq!(camera_cast_travel(3.6, Some(0.05), 0.08, 0.12), 0.12);
	}

	#[test]
	fn first_person_hides_face_not_body() {
		assert!(is_first_person_hidden_slot(CharacterPartSlot::HeadMesh));
		assert!(is_first_person_hidden_slot(CharacterPartSlot::Nose));
		assert!(is_first_person_hidden_slot(CharacterPartSlot::Hair));
		assert!(!is_first_person_hidden_slot(CharacterPartSlot::BodyMesh));
		assert!(!is_first_person_hidden_slot(CharacterPartSlot::NeckMesh));
		assert!(!is_first_person_hidden_slot(CharacterPartSlot::Clothing));
	}

	#[test]
	fn first_person_origin_sits_along_look() {
		let nose = Vec3::new(1.0, 2.0, 3.0);
		let origin = first_person_camera_origin(nose, Quat::IDENTITY, 0.12);
		assert!((origin - Vec3::new(1.0, 2.0, 2.88)).length() < 1e-5);
	}
}
