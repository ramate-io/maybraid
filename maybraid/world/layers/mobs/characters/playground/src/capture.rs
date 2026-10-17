//! Xvfb capture helpers (`MAYBRAID_PLAYGROUND_CAPTURE`, `MAYBRAID_PLAYGROUND_CAPTURE_NIGHT`).

use bevy::prelude::*;
use characters::{assembly::CharacterPartSlot, nodes::PartNode, CharacterMembers};
use game_commands::ui::GameCommandDrawerVisible;

use crate::camera::{orient_camera, CameraController, FaceCaptureRange};
use crate::character::{CharacterConfig, CharacterSpecies, PlaygroundCharacter};

#[derive(Resource, Debug, Default)]
pub struct PendingFaceCapture {
	pub range: Option<FaceCaptureRange>,
}

pub fn capture_mode() -> bool {
	std::env::var_os("MAYBRAID_PLAYGROUND_CAPTURE").is_some()
}

pub fn capture_night() -> bool {
	capture_mode() && std::env::var_os("MAYBRAID_PLAYGROUND_CAPTURE_NIGHT").is_some()
}

fn face_capture_range_from_env() -> FaceCaptureRange {
	match std::env::var("MAYBRAID_FACE_CAPTURE_RANGE")
		.as_deref()
		.map(str::trim)
		.map(str::to_ascii_lowercase)
	{
		Ok(value) if value == "warning-distance" || value == "distance" => {
			FaceCaptureRange::WarningDistance
		},
		_ => FaceCaptureRange::Close,
	}
}

pub fn queue_face_capture_from_env(mut pending: ResMut<PendingFaceCapture>) {
	if !capture_mode() {
		return;
	}
	if pending.range.is_none() {
		pending.range = Some(face_capture_range_from_env());
	}
}

/// Dim scene lighting so eye emissive reads at distance.
pub fn apply_capture_lighting(
	mut clear: ResMut<ClearColor>,
	mut lights: Query<&mut DirectionalLight>,
) {
	if !capture_night() {
		return;
	}
	clear.0 = Color::srgb(0.012, 0.016, 0.028);
	let mut idx = 0;
	for mut light in &mut lights {
		light.illuminance = if idx == 0 { 8.0 } else { 1.5 };
		light.shadow_maps_enabled = false;
		idx += 1;
	}
}

/// Frame both eyes once the fixed assembly has part hosts in the world.
pub fn apply_pending_face_capture(
	mut pending: ResMut<PendingFaceCapture>,
	config: Res<CharacterConfig>,
	roots: Query<(&CharacterMembers, &GlobalTransform), With<PlaygroundCharacter>>,
	parts: Query<&PartNode>,
	transforms: Query<&GlobalTransform>,
	mut cameras: Query<(&mut Transform, &mut CameraController), With<Camera3d>>,
) {
	let Some(range) = pending.range else {
		return;
	};
	let Some((members, root_tf)) = roots.iter().next() else {
		return;
	};
	let Some((eye, target)) =
		frame_from_eye_sockets(members, root_tf, config.species, range, &parts, &transforms)
	else {
		return;
	};
	let Ok((mut cam_tf, mut controller)) = cameras.single_mut() else {
		return;
	};
	orient_camera(&mut cam_tf, &mut controller, eye, target);
	pending.range = None;
}

fn frame_from_eye_sockets(
	members: &CharacterMembers,
	root_tf: &GlobalTransform,
	species: CharacterSpecies,
	range: FaceCaptureRange,
	parts: &Query<&PartNode>,
	transforms: &Query<&GlobalTransform>,
) -> Option<(Vec3, Vec3)> {
	let mut left = None;
	let mut right = None;
	for member in members.iter() {
		let Ok(part) = parts.get(member) else {
			continue;
		};
		let Ok(tf) = transforms.get(member) else {
			continue;
		};
		match part.slot {
			CharacterPartSlot::EyeLeft => left = Some(tf.translation()),
			CharacterPartSlot::EyeRight => right = Some(tf.translation()),
			_ => {},
		}
	}
	let (left, right) = left.zip(right)?;
	let eye_separation = (right - left).length();
	if eye_separation < 0.06 {
		return None;
	}
	let center = (left + right) * 0.5;
	let right_dir = (right - left).normalize();
	let mut forward = right_dir.cross(Vec3::Y);
	if forward.length_squared() < 1e-5 {
		forward = root_tf.rotation() * Vec3::Z;
	} else {
		forward = forward.normalize();
	}
	if forward.dot(root_tf.rotation() * Vec3::Z) < 0.0 {
		forward = -forward;
	}
	let standoff = match (species, range) {
		(CharacterSpecies::Braidman, FaceCaptureRange::Close) => (eye_separation * 3.2).max(0.42),
		(CharacterSpecies::Braidman, FaceCaptureRange::WarningDistance) => 2.65,
		(CharacterSpecies::Epiphant, FaceCaptureRange::Close) => (eye_separation * 3.6).max(0.55),
		(CharacterSpecies::Epiphant, FaceCaptureRange::WarningDistance) => 3.1,
	};
	let eye = center + forward * standoff;
	Some((eye, center))
}

/// Collapse the command drawer if capture env was set after plugin build.
pub fn ensure_capture_ui_hidden(mut visible: ResMut<GameCommandDrawerVisible>) {
	if capture_mode() {
		visible.0 = false;
	}
}
