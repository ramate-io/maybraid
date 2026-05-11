//! Sope's Banyan canopy as a **phase machine** on [`super::Hysteresis`] ([#252](https://github.com/ramate-io/maybraid/issues/252)).

use bevy_math::Vec3;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::BallStickNode;

use super::BranchOut;
use super::DepthBudget;
use super::Hysteresis;

/// Canonical name for consumers that only need the hysteresis type (render, anchors).
pub type SopesBanyanHysteresis = SopesBanyanChain;

/// Noise + descender tuning used when [`SopesBanyanAnchors`](crate::anchors::sopes_banyan::SopesBanyanAnchors) build seeds.
#[derive(Clone)]
pub struct SopesBanyanChainRule {
	pub noise: NoiseConfig,
	pub banyan_height: f32,
	pub descender_threshold: f32,
}

impl Default for SopesBanyanChainRule {
	fn default() -> Self {
		Self {
			noise: NoiseConfig::new(NoiseParams::default()),
			banyan_height: 20.0,
			descender_threshold: 0.12,
		}
	}
}

impl SopesBanyanChainRule {
	/// Hook for callers that refresh procedural state before [`crate::BallStickChain::build`] (default no-op).
	pub fn sync_noise_engine(&mut self) {}

	pub fn seed_hysteresis(&self, node: BallStickNode, max_depth: usize) -> SopesBanyanChain {
		let inner = BranchOut::up(node);
		SopesBanyanChain {
			noise: self.noise.clone(),
			banyan_height: self.banyan_height,
			descender_threshold: self.descender_threshold,
			max_depth,
			branch: inner.clone(),
			phase: SopesBanyanPhase::BranchOut(DepthBudget { inner, remaining: max_depth }),
			segment_index: 0,
			incoming_ray: Vec3::Y,
		}
	}
}

/// CLI-parseable fields for [`SopesBanyanChainRule`] (noise params flatten here).
#[cfg(feature = "clap")]
#[derive(Clone, Debug, clap::Args)]
#[command(rename_all = "kebab-case")]
pub struct SopesBanyanChainRuleArgs {
	#[command(flatten)]
	pub noise: NoiseParams,
	#[arg(long, default_value_t = 20.0)]
	pub banyan_height: f32,
	#[arg(long, default_value_t = 0.12)]
	pub descender_threshold: f32,
}

#[cfg(feature = "clap")]
impl Default for SopesBanyanChainRuleArgs {
	fn default() -> Self {
		Self {
			noise: NoiseParams::default(),
			banyan_height: 20.0,
			descender_threshold: 0.12,
		}
	}
}

#[cfg(feature = "clap")]
impl From<SopesBanyanChainRuleArgs> for SopesBanyanChainRule {
	fn from(a: SopesBanyanChainRuleArgs) -> Self {
		Self {
			noise: NoiseConfig::new(a.noise),
			banyan_height: a.banyan_height,
			descender_threshold: a.descender_threshold,
		}
	}
}

/// Flair-up segment: one biased [`BranchOut`] step from the current joint.
#[derive(Clone, Debug, PartialEq)]
pub struct StartFlairUp {
	pub projection: BranchOut,
}

impl StartFlairUp {
	pub fn sample_from_candidate(candidate: SopesBanyanPhase, noise: &NoiseConfig) -> SopesBanyanPhase {
		let _ = noise;
		if candidate.is_branch_out() && candidate.budget_remaining() < 2 {
			let node = *candidate.branch_node();
			let dof = candidate.ray_degrees_of_freedom().unwrap_or(0.14);
			SopesBanyanPhase::StartFlairUp(StartFlairUp {
				projection: BranchOut::up(node)
					.with_ray_degrees_of_freedom(dof * 0.35)
					.single_child(),
			})
		} else {
			candidate
		}
	}

