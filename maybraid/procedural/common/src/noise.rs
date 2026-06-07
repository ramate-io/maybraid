//! Shared **FastNoise Lite** configuration and sampling.
//!
//! [`NoiseConfig`] owns an immutable [`Arc`]`<`[`FastNoiseLite`]`>` plus the [`NoiseParams`] snapshot used to build it.
//! **There are no mutators:** [`FastNoiseLite`] already holds seed, fractal / octave settings, and noise type.
//! Spatial **frequency**, output **amplitude**, and [`NoiseParams::domain_weights`] (default [`Vec3::ONE`]) live in
//! [`NoiseParams`] and are applied in [`NoiseConfig`]'s **`sample_*`** helpers (coordinate scaling + gain).
//! Use **`raw_*`** only when you want engine output with no coordinate or amplitude shaping from this layer.

use std::sync::Arc;

use bevy_math::{Vec2, Vec3};
use fastnoise_lite::{FastNoiseLite, FractalType, NoiseType};

/// Parse [`NoiseType`] from CLI / config strings (kebab-case or snake_case).
pub fn noise_type_from_str(s: &str) -> Result<NoiseType, String> {
	match s.trim().to_ascii_lowercase().replace('_', "-").as_str() {
		"perlin" => Ok(NoiseType::Perlin),
		"open-simplex-2" | "open-simplex2" => Ok(NoiseType::OpenSimplex2),
		"open-simplex-2s" | "open-simplex2s" => Ok(NoiseType::OpenSimplex2S),
		"cellular" => Ok(NoiseType::Cellular),
		"value-cubic" | "valuecubic" => Ok(NoiseType::ValueCubic),
		"value" => Ok(NoiseType::Value),
		other => Err(format!("unknown noise type {other:?} (expected perlin, open-simplex-2, …)")),
	}
}

/// Parse comma-separated domain weights `x,y,z` for [`NoiseParams::domain_weights`].
pub fn domain_weights_from_str(s: &str) -> Result<Vec3, String> {
	let parts: Vec<&str> = s.split(',').map(str::trim).filter(|p| !p.is_empty()).collect();
	if parts.len() != 3 {
		return Err(format!("expected three comma-separated floats, got {s:?}"));
	}
	let x: f32 = parts[0].parse::<f32>().map_err(|e| e.to_string())?;
	let y: f32 = parts[1].parse::<f32>().map_err(|e| e.to_string())?;
	let z: f32 = parts[2].parse::<f32>().map_err(|e| e.to_string())?;
	Ok(Vec3::new(x, y, z))
}

#[cfg(feature = "serde")]
mod noise_type_serde {
	use super::{noise_type_from_str, NoiseType};
	use serde::{Deserialize, Deserializer, Serializer};

	pub fn serialize<S>(v: &NoiseType, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let name = match v {
			NoiseType::OpenSimplex2 => "open-simplex-2",
			NoiseType::OpenSimplex2S => "open-simplex-2s",
			NoiseType::Cellular => "cellular",
			NoiseType::Perlin => "perlin",
			NoiseType::ValueCubic => "value-cubic",
			NoiseType::Value => "value",
		};
		serializer.serialize_str(name)
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<NoiseType, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		noise_type_from_str(&s).map_err(serde::de::Error::custom)
	}
}

