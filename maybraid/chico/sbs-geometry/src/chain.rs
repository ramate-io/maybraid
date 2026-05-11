//! Ball-stick **chain** graph: nodes, edges, and pluggable [`Hysteresis`] state per node.

pub mod child_count;
pub mod degree_range;
pub mod length_range;
pub mod radius_range;
pub mod sopes_banyan;

use std::collections::VecDeque;
use std::ops::Range;

use bevy_math::Vec3;
use procedural_common::{NoiseConfig, NoiseParams};

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

	/// Optional hook for algorithms that expand a tree **without** materializing a full [`BallStickChain`].
	/// [`BallStickChain::build`] and [`ChainHysteresisRule`] drive growth instead; most implementations return an empty list.
	fn next_hysteresis(&self) -> Vec<Self>;
}

/// Methods required to grow a [`BallStickChain`] from seed hysteresis states.
pub trait BallStickGrowth: Hysteresis {
	fn depth(&self) -> usize;
	fn max_depth(&self) -> usize;

	fn with_ball_stick_node(self, node: BallStickNode) -> Self;

	fn sample_child_count(&self, parent: &BallStickNode, noise: &NoiseConfig) -> usize;

	fn project_ith_child_radius(
		&self,
		child_index: u32,
		parent: &BallStickNode,
		noise: &NoiseConfig,
	) -> f32;

	fn project_ith_child_ray(
		&self,
		child_index: u32,
		parent: &BallStickNode,
		incoming_ray: Vec3,
		noise: &NoiseConfig,
	) -> Vec3;
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

	pub fn build<R: ChainHysteresisRule<H> + ?Sized>(start: Vec<H>, rule: &R) -> Self
	where
		H: BallStickGrowth,
	{
		let mut chain = Self::default();
		let mut queue: VecDeque<(usize, H, Vec3)> = VecDeque::new();
		let noise = rule.noise();

		for h in start {
			let node = h.ball_stick_node();
			let idx = chain.push_node(node, h.clone());
			queue.push_back((idx, h, Vec3::Y));
		}

		while let Some((parent_idx, parent_h, incoming_ray)) = queue.pop_front() {
			if parent_h.depth() >= parent_h.max_depth() {
				continue;
			}

			let parent = chain.nodes[parent_idx].clone();
			let n_children = parent_h.sample_child_count(&parent, noise);

			for i in 0..n_children {
				let child_index = i as u32;
				let mut child_h =
					rule.generate_ith_child_hysteresis(child_index, &parent, &parent_h);
				let ray = child_h.project_ith_child_ray(child_index, &parent, incoming_ray, noise);
				let radius = child_h.project_ith_child_radius(child_index, &parent, noise);
				let child_node = BallStickNode::new(parent.position + ray, radius);
				child_h = child_h.with_ball_stick_node(child_node);
				let child_idx = chain.push_node(child_node, child_h.clone());
				chain.add_child(parent_idx, child_idx);
				queue.push_back((child_idx, child_h, ray));
			}
		}

		chain
	}
}

/// Default ball-stick hysteresis: ranges + bias ray, suitable for tests and simple rules (e.g. [`PeriodicHysteresisRule`]).
#[derive(Clone, Debug, PartialEq)]
pub struct BallStickHysteresis {
	pub node: BallStickNode,
	pub depth: usize,
	pub max_depth: usize,
	pub segment_index: usize,
	pub child_count: Range<usize>,
	pub length: Range<f32>,
	pub radius: Range<f32>,
	pub ray_degrees_of_freedom: f32,
	pub bias_ray: Vec3,
	/// Weight for blending incoming direction into [`Self::bias_ray`] (see [`degree_range::blend_direction`]).
	pub bias_blend: f32,
}

impl Default for BallStickHysteresis {
	fn default() -> Self {
		Self {
			node: BallStickNode::new(Vec3::ZERO, 0.05),
			depth: 0,
			max_depth: 4,
			segment_index: 0,
			child_count: 1..3,
			length: 0.2..0.5,
			radius: 0.02..0.08,
			ray_degrees_of_freedom: 0.14,
			bias_ray: Vec3::Y,
			bias_blend: 0.5,
		}
	}
}

impl Hysteresis for BallStickHysteresis {
	fn ball_stick_node(&self) -> BallStickNode {
		self.node
	}

	fn next_hysteresis(&self) -> Vec<Self> {
		Vec::new()
	}
}