	pub fn project_to_end(
		&self,
		noise: &NoiseConfig,
		segment_index: usize,
		incoming_ray: Vec3,
	) -> EndFlairUp {
		EndFlairUp { node: self.projection.project_tip(noise, segment_index, incoming_ray) }
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EndFlairUp {
	pub node: BallStickNode,
}

/// Descender segment: one downward-biased step.
#[derive(Clone, Debug, PartialEq)]
pub struct StartDescender {
	pub projection: BranchOut,
}

impl StartDescender {
	pub fn sample_from_candidate(
		candidate: SopesBanyanPhase,
		noise: &NoiseConfig,
		banyan_height: f32,
		descender_threshold: f32,
	) -> SopesBanyanPhase {
		let node = *candidate.branch_node();
		if candidate.is_branch_out()
			&& noise.sample_unit_3d(node.position.x, node.position.y, node.position.z) < descender_threshold
		{
			let drop_len = (banyan_height * 2.0).max(candidate.length_range_end().unwrap_or(0.5));
			SopesBanyanPhase::StartDescender(StartDescender {
				projection: BranchOut::down(node)
					.with_ray_degrees_of_freedom(0.0)
					.single_child()
					.with_length(drop_len * 0.92..drop_len * 1.08),
			})
		} else {
			candidate
		}
	}

	pub fn project_to_end(
		&self,
		noise: &NoiseConfig,
		segment_index: usize,
		incoming_ray: Vec3,
	) -> EndDescender {
		EndDescender { node: self.projection.project_tip(noise, segment_index, incoming_ray) }
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EndDescender {
	pub node: BallStickNode,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SopesBanyanPhase {
	BranchOut(DepthBudget<BranchOut>),
	StartFlairUp(StartFlairUp),
	EndFlairUp(EndFlairUp),
	StartDescender(StartDescender),
	EndDescender(EndDescender),
}

impl SopesBanyanPhase {
	pub fn node(&self) -> &BallStickNode {
		match self {
			Self::BranchOut(b) => &b.inner.node,
			Self::StartFlairUp(s) => &s.projection.node,
			Self::EndFlairUp(e) => &e.node,
			Self::StartDescender(s) => &s.projection.node,
			Self::EndDescender(e) => &e.node,
		}
	}

	fn is_branch_out(&self) -> bool {
		matches!(self, Self::BranchOut(_))
	}

	fn budget_remaining(&self) -> usize {
		match self {
			Self::BranchOut(b) => b.remaining,
			_ => 0,
		}
	}

	fn branch_node(&self) -> &BallStickNode {
		self.node()
	}

	fn ray_degrees_of_freedom(&self) -> Option<f32> {
		match self {
			Self::BranchOut(b) => Some(b.inner.ray_degrees_of_freedom),
			_ => None,
		}
	}

	fn length_range_end(&self) -> Option<f32> {
		match self {
			Self::BranchOut(b) => Some(b.inner.length.end),
			_ => None,
		}
	}

	fn after_branch_sampling(
		phase: SopesBanyanPhase,
		noise: &NoiseConfig,
		banyan_height: f32,
		descender_threshold: f32,
	) -> SopesBanyanPhase {
		let p = StartFlairUp::sample_from_candidate(phase, noise);
		StartDescender::sample_from_candidate(p, noise, banyan_height, descender_threshold)
	}
}

/// One limb of Sope's Banyan: [`SopesBanyanPhase`] plus shared noise and segment context for sampling.
#[derive(Clone)]
pub struct SopesBanyanChain {
	pub noise: NoiseConfig,
	pub banyan_height: f32,
	pub descender_threshold: f32,
	/// Original depth cap from anchors (render / tuning).
	pub max_depth: usize,
	/// Latest [`BranchOut`] parameters for consumers that read bias, ranges, etc.
	pub branch: BranchOut,
	pub phase: SopesBanyanPhase,
	pub segment_index: usize,
	pub incoming_ray: Vec3,
}

impl Hysteresis for SopesBanyanChain {
	fn ball_stick_node(&self) -> BallStickNode {
		*self.phase.node()
	}

	fn next_hysteresis(&self) -> Vec<Self> {
		match &self.phase {
			SopesBanyanPhase::BranchOut(budget) => {
				if budget.remaining == 0 {
					return Vec::new();
				}
				let p0 = SopesBanyanPhase::BranchOut(budget.clone());
				let p = SopesBanyanPhase::after_branch_sampling(
					p0,
					&self.noise,
					self.banyan_height,
					self.descender_threshold,
				);
				match p {
					SopesBanyanPhase::BranchOut(b2) => {
						let parent = b2.inner.node;
						let n = b2.inner.sample_child_count(&self.noise, &parent, self.segment_index);
						let mut out = Vec::new();
						for ci in 0..n {
							let ray = b2.inner.sample_ray(
								&self.noise,
								&parent,
								self.segment_index,
								ci as u32,
								self.incoming_ray,
							);
							let rad = b2.inner.sample_child_radius(
								&self.noise,
								&parent,
								self.segment_index,
								ci as u32,
							);
							let child_node = BallStickNode::new(parent.position + ray, rad);
							let mut inner = b2.inner.clone();
							inner.node = child_node;
							let nb = DepthBudget {
								inner,
								remaining: b2.remaining.saturating_sub(1),
							};
							out.push(SopesBanyanChain {
								phase: SopesBanyanPhase::BranchOut(nb.clone()),
								branch: nb.inner.clone(),
								segment_index: self.segment_index + 1,
								incoming_ray: child_node.position - parent.position,
								..self.clone()
							});
						}
						out
					}
					SopesBanyanPhase::StartFlairUp(s) => vec![SopesBanyanChain {
						phase: SopesBanyanPhase::StartFlairUp(s.clone()),
						branch: s.projection.clone(),
						..self.clone()
					}],
					SopesBanyanPhase::StartDescender(s) => vec![SopesBanyanChain {
						phase: SopesBanyanPhase::StartDescender(s.clone()),
						branch: s.projection.clone(),
						..self.clone()
					}],
					_ => Vec::new(),
				}
			}
			SopesBanyanPhase::StartFlairUp(s) => {
				let end_flair = s.project_to_end(&self.noise, self.segment_index, self.incoming_ray);
				let inc = end_flair.node.position - s.projection.node.position;
				vec![SopesBanyanChain {
					phase: SopesBanyanPhase::EndFlairUp(end_flair),
					segment_index: self.segment_index + 1,
					incoming_ray: inc,
					..self.clone()
				}]
			}
			SopesBanyanPhase::EndFlairUp(_) => Vec::new(),
			SopesBanyanPhase::StartDescender(s) => {
				let end_d = s.project_to_end(&self.noise, self.segment_index, self.incoming_ray);
				let inc = end_d.node.position - s.projection.node.position;
				vec![SopesBanyanChain {
					phase: SopesBanyanPhase::EndDescender(end_d),
					segment_index: self.segment_index + 1,
					incoming_ray: inc,
					..self.clone()
				}]
			}
			SopesBanyanPhase::EndDescender(_) => Vec::new(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn build_produces_graph() -> anyhow::Result<()> {
		let rule = SopesBanyanChainRule::default();
		let seed = rule.seed_hysteresis(BallStickNode::new(Vec3::ZERO, 0.05), 3);
		let chain = crate::BallStickChain::build(vec![seed]);
		assert!(chain.nodes.len() > 1);
		Ok(())
	}
}
