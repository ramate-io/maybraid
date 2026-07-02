use bevy::prelude::*;
use camera_controls::look::CameraLookEnabled;
use character_ui_menu::{CameraFocus, FocusRig};
use crozon_character_playground::CameraController;
use crozon_character_ui_menus::BODY_FOCUS;

use crate::{
	animation::AnimatedBodyRig,
	focus_reference::FocusReferenceRig,
	preview::{ConceptPreviewConfig, ConceptPreviewRoot},
	skinning::{
		bind_scales_ready, bone_map_ready, ActiveRigPose, BoneMap, CharacterRig, CharacterRigRole,
		NeedsSocketPlacement, RigBindScales,
	},
	ui::CreatorUiState,
};

/// Minimum posed `upper_neck` height before body shadow framing is trusted.
const BODY_POSE_LANDMARK_MIN_Y: f32 = 0.4;

/// One-shot camera move queued when the user selects an asset in the creator UI.
#[derive(Resource, Default)]
pub struct PendingCameraFocus {
	pub focus: Option<CameraFocus>,
	/// Label for debug logging (`ui-press`, `startup-default`, etc.).
	pub focus_trigger: Option<String>,
	/// Target captured on the first ready frame so animation does not chase the lerp.
	pub resolved_target: Option<Transform>,
}

const SNAP_DISTANCE: f32 = 0.04;
const SNAP_ANGLE: f32 = 0.03;

pub fn focus_debug_enabled() -> bool {
	std::env::var("CROZON_CAMERA_FOCUS_DEBUG").is_ok()
}

pub fn queue_camera_focus(
	pending: &mut PendingCameraFocus,
	focus: CameraFocus,
	trigger: impl Into<String>,
) {
	let trigger = trigger.into();
	if focus_debug_enabled() {
		info!("[camera-focus] queue trigger={trigger} {}", focus_summary(focus));
	}
	pending.focus_trigger = Some(trigger);
	pending.focus = Some(focus);
	pending.resolved_target = None;
}

/// Queue default body framing for the active species.
pub fn queue_species_default_camera_focus(
	pending: &mut PendingCameraFocus,
	ui_state: &mut CreatorUiState,
	config: &ConceptPreviewConfig,
	trigger: impl Into<String>,
) {
	let focus = default_focus_target(config);
	ui_state.last_selected = Some(focus);
	queue_camera_focus(pending, focus, trigger);
}

pub fn default_focus_target(_config: &ConceptPreviewConfig) -> CameraFocus {
	BODY_FOCUS
}

