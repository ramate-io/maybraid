//! Short-run polyline of quads + joints.

use bevy_math::Vec3;

use crate::panels::geometry::{PanelComposite, PanelGeom, DEFAULT_TILE_WIDTH};
use crate::panels::joint::Joint;
use crate::panels::quad::Quad;
use crate::placed::{Placed, Placement};

/// Default joint omission threshold (radians).
pub const DEFAULT_MIN_JOINT_ANGLE: f32 = 0.1;
/// Default edge-triangle omission threshold (radians).
pub const DEFAULT_MIN_EDGE_TRIANGLE_ANGLE: f32 = 0.1;

/// Short-run polyline of panel quads. Prefer splitting long paths upstream.
///
/// [`Self::decompose`] emits posed [`Quad`]s and [`Joint`]s (no scene / LOD).
///
/// Corner policy (independent thresholds):
/// - kink ≥ [`Self::min_joint_angle`] → joint on the **average** inbound/outbound angle
/// - kink ≥ [`Self::min_edge_triangle_angle`] → grow abutting quads' corner-facing edge
///   triangles to fill toward that average angle
///
/// Cross-segment **pitch** edge-triangle fill (rotation about the segment axis changing
/// segment-to-segment) is deferred; plan-dominant and slope-roll joints are handled.
#[derive(Debug, Clone, PartialEq)]
pub struct QuadPolyline {
	pub points: Vec<Vec3>,
	/// Quad depth / thickness in panel space (mapped to world via placement).
	pub depth: f32,
	pub tile_width: f32,
	pub tile_height: Option<f32>,
	pub min_joint_angle: f32,
	pub min_edge_triangle_angle: f32,
	/// Roll of the segment that ends at `points[0]` but is not part of this polyline.
	pub incoming_slope: Option<f32>,
}

impl Default for QuadPolyline {
	fn default() -> Self {
		Self {
			points: Vec::new(),
			depth: 1.0,
			tile_width: DEFAULT_TILE_WIDTH,
			tile_height: None,
			min_joint_angle: DEFAULT_MIN_JOINT_ANGLE,
			min_edge_triangle_angle: DEFAULT_MIN_EDGE_TRIANGLE_ANGLE,
			incoming_slope: None,
		}
	}
}

impl QuadPolyline {
	pub fn new(points: impl Into<Vec<Vec3>>, depth: f32) -> Self {
		Self {
			points: points.into(),
			depth: depth.max(1e-4),
			..Self::default()
		}
	}

	pub fn with_tile_width(mut self, tile_width: f32) -> Self {
		self.tile_width = tile_width.max(1e-4);
		self
	}

	pub fn with_tile_height(mut self, tile_height: f32) -> Self {
		self.tile_height = Some(tile_height.max(1e-4));
		self
	}

	pub fn with_min_joint_angle(mut self, min_joint_angle: f32) -> Self {
		self.min_joint_angle = min_joint_angle.max(0.0);
		self
	}

	pub fn with_min_edge_triangle_angle(mut self, min_edge_triangle_angle: f32) -> Self {
		self.min_edge_triangle_angle = min_edge_triangle_angle.max(0.0);
		self
	}

	pub fn with_incoming_slope(mut self, incoming_slope: f32) -> Self {
		self.incoming_slope = Some(incoming_slope);
		self
	}

	/// Expand into posed quads + joints (identity parent). Style-agnostic.
	pub fn decompose(&self) -> Vec<Placed<PanelGeom>> {
		self.decompose_composites()
			.into_iter()
			.map(|p| Placed {
				geom: PanelGeom::from(p.geom),
				placement: p.placement,
			})
			.collect()
	}

