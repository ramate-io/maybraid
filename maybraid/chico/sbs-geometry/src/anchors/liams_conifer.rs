//! **Stalk anchor rings** for **Liam's Conifer** ([RFC-183 §3.1.7.2](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/02-liam-s-conifer/README.md), [#244](https://github.com/ramate-io/maybraid/issues/244)).
//!
//! Dense vertical rings from ~10% to ~98% of stalk height with linearly tapering projection length.

use std::f32::consts::TAU;

use bevy_math::Vec3;

use super::stalk_perturbation::{HasStrictStalk, StalkPerturbation};
use super::strict_stalk::StrictStalk;
use super::Anchors;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::chain::liams_conifer::{LiamsConiferChain, LiamsConiferPhase};
use crate::chain::BranchOut;
use crate::chain::DepthBudget;
use crate::BallStickNode;

/// Tilt horizontal radial slightly toward −Y (RFC ~2° downward bias).
fn downward_biased_radial(radial_xz: Vec3, bias_radians: f32) -> Vec3 {
	let radial = Vec3::new(radial_xz.x, 0.0, radial_xz.z).normalize_or_zero();
	if radial.length_squared() < 1e-12 {
		return Vec3::NEG_Y;
	}
	let cos = bias_radians.cos();
	let sin = bias_radians.sin();
	Vec3::new(radial.x * cos, -sin, radial.z * cos).normalize_or_zero()
}

#[derive(Clone, Debug, PartialEq)]
pub struct LiamsConiferProtoAnchors {
	pub stalk: StrictStalk,
	/// First ring height as a fraction of [`StrictStalk::stalk_height`] (RFC ~0.10).
	pub first_ring_unit_height: f32,
	/// Last ring height fraction (RFC ~0.98).
	pub last_ring_unit_height: f32,
	/// Vertical spacing between rings as a fraction of stalk height (RFC ~0.04).
	pub ring_spacing_unit_height: f32,
	pub anchors_per_ring: u32,
	/// Max projection as a fraction of stalk height (RFC `0.05 * H`).
	pub max_projection_fraction_of_height: f32,
	/// Floor as a fraction of max projection (RFC `0.20 * ell_max`).
	pub min_projection_fraction_of_max: f32,
	pub downward_bias_radians: f32,
	pub branch_angle_tolerance: f32,
	pub branch_depth: usize,
	/// Limb joint radius at the ring anchor as a fraction of [`StrictStalk::stalk_base_radius`].
	pub branch_base_radius_fraction_of_stalk: f32,
	/// Smallest sampled child radius as a fraction of stalk base radius.
	pub branch_tip_radius_fraction_of_stalk: f32,
	/// Per-segment down-step on [`BranchOut::radius_range`] (lo, hi multipliers).
	pub branch_radius_child_scale: (f32, f32),
}

impl Default for LiamsConiferProtoAnchors {
	fn default() -> Self {
		let h = 30.0;
		Self {
			stalk: StrictStalk {
				stalk_height: h,
				stalk_base_anchor: Vec3::ZERO,
				stalk_base_radius: 0.025 * h,
			},
			first_ring_unit_height: 0.10,
			last_ring_unit_height: 0.98,
			ring_spacing_unit_height: 0.03,
			anchors_per_ring: 6,
			max_projection_fraction_of_height: 0.15,
			min_projection_fraction_of_max: 0.20,
			downward_bias_radians: 2.0_f32.to_radians(),
			branch_angle_tolerance: 8.0_f32.to_radians(),
			branch_depth: 3,
			branch_base_radius_fraction_of_stalk: 0.40,
			branch_tip_radius_fraction_of_stalk: 0.16,
			branch_radius_child_scale: (0.72, 0.80),
		}
	}
}

impl HasStrictStalk for LiamsConiferProtoAnchors {
	fn strict_stalk(&self) -> &StrictStalk {
		&self.stalk
	}
}