impl BallStickGrowth for BallStickHysteresis {
	fn depth(&self) -> usize {
		self.depth
	}

	fn max_depth(&self) -> usize {
		self.max_depth
	}

	fn with_ball_stick_node(mut self, node: BallStickNode) -> Self {
		self.node = node;
		self
	}

	fn sample_child_count(&self, parent: &BallStickNode, noise: &NoiseConfig) -> usize {
		child_count::sample_usize(noise, self.child_count.clone(), parent, self.segment_index)
	}

	fn project_ith_child_radius(
		&self,
		child_index: u32,
		parent: &BallStickNode,
		noise: &NoiseConfig,
	) -> f32 {
		radius_range::sample_f32(
			noise,
			self.radius.clone(),
			parent,
			self.segment_index,
			child_index,
		)
	}

	fn project_ith_child_ray(
		&self,
		child_index: u32,
		parent: &BallStickNode,
		incoming_ray: Vec3,
		noise: &NoiseConfig,
	) -> Vec3 {
		let mean = degree_range::blend_direction(incoming_ray, self.bias_ray, self.bias_blend);
		let u = noise.sample_signed_4d(
			parent.position.x + child_index as f32 * 0.37,
			parent.position.y,
			parent.position.z,
			self.segment_index as f32 + 11.0,
		);
		let v = noise.sample_signed_4d(
			parent.position.x,
			parent.position.y + child_index as f32 * 0.41,
			parent.position.z,
			self.segment_index as f32 + 13.0,
		);
		let dir = degree_range::perturb_direction(mean, self.ray_degrees_of_freedom, u, v);
		dir * length_range::sample_f32(
			noise,
			self.length.clone(),
			parent,
			self.segment_index,
			child_index,
		)
	}
}

pub trait ChainHysteresisRule<H: BallStickGrowth> {
	fn noise(&self) -> &NoiseConfig;

	fn generate_ith_child_hysteresis(
		&self,
		child_index: u32,
		parent: &BallStickNode,
		parent_hysteresis: &H,
	) -> H;
}

#[derive(Clone)]
pub struct PeriodicHysteresisRule {
	pub noise: NoiseConfig,
	pub period: usize,
	pub phase_hit: usize,
	pub phase_bias_ray: Vec3,
}

impl Default for PeriodicHysteresisRule {
	fn default() -> Self {
		Self {
			noise: NoiseConfig::new(NoiseParams::default()),
			period: 0,
			phase_hit: 0,
			phase_bias_ray: Vec3::NEG_Y,
		}
	}
}

impl ChainHysteresisRule<BallStickHysteresis> for PeriodicHysteresisRule {
	fn noise(&self) -> &NoiseConfig {
		&self.noise
	}

	fn generate_ith_child_hysteresis(
		&self,
		_child_index: u32,
		_parent: &BallStickNode,
		parent_hysteresis: &BallStickHysteresis,
	) -> BallStickHysteresis {
		let mut h = parent_hysteresis.clone();
		h.depth = parent_hysteresis.depth + 1;
		h.segment_index = parent_hysteresis.segment_index + 1;
		if self.period > 0 && (h.segment_index % self.period) == self.phase_hit {
			h.bias_ray = self.phase_bias_ray;
		}
		h
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use procedural_common::NoiseParams;

	fn rule() -> PeriodicHysteresisRule {
		PeriodicHysteresisRule {
			noise: NoiseConfig::new(NoiseParams {
				seed: 42,
				frequency: 2.0,
				amplitude: 1.0,
				octaves: 1,
				..Default::default()
			}),
			..Default::default()
		}
	}

	#[test]
	fn bias_blend_one_uses_bias_only() {
		let mut h = BallStickHysteresis::default();
		h.bias_ray = Vec3::NEG_Y;
		h.bias_blend = 1.0;
		let blended = degree_range::blend_direction(Vec3::X, h.bias_ray, h.bias_blend);
		assert!((blended - Vec3::NEG_Y).length() < 1e-4, "expected pure -Y, got {blended:?}");
	}

	#[test]
	fn build_chain_and_iterators_work() -> Result<()> {
		let start = vec![BallStickHysteresis { child_count: 1..2, ..Default::default() }];
		let chain = BallStickChain::build(start, &rule());
		assert!(chain.nodes().count() > 0);
		assert!(chain.segments().count() > 0);
		Ok(())
	}
}
