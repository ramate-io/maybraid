//! Higher-order paneling whose polyline nodes are dividing lines in 3D.
//!
//! Each consecutive pair of [`DividedNode`]s defines a four-point segment that
//! splits into two [`TessellatedTriangle`]s. Joint policy is deferred.

use bevy_math::Vec3;
use richmond_building_components::panels::{TessellatedTriangle, DEFAULT_TILE_WIDTH};

/// One polyline node: a dividing line between two points in space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DividedNode {
	pub a: Vec3,
	pub b: Vec3,
}

impl DividedNode {
	pub fn new(a: Vec3, b: Vec3) -> Self {
		Self { a, b }
	}
}

/// Polyline of dividing lines. Segment \(i\) uses `nodes[i]` and `nodes[i+1]`.
#[derive(Debug, Clone, PartialEq)]
pub struct DividedPaneling {
	pub nodes: Vec<DividedNode>,
	pub tile_width: f32,
}

impl DividedPaneling {
	pub fn new(nodes: impl Into<Vec<DividedNode>>, tile_width: f32) -> Self {
		Self { nodes: nodes.into(), tile_width: tile_width.max(1e-4) }
	}

	/// Two [`TessellatedTriangle`]s per consecutive node pair.
	///
	/// Diagonal split of the quadrilateral `(n0.a, n0.b, n1.b, n1.a)`:
	/// - \(T_0 = (n0.a,\ n0.b,\ n1.b)\)
	/// - \(T_1 = (n0.a,\ n1.b,\ n1.a)\)
	pub fn tessellated_triangles(&self) -> Vec<TessellatedTriangle> {
		if self.nodes.len() < 2 {
			return Vec::new();
		}
		let tw = self.tile_width;
		let mut out = Vec::with_capacity((self.nodes.len() - 1) * 2);
		for window in self.nodes.windows(2) {
			let n0 = window[0];
			let n1 = window[1];
			out.push(TessellatedTriangle::new(n0.a, n0.b, n1.b, tw));
			out.push(TessellatedTriangle::new(n0.a, n1.b, n1.a, tw));
		}
		out
	}
}

impl Default for DividedPaneling {
	fn default() -> Self {
		Self { nodes: Vec::new(), tile_width: DEFAULT_TILE_WIDTH }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn two_nodes_yield_two_triangles() {
		let n0 = DividedNode::new(Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0));
		let n1 = DividedNode::new(Vec3::new(2.0, 0.0, 0.0), Vec3::new(2.0, 0.0, -1.0));
		let dp = DividedPaneling::new([n0, n1], 1.0);
		let tris = dp.tessellated_triangles();
		assert_eq!(tris.len(), 2);
		assert_eq!(tris[0].a, n0.a);
		assert_eq!(tris[0].b, n0.b);
		assert_eq!(tris[0].c, n1.b);
		assert_eq!(tris[1].a, n0.a);
		assert_eq!(tris[1].b, n1.b);
		assert_eq!(tris[1].c, n1.a);
	}

	#[test]
	fn three_nodes_yield_four_triangles() {
		let nodes = [
			DividedNode::new(Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0)),
			DividedNode::new(Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 0.0)),
			DividedNode::new(Vec3::new(2.0, 0.0, 0.0), Vec3::new(2.0, 1.0, 0.0)),
		];
		assert_eq!(DividedPaneling::new(nodes, 0.5).tessellated_triangles().len(), 4);
	}
}