impl LiamsConiferProtoAnchors {
	pub fn ring_height_fractions(&self) -> Vec<f32> {
		let mut out = Vec::new();
		let mut z = self.first_ring_unit_height;
		while z <= self.last_ring_unit_height + 1e-6 {
			out.push(z);
			z += self.ring_spacing_unit_height;
		}
		out
	}

	fn ring_mix_u(&self, z_frac: f32) -> f32 {
		let a = self.first_ring_unit_height;
		let b = self.last_ring_unit_height;
		if (b - a).abs() < 1e-6 {
			return 0.0;
		}
		((z_frac - a) / (b - a)).clamp(0.0, 1.0)
	}

	pub fn projection_length(&self, u: f32) -> f32 {
		let h = self.stalk.stalk_height.max(1e-6);
		let ell_max = h * self.max_projection_fraction_of_height;
		let raw = ell_max * (1.0 - u.clamp(0.0, 1.0));
		raw.max(ell_max * self.min_projection_fraction_of_max)
	}

	fn limb_radius_range(&self) -> (f32, f32) {
		let base = self.stalk.stalk_base_radius.max(1e-4);
		let lo = (base * self.branch_tip_radius_fraction_of_stalk).max(0.035);
		let hi = (base * self.branch_base_radius_fraction_of_stalk).max(lo + 0.02);
		(lo, hi)
	}

	pub fn hysteresis_seeds(&self, chain_noise: NoiseConfig) -> Vec<LiamsConiferChain> {
		let mut out = Vec::new();
		let k = self.anchors_per_ring.max(1);
		let radial_eps = (self.stalk.stalk_base_radius * 0.05).max(1e-4);

		for z_frac in self.ring_height_fractions() {
			let u = self.ring_mix_u(z_frac);
			let proj = self.projection_length(u);

			for i in 0..k {
				let theta = TAU * (i as f32) / (k as f32);
				let radial = Vec3::new(theta.cos(), 0.0, theta.sin());
				let bias = downward_biased_radial(radial, self.downward_bias_radians);
				let pos = self.stalk.centroid_at_height_fraction(z_frac) + radial * radial_eps;

				let (rad_lo, rad_hi) = self.limb_radius_range();
				let seed_node = BallStickNode::new(pos, rad_hi);
				let first_frac = crate::chain::liams_conifer::SEGMENT_FRACS[0];
				let first_len = proj * first_frac;
				let branch = BranchOut::radial_out_horizontal(seed_node, radial)
					.with_hysteresis_context(chain_noise.clone(), 0, bias)
					.with_bias_blend(0.92)
					.with_ray_degrees_of_freedom(self.branch_angle_tolerance)
					.with_child_count(1..2)
					.with_radius_range(rad_lo..rad_hi)
					.with_radius_range_child_scale(self.branch_radius_child_scale)
					.with_length(first_len * 0.97..first_len * 1.03)
					.single_child();

				out.push(LiamsConiferChain::new(
					chain_noise.clone(),
					proj,
					self.branch_depth,
					LiamsConiferPhase::BranchOut(DepthBudget {
						inner: branch,
						remaining: self.branch_depth,
					}),
				));
			}
		}

		for a in self.stalk.point_to_point_anchors() {
			out.push(LiamsConiferChain::new(
				chain_noise.clone(),
				0.0,
				self.branch_depth,
				LiamsConiferPhase::Stalk(a),
			));
		}

		out
	}
}

