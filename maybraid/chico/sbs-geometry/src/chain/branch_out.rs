//! **Branch-out** state: joint [`BallStickNode`] plus noisy fan-out, child radius, and stick projection (bias, DOF, length).
//!
//! Sampling uses the small helpers in [`super::child_count`], [`super::radius_range`], [`super::length_range`], and [`super::degree_range`].

use std::ops::Range;

use bevy_math::Vec3;
use procedural_common::NoiseConfig;

use crate::BallStickNode;

use super::child_count;
use super::degree_range;
use super::length_range;
use super::radius_range;
use super::Hysteresis;

/// Picks number of children, lengths, directions, of branches
#[derive(Clone, Debug, PartialEq)]
pub struct BranchOut {
	pub node: BallStickNode,
	/// Half-open range for noisy child count.
	pub child_count: Range<usize>,
	/// Half-open range for noisy radius at each child joint.
	pub radius_range: Range<f32>,
	/// Half-open range for noisy segment length toward each child.
	pub length: Range<f32>,
	/// Degrees of freedom for noisy ray direction.
	pub ray_degrees_of_freedom: f32,
	/// Bias ray direction.
	pub bias_ray: Vec3,
	/// Bias ray blend weight.
	/// Blend weight into [`Self::bias_ray`] (`1` = bias only).
	pub bias_blend: f32,
}

impl Default for BranchOut {
	fn default() -> Self {
		Self {
			node: BallStickNode::new(Vec3::ZERO, 0.05),
			child_count: 1..3,
			radius_range: 0.02..0.08,
			length: 0.2..0.5,
			ray_degrees_of_freedom: 0.14,
			bias_ray: Vec3::Y,
			bias_blend: 0.5,
		}
	}
}

impl BranchOut {
	pub fn up(node: BallStickNode) -> Self {
		Self {
			node,
			child_count: 1..3,
			radius_range: 0.02..0.08,
			length: 0.2..0.5,
			ray_degrees_of_freedom: 0.14,
			bias_ray: Vec3::Y,
			bias_blend: 0.5,
		}
	}

	pub fn sample_child_count(
		&self,
		noise: &NoiseConfig,
		parent: &BallStickNode,
		segment_index: usize,
	) -> usize {
		child_count::sample_usize(noise, self.child_count.clone(), parent, segment_index)
	}

	pub fn sample_child_radius(
		&self,
		noise: &NoiseConfig,
		parent: &BallStickNode,
		segment_index: usize,
		child_index: u32,
	) -> f32 {
		radius_range::sample_f32(
			noise,
			self.radius_range.clone(),
			parent,
			segment_index,
			child_index,
		)
	}

	pub fn sample_ray(
		&self,
		noise: &NoiseConfig,
		parent: &BallStickNode,
		segment_index: usize,
		child_index: u32,
		incoming_ray: Vec3,
	) -> Vec3 {
		let mean = Self::blend_direction(incoming_ray, self.bias_ray, self.bias_blend);
		let u = noise.sample_signed_4d(
			parent.position.x + child_index as f32 * 0.37,
			parent.position.y,
			parent.position.z,
			segment_index as f32 + 11.0,
		);
		let v = noise.sample_signed_4d(
			parent.position.x,
			parent.position.y + child_index as f32 * 0.41,
			parent.position.z,
			segment_index as f32 + 13.0,
		);
		let dir = degree_range::perturb_direction(mean, self.ray_degrees_of_freedom, u, v);
		let len = length_range::sample_f32(
			noise,
			self.length.clone(),
			parent,
			segment_index,
			child_index,
		);
		dir * len
	}

	/// Blend incoming growth toward `bias_ray` with weight `t` in `[0, 1]`.
	pub fn blend_direction(incoming_ray: Vec3, bias_ray: Vec3, t: f32) -> Vec3 {
		degree_range::blend_direction(incoming_ray, bias_ray, t)
	}
}

impl Hysteresis for BranchOut {
	fn ball_stick_node(&self) -> BallStickNode {
		self.node
	}

	fn next_hysteresis(&self) -> Vec<Self> {
		todo!()
	}
}
