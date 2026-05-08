//! Shared **FastNoise Lite** configuration and sampling.
//!
//! [`NoiseConfig`] owns an immutable [`Arc`]`<`[`FastNoiseLite`]`>` plus the [`NoiseParams`] snapshot used to build it.
//! **There are no mutators:** [`FastNoiseLite`] already holds seed, fractal / octave settings, and noise type.
//! Spatial **frequency**, output **amplitude**, and [`NoiseParams::domain_weights`] (default [`Vec3::ONE`]) live in
//! [`NoiseParams`] and are applied in [`NoiseConfig`]'s **`sample_*`** helpers (coordinate scaling + gain).
//! Use **`raw_*`** only when you want engine output with no coordinate or amplitude shaping from this layer.

use std::sync::Arc;

use bevy_math::Vec3;
use fastnoise_lite::{FastNoiseLite, FractalType, NoiseType};

/// Authoring parameters: spatial frequency (coordinate multiplier), output amplitude, fractal octaves, seed, noise kind,
/// plus per-axis domain scaling for 3D sampling.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoiseParams {
	/// Passed through to [`FastNoiseLite::with_seed`].
	pub seed: i32,
	/// Multiplies coordinates **before** sampling (see [`NoiseConfig::sample_1d`] … [`NoiseConfig::sample_4d`]).
	pub frequency: f32,
	/// Multiplies sampled noise after octaves / fractal processing (output gain).
	pub amplitude: f32,
	/// `1` → no fractal combine; `> 1` → FBm with this octave count on the underlying generator.
	pub octaves: u32,
	pub noise_type: NoiseType,
	/// Per-axis multipliers on **world position** before frequency scaling in [`NoiseConfig::sample_3d_world`].
	/// Does not affect [`NoiseConfig::sample_3d`] `(x, y, z)` or [`NoiseConfig::raw_3d`].
	pub domain_weights: Vec3,
}

impl Default for NoiseParams {
	fn default() -> Self {
		Self {
			seed: 1337,
			frequency: 1.0,
			amplitude: 1.0,
			octaves: 1,
			noise_type: NoiseType::OpenSimplex2,
			domain_weights: Vec3::ONE,
		}
	}
}

impl NoiseParams {
	/// Configure [`FastNoiseLite`] from these params (internal sampling frequency fixed to **1.0** so spatial scaling is entirely [`NoiseParams::frequency`]).
	pub fn build_fast_noise(&self) -> FastNoiseLite {
		let mut n = FastNoiseLite::with_seed(self.seed);
		n.set_noise_type(Some(self.noise_type));
		n.set_frequency(Some(1.0));
		if self.octaves <= 1 {
			n.set_fractal_type(Some(FractalType::None));
		} else {
			n.set_fractal_type(Some(FractalType::FBm));
			n.set_fractal_octaves(Some(self.octaves as i32));
		}
		n
	}

	pub fn build_generator(&self) -> Arc<FastNoiseLite> {
		Arc::new(self.build_fast_noise())
	}
}

/// Immutable handle: shared generator + params used to construct it.
///
/// There is no `&mut self` API: change behavior by building a new [`NoiseConfig::new`] with updated
/// [`NoiseParams`]. The underlying [`FastNoiseLite`] already encodes seed, noise type, and fractal
/// (octave) state; this type does not re-expose setters to avoid duplicating that configuration surface.
#[derive(Clone)]
pub struct NoiseConfig {
	generator: Arc<FastNoiseLite>,
	params: NoiseParams,
}

impl NoiseConfig {
	pub fn new(params: NoiseParams) -> Self {
		let generator = params.build_generator();
		Self { generator, params }
	}

	pub fn params(&self) -> &NoiseParams {
		&self.params
	}

	pub fn generator(&self) -> &FastNoiseLite {
		self.generator.as_ref()
	}

	fn gain(&self, raw: f32) -> f32 {
		raw * self.params.amplitude
	}

	// --- Raw (direct FastNoise Lite; no coordinate frequency or amplitude from [`NoiseParams`]) ---

	pub fn raw_1d(&self, x: f32) -> f32 {
		self.generator.get_noise_2d(x, 0.0)
	}

	pub fn raw_2d(&self, x: f32, y: f32) -> f32 {
		self.generator.get_noise_2d(x, y)
	}

	pub fn raw_3d(&self, x: f32, y: f32, z: f32) -> f32 {
		self.generator.get_noise_3d(x, y, z)
	}

	/// FastNoise Lite is 3D max; the fourth coordinate **w** shears **x** (common “time” hack).
	pub fn raw_4d(&self, x: f32, y: f32, z: f32, w: f32) -> f32 {
		self.generator.get_noise_3d(x + w, y, z)
	}

	// --- Sampled (frequency on coordinates, then amplitude on output) ---

	pub fn sample_1d(&self, x: f32) -> f32 {
		let f = self.params.frequency;
		self.gain(self.generator.get_noise_2d(x * f, 0.0))
	}

	pub fn sample_2d(&self, x: f32, y: f32) -> f32 {
		let f = self.params.frequency;
		self.gain(self.generator.get_noise_2d(x * f, y * f))
	}

	pub fn sample_3d(&self, x: f32, y: f32, z: f32) -> f32 {
		let f = self.params.frequency;
		self.gain(self.generator.get_noise_3d(x * f, y * f, z * f))
	}

