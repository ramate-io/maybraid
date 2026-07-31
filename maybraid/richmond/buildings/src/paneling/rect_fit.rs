//! Best-fit ordinary rectangle for a ruled bay `{a0,a1,b0,b1}` in its average plane.

use bevy_math::{Vec2, Vec3};

/// Fitted world corners `(a0, a1, b0, b1)` of an ordinary rectangle, or [`None`] if degenerate.
///
/// Plane: average of normals for tris `(a0,a1,b1)` and `(a0,b1,b0)`. Frame: origin at
/// \(a_0\), \(+X\) along in-plane \(a_1-a_0\), \(+Y\) = unit normal, \(+Z\) completes RH.
/// Rectangle: width = in-plane \(\|a_1-a_0\|\), depth = mean in-plane distance of
/// \(b_0,b_1\) from the \(a_0{\to}a_1\) edge (same sign as the first non-zero).
pub fn fit_rectangle_corners(a0: Vec3, a1: Vec3, b0: Vec3, b1: Vec3) -> Option<[Vec3; 4]> {
	let n0 = (a1 - a0).cross(b1 - a0);
	let n1 = (b1 - a0).cross(b0 - a0);
	let n = n0 + n1;
	let n_len = n.length();
	if n_len < 1e-10 {
		return None;
	}
	let normal = n / n_len;

	let edge = a1 - a0;
	let edge_on = edge - normal * edge.dot(normal);
	let width = edge_on.length();
	if width < 1e-8 {
		return None;
	}
	let e0 = edge_on / width;
	let e1 = e0.cross(normal); // +Z in panel XZ

	let project = |p: Vec3| -> Vec2 {
		let d = p - a0;
		Vec2::new(d.dot(e0), d.dot(e1))
	};
	let zb0 = project(b0).y;
	let zb1 = project(b1).y;
	let sign = if zb0.abs() >= zb1.abs() {
		zb0.signum()
	} else {
		zb1.signum()
	};
	let sign = if sign.abs() < 0.5 { 1.0 } else { sign };
	let depth = ((zb0.abs() + zb1.abs()) * 0.5).max(1e-4) * sign;

	let fa0 = a0;
	let fa1 = a0 + e0 * width;
	let fb0 = a0 + e1 * depth;
	let fb1 = a0 + e0 * width + e1 * depth;
	Some([fa0, fa1, fb0, fb1])
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn planar_rectangle_is_unchanged() {
		let a0 = Vec3::ZERO;
		let a1 = Vec3::new(2.0, 0.0, 0.0);
		let b0 = Vec3::new(0.0, 0.0, 1.0);
		let b1 = Vec3::new(2.0, 0.0, 1.0);
		let [fa0, fa1, fb0, fb1] = fit_rectangle_corners(a0, a1, b0, b1).unwrap();
		assert!((fa0 - a0).length() < 1e-4);
		assert!((fa1 - a1).length() < 1e-4);
		assert!((fb0 - b0).length() < 1e-4);
		assert!((fb1 - b1).length() < 1e-4);
	}

	#[test]
	fn skew_bay_becomes_planar_rectangle() {
		let a0 = Vec3::ZERO;
		let a1 = Vec3::new(2.0, 0.0, 0.0);
		let b0 = Vec3::new(0.1, 0.2, 1.0);
		let b1 = Vec3::new(2.2, -0.1, 1.1);
		let [fa0, fa1, fb0, fb1] = fit_rectangle_corners(a0, a1, b0, b1).unwrap();
		let n0 = (fa1 - fa0).cross(fb1 - fa0).normalize();
		let n1 = (fb1 - fa0).cross(fb0 - fa0).normalize();
		assert!(n0.dot(n1) > 0.999, "fitted corners should be coplanar");
		let e0 = (fa1 - fa0).normalize();
		let e1 = (fb0 - fa0).normalize();
		assert!(e0.dot(e1).abs() < 1e-3, "edges should be orthogonal");
		assert!(((fa1 - fa0) - (fb1 - fb0)).length() < 1e-3);
		assert!(((fb0 - fa0) - (fb1 - fa1)).length() < 1e-3);
		assert!((fa1 - fa0).length() > 1.0);
		assert!((fb0 - fa0).length() > 0.5);
	}
}
