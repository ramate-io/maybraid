//! Short-run polyline of quads + joints.

use bevy_math::Vec3;

use crate::panels::geometry::{PanelGeometry, DEFAULT_TILE_WIDTH};
use crate::panels::joint::Joint;
use crate::panels::quad::Quad;
use crate::placed::{Placed, Placement};

/// Default joint omission threshold (radians).
pub const DEFAULT_MIN_JOINT_ANGLE: f32 = 0.1;
/// Default edge-triangle omission threshold (radians).
pub const DEFAULT_MIN_EDGE_TRIANGLE_ANGLE: f32 = 0.1;

/// Per-edge triangular extensions produced by [`QuadPolyline::edge_polygons`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EdgePolygons {
	pub edge_index: usize,
	pub left: Option<f32>,
	pub right: Option<f32>,
	pub top: Option<f32>,
	pub bottom: Option<f32>,
}

#[derive(Debug, Clone)]
struct VertexPolicy {
	joint: bool,
	edge_tri: bool,
	half_turn: f32,
	yaw_in: f32,
	yaw_out: f32,
}

/// Short-run polyline of panel quads. Prefer splitting long paths upstream.
///
/// All segments share a single authoring [`Self::roll`] (uniform). Plan kinks drive
/// left/right edge triangles and joints; when `|roll|` is non-zero, top/bottom edge
/// triangles use the same half-turn × depth bases. Callers that need varying slope
/// should split into multiple polylines or a higher-order construction.
///
/// Region APIs (no scene / LOD):
/// - [`Self::rectangles`] — rectangular body spans along each edge
/// - [`Self::edge_polygons`] — left/right/top/bottom bases per edge
/// - [`Self::joints`] — average-yaw joints at qualifying vertices
/// - [`Self::decompose`] — merged [`PanelGeometry::Quad`] + [`PanelGeometry::Joint`]
#[derive(Debug, Clone, PartialEq)]
pub struct QuadPolyline {
	pub points: Vec<Vec3>,
	/// Quad depth in panel space (mapped to world via placement).
	pub depth: f32,
	pub tile_width: f32,
	pub tile_height: Option<f32>,
	pub min_joint_angle: f32,
	pub min_edge_triangle_angle: f32,
	/// Uniform roll applied to every segment (radians). Default `0`.
	pub roll: f32,
	/// Incoming plan/slope cue at `points[0]` for a split span (compared to [`Self::roll`]).
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
			roll: 0.0,
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

	/// Uniform roll for every segment (radians).
	pub fn with_roll(mut self, roll: f32) -> Self {
		self.roll = roll;
		self
	}

	pub fn with_incoming_slope(mut self, incoming_slope: f32) -> Self {
		self.incoming_slope = Some(incoming_slope);
		self
	}

	fn vertex_policies(&self) -> Vec<VertexPolicy> {
		let points = &self.points;
		let n = points.len();
		let mut out = vec![
			VertexPolicy {
				joint: false,
				edge_tri: false,
				half_turn: 0.0,
				yaw_in: 0.0,
				yaw_out: 0.0,
			};
			n
		];
		if n < 2 {
			return out;
		}
		let min_joint = self.min_joint_angle.max(0.0);
		let min_edge = self.min_edge_triangle_angle.max(0.0);
		let uniform_roll = self.roll;

		if let Some(roll_in) = self.incoming_slope {
			let dout = points[1] - points[0];
			let yaw_out = yaw_along_xz(dout.x, dout.z);
			let droll = (uniform_roll - roll_in).abs();
			out[0].yaw_in = yaw_out;
			out[0].yaw_out = yaw_out;
			out[0].half_turn = 0.5 * droll;
			if droll >= min_edge {
				out[0].edge_tri = true;
			}
			if droll >= min_joint {
				out[0].joint = true;
			}
		}

		for i in 1..n - 1 {
			let din = points[i] - points[i - 1];
			let dout = points[i + 1] - points[i];
			let yaw_in = yaw_along_xz(din.x, din.z);
			let yaw_out = yaw_along_xz(dout.x, dout.z);
			let dyaw = wrap_pi(yaw_out - yaw_in).abs();
			out[i].yaw_in = yaw_in;
			out[i].yaw_out = yaw_out;
			out[i].half_turn = 0.5 * dyaw;
			if dyaw >= min_edge {
				out[i].edge_tri = true;
			}
			if dyaw >= min_joint {
				out[i].joint = true;
			}
		}
		out
	}

	/// Rectangular body spans along each edge (no edge triangles).
	pub fn rectangles(&self) -> Vec<Placed<Quad>> {
		let points = &self.points;
		if points.len() < 2 {
			return Vec::new();
		}
		let depth = self.depth.max(1e-4);
		let tile_width = self.tile_width.max(1e-4);
		let roll = self.roll;
		let mut out = Vec::new();
		for i in 0..points.len() - 1 {
			let a = points[i];
			let b = points[i + 1];
			let delta = b - a;
			let len = delta.length().max(1e-4);
			let yaw = yaw_along_xz(delta.x, delta.z);
			let mut quad = Quad::new(depth, tile_width).with_length(len);
			if let Some(th) = self.tile_height {
				quad = quad.with_tile_height(th);
			}
			out.push(Placed {
				geom: quad,
				placement: Placement::new(a, yaw).with_roll(roll),
			});
		}
		out
	}

