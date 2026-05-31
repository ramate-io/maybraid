//! **Waialea Palm** trunk anchor ([#255](https://github.com/ramate-io/maybraid/issues/255), [RFC §3.1.7.8](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/08-waialea-palm/README.md)).
//!
//! Arched trunk via [`crate::chain::ArchTrunk`]; crown fronds at render time from [`crate::sbs::waialea_palm::WaialeaPalmSbs`].

use bevy_math::Vec3;
use procedural_common::NoiseConfig;

use super::strict_stalk::StrictStalk;
use super::Anchors;
use crate::chain::waialea_palm::{WaialeaPalmChain, WaialeaPalmPhase};
use crate::chain::ArchTrunk;
use crate::chain::DepthBudget;

/// Default stalk height for playground Waialea palms.
pub const DEFAULT_STALK_HEIGHT: f32 = 12.0;

/// RFC slender trunk radius as a fraction of stalk height.
pub const DEFAULT_TRUNK_RADIUS_FRACTION_OF_HEIGHT: f32 = 0.025;

/// Trunk and crown layout before [`WaialeaPalmAnchors::hysteresis_seeds`].
#[derive(Clone, Debug, PartialEq)]
pub struct WaialeaPalmProtoAnchors {
	pub stalk: StrictStalk,
	/// Trunk height as a fraction of [`StrictStalk::stalk_height`] (RFC `0.85`).
	pub trunk_height_fraction: f32,
	/// Tip lateral offset as a fraction of trunk height (RFC `0.12`).
	pub arch_lateral_fraction: f32,
	/// Per-segment length as fractions of stalk height (RFC `0.05..0.08`).
	pub segment_length_fraction: (f32, f32),
	/// Stacked frond rings at the crown (RFC `2..3`).
	pub ring_count: u32,
	pub fronds_per_ring: u32,
	/// Vertical spacing between rings as a fraction of stalk height (RFC `0.015`).
	pub ring_spacing_fraction: f32,
	/// Lower-ring vertical bias for frond direction (RFC `0.10`).
	pub crown_vertical_bias_base: f32,
	/// Per-ring increase in vertical bias (RFC `0.18`).
	pub crown_vertical_bias_step: f32,
}

impl Default for WaialeaPalmProtoAnchors {
	fn default() -> Self {
		let h = DEFAULT_STALK_HEIGHT;
		Self {
			stalk: StrictStalk {
				stalk_height: h,
				stalk_base_anchor: Vec3::ZERO,
				stalk_base_radius: DEFAULT_TRUNK_RADIUS_FRACTION_OF_HEIGHT * h,
			},
			trunk_height_fraction: 0.85,
			arch_lateral_fraction: 0.12,
			segment_length_fraction: (0.05, 0.08),
			ring_count: 3,
			fronds_per_ring: 10,
			ring_spacing_fraction: 0.015,
			crown_vertical_bias_base: 0.10,
			crown_vertical_bias_step: 0.18,
		}
	}
}

impl WaialeaPalmProtoAnchors {
	pub fn trunk_height(&self) -> f32 {
		self.stalk.stalk_height.max(1e-6) * self.trunk_height_fraction
	}

	pub fn trunk_segment_count(&self) -> usize {
		let h = self.stalk.stalk_height.max(1e-6);
		let (lo, hi) = self.segment_length_fraction;
		let mean = (lo + hi) * 0.5 * h;
		(self.trunk_height() / mean.max(1e-4)).ceil().max(1.0) as usize
	}

	pub fn ring_spacing(&self) -> f32 {
		self.stalk.stalk_height.max(1e-6) * self.ring_spacing_fraction
	}

	/// RFC `base + ring * step` for ring index `ring`.
	pub fn ring_vertical_bias(&self, ring: u32) -> f32 {
		self.crown_vertical_bias_base + ring as f32 * self.crown_vertical_bias_step
	}

	pub fn hysteresis_seeds(&self, chain_noise: NoiseConfig) -> Vec<WaialeaPalmChain> {
		let h = self.stalk.stalk_height.max(1e-6);
		let trunk_h = self.trunk_height();
		let r = self.stalk.stalk_base_radius;
		let base = self.stalk.stalk_base_anchor;

		let arch = ArchTrunk::new(
			base,
			trunk_h,
			self.arch_lateral_fraction,
			r,
			chain_noise.clone(),
			h,
			self.segment_length_fraction,
			self.trunk_segment_count(),
		);

		vec![WaialeaPalmChain::new(
			chain_noise,
			WaialeaPalmPhase::Trunk(DepthBudget {
				inner: arch,
				remaining: self.trunk_segment_count(),
			}),
		)]
	}
}

/// Anchor recipe for [`crate::sbs::waialea_palm::WaialeaPalmSbs`].
#[derive(Clone, Debug, PartialEq)]
pub struct WaialeaPalmAnchors {
	proto: WaialeaPalmProtoAnchors,
}

impl WaialeaPalmAnchors {
	pub fn new(proto: WaialeaPalmProtoAnchors) -> Self {
		Self { proto }
	}

	pub fn proto(&self) -> &WaialeaPalmProtoAnchors {
		&self.proto
	}

	pub fn hysteresis_seeds(&self, chain_noise: NoiseConfig) -> Vec<WaialeaPalmChain> {
		self.proto.hysteresis_seeds(chain_noise)
	}
}

impl Anchors<WaialeaPalmChain> for WaialeaPalmAnchors {
	fn anchors(&self) -> Vec<WaialeaPalmChain> {
		self.hysteresis_seeds(NoiseConfig::new(procedural_common::NoiseParams::default()))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_trunk_segment_count_in_expected_band() {
		let proto = WaialeaPalmProtoAnchors::default();
		let n = proto.trunk_segment_count();
		assert!((10..=18).contains(&n), "segment count {n}");
	}
}
