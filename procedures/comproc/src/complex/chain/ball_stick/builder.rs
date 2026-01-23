use crate::noise::config::NoiseConfig;
use bevy::prelude::*;
use noise::NoiseFn;
use noise::Seedable;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Debug;
use std::hash::Hash;
use std::hash::Hasher;

#[derive(Debug, Clone)]
pub struct BallStickBuilder<
	N: NoiseFn<f64, 4> + Seedable + Debug + Clone,
	M: NoiseFn<f64, 3> + Seedable + Debug + Clone,
> {
	pub noise_4d: Option<NoiseConfig<4, N>>,
	pub noise_3d: Option<NoiseConfig<3, M>>,
	pub anchor: Vec3,
	pub initial_ray: Vec3,
	pub bias_ray: Vec3,
	pub bias_amount: f32,
	pub angle_tolerance: f32,
	pub initial_radius: f32,
	pub min_radius: f32,
	pub max_radius: f32,
	pub depth: usize,
	pub splitting_coefficient: f32,
	pub min_segment_length: f32,
	pub max_segment_length: f32,
}

impl<
		N: NoiseFn<f64, 4> + Seedable + Debug + Clone,
		M: NoiseFn<f64, 3> + Seedable + Debug + Clone,
	> BallStickBuilder<N, M>
{
	pub fn new() -> Self {
		Self {
			noise_4d: None,
			noise_3d: None,
			anchor: Vec3::ZERO,
			initial_ray: Vec3::ZERO,
			bias_ray: Vec3::ZERO,
			bias_amount: 0.0,
			angle_tolerance: 0.0,
			initial_radius: 0.0,
			min_radius: 0.0,
			max_radius: 0.0,
			depth: 0,
			splitting_coefficient: 0.0,
			min_segment_length: 0.0,
			max_segment_length: 0.0,
		}
	}

	pub fn common_tree_builder() -> Self {
		Self {
			noise_4d: None,
			noise_3d: None,
			anchor: Vec3::ZERO,
			initial_ray: Vec3::ZERO,
			bias_ray: Vec3::ZERO,
			bias_amount: 0.2,
			// 8 degrees of angle tolerance
			angle_tolerance: 2.0,
			initial_radius: 0.0,
			min_radius: 0.0,
			max_radius: 0.0,
			depth: 4,
			// 60% of the time the node will not split
			splitting_coefficient: 0.6,
			min_segment_length: 0.0,
			max_segment_length: 0.0,
		}
	}

	pub fn with_anchor(mut self, anchor: Vec3) -> Self {
		self.anchor = anchor;
		self
	}

	pub fn with_initial_ray(mut self, initial_ray: Vec3) -> Self {
		self.initial_ray = initial_ray;
		self
	}

	pub fn with_bias_ray(mut self, bias_ray: Vec3) -> Self {
		self.bias_ray = bias_ray;
		self
	}

	pub fn with_bias_amount(mut self, bias_amount: f32) -> Self {
		self.bias_amount = bias_amount;
		self
	}

	pub fn with_angle_tolerance(mut self, angle_tolerance: f32) -> Self {
		self.angle_tolerance = angle_tolerance;
		self
	}

	pub fn with_splitting_coefficient(mut self, splitting_coefficient: f32) -> Self {
		self.splitting_coefficient = splitting_coefficient;
		self
	}

	pub fn with_min_segment_length(mut self, min_segment_length: f32) -> Self {
		self.min_segment_length = min_segment_length;
		self
	}

	pub fn with_max_segment_length(mut self, max_segment_length: f32) -> Self {
		self.max_segment_length = max_segment_length;
		self
	}

	pub fn with_depth(mut self, depth: usize) -> Self {
		self.depth = depth;
		self
	}

	pub fn with_min_radius(mut self, min_radius: f32) -> Self {
		self.min_radius = min_radius;
		self
	}

	pub fn with_max_radius(mut self, max_radius: f32) -> Self {
		self.max_radius = max_radius;
		self
	}

	pub fn with_noise_config_3d(mut self, noise_config: NoiseConfig<3, M>) -> Self {
		self.noise_3d = Some(noise_config);
		self
	}

	pub fn with_noise_config_4d(mut self, noise_config: NoiseConfig<4, N>) -> Self {
		self.noise_4d = Some(noise_config);
		self
	}

	pub fn unit_freqo3(&self, position: Vec3) -> f64 {
		match &self.noise_3d {
			Some(noise) => noise.vec3_on_unit(position),
			None => 0.5,
		}
	}

	pub fn freqo4(&self, position: Vec4) -> f64 {
		match &self.noise_4d {
			Some(noise) => noise.vec4_freqo(position),
			None => 0.0,
		}
	}

	pub fn node_children_from(&self, position: Vec3) -> usize {
		// sample to get 0-1 value
		let sample = self.unit_freqo3(position) as f32;

		// floor sample/splitting_coefficient to get number of children
		let children = 1 + (sample / self.splitting_coefficient).floor() as usize;
		children
	}

	/// Gerenates a ray within the tolerance of the angle_tolerance
	pub fn unrestricted_ray_from(
		&self,
		position: Vec3,
		parent_ray: Vec3,
		child_index: usize,
	) -> Vec3 {
		// 1. Normalize parent
		let parent_dir = parent_ray.normalize();

		// 2. Compute biased mean direction (slow, stable correction)
		let bias_dir = self.bias_ray.normalize();
		let mean_dir = parent_dir.slerp(bias_dir, self.bias_amount);

		// 3. Sample 2D drift noise (independent!)
		let nx =
			self.freqo4(Vec4::new(child_index as f32 * -31.7, position.x, position.y, position.z))
				as f32;

		let nz =
			self.freqo4(Vec4::new(position.x, position.y, position.z, child_index as f32 * 31.7))
				as f32;

		// 4. Build perpendicular basis around *mean_dir*
		let up = if mean_dir.abs().y < 0.99 { Vec3::Y } else { Vec3::X };
		let tangent = mean_dir.cross(up).normalize();
		let bitangent = mean_dir.cross(tangent);

		// 5. Apply angular drift
		let drift = self.angle_tolerance; // radians
		let drift_vec = tangent * nx * drift + bitangent * nz * drift;

		// 6. Final direction
		(mean_dir + drift_vec).normalize()
	}

	/// Generates a ray within the angle and length tolerance
	pub fn ray_from(&self, position: Vec3, parent_ray: Vec3, child_index: usize) -> Vec3 {
		let direction = self.unrestricted_ray_from(position, parent_ray, child_index);

		// Independent noise for length
		// todo: if this scales with the noise_scale, we get bad adherence to the bias ray for some reason
		let n_length =
			self.freqo4(Vec4::new(position.x, position.y, child_index as f32 * -31.7, position.z))
				as f32;

		// Map [-1,1] → [0,1]
		let n_length = (n_length * 0.5 + 0.5).clamp(0.0, 1.0);

		let length = self.min_segment_length
			+ n_length * (self.max_segment_length - self.min_segment_length);

		direction * length
	}

	pub fn radius_from(&self, position: Vec3, child_index: usize) -> f32 {
		let sample =
			self.freqo4(Vec4::new(position.x, child_index as f32 * -31.7, position.y, position.z))
				as f32;

		// Map [-1,1] → [0,1]
		let sample = (sample * 0.5 + 0.5).clamp(0.0, 1.0);

		let radius = self.min_radius + sample * (self.max_radius - self.min_radius);
		radius
	}

	pub fn build(&self) -> BallStick {
		let mut ballstick = BallStick::new();

		let initial_node = BallStickNode::new(self.anchor, self.initial_radius);

		let mut queue = VecDeque::new();
		queue.push_back((initial_node.clone(), self.initial_ray.clone()));

		for _ in 0..self.depth {
			let mut next_queue = VecDeque::new();
			while let Some((node, ray)) = queue.pop_front() {
				let children = self.node_children_from(node.position);
				for i in 0..children {
					// generate the child attributes
					let child_ray = self.ray_from(node.position, ray, i);
					let child_position = node.position + child_ray;
					let child_radius = self.radius_from(node.position, i);
					let child_node = BallStickNode::new(child_position, child_radius);

					// add the child to the ballstick and queue it for processing
					ballstick.add_node(child_node.clone());
					ballstick.add_child(node.clone(), child_node.clone());
					next_queue.push_back((child_node.clone(), child_ray));
				}
			}
			// swap the queues
			queue = next_queue;
		}

		ballstick
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct BallStickNode {
	pub position: Vec3,
	pub radius: f32,
}

impl Eq for BallStickNode {}

impl Hash for BallStickNode {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.position.x.to_bits().hash(state);
		self.position.y.to_bits().hash(state);
		self.position.z.to_bits().hash(state);
		self.radius.to_bits().hash(state);
	}
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

#[derive(Debug, Clone)]
pub struct BallStick {
	nodes: HashMap<BallStickNode, HashSet<BallStickNode>>,
}

impl BallStick {
	fn new() -> Self {
		Self { nodes: HashMap::new() }
	}

	fn add_node(&mut self, node: BallStickNode) {
		// add node if the node is not already in the ballstick
		if !self.nodes.contains_key(&node) {
			self.nodes.insert(node, HashSet::new());
		}
	}

	fn add_child(&mut self, parent: BallStickNode, child: BallStickNode) {
		self.nodes.entry(parent).or_insert(HashSet::new()).insert(child);
	}

	pub fn get_children(&self, node: &BallStickNode) -> impl Iterator<Item = &BallStickNode> {
		self.nodes.get(node).map(|children| children.iter()).unwrap_or_default()
	}

	pub fn nodes(&self) -> impl Iterator<Item = &BallStickNode> {
		self.nodes.keys().collect::<Vec<&BallStickNode>>().into_iter()
	}

	pub fn segments<'a>(&'a self) -> impl Iterator<Item = BallStickSegment<'a>> {
		self.nodes
			.iter()
			.map(|(node, children)| {
				children.iter().map(|child| BallStickSegment { start: node, end: child })
			})
			.flatten()
			.collect::<Vec<BallStickSegment>>()
			.into_iter()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use noise::{Fbm, OpenSimplex};

	#[test]
	fn test_add_child() {
		let mut ballstick = BallStick::new();
		let parent = BallStickNode::new(Vec3::ZERO, 0.0);
		let child = BallStickNode::new(Vec3::new(0.0, 1.0, 0.0), 0.0);
		ballstick.add_child(parent.clone(), child.clone());
		assert_eq!(ballstick.nodes().count(), 1);

		assert_eq!(ballstick.get_children(&parent).count(), 1);
		assert_eq!(ballstick.get_children(&parent).next().unwrap().position, child.position);
	}

	#[test]
	fn test_ray_from() {
		let mut ballstick_builder =
			BallStickBuilder::<Fbm<OpenSimplex>, Fbm<OpenSimplex>>::common_tree_builder();

		// bias the ballstick towards the top
		ballstick_builder.angle_tolerance = 10.0;
		ballstick_builder.splitting_coefficient = 0.55;

		// anchor is on the ring of the trunk
		ballstick_builder.anchor = Vec3::new(0.0, 0.005, 0.005);

		// initial ray is sticking out to the side
		ballstick_builder.initial_ray = Vec3::new(0.0, 1.0, 1.0);
		ballstick_builder.bias_ray = Vec3::new(0.0, 1.0, 1.0);
		ballstick_builder.bias_amount = 0.2;

		// min segment length is 0.002
		ballstick_builder.min_segment_length = 0.002;

		// max segment length is 0.004
		ballstick_builder.max_segment_length = 0.01;

		// min radius is 0.002
		ballstick_builder.min_radius = 0.001;

		// max radius is 0.004
		ballstick_builder.max_radius = 0.002;

		let ballstick = ballstick_builder.build();
		let node = ballstick.nodes().next().unwrap();
		ballstick_builder.ray_from(node.position, Vec3::ONE, 0);
		// TODO: ray does not seem determinstic for some reason,
		// we may solve this by moving the whole thing to fastnoise.
	}

	#[test]
	fn test_builder_build() {
		let mut ballstick_builder =
			BallStickBuilder::<Fbm<OpenSimplex>, Fbm<OpenSimplex>>::common_tree_builder();

		// bias the ballstick towards the top
		ballstick_builder.angle_tolerance = 10.0;
		ballstick_builder.splitting_coefficient = 0.55;

		// anchor is on the ring of the trunk
		ballstick_builder.anchor = Vec3::new(0.0, 0.005, 0.005);

		// initial ray is sticking out to the side
		ballstick_builder.initial_ray = Vec3::new(0.0, 1.0, 1.0);
		ballstick_builder.bias_ray = Vec3::new(0.0, 1.0, 1.0);
		ballstick_builder.bias_amount = 0.2;

		// min segment length is 0.002
		ballstick_builder.min_segment_length = 0.002;

		// max segment length is 0.004
		ballstick_builder.max_segment_length = 0.01;

		// min radius is 0.002
		ballstick_builder.min_radius = 0.001;

		// max radius is 0.004
		ballstick_builder.max_radius = 0.002;

		let ballstick = ballstick_builder.build();
		assert!(ballstick.nodes().count() > 3);
		assert!(ballstick.segments().count() > 2);
	}
}
