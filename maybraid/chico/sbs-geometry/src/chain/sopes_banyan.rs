//! [`ChainHysteresisRule`](crate::ChainHysteresisRule) (and related hysteresis) for **Sope's Banyan** ball-stick chains.
//!
//! # Intent ([RFC-183 §3.1.7.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/06-sope-s-banyan/README.md), [#252](https://github.com/ramate-io/maybraid/issues/252))
//!
//! Sope's Banyan uses **long banyan-like chains** with an **upward torch bias** (closer to [Penmarch Torch §3.1.7.4](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/04-penmarch-torch/README.md) than [Honu Banyan §3.1.7.5](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/05-honu-banyan/README.md)): bias strength and effective growth angle should **rise with height** along the chain so the crown reads as a **tall, vase-like** lift rather than Honu's broad horizontal spread.
//!
//! The RFC still calls for **periodic downward descenders** (every third to fourth segment, slightly less frequent than Honu if we want a more vertical read). Implementation should alternate or phase hysteresis so most segments use upward `bias_ray` / canopy bias while selected indices switch to a **strongly downward** descender profile (tighter angle tolerance, different length/radius ranges).
//!
//! The rule stores authoring **[`NoiseParams`]** (CLI / config friendly); enable crate feature **`clap`** to derive `clap::Args` with a flattened noise group. A private [`NoiseConfig`] is built from those params for [`ChainHysteresisRule::noise`]. Because the cached config is `#[arg(skip)]` for clap, call [`SopesBanyanChainRule::sync_noise_engine`] once after parsing from the CLI so sampling matches the parsed flags.
//!
//! This module only owns the **chain growth rule**; stalk, anchors, sticks, balls, and jungle growths live in sibling crates/modules.

use bevy_math::Vec3;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::{BallStickNode, ChainHysteresisRule, Hysteresis};

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

/// Vertical angle from horizontal at the bottom of the canopy mix (RFC ~25°).
pub const TORCH_ELEVATION_LOW_RAD: f32 = 25.0f32.to_radians();
/// Vertical angle from horizontal at the top of the canopy mix (RFC ~70°).
pub const TORCH_ELEVATION_HIGH_RAD: f32 = 70.0f32.to_radians();
/// Canopy segment angle spread (RFC ~12°).
pub const CANOPY_RAY_DOF_RAD: f32 = 12.0f32.to_radians();
/// Descender segment angle spread (RFC ~6°).
pub const DESCENDER_RAY_DOF_RAD: f32 = 6.0f32.to_radians();

/// [`ChainHysteresisRule`] for Sope's Banyan: upward torch bias vs height, periodic downward descenders.
#[derive(Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct SopesBanyanChainRule {
	/// FastNoise authoring parameters (flattened into CLI when feature `clap` is enabled).
	#[cfg_attr(feature = "clap", command(flatten))]
	pub noise: NoiseParams,
	#[cfg_attr(feature = "clap", arg(skip))]
	noise_config: NoiseConfigCache,
	/// Normalizes [`Hysteresis::segment_index`] to \(u \in [0,1]\) for mixing torch elevation (RFC `u` along crown height).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 8))]
	pub segment_count_reference: usize,
	/// Descender every `descender_period` segments on the child [`Hysteresis::segment_index`] (RFC: every ~4th).
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
		Self {
			noise,
			noise_config,
			segment_count_reference,
			descender_period,
			descender_phase,
		}
	}

	/// Rebuild the FastNoise handle from [`Self::noise`]. Required once after `clap` fills this struct (skipped field defaults until this runs).
	pub fn sync_noise_engine(&mut self) {
		self.noise_config = NoiseConfigCache(NoiseConfig::new(self.noise));
	}

	/// Replace noise params and rebuild the cached [`NoiseConfig`].
	pub fn set_noise(&mut self, noise: NoiseParams) {
		self.noise = noise;
		self.sync_noise_engine();
	}

	/// Hysteresis for a chain seed at `position` (anchor / trunk centroid). `max_depth` bounds growth (RFC limb depth).
	pub fn seed_hysteresis(position: Vec3, max_depth: usize) -> Hysteresis {
		let mut h = Hysteresis::default();
		h.max_depth = max_depth;
		h.depth = 0;
		h.segment_index = 0;
		h.child_count = 1..4;
		h.length = 0.14..0.42;
		h.radius = 0.016..0.055;
		h.ray_degrees_of_freedom = CANOPY_RAY_DOF_RAD;
		h.bias_ray = canopy_torch_bias_from_position(position, 0.0);
		h
	}

	fn height_mix_u(&self, segment_index: usize) -> f32 {
		let denom = self.segment_count_reference.max(1) as f32;
		(segment_index as f32 / denom).clamp(0.0, 1.0)
	}

	fn is_descender_child_segment(&self, child_segment_index: usize) -> bool {
		if self.descender_period == 0 {
			return false;
		}
		child_segment_index > 0 && child_segment_index % self.descender_period == self.descender_phase
	}
}

impl ChainHysteresisRule for SopesBanyanChainRule {
	fn noise(&self) -> &NoiseConfig {
		&*self.noise_config
	}

	fn generate_ith_child_hysteresis(
		&self,
		_child_index: u32,
		parent: &BallStickNode,
		parent_h: &Hysteresis,
	) -> Hysteresis {
		let mut h = parent_h.clone();
		h.depth = parent_h.depth + 1;
		h.segment_index = parent_h.segment_index + 1;

		if self.is_descender_child_segment(h.segment_index) {
			h.bias_ray = Vec3::NEG_Y;
			h.child_count = 1..2;
			h.ray_degrees_of_freedom = DESCENDER_RAY_DOF_RAD;
			h.length = 0.22..0.52;
			h.radius = 0.012..0.045;
		} else {
			let u = self.height_mix_u(h.segment_index);
			h.bias_ray = canopy_torch_bias_from_position(parent.position, u);
			h.child_count = 1..4;
			h.ray_degrees_of_freedom = CANOPY_RAY_DOF_RAD;
			h.length = 0.14..0.42;
			h.radius = 0.016..0.055;
		}

		h
	}
}

/// Unit direction: horizontal radial from stalk axis through `position`, lifted by torch elevation mixed with `u`.
fn canopy_torch_bias_from_position(position: Vec3, u: f32) -> Vec3 {
	let radial = horizontal_radial(position);
	let elev = TORCH_ELEVATION_LOW_RAD + (TORCH_ELEVATION_HIGH_RAD - TORCH_ELEVATION_LOW_RAD) * u;
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
		let start = vec![(
			BallStickNode::new(Vec3::new(1.0, 0.0, 0.0), 0.05),
			SopesBanyanChainRule::seed_hysteresis(Vec3::new(1.0, 0.0, 0.0), 12),
		)];
		let chain = BallStickChain::build(start, &rule());
		assert!(chain.nodes.len() > 1);
		assert!(chain.segments().count() > 0);

		let has_descender_bias = chain.hysteresis.iter().any(|h| h.bias_ray.y < -0.9);
		assert!(has_descender_bias, "expected some descender hysteresis (bias toward -Y)");

		Ok(())
	}
}
