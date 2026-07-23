//! Multi-stream band composition for [`crate::complex::WatershedDepressionComplex`].
//!
//! Soft-voronoi policies (see plan / RFC-127 §3.1.3.4):
//! - **Thalweg / channel** — high-\(\gamma\) ownership (+ min-floor carve safety)
//! - **Rim / bank** — high-\(\gamma\) owned raise-only grade
//! - **Outer apron** — low-\(\gamma\) distance blend of bank targets, one apply
//! - **Water \(W\)** — one owned graded field; fill half-width ≤ channel

use crate::fill::{WaterFill, WaterGradePart, WaterSurface};
use crate::stream::{bank_levels, bed_levels};
use bevy_math::Vec2;
use jersey_terrain_stamps::{
	JerseyModulation, MultiPolylineBandModulation, MultiPolylineBandPart,
	MultiPolylineOffsetModulation, MultiPolylineOffsetPart, PolylineRegion, Region2D,
	RegionNoise,
};

/// Soft-voronoi \(\gamma\) for hard ownership (thalweg / rim / \(W\)).
pub const OWNERSHIP_GAMMA_HARD: f32 = 6.0;
/// Soft-voronoi \(\gamma\) for apron target blending.
pub const OWNERSHIP_GAMMA_SOFT: f32 = 0.35;
/// Default hard cap on shared add-only rim height noise (world units).
/// Keep modest — bank grade already sits at \(W+\mathrm{rim\_lift}\); large add-only
/// rim noise reads as knife ridges beside over-deep channels.
pub const DEFAULT_RIM_UPLIFT_CAP: f32 = 1.5;

/// One stream corridor's band geometry + water grade (composer input).
#[derive(Debug, Clone)]
pub struct StreamBandPart {
	pub path: Vec<Vec2>,
	/// Per-vertex water surface elevations along [`Self::path`].
	pub levels: Vec<f32>,
	pub half_width: f32,
	pub thalweg_half: f32,
	pub skirt_half: f32,
	pub apron_half: f32,
	pub node_blend: f32,
	pub freeboard: f32,
	pub rim_lift: f32,
	pub depth: f32,
	pub shore_indent_noise: Option<RegionNoise>,
	pub depth_noise: Option<RegionNoise>,
	pub apron_boundary_noise: Option<RegionNoise>,
}

/// Shared multi-stream shelf: apron / channel / thalweg / one fill.
#[derive(Debug, Clone)]
pub struct StreamBandComposer {
	pub parts: Vec<StreamBandPart>,
	/// Shared add-only rim height noise (applied once on the apron pass).
	pub rim_height: RegionNoise,
	/// Hard uplift budget for [`Self::rim_height`] (`+|sample|` capped).
	pub rim_uplift_cap: f32,
	pub ownership_gamma_hard: f32,
	pub ownership_gamma_soft: f32,
	pub fill_undercut: f32,
	pub shore_fade: f32,
}

/// Products of [`StreamBandComposer::compose`].
#[derive(Debug, Clone)]
pub struct ComposedStreamBands {
	pub modulations: Vec<JerseyModulation>,
	pub fill: Option<WaterFill>,
	pub wet_union: Option<Region2D>,
}

impl StreamBandComposer {
	pub fn new(parts: Vec<StreamBandPart>, rim_height: RegionNoise) -> Self {
		Self {
			parts,
			rim_height,
			rim_uplift_cap: DEFAULT_RIM_UPLIFT_CAP,
			ownership_gamma_hard: OWNERSHIP_GAMMA_HARD,
			ownership_gamma_soft: OWNERSHIP_GAMMA_SOFT,
			fill_undercut: 2.0,
			shore_fade: 2.0,
		}
	}

	pub fn with_rim_uplift_cap(mut self, cap: f32) -> Self {
		self.rim_uplift_cap = cap.max(0.0);
		self
	}

	pub fn with_fill_undercut(mut self, undercut: f32) -> Self {
		self.fill_undercut = undercut.max(0.0);
		self
	}

	pub fn with_shore_fade(mut self, fade: f32) -> Self {
		self.shore_fade = fade.max(0.25);
		self
	}

