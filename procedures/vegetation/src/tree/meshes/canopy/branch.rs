use bevy::prelude::*;
use noise::{Fbm, NoiseFn, OpenSimplex};
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;
use std::hash::Hasher;

#[derive(Debug, Clone)]
pub struct BranchBuilder {
	pub noise: Fbm<OpenSimplex>,
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
	pub noise_scale: f32,
}

impl BranchBuilder {
	pub fn new() -> Self {
		Self {
			noise: Fbm::new(0),
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
			noise_scale: 1000.0,
		}
	}

	pub fn common_tree_builder() -> Self {
		Self {
			noise: Fbm::new(0),
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
			noise_scale: 1000.0,
		}
	}

	pub fn node_children_from(&self, position: Vec3) -> usize {
		// sample to get 0-1 value
		let sample = self.noise.get([
			position.x as f64 * self.noise_scale as f64,
			position.y as f64 * self.noise_scale as f64,
			position.z as f64 * self.noise_scale as f64,
		]) as f32;

		// Map [-1,1] → [0,1]
		let sample = (sample * 0.5 + 0.5).clamp(0.0, 1.0);

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
		let nx = self.noise.get([
			position.x as f64 * self.noise_scale as f64,
			position.y as f64 * self.noise_scale as f64,
			position.z as f64 * self.noise_scale as f64,
			child_index as f64 * -31.7 * self.noise_scale as f64,
		]) as f32;

		let nz = self.noise.get([
			position.x as f64 * self.noise_scale as f64,
			position.y as f64 * self.noise_scale as f64,
			position.z as f64 * self.noise_scale as f64,
			child_index as f64 * 31.7 * self.noise_scale as f64, // decorrelate
		]) as f32;

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
		let n_length = self.noise.get([
			position.x as f64 * self.noise_scale as f64,
			position.y as f64 * self.noise_scale as f64,
			child_index as f64 * -31.7 * self.noise_scale as f64,
			position.z as f64 * self.noise_scale as f64,
		]) as f32;

		// Map [-1,1] → [0,1]
		let n_length = (n_length * 0.5 + 0.5).clamp(0.0, 1.0);

		let length = self.min_segment_length
			+ n_length * (self.max_segment_length - self.min_segment_length);

		direction * length
	}

	pub fn radius_from(&self, position: Vec3, child_index: usize) -> f32 {
		let sample = self.noise.get([
			position.x as f64 * self.noise_scale as f64,
			child_index as f64 * -31.7 * self.noise_scale as f64,
			position.y as f64 * self.noise_scale as f64,
			position.z as f64 * self.noise_scale as f64,
		]) as f32;

		// Map [-1,1] → [0,1]
		let sample = (sample * 0.5 + 0.5).clamp(0.0, 1.0);

		let radius = self.min_radius + sample * (self.max_radius - self.min_radius);
		radius
	}

	pub fn build(&self) -> Branch {
		let mut branch = Branch::new();

		let initial_node = BranchNode::new(self.anchor, self.initial_radius);

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
					let child_node = BranchNode::new(child_position, child_radius);

					// add the child to the branch and queue it for processing
					branch.add_node(child_node.clone());
					branch.add_child(node.clone(), child_node.clone());
					next_queue.push_back((child_node.clone(), child_ray));
				}
			}
			// swap the queues
			queue = next_queue;
		}

		branch
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct BranchNode {
	pub position: Vec3,
	pub radius: f32,
}

impl Eq for BranchNode {}

impl Hash for BranchNode {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.position.x.to_bits().hash(state);
		self.position.y.to_bits().hash(state);
		self.position.z.to_bits().hash(state);
		self.radius.to_bits().hash(state);
	}
}

impl BranchNode {
	pub fn new(position: Vec3, radius: f32) -> Self {
		Self { position, radius }
	}
}

#[derive(Debug, Clone)]
pub struct BranchSegment<'a> {
	pub start: &'a BranchNode,
	pub end: &'a BranchNode,
}

impl<'a> BranchSegment<'a> {
	pub fn ray(&self) -> Vec3 {
		self.end.position - self.start.position
	}
}