	/// Sample using [`NoiseParams::domain_weights`] on **world position**, then frequency and amplitude.
	pub fn sample_3d_world(&self, position: Vec3) -> f32 {
		self.sample_3d_weighted(position, self.params.domain_weights)
	}

	/// Scale **world position** per axis before frequency scaling and sampling (explicit weights).
	///
	/// Prefer [`NoiseConfig::sample_3d_world`] and [`NoiseParams::domain_weights`] for authoring. Use **0** on
	/// an axis to remove it from the noise domain, or values in **(0, 1)** to soften an axis.
	pub fn sample_3d_weighted(&self, position: Vec3, domain_weights: Vec3) -> f32 {
		let q = position * domain_weights;
		self.sample_3d(q.x, q.y, q.z)
	}

	pub fn sample_4d(&self, x: f32, y: f32, z: f32, w: f32) -> f32 {
		let f = self.params.frequency;
		self.gain(self.generator.get_noise_3d((x + w) * f, y * f, z * f))
	}

	/// **[0, 1]** inputs mapped to **[-1, 1]** per axis before [`NoiseConfig::sample_1d`].
	pub fn sample_unit_1d(&self, u: f32) -> f32 {
		let x = u * 2.0 - 1.0;
		self.sample_1d(x)
	}

	pub fn sample_unit_2d(&self, u: f32, v: f32) -> f32 {
		self.sample_2d(u * 2.0 - 1.0, v * 2.0 - 1.0)
	}

	pub fn sample_unit_3d(&self, u: f32, v: f32, w: f32) -> f32 {
		self.sample_3d(u * 2.0 - 1.0, v * 2.0 - 1.0, w * 2.0 - 1.0)
	}

	pub fn sample_unit_4d(&self, u: f32, v: f32, w: f32, t: f32) -> f32 {
		self.sample_4d(u * 2.0 - 1.0, v * 2.0 - 1.0, w * 2.0 - 1.0, t * 2.0 - 1.0)
	}
}

/// Build [`NoiseParams`] / [`NoiseConfig`] from a scalar seed lane plus **`frequency`**, **`amplitude`**, **`octaves`**.
///
/// The first argument is cast with `as i32` for [`NoiseParams::seed`].
pub trait FromScalarNoise {
	fn from_scalar(seed_scalar: f32, frequency: f32, amplitude: f32, octaves: u32) -> Self;
}

impl FromScalarNoise for NoiseParams {
	fn from_scalar(seed_scalar: f32, frequency: f32, amplitude: f32, octaves: u32) -> Self {
		Self { seed: seed_scalar as i32, frequency, amplitude, octaves, ..Default::default() }
	}
}

impl FromScalarNoise for NoiseConfig {
	fn from_scalar(seed_scalar: f32, frequency: f32, amplitude: f32, octaves: u32) -> Self {
		Self::new(NoiseParams::from_scalar(seed_scalar, frequency, amplitude, octaves))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use bevy_math::Vec3;

	fn example_params() -> NoiseParams {
		NoiseParams {
			seed: 7,
			frequency: 5.0,
			amplitude: 0.05,
			octaves: 1,
			noise_type: NoiseType::Perlin,
			..Default::default()
		}
	}

	#[test]
	fn config_samples_are_deterministic() -> Result<()> {
		let p = example_params();
		let a = NoiseConfig::new(p);
		let b = NoiseConfig::new(p);
		let x = 0.2_f32;
		assert_eq!(a.sample_3d(x, x, x), b.sample_3d(x, x, x));
		Ok(())
	}

	#[test]
	fn octaves_change_output_when_fractal_on() -> Result<()> {
		let base = NoiseParams {
			seed: 1,
			frequency: 1.0,
			amplitude: 1.0,
			noise_type: NoiseType::Perlin,
			octaves: 1,
			..Default::default()
		};
		let single = NoiseParams { octaves: 1, ..base };
		let multi = NoiseParams { octaves: 3, ..base };
		let a = NoiseConfig::new(single);
		let b = NoiseConfig::new(multi);
		assert_ne!(a.raw_3d(0.31, 0.27, 0.19), b.raw_3d(0.31, 0.27, 0.19));
		Ok(())
	}

	#[test]
	fn from_scalar_noise_params_sets_seed() -> Result<()> {
		let p = NoiseParams::from_scalar(42.7, 2.0, 0.5, 2);
		assert_eq!(p.seed, 42);
		assert_eq!(p.frequency, 2.0);
		assert_eq!(p.amplitude, 0.5);
		assert_eq!(p.octaves, 2);
		Ok(())
	}

	#[test]
	fn sample_3d_weighted_zero_axis_ignores_that_axis() -> Result<()> {
		let p = example_params();
		let n = NoiseConfig::new(p);
		let w = Vec3::new(1.0, 1.0, 0.0);
		let a = n.sample_3d_weighted(Vec3::new(1.0, 2.0, 3.0), w);
		let b = n.sample_3d_weighted(Vec3::new(1.0, 2.0, 99.0), w);
		assert_eq!(a, b);
		Ok(())
	}

	#[test]
	fn sample_3d_world_uses_params_domain_weights() -> Result<()> {
		let mut p = example_params();
		p.domain_weights = Vec3::new(1.0, 1.0, 0.0);
		let n = NoiseConfig::new(p);
		let a = n.sample_3d_world(Vec3::new(1.0, 2.0, 3.0));
		let b = n.sample_3d_world(Vec3::new(1.0, 2.0, 99.0));
		assert_eq!(a, b);
		Ok(())
	}
}