/// Authoring parameters: spatial frequency (coordinate multiplier), output amplitude, fractal octaves, seed, noise kind,
/// plus per-axis domain scaling for 3D sampling.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NoiseParams {
	/// Passed through to [`FastNoiseLite::with_seed`].
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1337))]
	pub seed: i32,
	/// Multiplies coordinates **before** sampling (see [`NoiseConfig::sample_1d`] … [`NoiseConfig::sample_4d`]).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1.0))]
	pub frequency: f32,
	/// Multiplies sampled noise after octaves / fractal processing (output gain).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1.0))]
	pub amplitude: f32,
	/// `1` → no fractal combine; `> 1` → FBm with this octave count on the underlying generator.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1))]
	pub octaves: u32,
	#[cfg_attr(feature = "serde", serde(with = "noise_type_serde"))]
	#[cfg_attr(
		feature = "clap",
		arg(
			long,
			value_parser = noise_type_from_str,
			default_value = "open-simplex-2"
		)
	)]
	pub noise_type: NoiseType,
	/// Per-axis multipliers on **world position** before frequency scaling in [`NoiseConfig::sample_3d_world`].
	/// Does not affect [`NoiseConfig::sample_3d`] `(x, y, z)` or [`NoiseConfig::raw_3d`].
	#[cfg_attr(
		feature = "clap",
		arg(
			long,
			value_parser = domain_weights_from_str,
			default_value = "1,1,1"
		)
	)]
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

	pub fn with_frequency(mut self, frequency: f32) -> Self {
		self.params.frequency = frequency;
		self
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

	pub fn sample_2d_world(&self, position: Vec2) -> f32 {
		self.sample_2d_weighted(position, self.params.domain_weights.truncate())
	}

	pub fn sample_2d_weighted(&self, position: Vec2, domain_weights: Vec2) -> f32 {
		let q = position * domain_weights;
		self.sample_2d(q.x, q.y)
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

	/// Sample 4D and remap from `[-1, 1]` to `[0, 1]`.
	pub fn sample_unit01_4d(&self, x: f32, y: f32, z: f32, w: f32) -> f32 {
		(self.sample_4d(x, y, z, w) * 0.5 + 0.5).clamp(0.0, 1.0)
	}

	/// Sample 4D and clamp to signed unit range.
	pub fn sample_signed_4d(&self, x: f32, y: f32, z: f32, w: f32) -> f32 {
		self.sample_4d(x, y, z, w).clamp(-1.0, 1.0)
	}

	/// Deterministically map a 4D sample to a half-open integer range `[lo, hi)`.
	pub fn sample_range_usize_4d(
		&self,
		lo: usize,
		hi: usize,
		x: f32,
		y: f32,
		z: f32,
		w: f32,
	) -> usize {
		if hi <= lo {
			return lo;
		}
		let u = self.sample_unit01_4d(x, y, z, w);
		let span = hi - lo;
		lo + ((u * span as f32).floor() as usize).min(span - 1)
	}

	/// Deterministically map a 4D sample to a float range.
	pub fn sample_range_f32_4d(&self, lo: f32, hi: f32, x: f32, y: f32, z: f32, w: f32) -> f32 {
		if hi <= lo {
			return lo;
		}
		let u = self.sample_unit01_4d(x, y, z, w);
		lo + u * (hi - lo)
	}

	/// World-space input mapped to [0, 1] range.
	pub fn sample_unit_1d(&self, x: f32) -> f32 {
		(self.sample_1d(x) + 1.0) * 0.5
	}

	pub fn sample_unit_2d(&self, x: f32, y: f32) -> f32 {
		(self.sample_2d(x, y) + 1.0) * 0.5
	}

	pub fn sample_unit_3d(&self, x: f32, y: f32, z: f32) -> f32 {
		(self.sample_3d(x, y, z) + 1.0) * 0.5
	}

	pub fn sample_unit_4d(&self, x: f32, y: f32, z: f32, w: f32) -> f32 {
		(self.sample_4d(x, y, z, w) + 1.0) * 0.5
	}
}

/// Build a value from procedural [`NoiseParams`] (seed lane, frequency, amplitude, octaves, …).
pub trait FromScalarNoise {
	fn from_scalar(noise: NoiseParams) -> Self;
}

pub trait BuildWithNoise<T> {
	/// Builds a resultant type from the noise params.
	fn build_with_noise(&self, noise: NoiseParams) -> T;
}

pub trait WithNoise {
	/// Reconstructs an instance of self with give noise params.
	fn with_noise(&self, noise: NoiseParams) -> Self;
}

/// Implements `BuildWithNoise` for types that implement `WithNoise`.
impl<T> BuildWithNoise<T> for T
where
	T: WithNoise,
{
	fn build_with_noise(&self, noise: NoiseParams) -> T {
		self.with_noise(noise)
	}
}

/// Override structural noise params on a composable builder.
pub trait SetNoiseParams {
	fn with_noise_params(self, params: NoiseParams) -> Self;
}

/// Parse compact `seed,frequency,amplitude,octaves` tuples into [`NoiseParams`].
pub fn noise_params_from_scalar_str(s: &str) -> Result<NoiseParams, String> {
	let parts: Vec<&str> = s.split(',').map(str::trim).filter(|p| !p.is_empty()).collect();
	if parts.len() != 4 {
		return Err(format!("expected seed,frequency,amplitude,octaves, got {s:?}"));
	}

	let seed_scalar = parts[0].parse::<f32>().map_err(|e| e.to_string())?;
	let frequency = parts[1].parse::<f32>().map_err(|e| e.to_string())?;
	let amplitude = parts[2].parse::<f32>().map_err(|e| e.to_string())?;
	let octaves = parts[3].parse::<u32>().map_err(|e| e.to_string())?;
	Ok(NoiseParams::from_scalar(seed_scalar, frequency, amplitude, octaves))
}

impl FromScalarNoise for NoiseParams {
	fn from_scalar(noise: NoiseParams) -> Self {
		noise
	}
}

impl NoiseParams {
	/// Build params from a scalar seed lane plus **`frequency`**, **`amplitude`**, **`octaves`**.
	///
	/// The seed lane is cast with `as i32` for [`Self::seed`].
	pub fn from_scalar(seed_scalar: f32, frequency: f32, amplitude: f32, octaves: u32) -> Self {
		Self { seed: seed_scalar as i32, frequency, amplitude, octaves, ..Default::default() }
	}

	pub fn with_seed(mut self, seed: i32) -> Self {
		self.seed = seed;
		self
	}

	pub fn build_scalar<T: FromScalarNoise>(&self) -> T {
		T::from_scalar(*self)
	}
}

impl SetNoiseParams for NoiseParams {
	fn with_noise_params(self, params: NoiseParams) -> Self {
		params
	}
}

impl FromScalarNoise for NoiseConfig {
	fn from_scalar(noise: NoiseParams) -> Self {
		Self::new(noise)
	}
}

impl SetNoiseParams for NoiseConfig {
	fn with_noise_params(self, params: NoiseParams) -> Self {
		Self::new(params)
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
