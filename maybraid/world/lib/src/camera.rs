//! Experimental world follow-cam: first/third POV and collision vs Fixed geometry.
//!
//! This lives in the world playground on purpose. Extract the orbit, R3 POV
//! toggle, and shapecast pull-in into a shared follow-cam crate when a second
//! playground needs the same rig.

use avian3d::prelude::{Collider, ShapeCastConfig, SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::{
	player::{CAMERA_DISTANCE, CAMERA_HEIGHT, CAMERA_LOOK_HEIGHT},
	CameraController, Player, PlayerCapsule, PlayerVisual, PlaygroundMode,
};
use lod_avian::PhysicsInteractionLayer;

const CAMERA_COLLISION_RADIUS: f32 = 0.18;
const CAMERA_COLLISION_SKIN: f32 = 0.08;
const CAMERA_COLLISION_MIN: f32 = 0.12;
/// Eyes sit a bit above the third-person look-at (chest).
const FIRST_PERSON_EYE_OFFSET: f32 = CAMERA_LOOK_HEIGHT + 0.12;

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
			camera_transform.translation = player.translation + Vec3::Y * FIRST_PERSON_EYE_OFFSET;
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

pub(crate) fn sync_pov_visibility(
	pov: Res<CameraPov>,
	mut visuals: Query<&mut Visibility, (With<PlayerVisual>, Without<PlayerCapsule>)>,
	mut capsules: Query<&mut Visibility, (With<PlayerCapsule>, Without<PlayerVisual>)>,
) {
	let first = *pov == CameraPov::FirstPerson;
	let has_visual = !visuals.is_empty();
	for mut visibility in &mut visuals {
		*visibility = if first { Visibility::Hidden } else { Visibility::Inherited };
	}
	for mut visibility in &mut capsules {
		*visibility = if first || has_visual { Visibility::Hidden } else { Visibility::Inherited };
	}
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
	fn sync_pov_visibility_queries_are_disjoint() {
		let mut app = App::new();
		app.init_resource::<CameraPov>().add_systems(Update, sync_pov_visibility);
		app.update();
	}
}