pub fn apply_camera_suggestion(
	time: Res<Time>,
	look_enabled: Option<Res<CameraLookEnabled>>,
	mut pending: ResMut<PendingCameraFocus>,
	shadow_rigs: Query<(&BoneMap, &CharacterRig, &GlobalTransform), With<FocusReferenceRig>>,
	shadow_body_pose: Query<
		(&BoneMap, &RigBindScales, &CharacterRig),
		(With<FocusReferenceRig>, With<ActiveRigPose>),
	>,
	shadow_head_rigs: Query<
		(&BoneMap, &CharacterRig),
		(With<FocusReferenceRig>, With<CharacterRig>, Without<NeedsSocketPlacement>),
	>,
	preview_body_pose: Query<
		(&BoneMap, &RigBindScales, &CharacterRig),
		(With<ConceptPreviewRoot>, With<AnimatedBodyRig>, With<ActiveRigPose>),
	>,
	preview_head_rigs: Query<
		(&BoneMap, &CharacterRig),
		(
			With<ConceptPreviewRoot>,
			With<CharacterRig>,
			Without<FocusReferenceRig>,
			Without<NeedsSocketPlacement>,
		),
	>,
	preview_rigs: Query<
		(&BoneMap, &CharacterRig, &GlobalTransform),
		(With<CharacterRig>, With<ConceptPreviewRoot>, Without<FocusReferenceRig>),
	>,
	bone_globals: Query<&GlobalTransform>,
	mut cameras: Query<(&mut Transform, &mut CameraController), With<Camera3d>>,
	mut last_wait_log: Local<Option<String>>,
) {
	if look_enabled.is_none_or(|enabled| enabled.0) {
		pending.focus = None;
		pending.focus_trigger = None;
		pending.resolved_target = None;
		return;
	}
	let Some(focus) = pending.focus else {
		return;
	};
	if !focus_target_ready(
		focus,
		&shadow_body_pose,
		&shadow_head_rigs,
		&preview_body_pose,
		&preview_head_rigs,
		&bone_globals,
	) {
		if focus_debug_enabled() {
			let trigger = pending.focus_trigger.clone();
			if last_wait_log.as_ref() != trigger.as_ref() {
				let status = if focus.uses_preview_sockets() {
					preview_head_rigs
						.iter()
						.find(|(_, rig)| rig.role == CharacterRigRole::Head)
						.map(|(map, _)| {
							format!(
								"source=preview head_bones={} socket={:?} socket_ready={}",
								map.by_name.len(),
								focus.socket,
								head_rig_focus_ready(map, focus.socket),
							)
						})
						.unwrap_or_else(|| "source=preview head_rig=missing".into())
				} else {
					match focus.rig {
						FocusRig::Body => shadow_body_pose
							.iter()
							.find(|(_, _, rig)| rig.role == CharacterRigRole::Body)
							.map(|(map, scales, _)| {
								format!(
									"source=shadow upper_neck_y={} pose_ready={}",
									fmt_y(bone_global_y(map, &bone_globals, "upper_neck")),
									shadow_body_pose_ready(map, scales, &bone_globals),
								)
							})
							.unwrap_or_else(|| "source=shadow body_rig=missing".into()),
						FocusRig::Head => shadow_rigs
							.iter()
							.find(|(_, rig, _)| rig.role == CharacterRigRole::Head)
							.map(|(map, _, _)| {
								let socket_ready = head_rig_focus_ready(map, focus.socket);
								let attached = shadow_head_rigs
									.iter()
									.any(|(_, rig)| rig.role == CharacterRigRole::Head);
								format!(
									"source=shadow head_bones={} socket={:?} socket_ready={socket_ready} attached={attached}",
									map.by_name.len(),
									focus.socket,
								)
							})
							.unwrap_or_else(|| "source=shadow head_rig=missing".into()),
					}
				};
				warn!(
					"[camera-focus] waiting for focus target trigger={:?} focus={} {status}",
					trigger,
					focus_summary(focus),
				);
				*last_wait_log = trigger;
			}
		}
		return;
	}
	last_wait_log.take();
	if pending.resolved_target.is_none() {
		let Some(target) =
			resolve_focus_transform(focus, &shadow_rigs, &preview_rigs, &bone_globals)
		else {
			if focus_debug_enabled() {
				warn!(
					"[camera-focus] resolve failed trigger={:?} focus={:?}",
					pending.focus_trigger,
					focus_summary(focus),
				);
			}
			return;
		};
		pending.resolved_target = Some(target);
	}
	let target = pending.resolved_target.unwrap();
	let Ok((mut transform, mut controller)) = cameras.single_mut() else {
		return;
	};

	let camera_before = transform.translation;
	let t = (time.delta_secs() * 6.0).clamp(0.0, 1.0);
	transform.translation = transform.translation.lerp(target.translation, t);
	transform.rotation = transform.rotation.slerp(target.rotation, t);

	let settled = transform.translation.distance(target.translation) < SNAP_DISTANCE
		&& transform.rotation.angle_between(target.rotation) < SNAP_ANGLE;

	if focus_debug_enabled() {
		log_focus_apply(
			pending.focus_trigger.as_deref().unwrap_or("?"),
			focus,
			&shadow_rigs,
			&preview_rigs,
			&bone_globals,
			camera_before,
			transform.translation,
			&target,
			settled,
		);
	}

	if settled {
		*transform = target;
		pending.focus = None;
		pending.focus_trigger = None;
		pending.resolved_target = None;
	} else {
		let (yaw, pitch) = yaw_pitch_from_rotation(transform.rotation);
		controller.yaw = yaw;
		controller.pitch = pitch;
	}
}

