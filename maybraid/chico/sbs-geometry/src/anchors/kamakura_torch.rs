//! **Kamakura Torch** — near-vertical flame variant stashed from early Penmarch work (linear 48°→70° crown).

use std::f32::consts::TAU;

use bevy_math::Vec3;
use procedural_common::NoiseConfig;

use super::stalk_perturbation::{HasStrictStalk, StalkPerturbation};
use super::strict_stalk::StrictStalk;
use super::Anchors;
use crate::chain::storybook_tree::{
	segment_fracs, storybook_branch_depth, StorybookTreeChain, StorybookTreePhase,
};
use crate::chain::{BranchOut, DepthBudget};
use crate::projection::vase_projection_length;
use crate::BallStickNode;
use procedural_common::NoiseParams;

/// Default total tree height `H`.
pub const DEFAULT_TREE_HEIGHT: f32 = 18.0;

/// RFC stalk height as a fraction of `H`.
pub const DEFAULT_STALK_HEIGHT_FRACTION: f32 = 0.70;

/// RFC stalk base radius as a fraction of `H`.
pub const DEFAULT_STALK_BASE_RADIUS_FRACTION: f32 = 0.03;

/// Vase projection span as fractions of `H` (RFC `0.10..0.45`).
pub const DEFAULT_PROJECTION_MIN_FRACTION_OF_HEIGHT: f32 = 0.10;
pub const DEFAULT_PROJECTION_MAX_FRACTION_OF_HEIGHT: f32 = 0.45;

/// RFC outer foliage: distance along limb past which balls may appear.
pub const DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION: f32 = 0.70;

/// Lowest ring along the stalk (unit height fraction).
pub const DEFAULT_FIRST_RING_UNIT_HEIGHT: f32 = 0.20;

/// Clamp epsilon for [`crate::projection::vase_profile`].
pub const DEFAULT_VASE_PROFILE_EPSILON: f32 = 0.08;

/// Center of the vase profile mix.
pub const DEFAULT_PROJECTION_CENTER_FRACTION: f32 = 0.5;

/// Elevation from horizontal at lowest / highest ring (`ring_u` 0 → 1); crown ~70°.
pub const DEFAULT_TORCH_BIAS_LOW_DEGREES: f32 = 48.0;
pub const DEFAULT_TORCH_BIAS_HIGH_DEGREES: f32 = 70.0;

/// [`BranchOut::ray_degrees_of_freedom`] at canopy seeds.
pub const DEFAULT_BRANCH_ANGLE_TOLERANCE_DEGREES: f32 = 24.0;

/// Bias direction for ring `u`: elevation from horizontal in `[low, high]` (RFC torch climb).
pub fn kamakura_torch_branch_direction(
	radial_xz: Vec3,
	ring_u: f32,
	low_degrees: f32,
	high_degrees: f32,
) -> Vec3 {
	let radial = Vec3::new(radial_xz.x, 0.0, radial_xz.z).normalize_or_zero();
	if radial.length_squared() < 1e-12 {
		return Vec3::Y;
	}
	let u = ring_u.clamp(0.0, 1.0);
	let elev = low_degrees + (high_degrees - low_degrees) * u;
	let y = elev.to_radians().tan();
	(radial + Vec3::Y * y).normalize_or_zero()
}

#[derive(Clone, Debug, PartialEq)]
pub struct KamakuraTorchProtoAnchors {
	pub tree_height: f32,
	pub stalk: StrictStalk,
	pub first_ring_unit_height: f32,
	pub last_ring_unit_height: f32,
	pub ring_spacing_unit_height: f32,
	pub anchors_per_ring: u32,
	pub projection_min_fraction_of_height: f32,
	pub projection_max_fraction_of_height: f32,
	pub vase_profile_epsilon: f32,
	pub projection_center_fraction: f32,
	pub torch_bias_low_degrees: f32,
	pub torch_bias_high_degrees: f32,
	pub branch_angle_tolerance: f32,
	pub bias_blend: f32,
	pub branch_depth: usize,
	pub child_count_min: u32,
	pub child_count_max: u32,
	pub outer_foliage_distance_fraction: f32,
	pub branch_base_radius_fraction_of_stalk: f32,
	pub branch_radius_child_scale: (f32, f32),
}

