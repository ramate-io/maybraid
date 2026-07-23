//! Shared apron / rim-height authoring for watershed depressions and complexes.

use crate::noise::{n01_at, scale_noise_freq};
use bevy_math::Vec2;
use jersey_terrain_stamps::RegionNoise;

/// Target berm width outside wet support for lake / stream rims (world units).
pub const TARGET_RIM_WIDTH: f32 = 10.0;

/// Deterministic salts for [`WatershedApronParams::sample_noise`].
#[derive(Debug, Clone, Copy)]
pub struct ApronNoiseSalts {
	pub indent_amp: u32,
	pub indent_freq: u32,
	pub rim_amp: u32,
	pub rim_freq: u32,
}

impl ApronNoiseSalts {
	/// Lake leaf salt family (`0x1A7E_*`).
	pub const LAKE: Self = Self {
		indent_amp: 0x1A7E_A70A,
		indent_freq: 0x1A7E_AF7E,
		rim_amp: 0x1A7E_A17A,
		rim_freq: 0x1A7E_F7E9,
	};

	/// Stream leaf salt family (`0x57EA_*`).
	pub const STREAM: Self = Self {
		indent_amp: 0x57EA_A70A,
		indent_freq: 0x57EA_AF7E,
		rim_amp: 0x57EA_A17A,
		rim_freq: 0x57EA_F7E9,
	};
}

/// Per-leaf apron outline + add-only rim height knobs (shared by lake and stream).
#[derive(Debug, Clone, Copy)]
pub struct WatershedApronParams {
	/// Power for noise frequency scaling: `f ∝ (ref / radius)^power`.
	pub noise_freq_power: f32,
	/// Per-leaf apron boundary indent as a fraction of apron width (low).
	pub indent_frac_min: f32,
	/// Per-leaf apron boundary indent as a fraction of apron width (high).
	pub indent_frac_max: f32,
	/// Per-leaf apron boundary frequency low (at [`crate::noise::NOISE_FREQ_REF_RADIUS`]).
	pub freq_min: f32,
	/// Per-leaf apron boundary frequency high (at [`crate::noise::NOISE_FREQ_REF_RADIUS`]).
	pub freq_max: f32,
	/// Per-leaf rim height-noise amplitude low (world units).
	pub rim_height_amp_min: f32,
	/// Per-leaf rim height-noise amplitude high (world units).
	pub rim_height_amp_max: f32,
	/// Per-leaf rim height-noise frequency low (at [`crate::noise::NOISE_FREQ_REF_RADIUS`]).
	pub rim_height_freq_min: f32,
	/// Per-leaf rim height-noise frequency high (at [`crate::noise::NOISE_FREQ_REF_RADIUS`]).
	pub rim_height_freq_max: f32,
}

impl Default for WatershedApronParams {
	fn default() -> Self {
		Self {
			noise_freq_power: 0.5,
			indent_frac_min: 0.12,
			indent_frac_max: 0.40,
			freq_min: 0.005,
			freq_max: 0.012,
			rim_height_amp_min: 15.0,
			rim_height_amp_max: 120.0,
			rim_height_freq_min: 0.005,
			rim_height_freq_max: 0.012,
		}
	}
}

/// Drawn apron boundary + add-only rim height noises for one complex shelf.
#[derive(Debug, Clone)]
pub struct WatershedApronNoise {
	pub apron: RegionNoise,
	/// Indent amplitude used when expanding lake-style outer fade.
	pub apron_amp: f32,
	pub rim_height: RegionNoise,
}

impl WatershedApronParams {
	/// Shared lake/stream rim-height noise: stronger, longer-wavelength berm.
	pub fn with_visible_rim_bank(mut self) -> Self {
		self.rim_height_amp_min = 10.0;
		self.rim_height_amp_max = 20.0;
		self.rim_height_freq_min = 0.008;
		self.rim_height_freq_max = 0.02;
		self
	}

	/// Sample per-leaf apron + rim noises at `anchor`.
	///
	/// `apron_width_for_amp` is the width the indent fraction multiplies
	/// (lake: apron width; stream: apron−skirt band). `scale_radius` is the
	/// characteristic size for [`scale_noise_freq`].
	pub fn sample_noise(
		&self,
		seed: u32,
		anchor: Vec2,
		apron_width_for_amp: f32,
		scale_radius: f32,
		salts: ApronNoiseSalts,
	) -> WatershedApronNoise {
		let apron_frac_lo = self
			.indent_frac_min
			.min(self.indent_frac_max)
			.clamp(0.0, 0.5);
		let apron_frac_hi = self
			.indent_frac_min
			.max(self.indent_frac_max)
			.clamp(0.0, 0.5);
		let apron_freq_lo = self.freq_min.min(self.freq_max).max(0.0);
		let apron_freq_hi = self.freq_min.max(self.freq_max).max(0.0);
		let apron_indent_frac = apron_frac_lo
			+ (apron_frac_hi - apron_frac_lo) * n01_at(seed, salts.indent_amp, anchor);
		let apron_amp = (apron_width_for_amp.max(0.0) * apron_indent_frac).max(0.01);
		let apron_freq_authored = apron_freq_lo
			+ (apron_freq_hi - apron_freq_lo) * n01_at(seed, salts.indent_freq, anchor);
		let apron_freq =
			scale_noise_freq(apron_freq_authored, scale_radius, self.noise_freq_power);
		let apron = RegionNoise::from_seed(seed.wrapping_add(6), apron_freq, apron_amp);

		let rim_amp_lo = self.rim_height_amp_min.min(self.rim_height_amp_max).max(0.0);
		let rim_amp_hi = self.rim_height_amp_min.max(self.rim_height_amp_max).max(0.0);
		let rim_freq_lo = self.rim_height_freq_min.min(self.rim_height_freq_max).max(0.0);
		let rim_freq_hi = self.rim_height_freq_min.max(self.rim_height_freq_max).max(0.0);
		let rim_height_amp =
			rim_amp_lo + (rim_amp_hi - rim_amp_lo) * n01_at(seed, salts.rim_amp, anchor);
		let rim_freq_authored =
			rim_freq_lo + (rim_freq_hi - rim_freq_lo) * n01_at(seed, salts.rim_freq, anchor);
		let rim_height_freq =
			scale_noise_freq(rim_freq_authored, scale_radius, self.noise_freq_power);
		let rim_height =
			RegionNoise::from_seed(seed.wrapping_add(7), rim_height_freq, rim_height_amp);

		WatershedApronNoise {
			apron,
			apron_amp,
			rim_height,
		}
	}
}

/// Per-leaf depth scale: `depth * (lo + span * u01)`.
pub fn jittered_depth(seed: u32, salt: u32, anchor: Vec2, depth: f32, lo: f32, span: f32) -> f32 {
	depth * (lo + span * n01_at(seed, salt, anchor))
}
