//! Experimental world follow-cam: first/third POV and collision vs Fixed geometry.
//!
//! This lives in the world playground on purpose. Extract the orbit, R3 POV
//! toggle, and shapecast pull-in into a shared follow-cam crate when a second
//! playground needs the same rig.
//!
//! First person copies the posed nose/head *position*, then applies look yaw/pitch
//! itself. The head does not pitch, so the camera is not parented to the socket
//! (that would lock look). We write [`GlobalTransform`] because this runs after
//! propagate; otherwise the renderer keeps the third-person pose.

use std::f32::consts::PI;

use avian3d::prelude::{Collider, ShapeCastConfig, SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::{
	player::{CAMERA_DISTANCE, CAMERA_HEIGHT, CAMERA_LOOK_HEIGHT},
	CameraController, Player, PlayerVisual, PlaygroundMode,
};
use crozon_characters::{
	find_member_rig, BoneMap, CharacterMembers, CharacterPartSlot, CharacterRig, CharacterRigRole,
	CharacterRoot, PartNode,
};
use lod_avian::PhysicsInteractionLayer;

const CAMERA_COLLISION_RADIUS: f32 = 0.18;
const CAMERA_COLLISION_SKIN: f32 = 0.08;
const CAMERA_COLLISION_MIN: f32 = 0.12;
/// Capsule fallback when the visual has no socketed nose / head part.
const FIRST_PERSON_EYE_OFFSET: f32 = CAMERA_LOOK_HEIGHT + 0.12;
/// Sit just in front of the face along the socket's authored +Z.
const FIRST_PERSON_FORWARD: f32 = 0.12;
const NOSE_SOCKET: &str = "nose_socket";
/// Free look relative to the body before the torso has to follow (neck-safe).
const MAX_LOOK_YAW: f32 = 60.0_f32.to_radians();
const BODY_TURN_RATE: f32 = 8.0;

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
	players: Query<Entity, (With<Player>, Without<Camera3d>)>,
	visuals: Query<&CharacterMembers, With<PlayerVisual>>,
	parts: Query<&PartNode>,
	rigs: Query<(Entity, &CharacterRig, &BoneMap)>,
	globals: Query<&GlobalTransform, Without<Camera3d>>,
	mut cameras: Query<(&mut Transform, &mut GlobalTransform, &CameraController), With<Camera3d>>,
) {
	if *mode != PlaygroundMode::Character {
		return;
	}
	let Ok(player_entity) = players.single() else {
		return;
	};
	let Ok(player) = globals.get(player_entity) else {
		return;
	};
	let Ok((mut camera_transform, mut camera_global, controller)) = cameras.single_mut() else {
		return;
	};

	let yaw = Quat::from_axis_angle(Vec3::Y, controller.yaw);
	let pitch = Quat::from_axis_angle(Vec3::X, controller.pitch);
	let rotation = yaw * pitch;
	let player_pos = player.translation();

	match *pov {
		CameraPov::FirstPerson => {
			camera_transform.translation =
				first_person_translation(player_pos, &visuals, &parts, &rigs, &globals);
			camera_transform.rotation = rotation;
		}
		CameraPov::ThirdPerson => {
			let offset = rotation * Vec3::new(0.0, 0.0, CAMERA_DISTANCE) + Vec3::Y * CAMERA_HEIGHT;
			let target = player_pos + Vec3::Y * CAMERA_LOOK_HEIGHT;
			let desired = target + offset;
			camera_transform.translation =
				obstructed_camera_translation(&spatial, target, desired, player_entity);
			camera_transform.look_at(target, Vec3::Y);
		}
	}
	// We run after TransformSystems::Propagate so rendering would otherwise
	// keep last frame's third-person GlobalTransform.
	*camera_global = GlobalTransform::from(*camera_transform);
}

/// When first-person look pulls too far off the torso, turn the body so you
/// cannot spin around and stare at the neck. Look stays clamped to the cone.
pub(crate) fn turn_body_with_look(
	time: Res<Time>,
	mode: Res<PlaygroundMode>,
	pov: Res<CameraPov>,
	mut cameras: Query<&mut CameraController, With<Camera3d>>,
	mut visuals: Query<
		&mut Transform,
		(With<PlayerVisual>, With<CharacterRoot>, Without<Camera3d>),
	>,
) {
	if *mode != PlaygroundMode::Character || *pov != CameraPov::FirstPerson {
		return;
	}
	let Ok(mut controller) = cameras.single_mut() else {
		return;
	};
	let Ok(mut visual) = visuals.single_mut() else {
		return;
	};

	let body = body_yaw(&visual);
	let target = follow_body_yaw(controller.yaw, body, MAX_LOOK_YAW);
	let step = wrap_to_pi(target - body);
	let max_step = BODY_TURN_RATE * time.delta_secs();
	let applied = step.abs().min(max_step).copysign(step);
	if applied.abs() > 1e-5 {
		set_body_yaw(&mut visual, body + applied);
	}
	controller.yaw = clamp_look_yaw(controller.yaw, body_yaw(&visual), MAX_LOOK_YAW);
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
	player_pos: Vec3,
	visuals: &Query<&CharacterMembers, With<PlayerVisual>>,
	parts: &Query<&PartNode>,
	rigs: &Query<(Entity, &CharacterRig, &BoneMap)>,
	globals: &Query<&GlobalTransform, Without<Camera3d>>,
) -> Vec3 {
	visuals
		.iter()
		.find_map(|members| first_person_anchor(members, parts, rigs, globals))
		.unwrap_or(player_pos + Vec3::Y * FIRST_PERSON_EYE_OFFSET)
}