impl Default for KamakuraTorchProtoAnchors {
	fn default() -> Self {
		let h = DEFAULT_TREE_HEIGHT;
		let stalk_h = h * DEFAULT_STALK_HEIGHT_FRACTION;
		Self {
			tree_height: h,
			stalk: StrictStalk {
				stalk_height: stalk_h,
				stalk_base_anchor: Vec3::ZERO,
				stalk_base_radius: DEFAULT_STALK_BASE_RADIUS_FRACTION * h,
			},
			first_ring_unit_height: DEFAULT_FIRST_RING_UNIT_HEIGHT,
			last_ring_unit_height: 1.0,
			ring_spacing_unit_height: 0.08 / DEFAULT_STALK_HEIGHT_FRACTION,
			anchors_per_ring: 6,
			projection_min_fraction_of_height: DEFAULT_PROJECTION_MIN_FRACTION_OF_HEIGHT,
			projection_max_fraction_of_height: DEFAULT_PROJECTION_MAX_FRACTION_OF_HEIGHT,
			vase_profile_epsilon: DEFAULT_VASE_PROFILE_EPSILON,
			projection_center_fraction: DEFAULT_PROJECTION_CENTER_FRACTION,
			torch_bias_low_degrees: DEFAULT_TORCH_BIAS_LOW_DEGREES,
			torch_bias_high_degrees: DEFAULT_TORCH_BIAS_HIGH_DEGREES,
			branch_angle_tolerance: DEFAULT_BRANCH_ANGLE_TOLERANCE_DEGREES.to_radians(),
			bias_blend: 1.0,
			branch_depth: 4,
			child_count_min: 1,
			child_count_max: 3,
			outer_foliage_distance_fraction: DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION,
			branch_base_radius_fraction_of_stalk: 0.12,
			branch_radius_child_scale: (0.75, 0.82),
		}
	}
}

impl KamakuraTorchProtoAnchors {
	pub fn ring_height_fractions(&self) -> Vec<f32> {
		let mut out = Vec::new();
		let mut z = self.first_ring_unit_height;
		while z <= self.last_ring_unit_height + 1e-6 {
			out.push(z);
			z += self.ring_spacing_unit_height;
		}
		out
	}

	pub fn ring_mix_u(&self, z_frac: f32) -> f32 {
		let a = self.first_ring_unit_height;
		let b = self.last_ring_unit_height;
		if (b - a).abs() < 1e-6 {
			return 0.0;
		}
		((z_frac - a) / (b - a)).clamp(0.0, 1.0)
	}

	pub fn projection_length(&self, u: f32) -> f32 {
		vase_projection_length(
			self.tree_height,
			self.projection_min_fraction_of_height,
			self.projection_max_fraction_of_height,
			u,
			self.vase_profile_epsilon,
			self.projection_center_fraction,
		)
	}

	fn limb_base_radius(&self) -> f32 {
		let base = self.stalk.stalk_base_radius.max(1e-4);
		(base * self.branch_base_radius_fraction_of_stalk).max(0.02)
	}

	pub fn hysteresis_seeds(&self, chain_noise: NoiseConfig) -> Vec<StorybookTreeChain> {
		let mut out = Vec::new();
		let k = self.anchors_per_ring.max(1);
		let radial_eps = (self.stalk.stalk_base_radius * 0.05).max(1e-4);
		let limb_r = self.limb_base_radius();
		let depth = storybook_branch_depth(self.branch_depth);
		let fracs = segment_fracs(depth);

		for z_frac in self.ring_height_fractions() {
			let u = self.ring_mix_u(z_frac);
			let proj = self.projection_length(u);

			for i in 0..k {
				let theta = TAU * (i as f32) / (k as f32);
				let radial = Vec3::new(theta.cos(), 0.0, theta.sin());
				let dir = kamakura_torch_branch_direction(
					radial,
					u,
					self.torch_bias_low_degrees,
					self.torch_bias_high_degrees,
				);
				let pos = self.stalk.centroid_at_height_fraction(z_frac) + radial * radial_eps;

				let seed_node = BallStickNode::new(pos, limb_r);
				let first_len = proj * fracs[0];
				let noise = chain_noise.clone();
				let branch = BranchOut::radial_out_horizontal(seed_node, radial)
					.with_hysteresis_context(noise.clone(), 0, dir)
					.with_bias_ray(dir)
					.with_bias_blend(1.0)
					.with_ray_degrees_of_freedom(self.branch_angle_tolerance)
					.with_child_count(
						self.child_count_min as usize
							..(self.child_count_max as usize).saturating_add(1),
					)
					.with_radius_range(limb_r..limb_r)
					.with_radius_range_child_scale(self.branch_radius_child_scale)
					.with_length(first_len * 0.97..first_len * 1.03);

				out.push(StorybookTreeChain::new(
					noise.clone().with_frequency(noise.params().frequency * 10.0),
					proj,
					depth,
					0.0,
					u,
					self.outer_foliage_distance_fraction,
					StorybookTreePhase::BranchOut(DepthBudget { inner: branch, remaining: depth }),
				));
			}
		}

		for a in self.stalk.point_to_point_anchors() {
			out.push(StorybookTreeChain::new(
				chain_noise.clone(),
				0.0,
				depth,
				0.0,
				0.0,
				self.outer_foliage_distance_fraction,
				StorybookTreePhase::Stalk(a),
			));
		}

		out
	}
}

