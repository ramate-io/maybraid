//! Hydrology nodes: hydraulic primitive + rim/apron [`HydroParams`].
//!
//! See [`crate::WATERSHED_CORRECTION`](../WATERSHED_CORRECTION.md): nodes are the
//! source of truth for hydraulic geometry and parameters;
//! [`Self::max_correction_extent`] only governs spatial discoverability.
//!
//! Terrain correction blends per-node candidates by
//! [`Self::point_classification`] priority: carve → rim → apron.

use crate::primitive::parameters::CorrectionStage;
use crate::primitive::hydro::{
	smoothmax_fold, smoothmin_fold, HydroFootprint, HydroPrimitive, SURFACE_SMOOTHMIN_K,
};
use bevy_math::Vec2;
use procedural_common::Bounds2;

pub use crate::primitive::parameters::HydroParams;

/// One hydrology entity indexed by hydraulic support ⊕ correction extent.
#[derive(Debug, Clone)]
pub struct HydroNode {
	pub primitive: HydroPrimitive,
	pub params: HydroParams,
	/// Max distance beyond hydraulic support at which any correction pass may
	/// need this node. Indexing only — not the final apron profile.
	pub max_correction_extent: f32,
}

impl HydroNode {
	pub fn new(
		primitive: HydroPrimitive,
		params: HydroParams,
		max_correction_extent: f32,
	) -> Self {
		Self { primitive, params, max_correction_extent: max_correction_extent.max(0.0) }
	}

	/// Conservative pad for indexing / broadphase.
	pub fn index_pad(&self) -> f32 {
		self.max_correction_extent
			.max(self.params.correction_pad())
			.max(self.primitive.influence_pad)
			+ self.params.boundary_noise_amp()
	}

	/// Representative interior sample (ellipse center / reach midpoint).
	pub fn sample_point(&self) -> Vec2 {
		match &self.primitive.footprint {
			HydroFootprint::Reach(seg) => (seg.a + seg.b) * 0.5,
			HydroFootprint::Ellipse(e) => e.center,
		}
	}

	/// AABB of hydraulic support expanded by [`Self::index_pad`].
	pub fn correction_index_bounds(&self) -> Bounds2 {
		let (mn, mx) = self.primitive.aabb();
		let pad = self.index_pad();
		Bounds2 { min: mn - Vec2::splat(pad), max: mx + Vec2::splat(pad) }
	}

	/// Occupancy SDF, optionally warped by shore [`HydroParams::boundary_noise`].
	pub fn phi(&self, p: Vec2) -> f32 {
		let mut d = self.primitive.phi(p);
		if let Some(noise) = &self.params.boundary_noise {
			d += noise.sample_boundary(p);
		}
		d
	}

	pub fn surface_level(&self, p: Vec2) -> f32 {
		self.primitive.surface_and_bed(p).0
	}

	pub fn bed_level(&self, p: Vec2) -> f32 {
		self.primitive.surface_and_bed(p).1
	}

	/// Where `p` sits relative to this node's hydraulic support and bands.
	///
	/// - Carve: inside wet support (\(\phi \le 0\))
	/// - Rim: \(0 < \phi < r_{\mathrm{rim}}\)
	/// - Apron: \(r_{\mathrm{rim}} \le \phi < r_{\mathrm{rim}} + r_{\mathrm{apron}}\)
	pub fn point_classification(&self, p: Vec2) -> Option<CorrectionStage> {
		let d = self.phi(p);
		let rim_w = self.params.rim.width.max(0.0);
		let apron_w = self.params.apron.width.max(0.0);
		if d <= 0.0 {
			Some(CorrectionStage::Carve)
		} else if d < rim_w {
			Some(CorrectionStage::Rim)
		} else if d < rim_w + apron_w {
			Some(CorrectionStage::Apron)
		} else {
			None
		}
	}

