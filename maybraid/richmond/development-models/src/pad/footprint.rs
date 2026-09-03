//! Pad support geometry. Rectangular flatten for now; grading footprints later.

use bevy::math::Vec2;

use crate::cell::yawed_plan_aabb_extent;

/// Building-skirt support geometry.
#[derive(Debug, Clone)]
pub enum PadFootprint {
	/// Rounded rectangle in a yawed local frame (building plan).
	Rect(PadRect),
}

/// Rounded rectangle: `half_extents` in the local frame after [`Self::yaw`].
#[derive(Debug, Clone, Copy)]
pub struct PadRect {
	pub center: Vec2,
	pub half_extents: Vec2,
	/// Radians about \(+Y\), same sense as [`crate::cell::yaw_about_xz`].
	pub yaw: f32,
	pub round: f32,
}

impl PadFootprint {
	pub fn sdf(&self, p: Vec2) -> f32 {
		match self {
			Self::Rect(r) => r.sdf(p),
		}
	}

	pub fn aabb(&self) -> (Vec2, Vec2) {
		match self {
			Self::Rect(r) => r.aabb(),
		}
	}
}

impl PadRect {
	/// World point → building-local frame (inverse of [`crate::cell::yaw_about_xz`]).
	///
	/// Bevy `Quat::from_rotation_y` maps local \((x,z)\) to
	/// \((x\cos\theta + z\sin\theta,\; -x\sin\theta + z\cos\theta)\).
	pub fn local(&self, p: Vec2) -> Vec2 {
		let (s, c) = self.yaw.sin_cos();
		let d = p - self.center;
		Vec2::new(c * d.x - s * d.y, s * d.x + c * d.y)
	}

	/// Rounded-box SDF in the yawed frame (IQ `sdRoundedBox`).
	pub fn sdf(&self, p: Vec2) -> f32 {
		let local = self.local(p);
		let round = self.round.max(0.0);
		let half = self.half_extents.max(Vec2::splat(round.max(1e-3)));
		let q = local.abs() - half + Vec2::splat(round);
		let outside = q.max(Vec2::ZERO).length() - round;
		let inside = q.x.max(q.y).min(0.0);
		outside + inside
	}

	pub fn aabb(&self) -> (Vec2, Vec2) {
		let full = self.half_extents * 2.0;
		let occ = yawed_plan_aabb_extent(full.x, full.y, self.yaw);
		let half = occ * 0.5 + Vec2::splat(self.round.max(0.0));
		(self.center - half, self.center + half)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::f32::consts::FRAC_PI_2;

	#[test]
	fn rect_sdf_negative_inside() {
		let r = PadRect {
			center: Vec2::ZERO,
			half_extents: Vec2::new(10.0, 4.0),
			yaw: 0.0,
			round: 0.0,
		};
		assert!(r.sdf(Vec2::ZERO) < -1.0);
		assert!(r.sdf(Vec2::new(9.0, 0.0)) < 0.0);
		assert!(r.sdf(Vec2::new(11.0, 0.0)) > 0.0);
		assert!(r.sdf(Vec2::new(0.0, 5.0)) > 0.0);
	}

	#[test]
	fn yawed_rect_tracks_local_x() {
		let r = PadRect {
			center: Vec2::ZERO,
			half_extents: Vec2::new(10.0, 4.0),
			yaw: FRAC_PI_2,
			round: 0.0,
		};
		// Local +x is world −z after +π/2 yaw (`Quat::from_rotation_y`).
		assert!(r.sdf(Vec2::new(0.0, -9.0)) < 0.0);
		assert!(r.sdf(Vec2::new(0.0, -11.0)) > 0.0);
		assert!(r.sdf(Vec2::new(5.0, 0.0)) > 0.0);
	}
}
