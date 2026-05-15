//! Perturb non-stalk anchors around a shared [`StrictStalk`].

use std::ops::Range;

use bevy_math::{Quat, Vec3};
use procedural_common::{NoiseConfig, NoiseParams};

use crate::anchors::strict_stalk::StrictStalk;
use crate::anchors::Anchors;
use crate::chain::point_to_point::PointToPoint;
use crate::chain::sopes_banyan::{SopesBanyanChain, SopesBanyanPhase};
use crate::chain::BranchOut;
use crate::{BallStickNode, Hysteresis};

/// Provides the stalk reference used to identify the unperturbed base anchor.
pub trait HasStrictStalk {
	fn strict_stalk(&self) -> &StrictStalk;
}

/// A sampled perturbation for one anchor seed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnchorPerturbation {
	pub stalk_base_anchor: Vec3,
	pub vertical_offset: f32,
	pub angular_offset_radians: f32,
	pub radius_offset: f32,
}

/// Hysteresis seeds opt into anchor perturbation without changing [`Hysteresis`].
pub trait PerturbAnchor: Hysteresis {
	fn perturb_anchor(self, perturbation: AnchorPerturbation) -> Self;
}

/// Wraps a strict-stalk-backed anchor generator and perturbs every non-base anchor.
#[derive(Clone, Debug, PartialEq)]
pub struct StalkPerturbation<A> {
	pub inner: A,
	pub noise: NoiseParams,
	pub vertical_offset: Range<f32>,
	pub angular_offset_radians: Range<f32>,
	pub radius_offset: Range<f32>,
}

impl<A> StalkPerturbation<A> {
	pub fn new(inner: A) -> Self {
		Self {
			inner,
			noise: NoiseParams::default(),
			vertical_offset: 0.0..0.0,
			angular_offset_radians: 0.0..0.0,
			radius_offset: 0.0..0.0,
		}
	}
}

impl<A: Default> Default for StalkPerturbation<A> {
	fn default() -> Self {
		Self {
			inner: A::default(),
			noise: NoiseParams::default(),
			vertical_offset: 0.0..0.0,
			angular_offset_radians: 0.0..0.0,
			radius_offset: 0.0..0.0,
		}
	}
}

impl HasStrictStalk for StrictStalk {
	fn strict_stalk(&self) -> &StrictStalk {
		self
	}
}

impl<A, T> Anchors<T> for StalkPerturbation<A>
where
	A: Anchors<T> + HasStrictStalk,
	T: PerturbAnchor,
{
	fn anchors(&self) -> Vec<T> {
		self.perturb_anchors(self.inner.anchors())
	}
}

impl<A> StalkPerturbation<A>
where
	A: HasStrictStalk,
{
	pub fn perturb_anchors<T>(&self, anchors: Vec<T>) -> Vec<T>
	where
		T: PerturbAnchor,
	{
		let stalk_base_anchor = self.inner.strict_stalk().stalk_base_anchor;
		let noise = NoiseConfig::new(self.noise);

		anchors
			.into_iter()
			.enumerate()
			.map(|(i, anchor)| {
				let node = anchor.ball_stick_node();
				if is_stalk_base(node.position, stalk_base_anchor) {
					return anchor;
				}

				let perturbation = AnchorPerturbation {
					stalk_base_anchor,
					vertical_offset: sample_range(
						&noise,
						self.vertical_offset.clone(),
						&node,
						i,
						0.0,
					),
					angular_offset_radians: sample_range(
						&noise,
						self.angular_offset_radians.clone(),
						&node,
						i,
						17.0,
					),
					radius_offset: sample_range(&noise, self.radius_offset.clone(), &node, i, 31.0),
				};
				anchor.perturb_anchor(perturbation)
			})
			.collect()
	}
}

impl PerturbAnchor for PointToPoint {
	fn perturb_anchor(mut self, perturbation: AnchorPerturbation) -> Self {
		self.start = perturb_node(self.start, perturbation);
		self.radius = self.start.radius;
		self
	}
}

impl PerturbAnchor for SopesBanyanChain {
	fn perturb_anchor(mut self, perturbation: AnchorPerturbation) -> Self {
		self.phase = perturb_sopes_phase(self.phase, perturbation);
		self
	}
}

