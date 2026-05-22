//! Liam's Conifer canopy: fixed three-segment sparse [`BranchOut`] ([#244](https://github.com/ramate-io/maybraid/issues/244)).

use procedural_common::{NoiseConfig, NoiseParams, SetNoiseParams};

use crate::anchors::stalk_perturbation::{perturb_node, AnchorPerturbation, PerturbAnchor};
use crate::BallStickNode;

use super::point_to_point::PointToPoint;
use super::{BranchOut, DepthBudget, Hysteresis};

/// RFC segment fractions of total projection length.
pub const SEGMENT_FRACS: [f32; 3] = [0.70, 0.15, 0.15];

#[derive(Clone)]
pub enum LiamsConiferPhase {
	Stalk(PointToPoint),
	BranchOut(DepthBudget<BranchOut>),
}

#[derive(Clone)]
pub struct LiamsConiferChain {
	pub noise: NoiseConfig,
	/// World length budget for this limb (from anchor taper at ring height).
	pub projection_length: f32,
	/// Segment budget for [`SEGMENT_FRACS`] (RFC default 3).
	pub branch_depth: usize,
	pub phase: LiamsConiferPhase,
}

impl LiamsConiferChain {
	pub fn new(
		noise: NoiseConfig,
		projection_length: f32,
		branch_depth: usize,
		phase: LiamsConiferPhase,
	) -> Self {
		Self { noise, projection_length, branch_depth, phase }
	}

	fn with_phase(&self, phase: LiamsConiferPhase) -> Self {
		Self {
			phase,
			noise: self.noise.clone(),
			projection_length: self.projection_length,
			branch_depth: self.branch_depth,
		}
	}

	fn segment_fraction(&self, remaining: usize) -> f32 {
		let depth = self.branch_depth.max(1).min(SEGMENT_FRACS.len());
		let seg_idx = depth.saturating_sub(remaining).min(SEGMENT_FRACS.len() - 1);
		SEGMENT_FRACS[seg_idx]
	}

	fn branch_children(&self, budget: &DepthBudget<BranchOut>) -> Vec<Self> {
		let frac = self.segment_fraction(budget.remaining);
		let len = self.projection_length * frac;
		let lo = len * 0.97;
		let hi = len * 1.03;

		let mut synced = budget.clone();
		synced.inner.noise = self.noise.clone();
		synced.inner.length = lo..hi;

		synced
			.next_hysteresis()
			.into_iter()
			.map(LiamsConiferPhase::BranchOut)
			.map(|phase| self.with_phase(phase))
			.collect()
	}

	/// [`BranchOut`] profile for anchor perturbation and render heuristics.
	pub fn active_branch_profile(&self) -> Option<&BranchOut> {
		match &self.phase {
			LiamsConiferPhase::BranchOut(b) => Some(&b.inner),
			_ => None,
		}
	}
}

impl LiamsConiferPhase {
	pub fn node(&self) -> &BallStickNode {
		match self {
			Self::Stalk(p) => &p.start,
			Self::BranchOut(b) => &b.inner.node,
		}
	}

	pub fn with_noise(self, noise: NoiseConfig) -> Self {
		match self {
			Self::BranchOut(mut b) => {
				b.inner = b.inner.with_noise(noise);
				Self::BranchOut(b)
			}
			other => other,
		}
	}
}

impl Hysteresis for LiamsConiferChain {
	fn ball_stick_node(&self) -> BallStickNode {
		*self.phase.node()
	}

	fn next_hysteresis(&self) -> Vec<Self> {
		match &self.phase {
			LiamsConiferPhase::Stalk(p) => p
				.next_hysteresis()
				.into_iter()
				.map(|p| self.with_phase(LiamsConiferPhase::Stalk(p)))
				.collect(),
			LiamsConiferPhase::BranchOut(budget) => self.branch_children(budget),
		}
	}
}

impl SetNoiseParams for LiamsConiferChain {
	fn with_noise_params(mut self, params: NoiseParams) -> Self {
		let noise = NoiseConfig::new(params);
		self.phase = self.phase.with_noise(noise.clone());
		self.noise = noise;
		self
	}
}

impl PerturbAnchor for LiamsConiferChain {
	fn perturb_anchor(mut self, perturbation: AnchorPerturbation) -> Self {
		self.phase = self.phase.perturb_anchor(perturbation);
		self
	}
}

impl LiamsConiferPhase {
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
	fn limb_reach_near_projection_length() -> anyhow::Result<()> {
		use crate::BallStickChain;

		let noise = NoiseConfig::new(NoiseParams { seed: 7, ..Default::default() });
		let seed = LiamsConiferChain::new(
			noise.clone(),
			4.5,
			3,
			LiamsConiferPhase::BranchOut(DepthBudget {
				inner: BranchOut::radial_out_horizontal(
					BallStickNode::new(Vec3::new(0.0, 3.0, 0.0), 0.04),
					Vec3::X,
				)
				.with_hysteresis_context(noise, 0, Vec3::X)
				.with_child_count(1..2)
				.single_child(),
				remaining: 3,
			}),
		);
		let root = seed.ball_stick_node().position;
		let chain = BallStickChain::build(vec![seed]);
		let max_dist = chain
			.nodes
			.iter()
			.map(|n| n.position.distance(root))
			.fold(0.0f32, f32::max);
		assert!(
			max_dist > 4.5 * 0.85,
			"limb span {max_dist} should approach projection length 4.5"
		);
		Ok(())
	}

	#[test]
	fn build_produces_sparse_graph() -> anyhow::Result<()> {
		let noise = NoiseConfig::new(NoiseParams::default());
		let seed = LiamsConiferChain::new(
			noise.clone(),
			1.5,
			3,
			LiamsConiferPhase::BranchOut(DepthBudget {
				inner: BranchOut::radial_out_horizontal(
					BallStickNode::new(Vec3::ZERO, 0.04),
					Vec3::X,
				)
				.with_hysteresis_context(noise, 0, Vec3::X)
				.with_child_count(1..2)
				.single_child(),
				remaining: 3,
			}),
		);
		let chain = crate::BallStickChain::build(vec![seed]);
		assert!(chain.nodes.len() > 1);
		Ok(())
	}
}