	/// Absolute bowl bed, graded to the free surface at the noisy shore.
	///
	/// Geometric bed falloff can disagree with [`Self::phi`]'s boundary noise;
	/// this forces depth → 0 as \(\phi \to 0^-\) so the carve meets the same
	/// shoreline the rim uses.
	pub fn carve_candidate(&self, _elevation: f32, p: Vec2) -> f32 {
		let w = self.surface_level(p);
		let geo_bed = self.primitive.surface_and_bed(p).1;
		let geo_depth = (w - geo_bed).max(0.0);
		let phi = self.phi(p);
		if phi >= 0.0 {
			return w;
		}
		let edge = self.params.boundary_noise_amp().max(2.0);
		let t = (-phi / edge).clamp(0.0, 1.0);
		let keep = t * t * (3.0 - 2.0 * t);
		w - geo_depth * keep
	}

	/// Absolute bank across the rim band (classification uses noisy [`Self::phi`]).
	pub fn rim_candidate(&self, _elevation: f32, p: Vec2) -> f32 {
		self.params.bank_target(self.surface_level(p), p)
	}

	/// Grade from bank at the (noisy) rim edge toward identity at the apron outer.
	///
	/// Uses occupancy \(\phi\) so the grade tracks the same shoreline as carve/rim.
	pub fn apron_candidate(&self, elevation: f32, p: Vec2) -> f32 {
		let d = self.phi(p);
		let rim_w = self.params.rim.width.max(0.0);
		let apron_w = self.params.apron.width.max(1e-3);
		// Smoothstep grade: 0 at rim/apron seam (full bank), 1 at outer (terrain).
		let u = ((d - rim_w) / apron_w).clamp(0.0, 1.0);
		let fade = u * u * (3.0 - 2.0 * u);
		let bank = self.params.bank_target(self.surface_level(p), p);
		// Raise-only: never cut below the incoming elevation while grading out.
		(elevation * fade + bank * (1.0 - fade)).max(elevation)
	}

	/// Blend intersecting nodes at `p` by class priority: carve → rim → apron.
	///
	/// Classify first, then evaluate candidates only for the winning class:
	/// - Carve: soft-min of carve candidates
	/// - Rim: soft-max of rim candidates, floored by soft-min of rim-node surfaces
	/// - Apron: soft-max of apron candidates
	/// - Else: `elevation` unchanged
	pub fn blend_terrain_elevation(nodes: &[&Self], elevation: f32, p: Vec2) -> f32 {
		let mut carves: Vec<&Self> = Vec::new();
		let mut rims: Vec<&Self> = Vec::new();
		let mut aprons: Vec<&Self> = Vec::new();

		for node in nodes {
			match node.point_classification(p) {
				Some(CorrectionStage::Carve) => carves.push(node),
				Some(CorrectionStage::Rim) => {
					if carves.is_empty() {
						rims.push(node);
					}
				}
				Some(CorrectionStage::Apron) => {
					if carves.is_empty() && rims.is_empty() {
						aprons.push(node);
					}
				}
				None => {}
			}
		}

		if !carves.is_empty() {
			let vals: Vec<f32> = carves.iter().map(|n| n.carve_candidate(elevation, p)).collect();
			return smoothmin_fold(&vals, SURFACE_SMOOTHMIN_K);
		}
		if !rims.is_empty() {
			let mut surfaces = Vec::with_capacity(rims.len());
			let mut raised = Vec::with_capacity(rims.len());
			for n in &rims {
				surfaces.push(n.surface_level(p));
				raised.push(n.rim_candidate(elevation, p));
			}
			let water = smoothmin_fold(&surfaces, SURFACE_SMOOTHMIN_K);
			return smoothmax_fold(&raised, SURFACE_SMOOTHMIN_K).max(water);
		}
		if !aprons.is_empty() {
			let vals: Vec<f32> = aprons.iter().map(|n| n.apron_candidate(elevation, p)).collect();
			return smoothmax_fold(&vals, SURFACE_SMOOTHMIN_K);
		}
		elevation
	}

	/// Soft-min free surface over **Carve** nodes at `p` (same ownership as terrain carve).
	///
	/// Rim / apron do not contribute — keeps \(W\) coupled to the local carved bed.
	pub fn blend_surface_elevation(nodes: &[&Self], p: Vec2) -> Option<f32> {
		let mut surfaces = Vec::new();
		for node in nodes {
			if matches!(
				node.point_classification(p),
				Some(CorrectionStage::Carve)
			) {
				surfaces.push(node.surface_level(p));
			}
		}
		if surfaces.is_empty() {
			None
		} else {
			Some(smoothmin_fold(&surfaces, SURFACE_SMOOTHMIN_K))
		}
	}
}

