//! Storybook Tree canopy as a variable-depth [`BranchOut`] phase machine ([#230](https://github.com/ramate-io/maybraid/issues/230)).

use procedural_common::{NoiseConfig, NoiseParams, SetNoiseParams};

use crate::anchors::stalk_perturbation::{perturb_node, AnchorPerturbation, PerturbAnchor};
use crate::BallStickNode;

use super::point_to_point::PointToPoint;
use super::{BranchOut, DepthBudget, Hysteresis};

/// RFC segment fractions of total projection length (sum to 1.0) for `3..=5` hops.
pub fn segment_fracs(depth: usize) -> Vec<f32> {
	match depth.clamp(3, 5) {
		3 => vec![0.50, 0.30, 0.20],
		4 => vec![0.50, 0.22, 0.18, 0.10],
		_ => vec![0.42, 0.22, 0.18, 0.10, 0.08],
	}
}

#[derive(Clone)]
pub enum StorybookTreePhase {
	Stalk(PointToPoint),
	BranchOut(DepthBudget<BranchOut>),
}

/// One canopy limb (or the stalk): projection budget, distance along limb, and phase state.
#[derive(Clone)]
pub struct StorybookTreeChain {
	pub noise: NoiseConfig,
	/// Total limb length budget from the anchor ring (world units).
	pub projection_length: f32,
	pub branch_depth: usize,
	/// Arc length from the ring anchor along this limb (for RFC outer foliage rule).
	pub distance_from_anchor: f32,
	/// Normalized ring height `0` = lowest, `1` = highest.
	pub ring_u: f32,
	/// Fraction of [`Self::projection_length`] past which outer foliage may appear (RFC `0.65`).
	pub outer_foliage_distance_fraction: f32,
	pub phase: StorybookTreePhase,
}

impl StorybookTreeChain {
	pub fn new(
		noise: NoiseConfig,
		projection_length: f32,
		branch_depth: usize,
		distance_from_anchor: f32,
		ring_u: f32,
		outer_foliage_distance_fraction: f32,
		phase: StorybookTreePhase,
	) -> Self {
		Self {
			noise,
			projection_length,
			branch_depth: branch_depth.clamp(3, 5),
			distance_from_anchor,
			ring_u,
			outer_foliage_distance_fraction,
			phase,
		}
	}

	fn with_phase(&self, phase: StorybookTreePhase) -> Self {
		Self {
			phase,
			noise: self.noise.clone(),
			projection_length: self.projection_length,
			branch_depth: self.branch_depth,
			distance_from_anchor: self.distance_from_anchor,
			ring_u: self.ring_u,
			outer_foliage_distance_fraction: self.outer_foliage_distance_fraction,
		}
	}

	fn with_distance(mut self, distance_from_anchor: f32) -> Self {
		self.distance_from_anchor = distance_from_anchor;
		self
	}

	fn segment_fraction(&self, remaining: usize) -> f32 {
		let fracs = segment_fracs(self.branch_depth);
		let depth = fracs.len();
		let seg_idx = depth.saturating_sub(remaining).min(depth.saturating_sub(1));
		fracs[seg_idx]
	}

	fn branch_children(&self, budget: &DepthBudget<BranchOut>) -> Vec<Self> {
		let frac = self.segment_fraction(budget.remaining);
		let len = self.projection_length * frac;
		let lo = len * 0.97;
		let hi = len * 1.03;
		let next_distance = self.distance_from_anchor + len;

		let mut synced = budget.clone();
		synced.inner.noise = self.noise.clone();
		synced.inner.length = lo..hi;

		synced
			.next_hysteresis()
			.into_iter()
			.map(StorybookTreePhase::BranchOut)
			.map(|phase| self.with_phase(phase).with_distance(next_distance))
			.collect()
	}

	pub fn active_branch_profile(&self) -> Option<&BranchOut> {
		match &self.phase {
			StorybookTreePhase::BranchOut(b) => Some(&b.inner),
			_ => None,
		}
	}

	/// Hop index from the ring anchor along this limb (`0` at the spoke).
	pub fn branch_order(&self) -> usize {
		match &self.phase {
			StorybookTreePhase::BranchOut(b) => self.branch_depth.saturating_sub(b.remaining),
			StorybookTreePhase::Stalk(_) => 0,
		}
	}
}

/// Whether `node_idx` has no children in a built [`crate::BallStickChain`].
pub fn is_graph_terminal<H: Hysteresis>(chain: &crate::BallStickChain<H>, node_idx: usize) -> bool {
	chain.children.get(node_idx).is_some_and(|c| c.is_empty())
}

