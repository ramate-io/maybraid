use bevy::prelude::*;
use camera_controls::look::CameraLookEnabled;
use crozon_character_playground::CameraController;
use crozon_characters::SocketRig;

use crate::{
	focus_reference::FocusReferenceRig,
	preview::ConceptPreviewConfig,
	skinning::{BoneMap, CharacterRig, CharacterRigRole},
	ui::{CameraFocus, CreatorUiState, UiAssetTarget},
};

/// One-shot camera move queued when the user selects an asset in the creator UI.
#[derive(Resource, Default)]
pub struct PendingCameraFocus {
	pub focus: Option<CameraFocus>,
}

const SNAP_DISTANCE: f32 = 0.04;
const SNAP_ANGLE: f32 = 0.03;

/// Queue the default body framing once when the playground starts (look locked).
pub fn queue_default_camera_focus(
	mut pending: ResMut<PendingCameraFocus>,
	config: Res<ConceptPreviewConfig>,
	mut ui_state: ResMut<CreatorUiState>,
	mut queued: Local<bool>,
) {
	if *queued {
		return;
	}
	*queued = true;
	let ConceptPreviewConfig::Braidman { config: braidman, .. } = config.as_ref();
	let target = UiAssetTarget::Body(braidman.body);
	ui_state.last_selected = Some(target);
	pending.focus = Some(target.camera_focus());
}

pub fn apply_camera_suggestion(
	time: Res<Time>,
	look_enabled: Option<Res<CameraLookEnabled>>,
	mut pending: ResMut<PendingCameraFocus>,
	rigs: Query<(&BoneMap, &CharacterRig), With<FocusReferenceRig>>,
	transforms: Query<&GlobalTransform>,
	mut cameras: Query<(&mut Transform, &mut CameraController), With<Camera3d>>,
) {
	if look_enabled.is_none_or(|enabled| enabled.0) {
		pending.focus = None;
		return;
	}
	let Some(focus) = pending.focus else {
		return;
	};
	let Some(target) = resolve_focus_transform(focus, &rigs, &transforms) else {
		return;
	};
	let Ok((mut transform, mut controller)) = cameras.single_mut() else {
		return;
	};
	let t = (time.delta_secs() * 6.0).clamp(0.0, 1.0);
	transform.translation = transform.translation.lerp(target.translation, t);
	transform.rotation = transform.rotation.slerp(target.rotation, t);

	let settled = transform.translation.distance(target.translation) < SNAP_DISTANCE
		&& transform.rotation.angle_between(target.rotation) < SNAP_ANGLE;
	if settled {
		*transform = target;
		pending.focus = None;
	} else {
		let (yaw, pitch) = yaw_pitch_from_rotation(transform.rotation);
		controller.yaw = yaw;
		controller.pitch = pitch;
	}
}

fn resolve_focus_transform(
	focus: CameraFocus,
	rigs: &Query<(&BoneMap, &CharacterRig), With<FocusReferenceRig>>,
	transforms: &Query<&GlobalTransform>,
) -> Option<Transform> {
	let role = match focus.rig {
		SocketRig::Body => CharacterRigRole::Body,
		SocketRig::Head => CharacterRigRole::Head,
	};
	for (bone_map, rig) in rigs {
		if rig.role != role {
			continue;
		}
		let bone_entity = bone_map.by_name.get(focus.socket)?;
		let socket = transforms.get(*bone_entity).ok()?;
		let camera_pos = socket_oriented_point(socket, focus.camera_offset);
		let look_at = socket_oriented_point(socket, focus.look_at_offset);
		return Some(Transform::from_translation(camera_pos).looking_at(look_at, Vec3::Y));
	}
	None
}

/// Map a meter offset along the socket's local axes into world space (no bone scale).
fn socket_oriented_point(socket: &GlobalTransform, local_offset: Vec3) -> Vec3 {
	socket.translation() + socket.rotation() * local_offset
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