/// Build reach-segment nodes from a graded polyline (one node per segment).
pub fn nodes_from_polyline(
	path: &[Vec2],
	levels: &[f32],
	half_width: f32,
	center_depth: f32,
	params: &HydroParams,
	max_correction_extent: f32,
) -> Vec<HydroNode> {
	use crate::primitive::hydro::{
		HydroElevation, HydroFootprint, ReachProfile, ReachSegment,
	};
	let n = path.len().min(levels.len());
	if n < 2 {
		return Vec::new();
	}
	let hw = half_width.max(1e-3);
	let depth = center_depth.max(0.25);
	let extent = max_correction_extent.max(params.correction_pad()).max(0.0);
	let mut out = Vec::with_capacity(n - 1);
	for i in 0..n - 1 {
		let a = path[i];
		let b = path[i + 1];
		if a.distance(b) <= 1e-4 {
			continue;
		}
		out.push(HydroNode::new(
			HydroPrimitive {
				footprint: HydroFootprint::Reach(ReachSegment {
					a,
					b,
					half_width: hw,
				}),
				elevation: HydroElevation::Reach(ReachProfile {
					surface_a: levels[i],
					surface_b: levels[i + 1],
					center_depth: depth,
				}),
				influence_pad: extent,
			},
			params.clone(),
			extent,
		));
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::primitive::hydro::{
		HydroElevation, HydroFootprint, ReachProfile, ReachSegment,
	};

	fn reach_node(half_width: f32) -> HydroNode {
		let mut params = HydroParams::default();
		params.rim.width = 4.0;
		params.apron.width = 8.0;
		HydroNode::new(
			HydroPrimitive {
				footprint: HydroFootprint::Reach(ReachSegment {
					a: Vec2::new(0.0, 0.0),
					b: Vec2::new(40.0, 0.0),
					half_width,
				}),
				elevation: HydroElevation::Reach(ReachProfile {
					surface_a: 30.0,
					surface_b: 30.0,
					center_depth: 3.0,
				}),
				influence_pad: 12.0,
			},
			params,
			12.0,
		)
	}

	#[test]
	fn classifies_carve_rim_apron_bands() -> anyhow::Result<()> {
		let node = reach_node(8.0);
		assert_eq!(node.point_classification(Vec2::new(20.0, 0.0)), Some(CorrectionStage::Carve));
		assert_eq!(node.point_classification(Vec2::new(20.0, 10.0)), Some(CorrectionStage::Rim));
		assert_eq!(node.point_classification(Vec2::new(20.0, 14.0)), Some(CorrectionStage::Apron));
		assert_eq!(node.point_classification(Vec2::new(20.0, 30.0)), None);
		Ok(())
	}

	#[test]
	fn blend_prefers_carve_over_rim() -> anyhow::Result<()> {
		let deep = reach_node(8.0);
		let mut shallow_params = HydroParams::default();
		shallow_params.rim.width = 4.0;
		shallow_params.apron.width = 8.0;
		let offset = HydroNode::new(
			HydroPrimitive {
				footprint: HydroFootprint::Reach(ReachSegment {
					a: Vec2::new(0.0, 6.0),
					b: Vec2::new(40.0, 6.0),
					half_width: 4.0,
				}),
				elevation: HydroElevation::Reach(ReachProfile {
					surface_a: 30.0,
					surface_b: 30.0,
					center_depth: 1.0,
				}),
				influence_pad: 12.0,
			},
			shallow_params,
			12.0,
		);
		// Inside deep channel; offset node may classify rim/apron nearby.
		let p = Vec2::new(20.0, 0.0);
		assert_eq!(deep.point_classification(p), Some(CorrectionStage::Carve));
		let h = HydroNode::blend_terrain_elevation(&[&deep, &offset], 40.0, p);
		assert!(h < 30.0 - 1.0, "carve should win: {h}");
		Ok(())
	}
}