fn focus_target_ready(
	focus: CameraFocus,
	shadow_body_pose: &Query<
		(&BoneMap, &RigBindScales, &CharacterRig),
		(With<FocusReferenceRig>, With<ActiveRigPose>),
	>,
	shadow_head_rigs: &Query<
		(&BoneMap, &CharacterRig),
		(With<FocusReferenceRig>, With<CharacterRig>, Without<NeedsSocketPlacement>),
	>,
	preview_body_pose: &Query<
		(&BoneMap, &RigBindScales, &CharacterRig),
		(With<ConceptPreviewRoot>, With<AnimatedBodyRig>, With<ActiveRigPose>),
	>,
	preview_head_rigs: &Query<
		(&BoneMap, &CharacterRig),
		(
			With<ConceptPreviewRoot>,
			With<CharacterRig>,
			Without<FocusReferenceRig>,
			Without<NeedsSocketPlacement>,
		),
	>,
	bone_globals: &Query<&GlobalTransform>,
) -> bool {
	if focus.uses_preview_sockets() {
		return preview_body_ready(preview_body_pose)
			&& preview_head_rigs.iter().any(|(bone_map, rig)| {
				rig.role == CharacterRigRole::Head && head_rig_focus_ready(bone_map, focus.socket)
			});
	}

	if !shadow_body_ready(shadow_body_pose, bone_globals) {
		return false;
	}

	match focus.rig {
		FocusRig::Body => true,
		FocusRig::Head => shadow_head_rigs.iter().any(|(bone_map, rig)| {
			rig.role == CharacterRigRole::Head && head_rig_focus_ready(bone_map, focus.socket)
		}),
	}
}

fn shadow_body_ready(
	shadow_body_pose: &Query<
		(&BoneMap, &RigBindScales, &CharacterRig),
		(With<FocusReferenceRig>, With<ActiveRigPose>),
	>,
	bone_globals: &Query<&GlobalTransform>,
) -> bool {
	shadow_body_pose.iter().any(|(bone_map, bind_scales, rig)| {
		rig.role == CharacterRigRole::Body
			&& shadow_body_pose_ready(bone_map, bind_scales, bone_globals)
	})
}

fn shadow_body_pose_ready(
	bone_map: &BoneMap,
	bind_scales: &RigBindScales,
	bone_globals: &Query<&GlobalTransform>,
) -> bool {
	bone_map_ready(bone_map)
		&& bind_scales_ready(bind_scales, bone_map)
		&& body_pose_landmark_ready(bone_map, bone_globals)
}

fn preview_body_ready(
	preview_body_pose: &Query<
		(&BoneMap, &RigBindScales, &CharacterRig),
		(With<ConceptPreviewRoot>, With<AnimatedBodyRig>, With<ActiveRigPose>),
	>,
) -> bool {
	preview_body_pose.iter().any(|(bone_map, bind_scales, rig)| {
		rig.role == CharacterRigRole::Body
			&& bone_map_ready(bone_map)
			&& bind_scales_ready(bind_scales, bone_map)
	})
}

fn body_pose_landmark_ready(bone_map: &BoneMap, bone_globals: &Query<&GlobalTransform>) -> bool {
	bone_global_y(bone_map, bone_globals, "upper_neck")
		.is_some_and(|y| y > BODY_POSE_LANDMARK_MIN_Y)
}

