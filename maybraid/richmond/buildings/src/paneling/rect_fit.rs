//! Oriented ordinary rectangle from lowest-edge vector + height + roll.
//!
//! - **Length / slope** (`e1`): direction of the authored lowest-edge vector.
//! - **Height** (`e0`): world `+Y` projected into the plane ⊥ `e1` at roll `0`,
//!   then rotated about `e1` by `roll`.
//! - Anchored at `origin`, extending `+e0 * height` and `+edge`.

use bevy_math::{EulerRot, Mat3, Quat, Vec3};
use richmond_building_components::Placement;

/// Oriented ordinary rectangle in world space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrientedRect {
	pub a0: Vec3,
	pub a1: Vec3,
	pub b0: Vec3,
	pub b1: Vec3,
	/// Unit height axis (kit `+X`).
	pub e0: Vec3,
	/// Unit length / lowest-edge axis (kit `+Z`).
	pub e1: Vec3,
	pub normal: Vec3,
	/// Span along [`Self::e0`].
	pub width: f32,
	/// Span along [`Self::e1`].
	pub depth: f32,
}

impl OrientedRect {
	/// Panel-kit placement for the sub-rectangle covering panel
	/// \(u\in[u_0,u_0+w]\), \(v\in[v_0,v_0+d]\) (origin at \(a_0\), \(+v\) along [`Self::e1`]).
	///
	/// Rectangle GLBs occupy \(X\in[0,1]\), \(Z\in[-1,0]\) (same Z sense as the
	/// right-triangle kit). Scale `(w, thick, d)`. Frame:
	/// kit \(+X{\to}e_0\), kit \(-Z{\to}e_1\) (so \(+Z{\to}-e_1\)), kit \(+Y{\to}\mathrm{normal}\).
	pub fn panel_placement(&self, u0: f32, v0: f32, w: f32, d: f32, thick_scale: f32) -> Placement {
		let origin = self.a0 + self.e0 * u0 + self.e1 * v0;
		let w = w.max(1e-4);
		let d = d.max(1e-4);
		let thick_scale = thick_scale.max(1e-4);
		// RH: columns = images of kit X,Y,Z. Mesh lives on −Z, so kit +Z → −e1.
		let ey = self.normal;
		let rotation = Quat::from_mat3(&Mat3::from_cols(self.e0, ey, -self.e1));
		let (yaw, pitch, roll) = rotation.to_euler(EulerRot::YXZ);
		Placement {
			translation: origin,
			yaw,
			pitch,
			roll,
			scale: Vec3::new(w, thick_scale, d),
		}
	}

	pub fn solid_placement(&self, thick_scale: f32) -> Placement {
		self.panel_placement(0.0, 0.0, self.width, self.depth, thick_scale)
	}
}

/// Unit height axis at roll `0`: world `+Y` projected into the plane ⊥ `e1`.
///
/// When `e1` is nearly vertical, falls back to a stable axis in that plane.
pub fn zero_roll_height_axis(e1: Vec3) -> Option<Vec3> {
	let e1 = e1.normalize_or_zero();
	if e1.length_squared() < 1e-12 {
		return None;
	}
	let y = Vec3::Y;
	let mut h = y - e1 * y.dot(e1);
	if h.length_squared() < 1e-10 {
		let fallback = if e1.x.abs() < 0.9 { Vec3::X } else { Vec3::Z };
		h = fallback - e1 * fallback.dot(e1);
	}
	if h.length_squared() < 1e-12 {
		return None;
	}
	Some(h.normalize())
}

/// Signed roll about `e1` that aligns zero-roll height with `height_dir`
/// (projected into the plane ⊥ `e1`).
pub fn roll_to_align_height(e1: Vec3, height_dir: Vec3) -> Option<f32> {
	let e1 = e1.normalize_or_zero();
	if e1.length_squared() < 1e-12 {
		return None;
	}
	let e0_zero = zero_roll_height_axis(e1)?;
	let target = height_dir - e1 * height_dir.dot(e1);
	if target.length_squared() < 1e-12 {
		return Some(0.0);
	}
	let target = target.normalize();
	let sin = e1.dot(e0_zero.cross(target));
	let cos = e0_zero.dot(target);
	Some(sin.atan2(cos))
}

