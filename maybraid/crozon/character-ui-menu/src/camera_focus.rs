use bevy_math::Vec3;
use crozon_characters::SocketRig;

/// Camera framing relative to a named rig socket bone.
///
/// Offsets are in world meters along the socket bone's local axes.
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