/// Head rig uses different landmarks than the body.
fn head_rig_focus_ready(bone_map: &BoneMap, socket: &str) -> bool {
	if socket == "root" {
		// `"root"` anchors on the rig entity once socket-attached, not a named bone.
		return true;
	}
	!bone_map.by_name.is_empty() && bone_map.by_name.contains_key(socket)
}

fn log_focus_apply(
	trigger: &str,
	focus: CameraFocus,
	shadow_rigs: &Query<(&BoneMap, &CharacterRig, &GlobalTransform), With<FocusReferenceRig>>,
	preview_rigs: &Query<
		(&BoneMap, &CharacterRig, &GlobalTransform),
		(With<CharacterRig>, With<ConceptPreviewRoot>, Without<FocusReferenceRig>),
	>,
	bone_globals: &Query<&GlobalTransform>,
	camera_before: Vec3,
	camera_after: Vec3,
	target: &Transform,
	settled: bool,
) {
	let shadow = rig_socket_report_shadow("shadow", focus, shadow_rigs, bone_globals);
	let preview = rig_socket_report_preview("preview", focus, preview_rigs, bone_globals);
	let root_delta_y = shadow
		.root_y
		.zip(preview.root_y)
		.map(|(shadow_y, preview_y)| shadow_y - preview_y);
	let look_at = target.translation + target.forward() * 2.0;
	let active = if focus.uses_preview_sockets() { &preview } else { &shadow };

	info!(
		"[camera-focus] apply trigger={trigger} settled={settled} source={} focus={} \
		camera_before=({:.3},{:.3},{:.3}) camera_after=({:.3},{:.3},{:.3}) \
		target=({:.3},{:.3},{:.3}) look_dir=({:.3},{:.3},{:.3}) \
		active_socket_y={} shadow_root_y={} preview_root_y={} root_delta_y={} \
		shadow_socket_y={} preview_socket_y={} shadow_rig_y={} preview_rig_y={}",
		focus.resolve_source_label(),
		focus_summary(focus),
		camera_before.x,
		camera_before.y,
		camera_before.z,
		camera_after.x,
		camera_after.y,
		camera_after.z,
		target.translation.x,
		target.translation.y,
		target.translation.z,
		look_at.x,
		look_at.y,
		look_at.z,
		fmt_y(active.socket_y),
		fmt_y(shadow.root_y),
		fmt_y(preview.root_y),
		root_delta_y.map(|d| format!("{d:.3}")).unwrap_or_else(|| "?".into()),
		fmt_y(shadow.socket_y),
		fmt_y(preview.socket_y),
		fmt_y(shadow.rig_root_y),
		fmt_y(preview.rig_root_y),
	);
}

fn fmt_y(y: Option<f32>) -> String {
	y.map(|y| format!("{y:.3}")).unwrap_or_else(|| "?".into())
}

struct RigSocketReport {
	rig_root_y: Option<f32>,
	root_y: Option<f32>,
	socket_y: Option<f32>,
}

fn rig_socket_report_shadow(
	label: &str,
	focus: CameraFocus,
	rigs: &Query<(&BoneMap, &CharacterRig, &GlobalTransform), With<FocusReferenceRig>>,
	bone_globals: &Query<&GlobalTransform>,
) -> RigSocketReport {
	rig_socket_report_inner(label, focus, rigs.iter(), bone_globals)
}

fn rig_socket_report_preview(
	label: &str,
	focus: CameraFocus,
	rigs: &Query<
		(&BoneMap, &CharacterRig, &GlobalTransform),
		(With<CharacterRig>, With<ConceptPreviewRoot>, Without<FocusReferenceRig>),
	>,
	bone_globals: &Query<&GlobalTransform>,
) -> RigSocketReport {
	rig_socket_report_inner(label, focus, rigs.iter(), bone_globals)
}

