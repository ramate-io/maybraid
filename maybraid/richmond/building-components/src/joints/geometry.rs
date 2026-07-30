//! Continuous joint geometry and placement helpers.

use bevy_math::Vec3;

use crate::panels::wrap_pi;
use crate::placed::{Placed, Placement};

/// Joint kit half-extent in \(X/Z\) (\([-0.5, 0.5]\)).
pub const JOINT_KIT_HALF: f32 = 0.5;
/// Base world radius when the kink is purely planar.
pub const JOINT_BASE_RADIUS: f32 = 0.15;
/// Extra world radius per radian of kink (plan, slope, or dihedral).
pub const JOINT_RADIUS_PER_SLOPE_RAD: f32 = 0.55;

/// Continuous joint form. Authored vertically: \(X,Z \in [-0.5, 0.5]\), \(Y \in [0, 1]\).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JointGeometry {
	/// Circular post / crease filler.
	Post(JointPost),
}

impl Default for JointGeometry {
	fn default() -> Self {
		Self::post()
	}
}

impl JointGeometry {
	pub fn post() -> Self {
		Self::Post(JointPost)
	}
}

/// Alias kept for migration; prefer [`JointGeometry`].
pub type Joint = JointGeometry;

/// Unit circular joint post.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct JointPost;

/// World \(X/Z\) scale from a kink angle (radians).
pub fn joint_xz_scale(kink_rad: f32) -> f32 {
	let radius = JOINT_BASE_RADIUS + JOINT_RADIUS_PER_SLOPE_RAD * kink_rad.abs();
	(radius / JOINT_KIT_HALF).max(1e-4)
}

impl JointPost {
	/// Placement at `cur` bridging inbound/outbound plan and slope angles.
	///
	/// `height` scales kit \(Y\) (kit \(Y \in [0, 1]\)).
	pub fn placed_at(
		cur: Vec3,
		yaw_in: f32,
		yaw_out: f32,
		roll_in: f32,
		roll_out: f32,
		height: f32,
	) -> Placed<JointGeometry> {
		let dyaw = wrap_pi(yaw_out - yaw_in).abs();
		let droll = (roll_out - roll_in).abs();
		let kink = dyaw.max(droll);
		let xz = joint_xz_scale(kink);
		let yaw = yaw_in + 0.5 * wrap_pi(yaw_out - yaw_in);
		Placed {
			geom: JointGeometry::post(),
			placement: Placement::new(cur, yaw).with_scale(Vec3::new(
				xz,
				height.max(1e-4),
				xz,
			)),
		}
	}

	/// Placement along a crease of length `height` with a given dihedral kink.
	pub fn placed_along_edge(mid: Vec3, yaw: f32, dihedral_kink: f32, height: f32) -> Placement {
		let xz = joint_xz_scale(dihedral_kink);
		Placement::new(mid, yaw).with_scale(Vec3::new(xz, height.max(1e-4), xz))
	}
}
