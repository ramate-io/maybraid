use std::collections::VecDeque;
use std::ops::Range;

use bevy_math::Vec3;
use procedural_common::{NoiseConfig, NoiseParams};

#[derive(Clone, Debug, PartialEq)]
pub struct BallStickNode {
	pub position: Vec3,
	pub radius: f32,
}

impl BallStickNode {
	pub fn new(position: Vec3, radius: f32) -> Self {
		Self { position, radius }
	}
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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BallStickChain {
	pub nodes: Vec<BallStickNode>,
	pub children: Vec<Vec<usize>>,
	pub hysteresis: Vec<Hysteresis>,
}

impl BallStickChain {
	fn push_node(&mut self, node: BallStickNode, h: Hysteresis) -> usize {
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

	pub fn segments<'a>(&'a self) -> impl Iterator<Item = BallStickSegment<'a>> {
		self.children
			.iter()
			.enumerate()
			.flat_map(move |(parent_idx, children)| {
				let start = &self.nodes[parent_idx];
				children.iter().map(move |child_idx| BallStickSegment {
					start,
					end: &self.nodes[*child_idx],
				})
			})
	}

	pub fn build<R: ChainHysteresisRule + ?Sized>(
		start_nodes: Vec<(BallStickNode, Hysteresis)>,
		rule: &R,
	) -> Self {
		let mut chain = Self::default();
		let mut queue: VecDeque<(usize, Hysteresis, Vec3)> = VecDeque::new();
		let noise = rule.noise();

		for (node, h) in start_nodes {
			let idx = chain.push_node(node, h.clone());
			queue.push_back((idx, h, Vec3::Y));
		}

		while let Some((parent_idx, parent_h, incoming_ray)) = queue.pop_front() {
			if parent_h.depth >= parent_h.max_depth {
				continue;
			}

			let parent = chain.nodes[parent_idx].clone();
			let n_children = parent_h.sample_child_count(&parent, noise);

			for i in 0..n_children {
				let child_index = i as u32;
				let child_h = rule.generate_ith_child_hysteresis(child_index, &parent, &parent_h);
				let ray = child_h.project_ith_child_ray(child_index, &parent, incoming_ray, noise);
				let radius = child_h.project_ith_child_radius(child_index, &parent, noise);
				let child = BallStickNode::new(parent.position + ray, radius);
				let child_idx = chain.push_node(child, child_h.clone());
				chain.add_child(parent_idx, child_idx);
				queue.push_back((child_idx, child_h, ray));
			}
		}

		chain
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hysteresis {
	pub depth: usize,
	pub max_depth: usize,
	pub segment_index: usize,
	pub child_count: Range<usize>,
	pub length: Range<f32>,
	pub radius: Range<f32>,
	pub ray_degrees_of_freedom: f32,
	pub bias_ray: Vec3,
}

impl Default for Hysteresis {
	fn default() -> Self {
		Self {
			depth: 0,
			max_depth: 4,
			segment_index: 0,
			child_count: 1..3,
			length: 0.2..0.5,
			radius: 0.02..0.08,
			ray_degrees_of_freedom: 0.14,
			bias_ray: Vec3::Y,
		}
	}
}

impl Hysteresis {
	/// How many children to spawn at this node (noise-driven, half-open range on [`Self::child_count`]).
	pub fn sample_child_count(&self, parent: &BallStickNode, noise: &NoiseConfig) -> usize {
		noise.sample_range_usize_4d(
			self.child_count.start,
			self.child_count.end,
			parent.position.x,
			parent.position.y,
			parent.position.z,
			self.segment_index as f32,
		)
	}

	pub fn blend_direction(&self, incoming_ray: Vec3) -> Vec3 {
		let prev = incoming_ray.normalize_or_zero();
		let prev = if prev.length_squared() < 1e-12 { Vec3::Y } else { prev };
		let bias = self.bias_ray.normalize_or_zero();
		let bias = if bias.length_squared() < 1e-12 { Vec3::Y } else { bias };
		prev.slerp(bias, 0.5).normalize_or_zero()
	}

	pub fn project_ith_child_length(
		&self,
		child_index: u32,
		parent: &BallStickNode,
		noise: &NoiseConfig,
	) -> f32 {
		noise.sample_range_f32_4d(
			self.length.start,
			self.length.end,
			parent.position.x + 3.0,
			parent.position.y,
			parent.position.z,
			self.segment_index as f32 + child_index as f32 * 0.19,
		)
	}

	pub fn project_ith_child_radius(
		&self,
		child_index: u32,
		parent: &BallStickNode,
		noise: &NoiseConfig,
	) -> f32 {
		noise.sample_range_f32_4d(
			self.radius.start,
			self.radius.end,
			parent.position.x,
			parent.position.y + 5.0,
			parent.position.z,
			self.segment_index as f32 + child_index as f32 * 0.23,
		)
	}

	pub fn project_ith_child_ray(
		&self,
		child_index: u32,
		parent: &BallStickNode,
		incoming_ray: Vec3,
		noise: &NoiseConfig,
	) -> Vec3 {
		let mean = self.blend_direction(incoming_ray);
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
		let dir = perturb_direction(mean, self.ray_degrees_of_freedom, u, v);
		dir * self.project_ith_child_length(child_index, parent, noise)
	}
}

pub trait ChainHysteresisRule {
	fn noise(&self) -> &NoiseConfig;

	fn generate_ith_child_hysteresis(
		&self,
		child_index: u32,
		parent: &BallStickNode,
		parent_hysteresis: &Hysteresis,
	) -> Hysteresis;
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

impl ChainHysteresisRule for PeriodicHysteresisRule {
	fn noise(&self) -> &NoiseConfig {
		&self.noise
	}

	fn generate_ith_child_hysteresis(
		&self,
		_child_index: u32,
		_parent: &BallStickNode,
		parent_hysteresis: &Hysteresis,
	) -> Hysteresis {
		let mut h = parent_hysteresis.clone();
		h.depth = parent_hysteresis.depth + 1;
		h.segment_index = parent_hysteresis.segment_index + 1;
		if self.period > 0 && (h.segment_index % self.period) == self.phase_hit {
			h.bias_ray = self.phase_bias_ray;
		}
		h
	}
}

fn perturb_direction(mean: Vec3, dof_rad: f32, u: f32, v: f32) -> Vec3 {
	let mean = mean.normalize_or_zero();
	let mean = if mean.length_squared() < 1e-12 { Vec3::Y } else { mean };
	let up = if mean.y.abs() < 0.99 { Vec3::Y } else { Vec3::X };
	let mut tangent = mean.cross(up);
	if tangent.length_squared() < 1e-12 {
		tangent = mean.cross(Vec3::Z);
	}
	tangent = tangent.normalize_or_zero();
	let bitangent = mean.cross(tangent).normalize_or_zero();
	let d = dof_rad.max(0.0);
	(mean + tangent * (d * u) + bitangent * (d * v)).normalize_or_zero()
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
	fn build_chain_and_iterators_work() -> Result<()> {
		let start = vec![(
			BallStickNode::new(Vec3::ZERO, 0.05),
			Hysteresis { child_count: 1..2, ..Default::default() },
		)];
		let chain = BallStickChain::build(start, &rule());
		assert!(chain.nodes().count() > 0);
		assert!(chain.segments().count() > 0);
		Ok(())
	}
}
