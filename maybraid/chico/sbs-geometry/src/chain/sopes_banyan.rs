//! [`ChainHysteresisRule`](crate::ChainHysteresisRule) (and related hysteresis) for **Sope's Banyan** ball-stick chains.
//!
//! # Intent ([RFC-183 §3.1.7.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/06-sope-s-banyan/README.md), [#252](https://github.com/ramate-io/maybraid/issues/252))
//!
//! Sope's Banyan uses **long banyan-like chains** with a **torch-leaning canopy** and **periodic downward descenders**. Growth state is [`SopesBanyanHysteresis`] (implements [`crate::Hysteresis`] / [`crate::BallStickGrowth`]).
//!
//! The rule stores authoring **[`NoiseParams`]** (CLI / config friendly); enable crate feature **`clap`** to derive `clap::Args` with a flattened noise group. A private [`NoiseConfig`] is built from those params for [`ChainHysteresisRule::noise`]. Because the cached config is `#[arg(skip)]` for clap, call [`SopesBanyanChainRule::sync_noise_engine`] once after parsing from the CLI so sampling matches the parsed flags.
//!
//! This module only owns the **chain growth rule**; stalk, anchors, sticks, balls, and jungle growths live in sibling crates/modules.

use bevy_math::Vec3;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::{BallStickGrowth, BallStickNode, ChainHysteresisRule, Hysteresis};

use super::{child_count, degree_range, length_range, radius_range};

/// [`NoiseConfig`] is skipped for `clap::Args` (built from [`NoiseParams`]); clap still needs `Default` for skipped fields.
#[derive(Clone)]
struct NoiseConfigCache(NoiseConfig);

impl Default for NoiseConfigCache {
	fn default() -> Self {
		Self(NoiseConfig::new(NoiseParams::default()))
	}
}

impl std::ops::Deref for NoiseConfigCache {
	type Target = NoiseConfig;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

/// Elevation from horizontal at low segment \(u\) (inner canopy).
pub const CANOPY_ELEV_LOW_U_RAD: f32 = 40.0f32.to_radians();
/// Elevation from horizontal at high \(u\) (crown top): shallow for a **horizontal** radial read.
pub const CANOPY_ELEV_HIGH_U_RAD: f32 = 11.0f32.to_radians();
/// Canopy segment angle spread.
pub const CANOPY_RAY_DOF_RAD: f32 = 1.0f32.to_radians();
/// Descender spread: tight so drops read **straight down**.
pub const DESCENDER_RAY_DOF_RAD: f32 = 0.6f32.to_radians();

/// Hysteresis state for Sope's Banyan chains: carries the current [`BallStickNode`] plus growth parameters.
#[derive(Clone, Debug, PartialEq)]
pub struct SopesBanyanHysteresis {
	pub node: BallStickNode,
	pub depth: usize,
	pub max_depth: usize,
	pub segment_index: usize,
	pub child_count: std::ops::Range<usize>,
	pub length: std::ops::Range<f32>,
	pub radius: std::ops::Range<f32>,
	pub ray_degrees_of_freedom: f32,
	pub bias_ray: Vec3,
	pub bias_blend: f32,
}

impl Hysteresis for SopesBanyanHysteresis {
	fn ball_stick_node(&self) -> BallStickNode {
		self.node
	}

	fn next_hysteresis(&self) -> Vec<Self> {
		Vec::new()
	}
}

impl BallStickGrowth for SopesBanyanHysteresis {
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
		radius_range::sample_f32(noise, self.radius.clone(), parent, self.segment_index, child_index)
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
		dir * length_range::sample_f32(noise, self.length.clone(), parent, self.segment_index, child_index)
	}
}

/// [`ChainHysteresisRule`] for Sope's Banyan: canopy torch bias vs height, periodic downward descenders.
#[derive(Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct SopesBanyanChainRule {
	/// FastNoise authoring parameters (flattened into CLI when feature `clap` is enabled).
	#[cfg_attr(feature = "clap", command(flatten))]
	pub noise: NoiseParams,
	#[cfg_attr(feature = "clap", arg(skip))]
	noise_config: NoiseConfigCache,
	/// Normalizes [`SopesBanyanHysteresis::segment_index`] to \(u \in [0,1]\) for mixing torch elevation.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 8))]
	pub segment_count_reference: usize,
	/// Descender every `descender_period` segments on the child [`SopesBanyanHysteresis::segment_index`].
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 4))]
	pub descender_period: usize,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0))]
	pub descender_phase: usize,
}

impl Default for SopesBanyanChainRule {
	fn default() -> Self {
		Self::new(NoiseParams::default(), 8, 4, 0)
	}
}