#[derive(Debug, Clone)]
pub struct Branch {
	nodes: HashMap<BranchNode, HashSet<BranchNode>>,
}

impl Branch {
	fn new() -> Self {
		Self { nodes: HashMap::new() }
	}

	fn add_node(&mut self, node: BranchNode) {
		// add node if the node is not already in the branch
		if !self.nodes.contains_key(&node) {
			self.nodes.insert(node, HashSet::new());
		}
	}

	fn add_child(&mut self, parent: BranchNode, child: BranchNode) {
		self.nodes.entry(parent).or_insert(HashSet::new()).insert(child);
	}

	pub fn get_children(&self, node: &BranchNode) -> impl Iterator<Item = &BranchNode> {
		self.nodes.get(node).map(|children| children.iter()).unwrap_or_default()
	}

	pub fn nodes(&self) -> impl Iterator<Item = &BranchNode> {
		self.nodes.keys().collect::<Vec<&BranchNode>>().into_iter()
	}

	pub fn segments<'a>(&'a self) -> impl Iterator<Item = BranchSegment<'a>> {
		self.nodes
			.iter()
			.map(|(node, children)| {
				children.iter().map(|child| BranchSegment { start: node, end: child })
			})
			.flatten()
			.collect::<Vec<BranchSegment>>()
			.into_iter()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_add_child() {
		let mut branch = Branch::new();
		let parent = BranchNode::new(Vec3::ZERO, 0.0);
		let child = BranchNode::new(Vec3::new(0.0, 1.0, 0.0), 0.0);
		branch.add_child(parent.clone(), child.clone());
		assert_eq!(branch.nodes().count(), 1);

		assert_eq!(branch.get_children(&parent).count(), 1);
		assert_eq!(branch.get_children(&parent).next().unwrap().position, child.position);
	}

	#[test]
	fn test_ray_from() {
		let mut branch_builder = BranchBuilder::common_tree_builder();

		// bias the branch towards the top
		branch_builder.angle_tolerance = 10.0;
		branch_builder.splitting_coefficient = 0.55;

		// anchor is on the ring of the trunk
		branch_builder.anchor = Vec3::new(0.0, 0.005, 0.005);

		// initial ray is sticking out to the side
		branch_builder.initial_ray = Vec3::new(0.0, 1.0, 1.0);
		branch_builder.bias_ray = Vec3::new(0.0, 1.0, 1.0);
		branch_builder.bias_amount = 0.2;

		// min segment length is 0.002
		branch_builder.min_segment_length = 0.002;

		// max segment length is 0.004
		branch_builder.max_segment_length = 0.01;

		// min radius is 0.002
		branch_builder.min_radius = 0.001;

		// max radius is 0.004
		branch_builder.max_radius = 0.002;

		let branch = branch_builder.build();
		let node = branch.nodes().next().unwrap();
		branch_builder.ray_from(node.position, Vec3::ONE, 0);
		// TODO: ray does not seem determinstic for some reason,
		// we may solve this by moving the whole thing to fastnoise.
	}

	#[test]
	fn test_builder_build() {
		let mut branch_builder = BranchBuilder::common_tree_builder();

		// bias the branch towards the top
		branch_builder.angle_tolerance = 10.0;
		branch_builder.splitting_coefficient = 0.55;

		// anchor is on the ring of the trunk
		branch_builder.anchor = Vec3::new(0.0, 0.005, 0.005);

		// initial ray is sticking out to the side
		branch_builder.initial_ray = Vec3::new(0.0, 1.0, 1.0);
		branch_builder.bias_ray = Vec3::new(0.0, 1.0, 1.0);
		branch_builder.bias_amount = 0.2;

		// min segment length is 0.002
		branch_builder.min_segment_length = 0.002;

		// max segment length is 0.004
		branch_builder.max_segment_length = 0.01;

		// min radius is 0.002
		branch_builder.min_radius = 0.001;

		// max radius is 0.004
		branch_builder.max_radius = 0.002;

		let branch = branch_builder.build();
		assert!(branch.nodes().count() > 3);
		assert!(branch.segments().count() > 2);
	}
}
