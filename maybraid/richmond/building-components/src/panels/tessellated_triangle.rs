//! Tessellated fill of a planar triangle with posed unit right-triangle kits.
//!
//! Corners are **2D panel-space** points \((X, Z)\). Mapping that plane into 3D
//! (pitch, wall stand-up, etc.) is a higher-order concern via parent [`Placement`].
//!
//! Kit footprint: \(X \in [0, 1]\), \(Z \in [0, 1]\) (right angle at the origin).
//! A general triangle is altitude-split into two right triangles; each half is one
//! scaled kit (yaw + non-uniform scale in the panel plane).

use bevy_math::{Vec2, Vec3};

use crate::panels::geometry::{PanelGeometry, RightTriangle};
use crate::panels::placement::yaw_along_xz;
use crate::placed::{Placed, Placement};

/// Three panel-space corners \((X, Z)\) filled by right-triangle kits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TessellatedTriangle {
	pub a: Vec2,
	pub b: Vec2,
	pub c: Vec2,
}

impl Default for TessellatedTriangle {
	fn default() -> Self {
		Self {
			a: Vec2::ZERO,
			b: Vec2::new(1.0, 0.0),
			c: Vec2::new(0.0, 1.0),
		}
	}
}

impl TessellatedTriangle {
	pub fn new(a: Vec2, b: Vec2, c: Vec2) -> Self {
		Self { a, b, c }
	}

	/// Expand into posed [`RightTriangle`] leaves (identity parent).
	pub fn decompose(self) -> Vec<Placed<PanelGeometry>> {
		let ab = self.b - self.a;
		let ac = self.c - self.a;
		if (ab.x * ac.y - ab.y * ac.x).abs() < 1e-12 {
			return Vec::new();
		}

		let edges = [
			(self.a, self.b, self.c),
			(self.b, self.c, self.a),
			(self.c, self.a, self.b),
		];
		let (ei, _) = edges
			.iter()
			.enumerate()
			.max_by(|(_, (p0, p1, _)), (_, (q0, q1, _))| {
				(*p1 - *p0)
					.length_squared()
					.partial_cmp(&(*q1 - *q0).length_squared())
					.unwrap_or(std::cmp::Ordering::Equal)
			})
			.expect("three edges");

		let (p0, p1, apex) = edges[ei];
		let edge = p1 - p0;
		let edge_len2 = edge.length_squared();
		if edge_len2 < 1e-12 {
			return Vec::new();
		}
		let t = ((apex - p0).dot(edge) / edge_len2).clamp(0.0, 1.0);
		let foot = p0 + edge * t;

		let mut out = Vec::new();
		out.extend(place_right_triangle(foot, p0, apex));
		out.extend(place_right_triangle(foot, p1, apex));
		out
	}
}

/// One kit for the right triangle with right angle at `right_angle`, legs to `leg_u` / `leg_v`.
///
/// Kit \(+X\) maps along the first leg, kit \(+Z\) along the second (\(X,Z \in [0, 1]\)).
/// Panel \(+Y\) is thickness; yaw alone orients the kit in the \(XZ\) plane.
fn place_right_triangle(
	right_angle: Vec2,
	leg_u: Vec2,
	leg_v: Vec2,
) -> Vec<Placed<PanelGeometry>> {
	let u = leg_u - right_angle;
	let v = leg_v - right_angle;
	let u_len = u.length();
	let v_len = v.length();
	if u_len < 1e-6 || v_len < 1e-6 {
		return Vec::new();
	}

	// 2D cross \(u_x v_z - u_z v_x\). Prefer winding where kit \(+X\) then \(+Z\)
	// matches the altitude half without mirroring.
	let cross = u.x * v.y - u.y * v.x;
	let (x_leg, sx, sz) = if cross >= 0.0 {
		(u, u_len, v_len)
	} else {
		(v, v_len, u_len)
	};
	let x_dir = x_leg / sx;

	let yaw = yaw_along_xz(x_dir.x, x_dir.y);
	vec![Placed::with_placement(
		PanelGeometry::RightTriangle(RightTriangle { mirror: None }),
		Placement::new(Vec3::new(right_angle.x, 0.0, right_angle.y), yaw)
			.with_scale(Vec3::new(sx, 1.0, sz)),
	)]
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn right_triangle_emits_kits() {
		let t = TessellatedTriangle::new(Vec2::ZERO, Vec2::new(2.0, 0.0), Vec2::new(0.0, 3.0));
		let pieces = t.decompose();
		assert!(!pieces.is_empty());
		assert!(pieces.iter().all(|p| matches!(p.geom, PanelGeometry::RightTriangle(_))));
		assert!(pieces.iter().all(|p| p.scale().x > 0.0 && p.scale().z > 0.0));
	}

	#[test]
	fn unit_right_triangle_two_halves() {
		let t = TessellatedTriangle::new(Vec2::ZERO, Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0));
		let pieces = t.decompose();
		assert_eq!(pieces.len(), 2);
		assert!(pieces.iter().all(|p| p.scale().x > 1e-4 && p.scale().z > 1e-4));
	}

	#[test]
	fn degenerate_is_empty() {
		let t = TessellatedTriangle::new(Vec2::ZERO, Vec2::new(1.0, 0.0), Vec2::new(2.0, 0.0));
		assert!(t.decompose().is_empty());
	}
}
