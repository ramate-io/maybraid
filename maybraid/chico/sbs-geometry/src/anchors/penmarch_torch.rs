//! **Penmarch Torch** stalk anchor rings ([#248](https://github.com/ramate-io/maybraid/issues/248), [RFC §3.1.7.4](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/04-penmarch-torch/README.md)).
//!
//! Lower rings fan outward with slight upward tilt; elevation ramps sharply only in the upper crown
//! (RFC torch flip). Some trees steepen only the top ring — see [`penmarch_elevation_degrees`].

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
use crate::render::mix_seed::mix_seed_below_fraction;
use crate::BallStickNode;
use procedural_common::NoiseParams;

pub const DEFAULT_TREE_HEIGHT: f32 = 18.0;
pub const DEFAULT_STALK_HEIGHT_FRACTION: f32 = 0.70;
pub const DEFAULT_STALK_BASE_RADIUS_FRACTION: f32 = 0.03;
pub const DEFAULT_PROJECTION_MIN_FRACTION_OF_HEIGHT: f32 = 0.10;
pub const DEFAULT_PROJECTION_MAX_FRACTION_OF_HEIGHT: f32 = 0.45;
pub const DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION: f32 = 0.70;
pub const DEFAULT_FIRST_RING_UNIT_HEIGHT: f32 = 0.20;
pub const DEFAULT_VASE_PROFILE_EPSILON: f32 = 0.08;
pub const DEFAULT_PROJECTION_CENTER_FRACTION: f32 = 0.5;
pub const DEFAULT_BRANCH_ANGLE_TOLERANCE_DEGREES: f32 = 24.0;

/// Slight upward fan on lower/mid rings (degrees from horizontal).
pub const DEFAULT_FLARE_ELEVATION_DEGREES: f32 = 28.0;
/// Shoulder elevation just before the crown flip.
pub const DEFAULT_SHOULDER_ELEVATION_DEGREES: f32 = 40.0;
/// Aggressive upward flip at the crown.
pub const DEFAULT_CROWN_ELEVATION_DEGREES: f32 = 82.0;
/// Normalized ring height where the flip begins (`ring_u` in `[0, 1]`).
pub const DEFAULT_CROWN_FLIP_RING_U: f32 = 0.72;
/// Fraction of trees that steepen only the top ring (others use gradual flip).
pub const DEFAULT_APEX_ONLY_FLIP_FRACTION: f32 = 0.35;

fn direction_from_elevation(radial_xz: Vec3, elevation_degrees: f32) -> Vec3 {
	let radial = Vec3::new(radial_xz.x, 0.0, radial_xz.z).normalize_or_zero();
	if radial.length_squared() < 1e-12 {
		return Vec3::Y;
	}
	let y = elevation_degrees.to_radians().tan();
	(radial + Vec3::Y * y).normalize_or_zero()
}

/// Elevation from horizontal for this ring (Penmarch torch silhouette).
pub fn penmarch_elevation_degrees(
	ring_u: f32,
	z_frac: f32,
	last_ring_z: f32,
	flare: f32,
	shoulder: f32,
	crown: f32,
	flip_start_u: f32,
	apex_only_flip: bool,
) -> f32 {
	let on_last_ring = (z_frac - last_ring_z).abs() < 1e-5;
	if apex_only_flip {
		if on_last_ring {
			return crown;
		}
		let u = ring_u.clamp(0.0, 1.0);
		let flip = flip_start_u.clamp(1e-4, 1.0);
		return flare + (shoulder - flare) * (u / flip).min(1.0);
	}

	let u = ring_u.clamp(0.0, 1.0);
	let flip = flip_start_u.clamp(1e-4, 0.999);
	if u < flip {
		return flare + (shoulder - flare) * (u / flip);
	}
	let t = ((u - flip) / (1.0 - flip)).clamp(0.0, 1.0);
	shoulder + (crown - shoulder) * t.powf(3.0)
}

pub fn penmarch_torch_branch_direction(
	radial_xz: Vec3,
	ring_u: f32,
	z_frac: f32,
	last_ring_z: f32,
	flare: f32,
	shoulder: f32,
	crown: f32,
	flip_start_u: f32,
	apex_only_flip: bool,
) -> Vec3 {
	let elev = penmarch_elevation_degrees(
		ring_u,
		z_frac,
		last_ring_z,
		flare,
		shoulder,
		crown,
		flip_start_u,
		apex_only_flip,
	);
	direction_from_elevation(radial_xz, elev)
}

#[derive(Clone, Debug, PartialEq)]
pub struct PenmarchTorchProtoAnchors {
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
	pub flare_elevation_degrees: f32,
	pub shoulder_elevation_degrees: f32,
	pub crown_elevation_degrees: f32,
	pub crown_flip_ring_u: f32,
	pub apex_only_flip_fraction: f32,
	pub branch_angle_tolerance: f32,
	pub branch_depth: usize,
	pub child_count_min: u32,
	pub child_count_max: u32,
	pub outer_foliage_distance_fraction: f32,
	pub branch_base_radius_fraction_of_stalk: f32,
	pub branch_radius_child_scale: (f32, f32),
}

impl Default for PenmarchTorchProtoAnchors {
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
			flare_elevation_degrees: DEFAULT_FLARE_ELEVATION_DEGREES,
			shoulder_elevation_degrees: DEFAULT_SHOULDER_ELEVATION_DEGREES,
			crown_elevation_degrees: DEFAULT_CROWN_ELEVATION_DEGREES,
			crown_flip_ring_u: DEFAULT_CROWN_FLIP_RING_U,
			apex_only_flip_fraction: DEFAULT_APEX_ONLY_FLIP_FRACTION,
			branch_angle_tolerance: DEFAULT_BRANCH_ANGLE_TOLERANCE_DEGREES.to_radians(),
			branch_depth: 4,
			child_count_min: 1,
			child_count_max: 3,
			outer_foliage_distance_fraction: DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION,
			branch_base_radius_fraction_of_stalk: 0.12,
			branch_radius_child_scale: (0.75, 0.82),
		}
	}
}

