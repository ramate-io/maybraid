//! Authored hydrology nodes with conservative correction extents.
//!
//! See [`crate::WATERSHED_CORRECTION`](../WATERSHED_CORRECTION.md): nodes are the
//! source of truth for hydraulic geometry and parameters;
//! [`Self::max_correction_extent`] only governs spatial discoverability.

use crate::hydro::{HydroPrimitive, DEFAULT_RIM_UPLIFT_CAP};
use bevy_math::Vec2;
use jersey_terrain_stamps::RegionNoise;
use procedural_common::Bounds2;

/// Authored carve / rim / apron knobs on a [`HydrologyNode`].
///
/// First try: full parameters on the node; correction cells blend rim/apron.
/// Fallback (if blending fails): demote to carve fields + extent only.
#[derive(Debug, Clone)]
pub struct HydroParameters {
	/// Absolute shelf / rim anchor (lakes). When set, rim bank ≈ `shelf_anchor + rim_lift`.
	pub shelf_anchor: Option<f32>,
	pub rim_lift: f32,
	pub rim_width: f32,
	pub apron_width: f32,
	pub rim_height: RegionNoise,
	pub rim_uplift_cap: f32,
	pub shore_fade: f32,
	pub fill_undercut: f32,
}

impl Default for HydroParameters {
	fn default() -> Self {
		Self {
			shelf_anchor: None,
			rim_lift: 1.1,
			rim_width: 4.0,
			apron_width: 8.0,
			rim_height: RegionNoise::from_seed(0, 0.02, 0.0),
			rim_uplift_cap: DEFAULT_RIM_UPLIFT_CAP,
			shore_fade: 2.5,
			fill_undercut: 2.0,
		}
	}
}

impl HydroParameters {
	pub fn correction_pad(&self) -> f32 {
		(self.rim_width + self.apron_width).max(0.0)
	}

	/// Raise-only bank target at a sample given free-surface \(W\).
	pub fn bank_target(&self, water_surface: f32, p: Vec2) -> f32 {
		let base = self
			.shelf_anchor
			.unwrap_or(water_surface)
			+ self.rim_lift.max(0.0);
		let mut rim_noise = self.rim_height.sample_height(p).abs();
		rim_noise = rim_noise.min(self.rim_uplift_cap.max(0.0));
		base + rim_noise
	}
}

/// One authored hydrology entity indexed by hydraulic support ⊕ correction extent.
#[derive(Debug, Clone)]
pub struct HydrologyNode {
	pub primitive: HydroPrimitive,
	pub parameters: HydroParameters,
	/// Max distance beyond hydraulic support at which any correction pass may
	/// need this node. Indexing only — not the final apron profile.
	pub max_correction_extent: f32,
}

impl HydrologyNode {
	pub fn new(
		primitive: HydroPrimitive,
		parameters: HydroParameters,
		max_correction_extent: f32,
	) -> Self {
		Self {
			primitive,
			parameters,
			max_correction_extent: max_correction_extent.max(0.0),
		}
	}

	/// Conservative pad for indexing / broadphase.
	pub fn index_pad(&self) -> f32 {
		self.max_correction_extent
			.max(self.parameters.correction_pad())
			.max(self.primitive.influence_pad)
	}

	/// AABB of hydraulic support expanded by [`Self::index_pad`].
	pub fn correction_index_bounds(&self) -> Bounds2 {
		let (mn, mx) = self.primitive.aabb();
		let pad = self.index_pad();
		Bounds2 {
			min: mn - Vec2::splat(pad),
			max: mx + Vec2::splat(pad),
		}
	}
}

/// Build reach-segment nodes from a graded polyline (one node per segment).
pub fn nodes_from_polyline(
	path: &[Vec2],
	levels: &[f32],
	half_width: f32,
	center_depth: f32,
	parameters: &HydroParameters,
	max_correction_extent: f32,
) -> Vec<HydrologyNode> {
	use crate::hydro::{HydroElevation, HydroFootprint};
	let n = path.len().min(levels.len());
	if n < 2 {
		return Vec::new();
	}
	let hw = half_width.max(1e-3);
	let depth = center_depth.max(0.25);
	let extent = max_correction_extent
		.max(parameters.correction_pad())
		.max(0.0);
	let mut out = Vec::with_capacity(n - 1);
	for i in 0..n - 1 {
		let a = path[i];
		let b = path[i + 1];
		if a.distance(b) <= 1e-4 {
			continue;
		}
		out.push(HydrologyNode::new(
			HydroPrimitive {
				footprint: HydroFootprint::ReachSegment {
					a,
					b,
					half_width: hw,
				},
				elevation: HydroElevation::ReachProfile {
					surface_a: levels[i],
					surface_b: levels[i + 1],
					center_depth: depth,
				},
				influence_pad: extent,
			},
			parameters.clone(),
			extent,
		));
	}
	out
}