	/// Left/right/top/bottom edge-triangle bases per edge (toward average plan angle).
	pub fn edge_polygons(&self) -> Vec<EdgePolygons> {
		let points = &self.points;
		if points.len() < 2 {
			return Vec::new();
		}
		let depth = self.depth.max(1e-4);
		let policies = self.vertex_policies();
		let use_top_bottom = self.roll.abs() > 1e-6;
		let n_edges = points.len() - 1;
		let mut out = Vec::with_capacity(n_edges);
		for i in 0..n_edges {
			let mut e = EdgePolygons {
				edge_index: i,
				..Default::default()
			};
			if policies[i].edge_tri {
				let b = policies[i].half_turn.tan() * depth;
				if b > 1e-6 {
					e.left = Some(b);
					if use_top_bottom {
						e.top = Some(b);
					}
				}
			}
			if policies[i + 1].edge_tri {
				let b = policies[i + 1].half_turn.tan() * depth;
				if b > 1e-6 {
					e.right = Some(b);
					if use_top_bottom {
						e.bottom = Some(b);
					}
				}
			}
			out.push(e);
		}
		out
	}

	/// Joints at vertices whose plan kink meets [`Self::min_joint_angle`].
	pub fn joints(&self) -> Vec<Placed<Joint>> {
		let points = &self.points;
		if points.len() < 2 {
			return Vec::new();
		}
		let policies = self.vertex_policies();
		let roll = self.roll;
		let mut out = Vec::new();

		if policies[0].joint {
			if let Some(roll_in) = self.incoming_slope {
				let a = points[0];
				let yaw = policies[0].yaw_out;
				let j = Joint::placed_at(a, yaw, yaw, roll_in, roll);
				out.push(Placed {
					geom: j.geom,
					placement: j.placement.with_roll(roll),
				});
			}
		}

		for i in 1..points.len() - 1 {
			if !policies[i].joint {
				continue;
			}
			let j = Joint::placed_at(
				points[i],
				policies[i].yaw_in,
				policies[i].yaw_out,
				roll,
				roll,
			);
			out.push(Placed {
				geom: j.geom,
				placement: Placement {
					roll,
					..j.placement
				},
			});
		}
		out
	}

	/// Merge [`Self::rectangles`] + [`Self::edge_polygons`] into quads, then append [`Self::joints`].
	pub fn decompose(&self) -> Vec<Placed<PanelGeometry>> {
		let rects = self.rectangles();
		let edges = self.edge_polygons();
		let mut out = Vec::with_capacity(rects.len() + 4);

		for (i, mut placed) in rects.into_iter().enumerate() {
			if let Some(e) = edges.get(i) {
				if let Some(b) = e.left {
					placed.geom = placed.geom.with_left(b);
				}
				if let Some(b) = e.right {
					placed.geom = placed.geom.with_right(b);
				}
				if let Some(b) = e.top {
					placed.geom = placed.geom.with_top(b);
				}
				if let Some(b) = e.bottom {
					placed.geom = placed.geom.with_bottom(b);
				}
			}
			// Shift origin when a left edge triangle is present (lower-left of full extent).
			let left_w = placed.geom.left.map(|b| b.abs()).unwrap_or(0.0);
			if left_w > 1e-6 {
				let yaw = placed.placement.yaw;
				let dir = bevy_math::Quat::from_rotation_y(yaw) * Vec3::X;
				placed.placement.translation -= dir * left_w;
			}
			out.push(Placed {
				geom: PanelGeometry::Quad(placed.geom),
				placement: placed.placement,
			});
		}

		for j in self.joints() {
			out.push(Placed {
				geom: PanelGeometry::Joint(j.geom),
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

/// Slope angle of an edge vs horizontal (joint kink sizing / incoming cues).
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
		assert_eq!(pl.rectangles().len(), 2);
		assert_eq!(pl.joints().len(), 1);
		let edges = pl.edge_polygons();
		assert!(edges[0].right.is_some());
		assert!(edges[1].left.is_some());
		let pieces = pl.decompose();
		assert_eq!(
			pieces
				.iter()
				.filter(|p| matches!(p.geom, PanelGeometry::Quad(_)))
				.count(),
			2
		);
		assert_eq!(
			pieces
				.iter()
				.filter(|p| matches!(p.geom, PanelGeometry::Joint(_)))
				.count(),
			1
		);
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
		assert!(pl.joints().is_empty());
		for e in pl.edge_polygons() {
			assert!(e.left.is_none() && e.right.is_none());
		}
		Ok(())
	}

	#[test]
	fn uniform_roll_applies_to_rectangles() -> anyhow::Result<()> {
		let pl = QuadPolyline::new(
			[Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0)],
			0.5,
		)
		.with_roll(0.3);
		let r = pl.rectangles();
		assert_eq!(r.len(), 1);
		assert!((r[0].roll() - 0.3).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn roll_enables_top_bottom_edge_polys() -> anyhow::Result<()> {
		let flat = QuadPolyline::new(
			[
				Vec3::new(0.0, 0.0, 0.0),
				Vec3::new(2.0, 0.0, 0.0),
				Vec3::new(2.0, 0.0, 2.0),
			],
			1.0,
		);
		assert!(flat.edge_polygons()[0].top.is_none());
		let rolled = flat.clone().with_roll(0.25);
		assert!(rolled.edge_polygons()[0].top.is_some() || rolled.edge_polygons()[0].bottom.is_some()
			|| rolled.edge_polygons()[1].top.is_some() || rolled.edge_polygons()[1].bottom.is_some());
		let e0 = &rolled.edge_polygons()[0];
		let e1 = &rolled.edge_polygons()[1];
		assert_eq!(e0.right.is_some(), e0.bottom.is_some());
		assert_eq!(e1.left.is_some(), e1.top.is_some());
		Ok(())
	}
}