fn rig_socket_report_inner<'a>(
	label: &str,
	focus: CameraFocus,
	rigs: impl Iterator<Item = (&'a BoneMap, &'a CharacterRig, &'a GlobalTransform)>,
	bone_globals: &Query<&GlobalTransform>,
) -> RigSocketReport {
	let role = match focus.rig {
		FocusRig::Body => CharacterRigRole::Body,
		FocusRig::Head => CharacterRigRole::Head,
	};
	for (bone_map, rig, rig_global) in rigs {
		if rig.role != role {
			continue;
		}
		let root_y = bone_global_y(bone_map, bone_globals, "root");
		let socket_y = focus_socket_global(focus, bone_map, rig_global, bone_globals)
			.map(|socket| socket.translation().y);
		if focus_debug_enabled() && socket_y.is_none() {
			warn!(
				"[camera-focus] {label} missing socket bone {:?} (map has {} bones)",
				focus.socket,
				bone_map.by_name.len(),
			);
		}
		return RigSocketReport { rig_root_y: Some(rig_global.translation().y), root_y, socket_y };
	}
	if focus_debug_enabled() {
		warn!("[camera-focus] {label} no rig for role {:?}", role);
	}
	RigSocketReport { rig_root_y: None, root_y: None, socket_y: None }
}

fn bone_global_y(
	bone_map: &BoneMap,
	bone_globals: &Query<&GlobalTransform>,
	bone_name: &str,
) -> Option<f32> {
	let entity = bone_map.by_name.get(bone_name)?;
	bone_globals.get(*entity).ok().map(|t| t.translation().y)
}

fn focus_summary(focus: CameraFocus) -> String {
	format!(
		"rig={:?} socket={} cam_offset={:?} look_at_offset={:?}",
		focus.rig, focus.socket, focus.camera_offset, focus.look_at_offset
	)
}

fn resolve_focus_transform(
	focus: CameraFocus,
	shadow_rigs: &Query<(&BoneMap, &CharacterRig, &GlobalTransform), With<FocusReferenceRig>>,
	preview_rigs: &Query<
		(&BoneMap, &CharacterRig, &GlobalTransform),
		(With<CharacterRig>, With<ConceptPreviewRoot>, Without<FocusReferenceRig>),
	>,
	bone_globals: &Query<&GlobalTransform>,
) -> Option<Transform> {
	if focus.uses_preview_sockets() {
		resolve_focus_transform_from_rigs(focus, preview_rigs.iter(), bone_globals)
	} else {
		resolve_focus_transform_from_rigs(focus, shadow_rigs.iter(), bone_globals)
	}
}

fn resolve_focus_transform_from_rigs<'a>(
	focus: CameraFocus,
	rigs: impl Iterator<Item = (&'a BoneMap, &'a CharacterRig, &'a GlobalTransform)>,
	bone_globals: &Query<&GlobalTransform>,
) -> Option<Transform> {
	let role = match focus.rig {
		FocusRig::Body => CharacterRigRole::Body,
		FocusRig::Head => CharacterRigRole::Head,
	};
	for (bone_map, rig, rig_global) in rigs {
		if rig.role != role {
			continue;
		}
		let Some(socket) = focus_socket_global(focus, bone_map, rig_global, bone_globals) else {
			continue;
		};
		let camera_pos = socket_oriented_point(&socket, focus.camera_offset);
		let look_at = socket_oriented_point(&socket, focus.look_at_offset);
		return Some(Transform::from_translation(camera_pos).looking_at(look_at, Vec3::Y));
	}
	None
}

/// `"root"` framing anchors on the rig entity; other sockets use named bones.
fn focus_socket_global(
	focus: CameraFocus,
	bone_map: &BoneMap,
	rig_global: &GlobalTransform,
	bone_globals: &Query<&GlobalTransform>,
) -> Option<GlobalTransform> {
	if focus.socket == "root" {
		return Some(*rig_global);
	}
	let bone_entity = bone_map.by_name.get(focus.socket)?;
	bone_globals.get(*bone_entity).ok().copied()
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
