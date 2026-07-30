//! Tessellated fill of an arbitrary 3D triangle with posed unit right-triangle kits.

use std::f32::consts::PI;

use bevy_math::{EulerRot, Mat3, Quat, Vec2, Vec3};

use crate::panels::geometry::{
	fitted_tile_count, PanelGeometry, RightTriangle, DEFAULT_TILE_WIDTH,
};
use crate::placed::{Placed, Placement};

/// Three world-space corners filled by right-triangle panel kits.
///
/// v1 allows full rotation + non-uniform scale on each kit. A later pass may lock
/// orientation to a fixed tessellation-normal axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TessellatedTriangle {
	pub a: Vec3,
	pub b: Vec3,
	pub c: Vec3,
	/// Suggested kit leg length; fitted so tiles span each altitude-split leg exactly.
	pub tile_width: f32,
}

impl Default for TessellatedTriangle {
	fn default() -> Self {
		Self {
			a: Vec3::ZERO,
			b: Vec3::new(1.0, 0.0, 0.0),
			c: Vec3::new(0.0, 0.0, -1.0),
			tile_width: DEFAULT_TILE_WIDTH,
		}
	}
}

impl TessellatedTriangle {
	pub fn new(a: Vec3, b: Vec3, c: Vec3, tile_width: f32) -> Self {
		Self { a, b, c, tile_width: tile_width.max(1e-4) }
	}

	/// Expand into posed [`RightTriangle`] leaves (identity parent).
	pub fn decompose(self) -> Vec<Placed<PanelGeometry>> {
		let ab = self.b - self.a;
		let ac = self.c - self.a;
		let normal = ab.cross(ac);
		if normal.length_squared() < 1e-12 {
			return Vec::new();
		}

		// Work in 2D of the triangle plane with origin at `a`.
		let x_axis = ab.normalize();
		let z_axis = normal.normalize().cross(x_axis).normalize();
		let to_uv = |p: Vec3| -> Vec2 {
			let d = p - self.a;
			Vec2::new(d.dot(x_axis), d.dot(z_axis))
		};
		let pa = Vec2::ZERO;
		let pb = to_uv(self.b);
		let pc = to_uv(self.c);

		let edges = [
			(pa, pb, self.a, self.b, self.c),
			(pb, pc, self.b, self.c, self.a),
			(pc, pa, self.c, self.a, self.b),
		];
		let (ei, _) = edges
			.iter()
			.enumerate()
			.max_by(|(_, (u0, u1, ..)), (_, (v0, v1, ..))| {
				(u1 - u0)
					.length_squared()
					.partial_cmp(&(v1 - v0).length_squared())
					.unwrap_or(std::cmp::Ordering::Equal)
			})
			.expect("three edges");

		let (u0, u1, p0, p1, apex) = edges[ei];
		let edge = u1 - u0;
		let edge_len2 = edge.length_squared();
		if edge_len2 < 1e-12 {
			return Vec::new();
		}
		let t = ((to_uv(apex) - u0).dot(edge) / edge_len2).clamp(0.0, 1.0);
		let foot_uv = u0 + edge * t;
		let foot_world = self.a + x_axis * foot_uv.x + z_axis * foot_uv.y;

		let mut out = Vec::new();
		// Right angle is at the foot of the altitude.
		out.extend(tile_right_triangle(foot_world, p0, apex, self.tile_width));
		out.extend(tile_right_triangle(foot_world, p1, apex, self.tile_width));
		out
	}
}