/// Nose part (already socketed) first — same entities we hide. BoneMap is fallback.
fn first_person_anchor(
	members: &CharacterMembers,
	parts: &Query<&PartNode>,
	rigs: &Query<(Entity, &CharacterRig, &BoneMap)>,
	globals: &Query<&GlobalTransform, Without<Camera3d>>,
) -> Option<Vec3> {
	for slot in [CharacterPartSlot::Nose, CharacterPartSlot::HeadMesh] {
		for member in members.iter() {
			let Ok(part) = parts.get(member) else {
				continue;
			};
			if part.slot != slot {
				continue;
			}
			if let Ok(global) = globals.get(member) {
				return Some(socket_forward_point(global, FIRST_PERSON_FORWARD));
			}
		}
	}
	for role in [CharacterRigRole::Head, CharacterRigRole::Body] {
		let Some((_, map)) = find_member_rig(members, role, rigs) else {
			continue;
		};
		let Some(&bone) = map.by_name.get(NOSE_SOCKET) else {
			continue;
		};
		if let Ok(global) = globals.get(bone) {
			return Some(socket_forward_point(global, FIRST_PERSON_FORWARD));
		}
	}
	None
}

/// Character sockets author +Z as face-forward (not Bevy camera -Z).
pub(crate) fn socket_forward_point(socket: &GlobalTransform, forward_m: f32) -> Vec3 {
	socket.translation() + socket.rotation() * Vec3::Z * forward_m
}

pub(crate) fn wrap_to_pi(angle: f32) -> f32 {
	(angle + PI).rem_euclid(2.0 * PI) - PI
}

/// Bevy camera yaw: forward at 0 is -Z.
pub(crate) fn yaw_from_xz_forward(dir: Vec3) -> f32 {
	(-dir.x).atan2(-dir.z)
}

fn body_yaw(visual: &Transform) -> f32 {
	yaw_from_xz_forward(-*visual.forward())
}

fn set_body_yaw(visual: &mut Transform, yaw: f32) {
	let forward = Quat::from_axis_angle(Vec3::Y, yaw) * -Vec3::Z;
	visual.look_to(-forward, Vec3::Y);
}

pub(crate) fn follow_body_yaw(look_yaw: f32, body_yaw: f32, max_delta: f32) -> f32 {
	let delta = wrap_to_pi(look_yaw - body_yaw);
	look_yaw - delta.clamp(-max_delta, max_delta)
}

pub(crate) fn clamp_look_yaw(look_yaw: f32, body_yaw: f32, max_delta: f32) -> f32 {
	let delta = wrap_to_pi(look_yaw - body_yaw);
	body_yaw + delta.clamp(-max_delta, max_delta)
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
	fn first_person_origin_sits_along_socket_plus_z() {
		let socket = GlobalTransform::from_translation(Vec3::new(1.0, 2.0, 3.0));
		let origin = socket_forward_point(&socket, 0.12);
		assert!((origin - Vec3::new(1.0, 2.0, 3.12)).length() < 1e-5);
	}

	#[test]
	fn first_person_look_is_independent_of_socket() {
		let yaw = Quat::from_axis_angle(Vec3::Y, 0.5);
		let pitch = Quat::from_axis_angle(Vec3::X, -0.3);
		let rotation = yaw * pitch;
		let socket = GlobalTransform::from_translation(Vec3::new(10.0, 1.6, -4.0));
		let translation = socket_forward_point(&socket, 0.12);
		let camera = Transform { translation, rotation, ..default() };
		assert!((camera.translation.y - 1.6).abs() < 1e-4);
		assert_eq!(camera.rotation, rotation);
	}

	#[test]
	fn wrap_to_pi_folds_full_turns() {
		assert!((wrap_to_pi(PI + 0.1) + PI - 0.1).abs() < 1e-5);
		assert!(wrap_to_pi(0.2).abs() < 0.21);
	}

	#[test]
	fn body_stays_put_inside_look_cone() {
		let look = 0.3;
		let body = 0.0;
		assert!((follow_body_yaw(look, body, MAX_LOOK_YAW) - body).abs() < 1e-5);
		assert!((clamp_look_yaw(look, body, MAX_LOOK_YAW) - look).abs() < 1e-5);
	}

	#[test]
	fn body_follows_when_look_exceeds_cone() {
		let look = 2.0;
		let body = 0.0;
		let target = follow_body_yaw(look, body, MAX_LOOK_YAW);
		assert!((target - (look - MAX_LOOK_YAW)).abs() < 1e-5);
		let clamped = clamp_look_yaw(look, body, MAX_LOOK_YAW);
		assert!((clamped - MAX_LOOK_YAW).abs() < 1e-5);
	}
}