	/// Emit apron → channel → thalweg modulations plus one owned \(W\) fill.
	pub fn compose(&self) -> ComposedStreamBands {
		if self.parts.is_empty() {
			return ComposedStreamBands {
				modulations: Vec::new(),
				fill: None,
				wet_union: None,
			};
		}

		let mut apron_parts = Vec::with_capacity(self.parts.len());
		let mut channel_parts = Vec::with_capacity(self.parts.len());
		let mut thalweg_parts = Vec::with_capacity(self.parts.len());
		let mut wet_cores = Vec::with_capacity(self.parts.len());
		let mut grade_parts = Vec::with_capacity(self.parts.len());

		for part in &self.parts {
			if part.path.len() < 2 || part.levels.len() < 2 {
				continue;
			}
			let banks = bank_levels(&part.levels, part.rim_lift);
			let beds = bed_levels(&part.levels, part.freeboard);
			let apron_fade = ((part.apron_half - part.skirt_half) * 0.85).max(1.0);
			let channel_fade = (part.half_width * 0.15)
				.max(0.35)
				.min(part.half_width * 0.35);
			let thalweg_fade = (part.thalweg_half * 0.35).max(0.4);

			let mut apron = MultiPolylineBandPart::new(
				part.path.clone(),
				banks,
				part.apron_half,
				apron_fade,
			)
			.with_node_blend(part.node_blend);
			if let Some(n) = part.apron_boundary_noise.clone() {
				apron = apron.with_boundary_noise(n);
			}
			apron_parts.push(apron);

			let mut channel = MultiPolylineBandPart::new(
				part.path.clone(),
				beds,
				part.half_width,
				channel_fade,
			)
			.with_node_blend(part.node_blend);
			if let Some(n) = part.shore_indent_noise.clone() {
				channel = channel.with_boundary_noise(n);
			}
			channel_parts.push(channel);

			let mut thalweg = MultiPolylineOffsetPart::new(
				part.path.clone(),
				part.thalweg_half,
				thalweg_fade,
				-part.depth.max(0.0),
			);
			if let Some(n) = part.shore_indent_noise.clone() {
				thalweg = thalweg.with_boundary_noise(n);
			}
			thalweg_parts.push(thalweg);

			wet_cores.push(Region2D::Polyline(PolylineRegion::new(
				part.path.clone(),
				part.half_width,
			)));
			grade_parts.push(WaterGradePart {
				path: part.path.clone(),
				levels: part.levels.clone(),
				node_blend: part.node_blend,
			});
		}

		if apron_parts.is_empty() {
			return ComposedStreamBands {
				modulations: Vec::new(),
				fill: None,
				wet_union: None,
			};
		}

		let apron = JerseyModulation::MultiPolylineBand(
			MultiPolylineBandModulation::new(apron_parts, self.ownership_gamma_soft)
				.raise_only()
				.with_height_noise_add_only(self.rim_height.clone())
				.with_height_noise_cap(self.rim_uplift_cap),
		);
		// Hard ownership only (no min-compose): a lower neighbor's bed winning
		// inside an uphill corridor is what dug "ridge glue" canyons.
		let channel = JerseyModulation::MultiPolylineBand(
			MultiPolylineBandModulation::new(channel_parts, self.ownership_gamma_hard)
				.depression_only()
				.with_min_compose(false),
		);
		let mut thalweg_mod =
			MultiPolylineOffsetModulation::new(thalweg_parts, self.ownership_gamma_hard)
				.winner_take_all();
		// Use the first part's depth noise if present (shared-ish bed roughness).
		if let Some(dn) = self.parts.iter().find_map(|p| p.depth_noise.clone()) {
			thalweg_mod = thalweg_mod.with_height_noise(dn);
		}
		let thalweg = JerseyModulation::MultiPolylineOffset(thalweg_mod);

		let wet_union = match wet_cores.len() {
			0 => None,
			1 => wet_cores.pop(),
			_ => Some(Region2D::union(wet_cores)),
		};

		// Fill ⊆ carve: union of channel stadiums (not liberal fill ribbons).
		let fill_region = wet_union.clone().unwrap_or_else(|| {
			Region2D::Polyline(PolylineRegion::new(
				self.parts[0].path.clone(),
				self.parts[0].half_width,
			))
		});
		let fill = WaterFill {
			region: fill_region,
			inner_radius: 0.0,
			outer_radius: self.shore_fade,
			noise: None,
			surface: WaterSurface::OwnedGraded {
				parts: grade_parts,
				ownership_gamma: self.ownership_gamma_hard,
			},
			terrain_undercut: self.fill_undercut,
		};

		ComposedStreamBands {
			modulations: vec![apron, channel, thalweg],
			fill: Some(fill),
			wet_union,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn parallel_parts() -> Vec<StreamBandPart> {
		vec![
			StreamBandPart {
				path: vec![Vec2::new(0.0, 0.0), Vec2::new(80.0, 0.0)],
				levels: vec![50.0, 48.0],
				half_width: 6.0,
				thalweg_half: 2.0,
				skirt_half: 10.0,
				apron_half: 16.0,
				node_blend: 4.0,
				freeboard: 2.0,
				rim_lift: 1.0,
				depth: 5.0,
				shore_indent_noise: None,
				depth_noise: None,
				apron_boundary_noise: None,
			},
			StreamBandPart {
				path: vec![Vec2::new(0.0, 14.0), Vec2::new(80.0, 14.0)],
				levels: vec![49.0, 47.0],
				half_width: 6.0,
				thalweg_half: 2.0,
				skirt_half: 10.0,
				apron_half: 16.0,
				node_blend: 4.0,
				freeboard: 2.0,
				rim_lift: 1.0,
				depth: 5.0,
				shore_indent_noise: None,
				depth_noise: None,
				apron_boundary_noise: None,
			},
		]
	}

	#[test]
	fn compose_emits_three_mods_and_one_fill() -> anyhow::Result<()> {
		let composed = StreamBandComposer::new(
			parallel_parts(),
			RegionNoise::from_seed(1, 0.05, 2.0),
		)
		.with_rim_uplift_cap(3.0)
		.compose();
		assert_eq!(composed.modulations.len(), 3);
		assert!(composed.fill.is_some());
		assert!(composed.wet_union.is_some());
		Ok(())
	}

	#[test]
	fn apron_blend_has_no_hard_step_on_medial_axis() -> anyhow::Result<()> {
		let composed = StreamBandComposer::new(
			parallel_parts(),
			RegionNoise::from_seed(2, 0.02, 0.0),
		)
		.with_rim_uplift_cap(0.0)
		.compose();
		let apron = &composed.modulations[0];
		let base = 20.0;
		let mut samples = Vec::new();
		for i in 0..11 {
			let z = i as f32 * 1.4;
			let h = apron.modify_elevation(base, 40.0, z);
			samples.push(h);
		}
		let mut max_step: f32 = 0.0;
		for w in samples.windows(2) {
			max_step = max_step.max((w[1] - w[0]).abs());
		}
		assert!(
			max_step < 4.0,
			"apron medial-axis steps should be soft, max_step={max_step}, samples={samples:?}"
		);
		Ok(())
	}
}