impl SopesBanyanChainRule {
	pub fn new(
		noise: NoiseParams,
		segment_count_reference: usize,
		descender_period: usize,
		descender_phase: usize,
	) -> Self {
		let noise_config = NoiseConfigCache(NoiseConfig::new(noise));
		Self { noise, noise_config, segment_count_reference, descender_period, descender_phase }
	}

	pub fn sync_noise_engine(&mut self) {
		self.noise_config = NoiseConfigCache(NoiseConfig::new(self.noise));
	}

	pub fn set_noise(&mut self, noise: NoiseParams) {
		self.noise = noise;
		self.sync_noise_engine();
	}

	/// Seed hysteresis for a canopy anchor (node position + radius from anchors).
	pub fn seed_hysteresis(seed_node: BallStickNode, max_depth: usize) -> SopesBanyanHysteresis {
		SopesBanyanHysteresis {
			node: seed_node,
			depth: 0,
			max_depth,
			segment_index: 0,
			child_count: 1..4,
			length: 0.42..2.0,
			radius: 0.016..0.055,
			ray_degrees_of_freedom: CANOPY_RAY_DOF_RAD,
			bias_ray: canopy_torch_bias_from_position(seed_node.position, 0.0),
			bias_blend: 0.5,
		}
	}

	fn height_mix_u(&self, segment_index: usize) -> f32 {
		let denom = self.segment_count_reference.max(1) as f32;
		(segment_index as f32 / denom).clamp(0.0, 1.0)
	}

	fn is_descender_child_segment(&self, child_segment_index: usize) -> bool {
		if self.descender_period == 0 {
			return false;
		}
		child_segment_index > 0
			&& child_segment_index % self.descender_period == self.descender_phase
	}
}

impl ChainHysteresisRule<SopesBanyanHysteresis> for SopesBanyanChainRule {
	fn noise(&self) -> &NoiseConfig {
		&*self.noise_config
	}

	fn generate_ith_child_hysteresis(
		&self,
		_child_index: u32,
		parent: &BallStickNode,
		parent_h: &SopesBanyanHysteresis,
	) -> SopesBanyanHysteresis {
		let mut h = parent_h.clone();
		h.depth = parent_h.depth + 1;
		h.segment_index = parent_h.segment_index + 1;

		if self.is_descender_child_segment(h.segment_index) {
			h.bias_ray = Vec3::NEG_Y;
			h.bias_blend = 1.0;
			h.child_count = 1..2;
			h.ray_degrees_of_freedom = DESCENDER_RAY_DOF_RAD;
			h.length = 0.14..0.42;
			h.radius = 0.012..0.045;
		} else {
			let u = self.height_mix_u(h.segment_index);
			h.bias_ray = canopy_torch_bias_from_position(parent.position, u);
			h.bias_blend = 0.5;
			h.child_count = 1..4;
			h.ray_degrees_of_freedom = CANOPY_RAY_DOF_RAD;
			h.length = 0.14..0.42;
			h.radius = 0.016..0.055;
		}

		h
	}
}

/// Horizontal radial through `position`, with elevation **decreasing** as `u` increases.
fn canopy_torch_bias_from_position(position: Vec3, u: f32) -> Vec3 {
	let radial = horizontal_radial(position);
	let u = u.clamp(0.0, 1.0);
	let elev = CANOPY_ELEV_LOW_U_RAD + (CANOPY_ELEV_HIGH_U_RAD - CANOPY_ELEV_LOW_U_RAD) * u;
	let out = radial * elev.cos() + Vec3::Y * elev.sin();
	out.normalize_or_zero()
}

fn horizontal_radial(position: Vec3) -> Vec3 {
	let r = Vec3::new(position.x, 0.0, position.z);
	if r.length_squared() < 1e-10 {
		Vec3::X
	} else {
		r.normalize()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::BallStickChain;
	use anyhow::Result;
	use procedural_common::NoiseParams;

	fn rule() -> SopesBanyanChainRule {
		SopesBanyanChainRule::new(
			NoiseParams {
				seed: 7,
				frequency: 2.0,
				amplitude: 1.0,
				octaves: 1,
				..Default::default()
			},
			8,
			4,
			0,
		)
	}

	#[test]
	fn sope_chain_builds_and_hits_descender_phase() -> Result<()> {
		let seed = BallStickNode::new(Vec3::new(1.0, 0.0, 0.0), 0.05);
		let start = vec![SopesBanyanChainRule::seed_hysteresis(seed, 12)];
		let chain = BallStickChain::build(start, &rule());
		assert!(chain.nodes.len() > 1);
		assert!(chain.segments().count() > 0);

		let has_descender_bias = chain.hysteresis.iter().any(|h| h.bias_ray.y < -0.9);
		assert!(has_descender_bias, "expected some descender hysteresis (bias toward -Y)");

		Ok(())
	}
}