	/// Same as [`Self::decompose`] but typed as composites only.
	pub fn decompose_composites(&self) -> Vec<Placed<PanelComposite>> {
		let points = &self.points;
		if points.len() < 2 {
			return Vec::new();
		}

		let min_joint = self.min_joint_angle.max(0.0);
		let min_edge = self.min_edge_triangle_angle.max(0.0);
		let depth = self.depth.max(1e-4);
		let tile_width = self.tile_width.max(1e-4);
		let n_edges = points.len() - 1;

		// Per-vertex plan kink (and whether edge triangles / joints apply).
		let mut edge_tri_at = vec![false; points.len()];
		let mut joint_at = vec![false; points.len()];
		let mut half_turn = vec![0.0f32; points.len()];

		if let Some(roll_in) = self.incoming_slope {
			let a = points[0];
			let b = points[1];
			let dout = b - a;
			let roll_out = roll_along_slope(dout.x, dout.y, dout.z);
			let droll = (roll_out - roll_in).abs();
			if droll >= min_edge {
				edge_tri_at[0] = true;
				half_turn[0] = 0.5 * droll;
			}
			if droll >= min_joint {
				joint_at[0] = true;
			}
		}

		for i in 1..points.len() - 1 {
			let prev = points[i - 1];
			let cur = points[i];
			let next = points[i + 1];
			let din = cur - prev;
			let dout = next - cur;
			let yaw_in = yaw_along_xz(din.x, din.z);
			let yaw_out = yaw_along_xz(dout.x, dout.z);
			let roll_in = roll_along_slope(din.x, din.y, din.z);
			let roll_out = roll_along_slope(dout.x, dout.y, dout.z);
			let dyaw = wrap_pi(yaw_out - yaw_in).abs();
			let droll = (roll_out - roll_in).abs();
			let kink = dyaw.max(droll);
			half_turn[i] = 0.5 * wrap_pi(yaw_out - yaw_in).abs().max(droll);
			if kink >= min_edge {
				edge_tri_at[i] = true;
			}
			if kink >= min_joint {
				joint_at[i] = true;
			}
		}

		let mut out = Vec::new();

		for i in 0..n_edges {
			let a = points[i];
			let b = points[i + 1];
			let delta = b - a;
			let len = delta.length().max(1e-4);
			let yaw = yaw_along_xz(delta.x, delta.z);
			let roll = roll_along_slope(delta.x, delta.y, delta.z);
			let dir = delta / len;

			// Edge triangles fill toward the average angle: use tan(half_turn)*depth as base.
			let left_base = if edge_tri_at[i] {
				Some(half_turn[i].tan() * depth)
			} else {
				None
			};
			let right_base = if edge_tri_at[i + 1] {
				Some(half_turn[i + 1].tan() * depth)
			} else {
				None
			};

			let mut quad = Quad::new(depth, tile_width).with_length(len);
			if let Some(th) = self.tile_height {
				quad = quad.with_tile_height(th);
			}
			if let Some(b) = left_base {
				if b > 1e-6 {
					quad = quad.with_left(b);
				}
			}
			if let Some(b) = right_base {
				if b > 1e-6 {
					quad = quad.with_right(b);
				}
			}

			// Place quad so its rectangle centerline follows the edge: lower-left of the
			// full extent sits such that the rectangle runs along +X local = edge dir.
			// Anchor at `a`, yaw along edge; local +X = edge, local -Z = depth (toward
			// the left of travel for a vertical wall this is inward — callers rotate).
			let left_w = quad.left.map(|b| b.abs()).unwrap_or(0.0);
			let origin = a - dir * left_w;
			out.push(Placed {
				geom: PanelComposite::Quad(quad),
				placement: Placement::new(origin, yaw).with_roll(roll),
			});
		}

		if joint_at[0] {
			if let Some(roll_in) = self.incoming_slope {
				let a = points[0];
				let b = points[1];
				let dout = b - a;
				let yaw_out = yaw_along_xz(dout.x, dout.z);
				let roll_out = roll_along_slope(dout.x, dout.y, dout.z);
				let j = Joint::placed_at(a, yaw_out, yaw_out, roll_in, roll_out);
				out.push(Placed {
					geom: PanelComposite::Joint(j.geom),
					placement: j.placement,
				});
			}
		}

		for i in 1..points.len() - 1 {
			if !joint_at[i] {
				continue;
			}
			let prev = points[i - 1];
			let cur = points[i];
			let next = points[i + 1];
			let din = cur - prev;
			let dout = next - cur;
			let yaw_in = yaw_along_xz(din.x, din.z);
			let yaw_out = yaw_along_xz(dout.x, dout.z);
			let roll_in = roll_along_slope(din.x, din.y, din.z);
			let roll_out = roll_along_slope(dout.x, dout.y, dout.z);
			let j = Joint::placed_at(cur, yaw_in, yaw_out, roll_in, roll_out);
			out.push(Placed {
				geom: PanelComposite::Joint(j.geom),
				placement: j.placement,
			});
		}

		out
	}
}

/// Plan yaw for a horizontal displacement \((\mathrm{d}x, \mathrm{d}z)\).
pub fn yaw_along_xz(dx: f32, dz: f32) -> f32 {
	(-dz).atan2(dx)
}

/// Slope roll about local \(+Z\) for an edge \(\Delta = (\mathrm{d}x,\mathrm{d}y,\mathrm{d}z)\).
pub fn roll_along_slope(dx: f32, dy: f32, dz: f32) -> f32 {
	let horiz = (dx * dx + dz * dz).sqrt();
	dy.atan2(horiz.max(1e-8))
}

pub(crate) fn wrap_pi(mut a: f32) -> f32 {
	while a > std::f32::consts::PI {
		a -= std::f32::consts::TAU;
	}
	while a < -std::f32::consts::PI {
		a += std::f32::consts::TAU;
	}
	a
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn l_shape_emits_joint_and_edge_tris() -> anyhow::Result<()> {
		let pl = QuadPolyline::new(
			[
				Vec3::new(0.0, 0.0, 0.0),
				Vec3::new(2.0, 0.0, 0.0),
				Vec3::new(2.0, 0.0, 2.0),
			],
			1.0,
		);
		let pieces = pl.decompose_composites();
		assert_eq!(pieces.iter().filter(|p| matches!(p.geom, PanelComposite::Quad(_))).count(), 2);
		assert_eq!(
			pieces
				.iter()
				.filter(|p| matches!(p.geom, PanelComposite::Joint(_)))
				.count(),
			1
		);
		let quads: Vec<_> = pieces
			.iter()
			.filter_map(|p| match &p.geom {
				PanelComposite::Quad(q) => Some(*q),
				_ => None,
			})
			.collect();
		assert!(quads[0].right.is_some());
		assert!(quads[1].left.is_some());
		Ok(())
	}

	#[test]
	fn small_kink_omits_joint_and_edge() -> anyhow::Result<()> {
		let pl = QuadPolyline::new(
			[
				Vec3::new(0.0, 0.0, 0.0),
				Vec3::new(2.0, 0.0, 0.0),
				Vec3::new(4.0, 0.0, 0.1),
			],
			1.0,
		)
		.with_min_joint_angle(DEFAULT_MIN_JOINT_ANGLE)
		.with_min_edge_triangle_angle(DEFAULT_MIN_EDGE_TRIANGLE_ANGLE);
		let pieces = pl.decompose_composites();
		assert!(!pieces.iter().any(|p| matches!(p.geom, PanelComposite::Joint(_))));
		for p in &pieces {
			if let PanelComposite::Quad(q) = &p.geom {
				assert!(q.left.is_none() && q.right.is_none());
			}
		}
		Ok(())
	}
}
