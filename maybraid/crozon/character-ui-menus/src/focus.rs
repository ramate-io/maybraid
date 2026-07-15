use bevy_math::Vec3;
use character_ui_menu::{CameraFocus, FocusRig};

pub const BODY_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Body, "root", Vec3::new(-1.0, 1.0, 4.0), Vec3::new(2.0, 0.0, -2.0));

pub const HEAD_ROOT_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Head, "root", Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 0.05, 0.0));

pub const CROWN_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Head, "crown_socket", Vec3::new(0.0, 0.15, 1.0), Vec3::ZERO);

pub const EYE_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Head, "eye_socket.L", Vec3::new(0.0, 0.0, 0.35), Vec3::ZERO);

pub const NOSE_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Head, "nose_socket", Vec3::new(0.0, 0.0, 0.25), Vec3::ZERO);

pub const MOUTH_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Head, "mouth_socket", Vec3::new(0.0, 0.0, 0.25), Vec3::ZERO);

pub const EAR_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Head, "ear_socket.L", Vec3::new(0.55, 0.0, 0.3), Vec3::ZERO);

/// Chupri framing — closer for the ~1ft (~0.15×) creature.
pub const CHUPRI_BODY_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Body, "root", Vec3::new(-0.2, 0.2, 0.8), Vec3::new(0.3, 0.05, -0.3));

/// Tipple framing — same close framing as Chupri (~1ft / 0.15×).
pub const TIPPLE_BODY_FOCUS: CameraFocus = CHUPRI_BODY_FOCUS;

/// Medium framing for ~2ft birds (Topple / Kispar at 0.30×).
pub const SMALL_BIRD_BODY_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Body, "root", Vec3::new(-0.4, 0.4, 1.5), Vec3::new(0.6, 0.05, -0.5));

/// Spibmom framing — pulled back for the 2× head rig and long neck.
pub const SPIBMOM_BODY_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Body, "root", Vec3::new(-1.2, 2.0, 10.0), Vec3::new(2.0, 0.0, -2.0));

/// Grener (~3 m shark) — BODY_FOCUS angle, pulled back for the larger silhouette.
pub const GRENER_BODY_FOCUS: CameraFocus = CameraFocus::new(
	FocusRig::Body,
	"dorsal_socket",
	Vec3::new(-1.5, 1.5, 7.0),
	Vec3::new(2.0, 0.0, -2.0),
);

/// Thumplus (~6 m whale) — same angle, backed up further.
pub const THUMPLUS_BODY_FOCUS: CameraFocus = CameraFocus::new(
	FocusRig::Body,
	"dorsal_socket",
	Vec3::new(-3.0, 3.0, 14.0),
	Vec3::new(4.0, 0.0, -4.0),
);

/// Mistler (~1 ft sprite fish) — BODY_FOCUS angle at Tipple/Chupri distance.
pub const MISTLER_BODY_FOCUS: CameraFocus = CameraFocus::new(
	FocusRig::Body,
	"dorsal_socket",
	Vec3::new(-0.3, 0.3, 1.2),
	Vec3::new(0.4, 0.05, -0.4),
);

pub const SPIBMOM_HEAD_ROOT_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Head, "root", Vec3::new(0.0, 1.0, 7.0), Vec3::new(0.0, 0.05, 0.0));

pub const SPIBMOM_CROWN_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Head, "crown_socket", Vec3::new(0.0, 0.15, 5.0), Vec3::ZERO);

pub const SPIBMOM_EYE_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Head, "eye_socket.L", Vec3::new(0.0, 0.0, 3.1), Vec3::ZERO);

pub const SPIBMOM_NOSE_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Head, "nose_socket", Vec3::new(0.0, 0.0, 3.1), Vec3::ZERO);
