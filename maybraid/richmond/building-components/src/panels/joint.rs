//! Circular / post joint between abutting panel segments.

use bevy_math::Vec3;

use crate::panels::placement::wrap_pi;
use crate::placed::{Placed, Placement};

/// Joint kit half-extent in \(X/Z\) (\([-0.5, 0.5]\)).
pub const JOINT_KIT_HALF: f32 = 0.5;
/// Base world radius when the kink is purely planar.
pub const JOINT_BASE_RADIUS: f32 = 0.15;
/// Extra world radius per radian of vertical (slope) kink.
pub const JOINT_RADIUS_PER_SLOPE_RAD: f32 = 0.55;

/// Circular joint filler posed on the average inbound/outbound angle.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Joint;

impl Joint {
	/// Placement for a joint at `cur` bridging inbound/outbound plan and slope angles.
	pub fn placed_at(
		cur: Vec3,
		yaw_in: f32,
		yaw_out: f32,
		roll_in: f32,
		roll_out: f32,
	) -> Placed<Joint> {
		let dyaw = wrap_pi(yaw_out - yaw_in).abs();
		let droll = (roll_out - roll_in).abs();
		let kink = dyaw.max(droll);
		let radius = JOINT_BASE_RADIUS + JOINT_RADIUS_PER_SLOPE_RAD * kink;
		let xz = (radius / JOINT_KIT_HALF).max(1e-4);
		let yaw = yaw_in + 0.5 * wrap_pi(yaw_out - yaw_in);
		Placed {
			geom: Joint,
			placement: Placement::new(cur, yaw).with_scale(Vec3::new(xz, 1.0, xz)),
		}
	}
}
