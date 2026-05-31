//! Smooth arched trunk hysteresis for palms ([#255](https://github.com/ramate-io/maybraid/issues/255)).
//!
//! Walks a predetermined lateral lean curve with one child per step—tighter and smoother than
//! reusing [`super::BranchOut`] bias for persistent arch.

use std::ops::Range;

use bevy_math::Vec3;
use procedural_common::{NoiseConfig, NoiseParams, SetNoiseParams};

use crate::BallStickNode;

use super::length_range;
use super::Hysteresis;

/// World position along the trunk arch at normalized height `t` in `[0, 1]`.
///
/// Lateral offset is zero at the base and reaches `arch_lateral_fraction * trunk_height` at the tip.
pub fn arch_point(base: Vec3, trunk_height: f32, arch_lateral_fraction: f32, t: f32) -> Vec3 {
	let t = t.clamp(0.0, 1.0);
	let lean = arch_lateral_fraction * trunk_height * (2.0 * t - t * t);
	base + Vec3::new(lean, t * trunk_height, 0.0)
}

/// Single-child hysteresis that samples the next point on an arched trunk polyline.
#[derive(Clone)]
pub struct ArchTrunk {
	pub node: BallStickNode,
	pub base: Vec3,
	pub trunk_height: f32,
	pub arch_lateral_fraction: f32,
	pub radius: f32,
	pub noise: NoiseConfig,
	/// Stalk height `H` for segment length ranges (`fraction * H`).
	pub stalk_height: f32,
	pub segment_length_fraction: (f32, f32),
	pub step: usize,
	pub total_steps: usize,
}

impl ArchTrunk {
	pub fn new(
		base: Vec3,
		trunk_height: f32,
		arch_lateral_fraction: f32,
		radius: f32,
		noise: NoiseConfig,
		stalk_height: f32,
		segment_length_fraction: (f32, f32),
		total_steps: usize,
	) -> Self {
		let node = BallStickNode::new(base, radius);
		Self {
			node,
			base,
			trunk_height,
			arch_lateral_fraction,
			radius,
			noise,
			stalk_height,
			segment_length_fraction,
			step: 0,
			total_steps: total_steps.max(1),
		}
	}

	fn length_range(&self) -> Range<f32> {
		let h = self.stalk_height.max(1e-6);
		let (lo, hi) = self.segment_length_fraction;
		h * lo..h * hi
	}

}

impl Hysteresis for ArchTrunk {
	fn ball_stick_node(&self) -> BallStickNode {
		self.node
	}

	fn next_hysteresis(&self) -> Vec<Self> {
		if self.step >= self.total_steps {
			return Vec::new();
		}
		let next_step = self.step + 1;
		let t = next_step as f32 / self.total_steps as f32;
		let pos = arch_point(self.base, self.trunk_height, self.arch_lateral_fraction, t);
		let _len = length_range::sample_f32(
			&self.noise,
			self.length_range(),
			&self.node,
			self.step,
			0,
		);
		vec![Self {
			node: BallStickNode::new(pos, self.radius),
			step: next_step,
			..self.clone()
		}]
	}
}

impl SetNoiseParams for ArchTrunk {
	fn with_noise_params(mut self, params: NoiseParams) -> Self {
		self.noise = NoiseConfig::new(params);
		self
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn arch_point_reaches_lateral_tip() -> Result<()> {
		let base = Vec3::ZERO;
		let h = 10.0;
		let tip = arch_point(base, h, 0.12, 1.0);
		assert!((tip.y - h).abs() < 1e-4);
		assert!((tip.x - 0.12 * h).abs() < 1e-3);
		Ok(())
	}

	#[test]
	fn build_chain_leans_horizontally() -> Result<()> {
		let h = 12.0;
		let trunk_h = 0.85 * h;
		let seed = ArchTrunk::new(
			Vec3::ZERO,
			trunk_h,
			0.12,
			0.025 * h,
			NoiseConfig::new(NoiseParams::default()),
			h,
			(0.05, 0.08),
			12,
		);
		let chain = crate::BallStickChain::build(vec![seed]);
		let tip = chain
			.nodes
			.iter()
			.max_by(|a, b| a.position.y.partial_cmp(&b.position.y).unwrap())
			.map(|n| n.position)
			.unwrap_or(Vec3::ZERO);
		assert!(tip.x > 0.5, "expected lateral lean at tip, got {tip:?}");
		assert!((tip.y - trunk_h).abs() < trunk_h * 0.1);
		Ok(())
	}
}