impl PenmarchTorchProtoAnchors {
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
		let last_ring_z = self.last_ring_unit_height;
		let seed_lane = chain_noise.params().seed.wrapping_mul(0xA11E) as usize;
		let apex_only_flip = mix_seed_below_fraction(
			seed_lane,
			self.stalk.centroid_at_height_fraction(0.5),
			self.apex_only_flip_fraction,
		);

		for z_frac in self.ring_height_fractions() {
			let u = self.ring_mix_u(z_frac);
			let proj = self.projection_length(u);

			for i in 0..k {
				let theta = TAU * (i as f32) / (k as f32);
				let radial = Vec3::new(theta.cos(), 0.0, theta.sin());
				let pos = self.stalk.centroid_at_height_fraction(z_frac) + radial * radial_eps;
				let dir = penmarch_torch_branch_direction(
					radial,
					u,
					z_frac,
					last_ring_z,
					self.flare_elevation_degrees,
					self.shoulder_elevation_degrees,
					self.crown_elevation_degrees,
					self.crown_flip_ring_u,
					apex_only_flip,
				);

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

impl HasStrictStalk for PenmarchTorchProtoAnchors {
	fn strict_stalk(&self) -> &StrictStalk {
		&self.stalk
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct PenmarchTorchAnchors {
	pub perturbation: StalkPerturbation<PenmarchTorchProtoAnchors>,
}

impl PenmarchTorchAnchors {
	pub fn new(proto: PenmarchTorchProtoAnchors) -> Self {
		Self { perturbation: StalkPerturbation::new(proto) }
	}

	pub fn with_perturbation(mut self, perturbation: PenmarchTorchAnchorPerturbation) -> Self {
		self.perturbation.noise = perturbation.noise;
		self.perturbation.vertical_offset = perturbation.vertical_offset;
		self.perturbation.angular_scale = perturbation.angular_scale;
		self.perturbation.radius_offset = perturbation.radius_offset;
		self
	}

	pub fn proto(&self) -> &PenmarchTorchProtoAnchors {
		&self.perturbation.inner
	}

	pub fn hysteresis_seeds(&self, chain_noise: NoiseConfig) -> Vec<StorybookTreeChain> {
		let seeds = self.proto().hysteresis_seeds(chain_noise);
		self.perturbation.perturb_anchors(seeds)
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct PenmarchTorchAnchorPerturbation {
	pub noise: NoiseParams,
	pub vertical_offset: std::ops::Range<f32>,
	pub angular_scale: std::ops::Range<f32>,
	pub radius_offset: std::ops::Range<f32>,
}

impl Default for PenmarchTorchAnchorPerturbation {
	fn default() -> Self {
		Self {
			noise: NoiseParams::default(),
			vertical_offset: -1.0..1.0,
			angular_scale: 0.0..0.5,
			radius_offset: -0.05..0.05,
		}
	}
}

impl HasStrictStalk for PenmarchTorchAnchors {
	fn strict_stalk(&self) -> &StrictStalk {
		self.proto().strict_stalk()
	}
}

impl Default for PenmarchTorchAnchors {
	fn default() -> Self {
		Self::new(PenmarchTorchProtoAnchors::default())
	}
}

impl Anchors<StorybookTreeChain> for PenmarchTorchAnchors {
	fn anchors(&self) -> Vec<StorybookTreeChain> {
		self.hysteresis_seeds(NoiseConfig::new(NoiseParams::default()))
	}
}

impl Anchors<StorybookTreeChain> for PenmarchTorchProtoAnchors {
	fn anchors(&self) -> Vec<StorybookTreeChain> {
		self.hysteresis_seeds(NoiseConfig::new(NoiseParams::default()))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn vase_projection_rim_longer_than_base() {
		let a = PenmarchTorchProtoAnchors::default();
		let l_low = a.projection_length(a.ring_mix_u(a.first_ring_unit_height));
		let l_high = a.projection_length(a.ring_mix_u(a.last_ring_unit_height));
		assert!(l_high > l_low);
	}

	#[test]
	fn crown_flip_is_much_steeper_than_flare() {
		let elev = |u: f32| {
			penmarch_elevation_degrees(u, u, 1.0, 28.0, 40.0, 82.0, 0.72, false)
		};
		assert!(elev(0.0) < 35.0);
		assert!(elev(1.0) > 75.0);
		assert!(elev(1.0) - elev(0.5) > elev(0.5) - elev(0.0));
	}

	#[test]
	fn apex_only_mode_steepens_last_ring_only() {
		let last = penmarch_elevation_degrees(1.0, 1.0, 1.0, 28.0, 40.0, 82.0, 0.72, true);
		let mid = penmarch_elevation_degrees(0.5, 0.5, 1.0, 28.0, 40.0, 82.0, 0.72, true);
		assert!(last > 75.0);
		assert!(mid < 45.0);
	}

	#[test]
	fn anchors_count_matches_rings_times_spokes_plus_stalk() {
		let proto =
			PenmarchTorchProtoAnchors { ring_spacing_unit_height: 0.20, ..Default::default() };
		let ring_count = proto.ring_height_fractions().len();
		let spokes = proto.anchors_per_ring as usize;
		assert_eq!(
			PenmarchTorchAnchors::new(proto).anchors().len(),
			ring_count * spokes + 1
		);
	}
}