impl StorybookTreePhase {
	pub fn node(&self) -> &BallStickNode {
		match self {
			Self::Stalk(p) => &p.start,
			Self::BranchOut(b) => &b.inner.node,
		}
	}

	fn with_noise(self, noise: NoiseConfig) -> Self {
		match self {
			Self::BranchOut(mut b) => {
				b.inner = b.inner.with_noise(noise);
				Self::BranchOut(b)
			}
			other => other,
		}
	}
}

impl Hysteresis for StorybookTreeChain {
	fn ball_stick_node(&self) -> BallStickNode {
		*self.phase.node()
	}

	fn next_hysteresis(&self) -> Vec<Self> {
		match &self.phase {
			StorybookTreePhase::Stalk(p) => p
				.next_hysteresis()
				.into_iter()
				.map(|p| self.with_phase(StorybookTreePhase::Stalk(p)))
				.collect(),
			StorybookTreePhase::BranchOut(budget) => self.branch_children(budget),
		}
	}
}

impl SetNoiseParams for StorybookTreeChain {
	fn with_noise_params(mut self, params: NoiseParams) -> Self {
		let noise = NoiseConfig::new(params);
		self.phase = self.phase.with_noise(noise.clone());
		self.noise = noise;
		self
	}
}

impl PerturbAnchor for StorybookTreeChain {
	fn perturb_anchor(mut self, perturbation: AnchorPerturbation) -> Self {
		self.phase = self.phase.perturb_anchor(perturbation);
		self
	}
}

impl StorybookTreePhase {
	fn perturb_anchor(self, perturbation: AnchorPerturbation) -> Self {
		match self {
			Self::Stalk(mut p) => {
				p.start = perturb_node(p.start, perturbation);
				Self::Stalk(p)
			}
			Self::BranchOut(mut b) => {
				b.inner = perturb_branch_out(b.inner, perturbation);
				Self::BranchOut(b)
			}
		}
	}
}

fn perturb_branch_out(mut branch: BranchOut, perturbation: AnchorPerturbation) -> BranchOut {
	branch.node = perturb_node(branch.node, perturbation);
	branch.incoming_ray = super::degree_range::perturb_direction(
		branch.incoming_ray,
		perturbation.angular_scale,
		perturbation.angular_u,
		perturbation.angular_v,
	);
	branch.bias_ray = super::degree_range::perturb_direction(
		branch.bias_ray,
		perturbation.angular_scale,
		perturbation.angular_u,
		perturbation.angular_v,
	);
	branch.radius_range = (branch.radius_range.start + perturbation.radius_offset).max(1e-4)
		..(branch.radius_range.end + perturbation.radius_offset).max(1e-4);
	branch
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::Vec3;
	use procedural_common::NoiseParams;

	#[test]
	fn segment_fracs_sum_to_one() -> anyhow::Result<()> {
		for depth in 3..=5 {
			let fracs = segment_fracs(depth);
			assert_eq!(fracs.len(), depth);
			let sum: f32 = fracs.iter().sum();
			assert!((sum - 1.0).abs() < 1e-5, "depth {depth} sum {sum}");
		}
		Ok(())
	}

	#[test]
	fn limb_reach_near_projection_length() -> anyhow::Result<()> {
		let noise = NoiseConfig::new(NoiseParams { seed: 7, ..Default::default() });
		let fracs = segment_fracs(4);
		let proj = 8.0;
		let first_len = proj * fracs[0];
		let seed = StorybookTreeChain::new(
			noise.clone(),
			proj,
			4,
			0.0,
			0.0,
			0.65,
			StorybookTreePhase::BranchOut(DepthBudget {
				inner: BranchOut::radial_out_horizontal(
					BallStickNode::new(Vec3::new(0.0, 5.0, 0.0), 0.04),
					Vec3::X,
				)
				.with_hysteresis_context(noise, 0, Vec3::X)
				.with_child_count(1..3)
				.with_length(first_len * 0.97..first_len * 1.03),
				remaining: 4,
			}),
		);
		let root = seed.ball_stick_node().position;
		let chain = crate::BallStickChain::build(vec![seed]);
		let max_dist = chain
			.nodes
			.iter()
			.map(|n| n.position.distance(root))
			.fold(0.0f32, f32::max);
		assert!(max_dist > proj * 0.7, "limb span {max_dist} vs projection {proj}");
		Ok(())
	}
}