/// Build an oriented rectangle, or [`None`] if `edge` / `height` are degenerate.
pub fn orient_rectangle(origin: Vec3, edge: Vec3, height: f32, roll: f32) -> Option<OrientedRect> {
	let depth = edge.length();
	let width = height;
	if depth < 1e-8 || width < 1e-8 {
		return None;
	}
	let e1 = edge / depth;
	let e0_zero = zero_roll_height_axis(e1)?;
	let e0 = Quat::from_axis_angle(e1, roll) * e0_zero;
	let normal = e0.cross(e1).normalize_or_zero();
	if normal.length_squared() < 1e-12 {
		return None;
	}
	let a0 = origin;
	let a1 = a0 + e0 * width;
	let b0 = a0 + edge;
	let b1 = a1 + edge;
	Some(OrientedRect {
		a0,
		a1,
		b0,
		b1,
		e0,
		e1,
		normal,
		width,
		depth,
	})
}

/// Fallback when orientation fails (keeps call sites infallible).
pub fn fallback_oriented(origin: Vec3, edge: Vec3, height: f32) -> OrientedRect {
	let depth = edge.length().max(1e-4);
	let width = height.max(1e-4);
	let e1 = if edge.length_squared() > 1e-12 {
		edge.normalize()
	} else {
		Vec3::Z
	};
	let e0 = zero_roll_height_axis(e1).unwrap_or(Vec3::Y);
	let normal = e0.cross(e1).normalize_or_zero();
	let a0 = origin;
	OrientedRect {
		a0,
		a1: a0 + e0 * width,
		b0: a0 + e1 * depth,
		b1: a0 + e0 * width + e1 * depth,
		e0,
		e1,
		normal,
		width,
		depth,
	}
}

/// Panel-space axis-aligned inset margins (same units as oriented width/depth).
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
	fn horizontal_edge_zero_roll_height_is_plus_y() {
		let o = orient_rectangle(Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0), 1.5, 0.0).unwrap();
		assert!((o.e0 - Vec3::Y).length() < 1e-4, "e0={:?}", o.e0);
		assert!((o.e1 - Vec3::X).length() < 1e-4);
		assert!((o.width - 1.5).abs() < 1e-4);
		assert!((o.depth - 2.0).abs() < 1e-4);
		assert!((o.a1 - Vec3::new(0.0, 1.5, 0.0)).length() < 1e-4);
		assert!((o.b0 - Vec3::new(2.0, 0.0, 0.0)).length() < 1e-4);
	}

	#[test]
	fn roll_pi_flips_height_to_minus_y() {
		let o = orient_rectangle(Vec3::ZERO, Vec3::Z * 2.0, 1.0, std::f32::consts::PI).unwrap();
		assert!((o.e0 + Vec3::Y).length() < 1e-3, "e0={:?}", o.e0);
	}

	#[test]
	fn roll_to_align_matches_cross_section_edge() {
		let edge = Vec3::new(0.0, 0.0, 3.0);
		let height_dir = Vec3::new(2.0, 0.0, 0.0);
		let roll = roll_to_align_height(edge.normalize(), height_dir).unwrap();
		let o = orient_rectangle(Vec3::ZERO, edge, 2.0, roll).unwrap();
		assert!(
			o.e0.dot(height_dir.normalize()) > 0.99,
			"e0={:?} should align with +X",
			o.e0
		);
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

	#[test]
	fn unit_square_kit_maps_far_corner_to_b1() {
		let o = orient_rectangle(Vec3::ZERO, Vec3::new(0.0, 0.0, 1.0), 2.0, 0.0).unwrap();
		let p = o.solid_placement(0.75);
		assert!((p.scale - Vec3::new(2.0, 0.75, 1.0)).length() < 1e-4);
		// Kit far corner is local (1, 0, -1) — GLB Z ∈ [-1, 0].
		let far = p.rotation() * Vec3::new(p.scale.x, 0.0, -p.scale.z) + p.translation;
		assert!((far - o.b1).length() < 1e-3);
	}

	#[test]
	fn wall_edge_plus_x_maps_kit_neg_z_to_plus_x() {
		let o = orient_rectangle(Vec3::ZERO, Vec3::new(4.0, 0.0, 0.0), 3.0, 0.0).unwrap();
		assert!((o.b0 - Vec3::new(4.0, 0.0, 0.0)).length() < 1e-4);
		let p = o.solid_placement(0.75);
		let edge_end = p.rotation() * Vec3::new(0.0, 0.0, -p.scale.z) + p.translation;
		assert!(
			(edge_end - Vec3::new(4.0, 0.0, 0.0)).length() < 1e-3,
			"kit −Z should land on authored +edge, got {:?}",
			edge_end
		);
		let far = p.rotation() * Vec3::new(p.scale.x, 0.0, -p.scale.z) + p.translation;
		assert!((far - Vec3::new(4.0, 3.0, 0.0)).length() < 1e-3);
	}
}