impl Anchors<LiamsConiferChain> for LiamsConiferProtoAnchors {
	fn anchors(&self) -> Vec<LiamsConiferChain> {
		self.hysteresis_seeds(NoiseConfig::new(NoiseParams::default()))
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct LiamsConiferAnchors {
	pub perturbation: StalkPerturbation<LiamsConiferProtoAnchors>,
}

impl LiamsConiferAnchors {
	pub fn new(proto: LiamsConiferProtoAnchors) -> Self {
		Self { perturbation: StalkPerturbation::new(proto) }
	}

	pub fn with_perturbation(mut self, perturbation: LiamsConiferAnchorPerturbation) -> Self {
		self.perturbation.noise = perturbation.noise;
		self.perturbation.vertical_offset = perturbation.vertical_offset;
		self.perturbation.angular_scale = perturbation.angular_scale;
		self.perturbation.radius_offset = perturbation.radius_offset;
		self
	}

	pub fn proto(&self) -> &LiamsConiferProtoAnchors {
		&self.perturbation.inner
	}

	pub fn hysteresis_seeds(&self, chain_noise: NoiseConfig) -> Vec<LiamsConiferChain> {
		let seeds = self.proto().hysteresis_seeds(chain_noise);
		self.perturbation.perturb_anchors(seeds)
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct LiamsConiferAnchorPerturbation {
	pub noise: NoiseParams,
	pub vertical_offset: std::ops::Range<f32>,
	pub angular_scale: std::ops::Range<f32>,
	pub radius_offset: std::ops::Range<f32>,
}

impl Default for LiamsConiferAnchorPerturbation {
	fn default() -> Self {
		Self {
			noise: NoiseParams::default(),
			vertical_offset: -1.0..1.0,
			angular_scale: 0.0..0.5,
			radius_offset: -0.05..0.05,
		}
	}
}

impl HasStrictStalk for LiamsConiferAnchors {
	fn strict_stalk(&self) -> &StrictStalk {
		self.proto().strict_stalk()
	}
}

impl Default for LiamsConiferAnchors {
	fn default() -> Self {
		Self::new(LiamsConiferProtoAnchors::default())
	}
}

impl Anchors<LiamsConiferChain> for LiamsConiferAnchors {
	fn anchors(&self) -> Vec<LiamsConiferChain> {
		self.hysteresis_seeds(NoiseConfig::new(NoiseParams::default()))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn projection_length_shrinks_with_ring_height() {
		let a = LiamsConiferProtoAnchors::default();
		let z_low = a.first_ring_unit_height;
		let z_high = a.last_ring_unit_height;
		let l0 = a.projection_length(a.ring_mix_u(z_low));
		let l1 = a.projection_length(a.ring_mix_u(z_high));
		assert!(l0 > l1, "upper rings should get shorter projections");
	}

	#[test]
	fn ring_count_matches_spacing_band() {
		let a = LiamsConiferProtoAnchors::default();
		let rings = a.ring_height_fractions();
		let expected = ((a.last_ring_unit_height - a.first_ring_unit_height)
			/ a.ring_spacing_unit_height)
			.floor() as usize
			+ 1;
		assert_eq!(rings.len(), expected);
		assert!((rings[0] - 0.10).abs() < 1e-6);
		assert!(
			rings[expected - 1] <= a.last_ring_unit_height + 1e-6,
			"last ring should reach the top band"
		);
	}

	#[test]
	fn base_ring_projection_scales_with_stalk_height() {
		let a = LiamsConiferProtoAnchors::default();
		let proj = a.projection_length(a.ring_mix_u(a.first_ring_unit_height));
		assert!(
			(proj - a.stalk.stalk_height * a.max_projection_fraction_of_height).abs() < 1e-4,
			"expected max projection at lowest ring"
		);
	}

	#[test]
	fn anchors_count_matches_rings_times_spokes_plus_stalk() {
		let proto = LiamsConiferProtoAnchors {
			ring_spacing_unit_height: 0.20,
			..Default::default()
		};
		let ring_count = proto.ring_height_fractions().len();
		let spokes = proto.anchors_per_ring as usize;
		let a = LiamsConiferAnchors::new(proto);
		assert_eq!(a.anchors().len(), ring_count * spokes + 1);
	}

	#[test]
	fn limb_radii_taper_down_chain_steps() {
		let a = LiamsConiferProtoAnchors::default();
		let (lo, hi) = a.limb_radius_range();
		let (_s_lo, s_hi) = a.branch_radius_child_scale;
		assert!(hi > lo);
		let step1_hi = hi * s_hi;
		assert!(step1_hi < hi, "down-stepping should thin limbs along the chain");
	}
}
