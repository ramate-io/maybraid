use bevy_math::Vec3;
use character_ui_menu::CameraFocus;
use crozon_characters::SocketRig;

pub const BODY_FOCUS: CameraFocus =
	CameraFocus::new(SocketRig::Body, "root", Vec3::new(-1.0, 1.0, 4.0), Vec3::new(2.0, 0.0, -2.0));

pub const HEAD_ROOT_FOCUS: CameraFocus =
	CameraFocus::new(SocketRig::Head, "root", Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 0.05, 0.0));

pub const CROWN_FOCUS: CameraFocus =
	CameraFocus::new(SocketRig::Head, "crown_socket", Vec3::new(0.0, 0.15, 1.0), Vec3::ZERO);

pub const EYE_FOCUS: CameraFocus =
	CameraFocus::new(SocketRig::Head, "eye_socket.L", Vec3::new(0.0, 0.0, 0.35), Vec3::ZERO);

pub const NOSE_FOCUS: CameraFocus =
	CameraFocus::new(SocketRig::Head, "nose_socket", Vec3::new(0.0, 0.0, 0.25), Vec3::ZERO);

pub const MOUTH_FOCUS: CameraFocus =
	CameraFocus::new(SocketRig::Head, "mouth_socket", Vec3::new(0.0, 0.0, 0.25), Vec3::ZERO);

pub const EAR_FOCUS: CameraFocus =
	CameraFocus::new(SocketRig::Head, "ear_socket.L", Vec3::new(0.55, 0.0, 0.3), Vec3::ZERO);