fn perturb_sopes_phase(
	phase: SopesBanyanPhase,
	perturbation: AnchorPerturbation,
) -> SopesBanyanPhase {
	match phase {
		SopesBanyanPhase::Stalk(mut p) => {
			p.start = perturb_node(p.start, perturbation);
			SopesBanyanPhase::Stalk(p)
		}
		SopesBanyanPhase::BranchOut(mut b) => {
			b.inner = perturb_branch_out(b.inner, perturbation);
			SopesBanyanPhase::BranchOut(b)
		}
		SopesBanyanPhase::StartFlairUp(mut s) => {
			s.projection = perturb_branch_out(s.projection, perturbation);
			SopesBanyanPhase::StartFlairUp(s)
		}
		SopesBanyanPhase::EndFlairUp(mut e) => {
			e.node = perturb_node(e.node, perturbation);
			SopesBanyanPhase::EndFlairUp(e)
		}
		SopesBanyanPhase::StartDescender(mut s) => {
			s.projection = perturb_branch_out(s.projection, perturbation);
			SopesBanyanPhase::StartDescender(s)
		}
		SopesBanyanPhase::EndDescender(mut e) => {
			e.node = perturb_node(e.node, perturbation);
			SopesBanyanPhase::EndDescender(e)
		}
	}
}

fn perturb_branch_out(mut branch: BranchOut, perturbation: AnchorPerturbation) -> BranchOut {
	branch.node = perturb_node(branch.node, perturbation);
	let rotation = Quat::from_rotation_y(perturbation.angular_offset_radians);
	branch.incoming_ray = rotation * branch.incoming_ray;
	branch.bias_ray = rotation * branch.bias_ray;
	branch.radius_range = (branch.radius_range.start + perturbation.radius_offset).max(1e-4)
		..(branch.radius_range.end + perturbation.radius_offset).max(1e-4);
	branch
}

fn perturb_node(mut node: BallStickNode, perturbation: AnchorPerturbation) -> BallStickNode {
	let rel = node.position - perturbation.stalk_base_anchor;
	let rotation = Quat::from_rotation_y(perturbation.angular_offset_radians);
	node.position =
		perturbation.stalk_base_anchor + rotation * rel + Vec3::Y * perturbation.vertical_offset;
	node.radius = (node.radius + perturbation.radius_offset).max(1e-4);
	node
}

fn sample_range(
	noise: &NoiseConfig,
	range: Range<f32>,
	node: &BallStickNode,
	anchor_index: usize,
	lane: f32,
) -> f32 {
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	let t = noise.sample_unit_4d(
		node.position.x + lane,
		node.position.y + anchor_index as f32,
		node.position.z,
		lane,
	);
	lo + t * (hi - lo)
}

fn is_stalk_base(position: Vec3, stalk_base_anchor: Vec3) -> bool {
	position.distance_squared(stalk_base_anchor) <= 1e-8
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::anchors::sopes_banyan::SopesBanyanProtoAnchors;

	#[test]
	fn perturbs_ring_anchors_but_not_stalk_base() -> anyhow::Result<()> {
		let anchors =
			SopesBanyanProtoAnchors { ring_count: 1, anchors_per_ring: 1, ..Default::default() };
		let perturbation = StalkPerturbation {
			inner: anchors,
			noise: NoiseParams::default(),
			vertical_offset: 1.0..1.0,
			angular_offset_radians: 0.5..0.5,
			radius_offset: 0.2..0.2,
		};

		let seeds = perturbation.anchors();
		assert_eq!(seeds.len(), 2);

		let branch_node = seeds[0].ball_stick_node();
		assert!((branch_node.position.y - 9.0).abs() < 1e-4);
		assert!(branch_node.position.z.abs() > 1e-4);
		assert!((branch_node.radius - 0.45).abs() < 1e-4);

		let stalk_node = seeds[1].ball_stick_node();
		assert_eq!(stalk_node.position, Vec3::ZERO);
		assert_eq!(stalk_node.radius, 0.75);
		Ok(())
	}
}
