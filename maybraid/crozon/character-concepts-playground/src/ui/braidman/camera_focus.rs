use bevy::prelude::*;
use crozon_characters::SocketRig;

use super::UiAssetTarget;

/// Camera framing relative to a named rig socket bone.
///
/// Offsets are in **world meters** along the socket bone's local axes (rotation only),
/// ignoring bind-pose stretch scale on the bone chain.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraFocus {
	pub rig: SocketRig,
	pub socket: &'static str,
	pub camera_offset: Vec3,
	pub look_at_offset: Vec3,
}

impl CameraFocus {
	pub const fn new(
		rig: SocketRig,
		socket: &'static str,
		camera_offset: Vec3,
		look_at_offset: Vec3,
	) -> Self {
		Self { rig, socket, camera_offset, look_at_offset }
	}

	/// Socketed head features frame the visible preview; root pivots use the shadow rig.
	pub fn uses_preview_sockets(self) -> bool {
		matches!(self.rig, SocketRig::Head) && self.socket != "root"
	}

	pub fn resolve_source_label(self) -> &'static str {
		if self.uses_preview_sockets() {
			"preview"
		} else {
			"shadow"
		}
	}
}

impl UiAssetTarget {
	pub const fn camera_focus(self) -> CameraFocus {
		match self {
			// Root pivot sits at the ground; aim at upper torso for full-body framing.
			Self::Body(_) | Self::Animation(_) | Self::Clothing(_) => CameraFocus::new(
				SocketRig::Body,
				"root",
				Vec3::new(-1.0, 1.0, 4.0),
				Vec3::new(2.0, 0.0, -2.0),
			),
			// Head rig root is anchored at the neck base; bias look-at toward face height.
			Self::Head(_) => CameraFocus::new(
				SocketRig::Head,
				"root",
				Vec3::new(0.0, 0.0, 1.0),
				Vec3::new(0.0, 0.05, 0.0),
			),
			Self::Hair(_) => CameraFocus::new(
				SocketRig::Head,
				"crown_socket",
				Vec3::new(0.0, 0.15, 1.0),
				Vec3::ZERO,
			),
			Self::Eye(_) => CameraFocus::new(
				SocketRig::Head,
				"eye_socket.L",
				Vec3::new(0.0, 0.0, 0.35),
				Vec3::ZERO,
			),
			Self::Nose(_) => CameraFocus::new(
				SocketRig::Head,
				"nose_socket",
				Vec3::new(0.0, 0.0, 0.25),
				Vec3::ZERO,
			),
			Self::Mouth(_) => CameraFocus::new(
				SocketRig::Head,
				"mouth_socket",
				Vec3::new(0.0, 0.0, 0.25),
				Vec3::ZERO,
			),
			Self::Ear(_) => CameraFocus::new(
				SocketRig::Head,
				"ear_socket.L",
				Vec3::new(0.55, 0.0, 0.3),
				Vec3::ZERO,
			),
		}
	}
}
