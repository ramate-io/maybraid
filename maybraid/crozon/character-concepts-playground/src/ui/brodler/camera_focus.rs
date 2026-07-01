use bevy::prelude::*;
use crozon_characters::SocketRig;

use super::UiAssetTarget;

/// Camera framing relative to a named rig socket bone.
pub use crate::ui::braidman::CameraFocus;

impl UiAssetTarget {
	pub const fn camera_focus(self) -> CameraFocus {
		match self {
			Self::Head(_) => CameraFocus::new(
				SocketRig::Head,
				"root",
				Vec3::new(0.0, 0.0, 1.0),
				Vec3::new(0.0, 0.05, 0.0),
			),
			Self::Horns(_) => CameraFocus::new(
				SocketRig::Head,
				"crown_socket",
				Vec3::new(0.0, 0.15, 1.0),
				Vec3::ZERO,
			),
			Self::Hair(_) => CameraFocus::new(
				SocketRig::Head,
				"crown_socket",
				Vec3::new(0.0, 0.15, 1.0),
				Vec3::ZERO,
			),
			Self::Clothing(_) | Self::Body | Self::Animation(_) => CameraFocus::new(
				SocketRig::Body,
				"root",
				Vec3::new(-1.0, 1.0, 4.0),
				Vec3::new(2.0, 0.0, -2.0),
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
