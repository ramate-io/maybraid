//! Shared quadruped pose helpers.

use bevy::prelude::*;
use crozon_rigs::{BoneRotation, RigPoseLayer};

use crate::assembly::{SocketAttachment, SocketRig};

/// Pitch the quadruped `neck` bone; pair with [`head_socket_counterpose`] so the
/// head stays level on the pitched neck.
pub fn neck_pitch_rotation(pitch_radians: f32) -> BoneRotation {
	BoneRotation::pitch_x("neck", pitch_radians)
}

/// Head-socket local transform that cancels [`neck_pitch_rotation`].
pub fn head_socket_counterpose(pitch_radians: f32) -> Transform {
	Transform::from_rotation(Quat::from_rotation_x(-pitch_radians))
}

/// Convenience: body-rig `head_socket` attachment with neck counterpose.
pub fn head_socket_attachment(pitch_radians: f32) -> SocketAttachment {
	SocketAttachment {
		rig: SocketRig::Body,
		bone: "head_socket",
		local_transform: head_socket_counterpose(pitch_radians),
	}
}

/// Pose layer that only pitches the quadruped neck.
pub fn neck_pitch_layer(label: &'static str, pitch_radians: f32) -> RigPoseLayer {
	RigPoseLayer::new(label).with_rotation(neck_pitch_rotation(pitch_radians))
}
