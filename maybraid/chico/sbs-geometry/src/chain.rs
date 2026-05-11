//! Ball-stick **chain** graph: nodes, edges, and pluggable [`Hysteresis`] state per node.

pub mod branch_out;
pub mod child_count;
pub mod degree_range;
pub mod depth_budget;
pub mod length_range;
pub mod radius_range;
pub mod sopes_banyan;

use bevy_math::Vec3;

pub use branch_out::BranchOut;
pub use depth_budget::DepthBudget;

/// One vertex in the ball-stick graph (position + ball radius at that joint).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BallStickNode {
	pub position: Vec3,
	pub radius: f32,
}

impl BallStickNode {
	pub fn new(position: Vec3, radius: f32) -> Self {
		Self { position, radius }
	}
}

/// Hysteresis state carried per chain node: minimal surface for non-builder consumers.
///
/// Heavy expansion (noisy child counts, rays, radii) lives on [`BallStickGrowth`]; [`BallStickChain::build`] uses that supertrait.
pub trait Hysteresis: Clone {
	/// Geometry for this state (typically a [`BallStickNode`] field on the implementing struct).
	fn ball_stick_node(&self) -> BallStickNode;

	/// Generates the list of states to which this state transitions.
	/// This is the fan out.
	fn next_hysteresis(&self) -> Vec<Self>;
}
#[derive(Debug, Clone)]
pub struct BallStickSegment<'a> {
	pub start: &'a BallStickNode,
	pub end: &'a BallStickNode,
}

impl<'a> BallStickSegment<'a> {
	pub fn ray(&self) -> Vec3 {
		self.end.position - self.start.position
	}
}

#[derive(Clone, Debug)]
pub struct BallStickChain<H>
where
	H: Hysteresis,
{
	pub nodes: Vec<BallStickNode>,
	pub children: Vec<Vec<usize>>,
	pub hysteresis: Vec<H>,
}

impl<H: Hysteresis> Default for BallStickChain<H> {
	fn default() -> Self {
		Self { nodes: Vec::new(), children: Vec::new(), hysteresis: Vec::new() }
	}
}

impl<H: Hysteresis> BallStickChain<H> {
	fn push_node(&mut self, node: BallStickNode, h: H) -> usize {
		let i = self.nodes.len();
		self.nodes.push(node);
		self.children.push(Vec::new());
		self.hysteresis.push(h);
		i
	}

	fn add_child(&mut self, parent: usize, child: usize) {
		self.children[parent].push(child);
	}

	pub fn nodes(&self) -> impl Iterator<Item = &BallStickNode> {
		self.nodes.iter()
	}

	/// Parallel walk of built geometry and per-node hysteresis (same order as [`Self::nodes`]).
	pub fn nodes_with_hysteresis(&self) -> impl Iterator<Item = (&BallStickNode, &H)> {
		self.nodes.iter().zip(self.hysteresis.iter())
	}

	pub fn segments<'a>(&'a self) -> impl Iterator<Item = BallStickSegment<'a>> + 'a {
		self.children.iter().enumerate().flat_map(move |(parent_idx, children)| {
			let start = &self.nodes[parent_idx];
			children
				.iter()
				.map(move |child_idx| BallStickSegment { start, end: &self.nodes[*child_idx] })
		})
	}

	/// Each graph edge with hysteresis at the parent (start) and child (end) nodes.
	pub fn segments_with_hysteresis<'a>(
		&'a self,
	) -> impl Iterator<Item = (BallStickSegment<'a>, &'a H, &'a H)> + 'a {
		self.children.iter().enumerate().flat_map(move |(parent_idx, children)| {
			let start = &self.nodes[parent_idx];
			let parent_h = &self.hysteresis[parent_idx];
			children.iter().map(move |child_idx| {
				let seg = BallStickSegment { start, end: &self.nodes[*child_idx] };
				(seg, parent_h, &self.hysteresis[*child_idx])
			})
		})
	}
}