/// Fill right triangle with right angle at `right_angle`, legs toward `leg_u` and `leg_v`.
fn tile_right_triangle(
	right_angle: Vec3,
	leg_u: Vec3,
	leg_v: Vec3,
	tile_width: f32,
) -> Vec<Placed<PanelGeometry>> {
	let u_vec = leg_u - right_angle;
	let v_vec = leg_v - right_angle;
	let u_len = u_vec.length();
	let v_len = v_vec.length();
	if u_len < 1e-6 || v_len < 1e-6 {
		return Vec::new();
	}

	let x_axis = u_vec / u_len;
	// Kit −Z maps to V, so kit +Z = −V_dir.
	let v_dir = v_vec / v_len;
	let z_axis = -v_dir;
	let y_axis = x_axis.cross(z_axis);
	if y_axis.length_squared() < 1e-12 {
		return Vec::new();
	}
	let y_axis = y_axis.normalize();
	// Re-orthogonalize Z against X/Y.
	let z_axis = y_axis.cross(x_axis).normalize();

	let mat = Mat3::from_cols(x_axis, y_axis, z_axis);
	if mat.determinant() < 0.0 {
		// Flip Y if winding inverted.
		let y_axis = -y_axis;
		let z_axis = y_axis.cross(x_axis).normalize();
		return tile_with_basis(right_angle, x_axis, y_axis, z_axis, u_len, v_len, tile_width);
	}
	tile_with_basis(right_angle, x_axis, y_axis, z_axis, u_len, v_len, tile_width)
}

fn tile_with_basis(
	origin: Vec3,
	x_axis: Vec3,
	y_axis: Vec3,
	z_axis: Vec3,
	u_len: f32,
	v_len: f32,
	tile_width: f32,
) -> Vec<Placed<PanelGeometry>> {
	let mat = Mat3::from_cols(x_axis, y_axis, z_axis);
	let q = Quat::from_mat3(&mat);
	let (yaw, pitch, roll) = q.to_euler(EulerRot::YXZ);
	let parent = Placement {
		translation: origin,
		yaw,
		pitch,
		roll,
		scale: Vec3::ONE,
	};

	let nu = fitted_tile_count(u_len, tile_width);
	let nv = fitted_tile_count(v_len, tile_width);
	let du = u_len / nu as f32;
	let dv = v_len / nv as f32;

	let mut out = Vec::new();
	for i in 0..nu {
		// Staircase: cells whose far corner stays on the hypotenuse side of u/L + v/W <= 1.
		let max_j = ((nu - i) * nv + nu - 1) / nu;
		for j in 0..max_j {
			let local = Placement::new(Vec3::new(i as f32 * du, 0.0, -(j as f32 * dv)), 0.0)
				.with_scale(Vec3::new(du, 1.0, dv));
			out.push(Placed {
				geom: PanelGeometry::RightTriangle(RightTriangle { mirror: None }),
				placement: parent.compose_child(local),
			});
			// Complement when the unit square is fully inside the right triangle.
			let i2 = i + 1;
			let j2 = j + 1;
			if (i2 as f32) / (nu as f32) + (j2 as f32) / (nv as f32) <= 1.0 + 1e-4 {
				let complement = Placement::new(
					Vec3::new(i2 as f32 * du, 0.0, -(j2 as f32 * dv)),
					PI,
				)
				.with_scale(Vec3::new(du, 1.0, dv));
				out.push(Placed {
					geom: PanelGeometry::RightTriangle(RightTriangle { mirror: None }),
					placement: parent.compose_child(complement),
				});
			}
		}
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn unit_right_triangle_emits_kits() {
		let t = TessellatedTriangle::new(
			Vec3::ZERO,
			Vec3::new(2.0, 0.0, 0.0),
			Vec3::new(0.0, 0.0, -2.0),
			1.0,
		);
		let pieces = t.decompose();
		assert!(!pieces.is_empty());
		assert!(pieces.iter().all(|p| matches!(p.geom, PanelGeometry::RightTriangle(_))));
	}

	#[test]
	fn degenerate_is_empty() {
		let t = TessellatedTriangle::new(
			Vec3::ZERO,
			Vec3::new(1.0, 0.0, 0.0),
			Vec3::new(2.0, 0.0, 0.0),
			1.0,
		);
		assert!(t.decompose().is_empty());
	}
}
