use bevy::prelude::*;
use camera_controls::look::CameraLookEnabled;
use crozon_character_playground::CameraController;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CameraSuggestion {
	#[default]
	FullBody,
	Torso,
	Head,
	Eyes,
	Face,
	Ears,
}

/// One-shot camera move queued when the user selects an asset in the creator UI.
#[derive(Resource, Default)]
pub struct PendingCameraFocus {
	pub suggestion: Option<CameraSuggestion>,
}

const SNAP_DISTANCE: f32 = 0.04;
const SNAP_ANGLE: f32 = 0.03;

pub fn apply_camera_suggestion(
	time: Res<Time>,
	look_enabled: Option<Res<CameraLookEnabled>>,
	mut pending: ResMut<PendingCameraFocus>,
	mut cameras: Query<(&mut Transform, &mut CameraController), With<Camera3d>>,
) {
	if look_enabled.is_none_or(|enabled| enabled.0) {
		pending.suggestion = None;
		return;
	}
	let Some(suggestion) = pending.suggestion else {
		return;
	};
	let Ok((mut transform, mut controller)) = cameras.single_mut() else {
		return;
	};
	let target = suggested_transform(suggestion);
	let t = (time.delta_secs() * 6.0).clamp(0.0, 1.0);
	transform.translation = transform.translation.lerp(target.translation, t);
	transform.rotation = transform.rotation.slerp(target.rotation, t);

	let settled = transform.translation.distance(target.translation) < SNAP_DISTANCE
		&& transform.rotation.angle_between(target.rotation) < SNAP_ANGLE;
	if settled {
		*transform = target;
		pending.suggestion = None;
	} else {
		let (yaw, pitch) = yaw_pitch_from_rotation(transform.rotation);
		controller.yaw = yaw;
		controller.pitch = pitch;
	}
}

fn suggested_transform(suggestion: CameraSuggestion) -> Transform {
	let (camera_pos, look_at) = match suggestion {
		CameraSuggestion::FullBody => (Vec3::new(0.0, 1.45, 3.3), Vec3::new(0.0, 1.0, 0.0)),
		CameraSuggestion::Torso => (Vec3::new(0.0, 1.25, 2.15), Vec3::new(0.0, 1.05, 0.0)),
		CameraSuggestion::Head => (Vec3::new(0.0, 1.72, 1.55), Vec3::new(0.0, 1.55, 0.0)),
		CameraSuggestion::Eyes => (Vec3::new(0.0, 1.72, 1.15), Vec3::new(0.0, 1.62, 0.0)),
		CameraSuggestion::Face => (Vec3::new(0.0, 1.60, 1.20), Vec3::new(0.0, 1.50, 0.0)),
		CameraSuggestion::Ears => (Vec3::new(0.95, 1.62, 1.25), Vec3::new(0.0, 1.54, 0.0)),
	};
	Transform::from_translation(camera_pos).looking_at(look_at, Vec3::Y)
}

fn yaw_pitch_from_rotation(rotation: Quat) -> (f32, f32) {
	let (x, y, z, w) = (rotation.x, rotation.y, rotation.z, rotation.w);
	let sin_yaw = 2.0 * (w * y + x * z);
	let cos_yaw = 1.0 - 2.0 * (y * y + z * z);
	let yaw = sin_yaw.atan2(cos_yaw);
	let sin_pitch = 2.0 * (w * x - y * z);
	let pitch = sin_pitch.asin();
	(yaw, pitch)
}
