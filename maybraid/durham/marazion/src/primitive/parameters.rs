//! Per-node rim / apron parameters for hydro correction.
//!
//! Every [`crate::primitive::node::HydroNode`] carries [`HydroParams`]. Leaf
//! stamps draw noise from the nested recipe fields, then set band widths /
//! [`HydroParams::rim_height`] before emit.

pub mod apron;
pub mod rim;

pub use apron::ApronParams;
pub use rim::RimParams;

use bevy_math::Vec2;
use jersey_terrain_stamps::RegionNoise;

/// Which watershed correction pass to apply at a sample.
///
/// Still used by [`crate::primitive::node::HydroNode::point_classification`] and
/// Durham carve / rim / apron stage cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorrectionStage {
	Carve,
	Rim,
	Apron,
}

/// Target berm width outside wet support for lake / stream rims (world units).
pub const TARGET_RIM_WIDTH: f32 = 25.0;

/// Default hard cap on add-only rim height noise.
pub const DEFAULT_RIM_UPLIFT_CAP: f32 = 1.5;

/// Per-node carve / rim / apron knobs.
///
/// Classification bands: carve \(\phi \le 0\), rim \(0 < \phi < r_{\mathrm{rim}}\),
/// apron \(r_{\mathrm{rim}} \le \phi < r_{\mathrm{rim}} + r_{\mathrm{apron}}\).
#[derive(Debug, Clone)]
pub struct HydroParams {
	pub rim: RimParams,
	pub apron: ApronParams,
	/// Drawn add-only rim height noise (baked at leaf construct).
	pub rim_height: RegionNoise,
	/// Optional shore outline: warps \(\phi\) via `φ += sample_boundary`.
	pub boundary_noise: Option<RegionNoise>,
}

impl Default for HydroParams {
	fn default() -> Self {
		Self {
			rim: RimParams::default(),
			apron: ApronParams::default(),
			rim_height: RegionNoise::from_seed(0, 0.02, 0.0),
			boundary_noise: None,
		}
	}
}

impl HydroParams {
	pub fn correction_pad(&self) -> f32 {
		(self.rim.width + self.apron.width).max(0.0)
	}

	/// Peak absolute amplitude of [`Self::boundary_noise`] (0 when unset).
	pub fn boundary_noise_amp(&self) -> f32 {
		self.boundary_noise
			.as_ref()
			.map(|n| n.noise.params().amplitude.abs())
			.unwrap_or(0.0)
	}

	/// Raise-only bank target at a sample given free-surface \(W\).
	pub fn bank_target(&self, water_surface: f32, p: Vec2) -> f32 {
		let base = self.rim.shelf_anchor.unwrap_or(water_surface) + self.rim.lift.max(0.0);
		let mut rim_noise = self.rim_height.sample_height(p).abs();
		rim_noise = rim_noise.min(self.rim.uplift_cap.max(0.0));
		base + rim_noise
	}
}

/// Shared rim / apron policy used when wrapping bare primitives (tests / helpers).
#[derive(Debug, Clone)]
pub struct ComplexParams {
	pub rim_lift: f32,
	pub rim_width: f32,
	pub apron_width: f32,
	pub rim_height: RegionNoise,
	pub rim_uplift_cap: f32,
}

impl Default for ComplexParams {
	fn default() -> Self {
		Self {
			rim_lift: 1.1,
			rim_width: 4.0,
			apron_width: 8.0,
			rim_height: RegionNoise::from_seed(0, 0.02, 0.0),
			rim_uplift_cap: DEFAULT_RIM_UPLIFT_CAP,
		}
	}
}

impl ComplexParams {
	pub fn with_rim_noise(mut self, noise: RegionNoise, cap: f32) -> Self {
		self.rim_height = noise;
		self.rim_uplift_cap = cap.max(0.0);
		self
	}

	/// Bake into per-node [`HydroParams`] (no shelf / shore noise).
	pub fn into_params(self) -> HydroParams {
		HydroParams {
			rim: RimParams {
				width: self.rim_width,
				lift: self.rim_lift,
				shelf_anchor: None,
				uplift_cap: self.rim_uplift_cap,
				..RimParams::default()
			},
			apron: ApronParams {
				width: self.apron_width,
				..ApronParams::default()
			},
			rim_height: self.rim_height,
			boundary_noise: None,
		}
	}
}
