//! **Stalk anchor rings** and projection policy for **Sope's Banyan** ([RFC-183 §3.1.7.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/06-sope-s-banyan/README.md), [#252](https://github.com/ramate-io/maybraid/issues/252)).
//!
//! # Intent
//!
//! Anchoring follows [§3.1.3 Ball-stick anchors](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/03-ball-stick-anchors/README.md): positions, initial rays, bias directions, and local scale for each canopy chain, usually emitted from the **stalk radial centroid** so limbs read as emerging from trunk mass.
//!
//! Compared to [Honu Banyan](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/05-honu-banyan/README.md), Sope's places rings **much lower**: radial work begins around **40%** of total height \(z_{\min} \approx 0.40 H\), extending to ~**90%** with **5–7** rings at spacing ~**0.08 H**, **6–8** anchors per ring. **Projection length** uses a **vase-like widening**: compact near the bottom of the anchor band, then **`mix` toward longer projections** with normalized height \(u\) (RFC uses `sqrt(u)` between min/max lengths ~**0.25 H** and **0.70 H**).
//!
//! # Same ball-stick graph as the stalk
//!
//! Sope's Banyan composes a **[`StrictStalk`](super::strict_stalk::StrictStalk)** (straight vertical centroid) **with** ring seeds at [`StrictStalk::centroid_at_height_fraction`]. Output is deterministic; a composing type can perturb or drop anchors later.

use std::f32::consts::TAU;

use bevy_math::Vec3;

use super::strict_stalk::StrictStalk;
use super::Anchors;
use crate::{BallStickNode, Hysteresis, SopesBanyanChainRule};

/// RFC-style ring band and vase profile over [`StrictStalk::height`].
#[derive(Clone, Debug, PartialEq)]
pub struct SopesBanyanAnchors {
	/// Vertical extent and base for ring placement.
	pub stalk: StrictStalk,
	/// First ring height as a fraction of [`StrictStalk::height`] above [`StrictStalk::base_anchor`] (RFC ~0.4).
	pub first_ring_unit_height: f32,
	/// Last ring height fraction (RFC ~0.9).
	pub last_ring_unit_height: f32,
	pub ring_count: u32,
	pub anchors_per_ring: u32,
	/// Vase mix endpoints as fractions of stalk height: `length ≈ H * mix(min, max, sqrt(u))` with ring index `u`.
	pub projection_min_fraction_of_height: f32,
	pub projection_max_fraction_of_height: f32,
	/// [`Hysteresis::max_depth`] at the first ring (RFC limb depth ~5 segments).
	pub max_depth_first_ring: usize,
	/// [`Hysteresis::max_depth`] at the last ring (~8).
	pub max_depth_last_ring: usize,
}

impl Default for SopesBanyanAnchors {
	fn default() -> Self {
		Self {
			stalk: StrictStalk {
				height: 10.0,
				base_anchor: Vec3::ZERO,
				base_radius: 0.75,
			},
			first_ring_unit_height: 0.40,
			last_ring_unit_height: 0.90,
			ring_count: 6,
			anchors_per_ring: 7,
			projection_min_fraction_of_height: 0.25,
			projection_max_fraction_of_height: 0.70,
			max_depth_first_ring: 5,
			max_depth_last_ring: 8,
		}
	}
}

impl SopesBanyanAnchors {
	/// Normalized index along rings in `[0, 1]` (0 = lowest ring, 1 = highest).
	fn ring_mix_u(ring_index: u32, ring_count: u32) -> f32 {
		if ring_count <= 1 {
			return 0.0;
		}
		(ring_index as f32 / (ring_count - 1) as f32).clamp(0.0, 1.0)
	}

	/// Vase-shaped projection length for this ring (world units).
	fn projection_length(&self, u: f32) -> f32 {
		let h = self.stalk.height.max(1e-6);
		let t = u.sqrt();
		let f = self.projection_min_fraction_of_height
			+ (self.projection_max_fraction_of_height - self.projection_min_fraction_of_height) * t;
		h * f
	}

	fn max_depth_for_ring(&self, u: f32) -> usize {
		let a = self.max_depth_first_ring as f32;
		let b = self.max_depth_last_ring as f32;
		(a + (b - a) * u).round().max(0.0) as usize
	}

	fn height_fraction_for_ring(&self, u: f32) -> f32 {
		let a = self.first_ring_unit_height;
		let b = self.last_ring_unit_height;
		a + (b - a) * u
	}
}

impl Anchors for SopesBanyanAnchors {
	fn anchors(&self) -> Vec<(BallStickNode, Hysteresis)> {
		let mut out = Vec::new();
		let n = self.ring_count.max(1);
		let k = self.anchors_per_ring.max(1);
		let radial_eps = (self.stalk.base_radius * 0.08).max(1e-4);

		for r in 0..n {
			let u = Self::ring_mix_u(r, n);
			let y_frac = self.height_fraction_for_ring(u);
			let proj = self.projection_length(u);
			let max_depth = self.max_depth_for_ring(u);

			for i in 0..k {
				let theta = TAU * (i as f32) / (k as f32);
				let offset = Vec3::new(theta.cos(), 0.0, theta.sin()) * radial_eps;
				let pos = self.stalk.centroid_at_height_fraction(y_frac) + offset;

				let mut h = SopesBanyanChainRule::seed_hysteresis(pos, max_depth);
				let lo = proj * 0.97;
				let hi = proj * 1.03;
				h.length = lo..hi;

				let radius = (self.stalk.base_radius * (0.02 + 0.03 * u)).max(1e-4);
				out.push((BallStickNode::new(pos, radius), h));
			}
		}

		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn vase_projection_grows_with_ring_height() {
		let a = SopesBanyanAnchors {
			stalk: StrictStalk {
				height: 10.0,
				base_anchor: Vec3::ZERO,
				base_radius: 0.5,
			},
			ring_count: 5,
			..Default::default()
		};
		let l0 = a.projection_length(SopesBanyanAnchors::ring_mix_u(0, 5));
		let l1 = a.projection_length(SopesBanyanAnchors::ring_mix_u(4, 5));
		assert!(l1 > l0, "upper rings should get longer vase projections");

		let d0 = a.max_depth_for_ring(0.0);
		let d1 = a.max_depth_for_ring(1.0);
		assert!(d1 >= d0, "segment budget should not shrink upward by default");
	}

	#[test]
	fn anchors_count_matches_rings_times_spokes() {
		let a = SopesBanyanAnchors {
			ring_count: 3,
			anchors_per_ring: 4,
			..Default::default()
		};
		assert_eq!(a.anchors().len(), 12);
	}
}
