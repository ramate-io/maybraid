//! Placement remaps between lower-left panel space and domain kit conventions.

use bevy_math::Vec3;

use crate::placed::Placement;

/// Pitch that tips kit \(+Z\) (authored height edge) to world \(+Y\).
///
/// Matches [`crate::partitions::PANEL_TO_WALL_PITCH`].
pub const WALL_STANDUP_PITCH: f32 = std::f32::consts::FRAC_PI_2;

/// Convert panel-space lower-left rectangle placement (\(X,Z \in [0, 1]\)) to a
/// centered floor kit (\(X,Z \in [-1, 1]\)).
///
/// Panel scale is full edge length; floor scale is half-extent (\(L/2\)).
pub fn to_centered_rect_placement(p: Placement) -> Placement {
	let s = p.scale;
	// Unit-square panel: center at local \((0.5, 0, 0.5)\) before scale.
	let local_mid = Vec3::new(s.x * 0.5, 0.0, s.z * 0.5);
	Placement {
		translation: p.translation + p.rotation() * local_mid,
		yaw: p.yaw,
		pitch: p.pitch,
		roll: p.roll,
		scale: Vec3::new((s.x * 0.5).max(1e-4), s.y, (s.z * 0.5).max(1e-4)),
	}
}

/// Apply wall stand-up pitch to a ground-authored panel placement.
///
/// Keeps scale as \((\texttt{length}, \texttt{thick}, \texttt{height})\) on \((X,Y,Z)\).
pub fn with_wall_standup_pitch(p: Placement) -> Placement {
	Placement {
		translation: p.translation,
		yaw: p.yaw,
		pitch: p.pitch + WALL_STANDUP_PITCH,
		roll: p.roll,
		scale: p.scale,
	}
}
