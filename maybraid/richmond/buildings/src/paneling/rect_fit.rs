//! Best-fit ordinary rectangle for a ruled bay `{a0,a1,b0,b1}` in its average plane.

use bevy_math::{EulerRot, Mat3, Quat, Vec3};
use richmond_building_components::Placement;

/// Fitted ordinary rectangle in world space (edge-aligned to \(a_0{\to}a_1\)).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FittedRect {
	pub a0: Vec3,
	pub a1: Vec3,
	pub b0: Vec3,
	pub b1: Vec3,
	/// Unit \(+X\) (along \(a_0{\to}a_1\) in plane).
	pub e0: Vec3,
	/// Unit toward the \(b\)-rail in plane.
	pub e1: Vec3,
	pub normal: Vec3,
	pub width: f32,
	pub depth: f32,
}

impl FittedRect {
	/// Panel-kit placement for the sub-rectangle covering panel
	/// \(u\in[u_0,u_0+w]\), \(v\in[v_0,v_0+d]\) (origin at \(a_0\), \(+v\) along [`Self::e1`]).
	///
	/// Kit footprint is \(X\in[0,1]\), \(Z\in[-1,0]\); scale is `(w, thickness, d)`.
	pub fn panel_placement(&self, u0: f32, v0: f32, w: f32, d: f32, thickness: f32) -> Placement {
		let origin = self.a0 + self.e0 * u0 + self.e1 * v0;
		// Kit +Z → −e1 so kit −Z (depth) lands along +e1 toward the b-rail.
		let rotation = Quat::from_mat3(&Mat3::from_cols(self.e0, self.normal, -self.e1));
		let (yaw, pitch, roll) = rotation.to_euler(EulerRot::YXZ);
		Placement {
			translation: origin,
			yaw,
			pitch,
			roll,
			scale: Vec3::new(w.max(1e-4), thickness.max(1e-4), d.max(1e-4)),
		}
	}

	pub fn solid_placement(&self, thickness: f32) -> Placement {
		self.panel_placement(0.0, 0.0, self.width, self.depth, thickness)
	}
}

/// Fitted world corners `(a0, a1, b0, b1)` of an ordinary rectangle, or [`None`] if degenerate.
pub fn fit_rectangle_corners(a0: Vec3, a1: Vec3, b0: Vec3, b1: Vec3) -> Option<[Vec3; 4]> {
	fit_rectangle(a0, a1, b0, b1).map(|f| [f.a0, f.a1, f.b0, f.b1])
}

/// Fit an ordinary rectangle for bay corners, or [`None`] if degenerate.
///
/// Plane: average of normals for tris `(a0,a1,b1)` and `(a0,b1,b0)`. Frame: origin at
/// \(a_0\), \(+X\) along in-plane \(a_1-a_0\), depth = mean in-plane distance of
/// \(b_0,b_1\) from the \(a_0{\to}a_1\) edge.
pub fn fit_rectangle(a0: Vec3, a1: Vec3, b0: Vec3, b1: Vec3) -> Option<FittedRect> {
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
	let e1_raw = e0.cross(normal);

	let project_v = |p: Vec3| (p - a0).dot(e1_raw);
	let zb0 = project_v(b0);
	let zb1 = project_v(b1);
	let sign = if zb0.abs() >= zb1.abs() {
		zb0.signum()
	} else {
		zb1.signum()
	};
	let sign = if sign.abs() < 0.5 { 1.0 } else { sign };
	let depth = ((zb0.abs() + zb1.abs()) * 0.5).max(1e-4);
	let e1 = e1_raw * sign;

	let fa0 = a0;
	let fa1 = a0 + e0 * width;
	let fb0 = a0 + e1 * depth;
	let fb1 = a0 + e0 * width + e1 * depth;
	Some(FittedRect {
		a0: fa0,
		a1: fa1,
		b0: fb0,
		b1: fb1,
		e0,
		e1,
		normal: e0.cross(e1).normalize(),
		width,
		depth,
	})
}

/// Panel-space axis-aligned inset margins (same units as fitted width/depth).
///
/// Zero margins → solid fill. Positive margins punch a rectangular opening framed
/// by up to four [`PanelGeometry::Rectangle`](richmond_building_components::panels::PanelGeometry) kits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectInset {
	pub left: f32,
	pub right: f32,
	/// From the \(a\)-rail (\(v = 0\)).
	pub bottom: f32,
	/// From the \(b\)-rail (\(v = \texttt{depth}\)).
	pub top: f32,
}

impl RectInset {
	pub const ZERO: Self = Self {
		left: 0.0,
		right: 0.0,
		bottom: 0.0,
		top: 0.0,
	};

	pub fn uniform(m: f32) -> Self {
		let m = m.max(0.0);
		Self {
			left: m,
			right: m,
			bottom: m,
			top: m,
		}
	}

	pub fn new(left: f32, right: f32, bottom: f32, top: f32) -> Self {
		Self {
			left: left.max(0.0),
			right: right.max(0.0),
			bottom: bottom.max(0.0),
			top: top.max(0.0),
		}
	}

	pub fn is_solid(&self) -> bool {
		self.left <= 1e-6 && self.right <= 1e-6 && self.bottom <= 1e-6 && self.top <= 1e-6
	}

	/// Sub-rectangles `(u0, v0, w, d)` covering the outer rect minus the inset hole.
	pub fn frame_pieces(self, width: f32, depth: f32) -> Vec<(f32, f32, f32, f32)> {
		let width = width.max(1e-4);
		let depth = depth.max(1e-4);
		if self.is_solid() {
			return vec![(0.0, 0.0, width, depth)];
		}
		let u0 = self.left.min(width);
		let u1 = (width - self.right).clamp(0.0, width);
		let v0 = self.bottom.min(depth);
		let v1 = (depth - self.top).clamp(0.0, depth);
		if u1 <= u0 + 1e-4 || v1 <= v0 + 1e-4 {
			// Margins ate the hole — solid fill.
			return vec![(0.0, 0.0, width, depth)];
		}
		let mut pieces = Vec::new();
		if v0 > 1e-4 {
			pieces.push((0.0, 0.0, width, v0));
		}
		if depth - v1 > 1e-4 {
			pieces.push((0.0, v1, width, depth - v1));
		}
		if u0 > 1e-4 {
			pieces.push((0.0, v0, u0, v1 - v0));
		}
		if width - u1 > 1e-4 {
			pieces.push((u1, v0, width - u1, v1 - v0));
		}
		pieces
	}
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

	#[test]
	fn inset_frame_has_four_pieces() {
		let pieces = RectInset::uniform(0.25).frame_pieces(2.0, 1.0);
		assert_eq!(pieces.len(), 4);
	}

	#[test]
	fn zero_inset_is_solid() {
		assert_eq!(RectInset::ZERO.frame_pieces(2.0, 1.0).len(), 1);
	}
}