impl HasStrictStalk for KamakuraTorchProtoAnchors {
	fn strict_stalk(&self) -> &StrictStalk {
		&self.stalk
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct KamakuraTorchAnchors {
	pub perturbation: StalkPerturbation<KamakuraTorchProtoAnchors>,
}

impl KamakuraTorchAnchors {
	pub fn new(proto: KamakuraTorchProtoAnchors) -> Self {
		Self { perturbation: StalkPerturbation::new(proto) }
	}

	pub fn with_perturbation(mut self, perturbation: KamakuraTorchAnchorPerturbation) -> Self {
		self.perturbation.noise = perturbation.noise;
		self.perturbation.vertical_offset = perturbation.vertical_offset;
		self.perturbation.angular_scale = perturbation.angular_scale;
		self.perturbation.radius_offset = perturbation.radius_offset;
		self
	}

	pub fn proto(&self) -> &KamakuraTorchProtoAnchors {
		&self.perturbation.inner
	}

	pub fn hysteresis_seeds(&self, chain_noise: NoiseConfig) -> Vec<StorybookTreeChain> {
		let seeds = self.proto().hysteresis_seeds(chain_noise);
		self.perturbation.perturb_anchors(seeds)
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct KamakuraTorchAnchorPerturbation {
	pub noise: NoiseParams,
	pub vertical_offset: std::ops::Range<f32>,
	pub angular_scale: std::ops::Range<f32>,
	pub radius_offset: std::ops::Range<f32>,
}

impl Default for KamakuraTorchAnchorPerturbation {
	fn default() -> Self {
		Self {
			noise: NoiseParams::default(),
			vertical_offset: -1.0..1.0,
			angular_scale: 0.0..0.5,
			radius_offset: -0.05..0.05,
		}
	}
}

impl HasStrictStalk for KamakuraTorchAnchors {
	fn strict_stalk(&self) -> &StrictStalk {
		self.proto().strict_stalk()
	}
}

impl Default for KamakuraTorchAnchors {
	fn default() -> Self {
		Self::new(KamakuraTorchProtoAnchors::default())
	}
}

impl Anchors<StorybookTreeChain> for KamakuraTorchAnchors {
	fn anchors(&self) -> Vec<StorybookTreeChain> {
		self.hysteresis_seeds(NoiseConfig::new(NoiseParams::default()))
	}
}

impl Anchors<StorybookTreeChain> for KamakuraTorchProtoAnchors {
	fn anchors(&self) -> Vec<StorybookTreeChain> {
		self.hysteresis_seeds(NoiseConfig::new(NoiseParams::default()))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn vase_projection_rim_longer_than_base() {
		let a = KamakuraTorchProtoAnchors::default();
		let l_low = a.projection_length(a.ring_mix_u(a.first_ring_unit_height));
		let l_high = a.projection_length(a.ring_mix_u(a.last_ring_unit_height));
		assert!(l_high > l_low, "rim {l_high} should exceed base {l_low}");
	}

	#[test]
	fn kamakura_torch_branch_direction_steepens_toward_crown() {
		let radial = Vec3::new(1.0, 0.0, 0.0);
		let low = kamakura_torch_branch_direction(
			radial,
			0.0,
			DEFAULT_TORCH_BIAS_LOW_DEGREES,
			DEFAULT_TORCH_BIAS_HIGH_DEGREES,
		);
		let high = kamakura_torch_branch_direction(
			radial,
			1.0,
			DEFAULT_TORCH_BIAS_LOW_DEGREES,
			DEFAULT_TORCH_BIAS_HIGH_DEGREES,
		);
		assert!(high.y > low.y);
		let elev = |v: Vec3| v.y.atan2(Vec3::new(v.x, 0.0, v.z).length()).to_degrees();
		assert!(elev(high) > 45.0);
		assert!((elev(high) - 70.0).abs() < 2.0);
	}

	#[test]
	fn anchors_count_matches_rings_times_spokes_plus_stalk() {
		let proto =
			KamakuraTorchProtoAnchors { ring_spacing_unit_height: 0.20, ..Default::default() };
		let ring_count = proto.ring_height_fractions().len();
		let spokes = proto.anchors_per_ring as usize;
		let a = KamakuraTorchAnchors::new(proto);
		assert_eq!(a.anchors().len(), ring_count * spokes + 1);
	}
}
