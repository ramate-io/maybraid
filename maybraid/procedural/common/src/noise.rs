//! Shared **FastNoise Lite** configuration and sampling.
//!
//! [`NoiseConfig`] owns an immutable [`Arc`]`<`[`FastNoiseLite`]`>` plus the [`NoiseParams`] snapshot used to build it.
//! **There are no mutators:** [`FastNoiseLite`] already holds seed, fractal / octave settings, and noise type.
//! Spatial **frequency** and output **amplitude** live in [`NoiseParams`] and are applied in [`NoiseConfig`]'s
//! **`sample_*`** helpers (coordinate scaling + gain). Use [`NoiseConfig::raw_3d`] when you want engine output
//! with no coordinate or amplitude shaping from this layer.
//!
//! Geometry is generated in tree-local space, so samples are taken over local positions; callers
//! that want per-instance variation supply a different [`NoiseParams::seed`] (or shift the **w**
//! salt lane of the `*_4d` helpers) rather than offsetting the sampled positions.

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

/// Authoring parameters: spatial frequency (coordinate multiplier), output amplitude, fractal octaves, seed, noise kind.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NoiseParams {
	/// Passed through to [`FastNoiseLite::with_seed`].
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1337))]
	pub seed: i32,
	/// Multiplies coordinates **before** sampling.
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
}

impl Default for NoiseParams {
	fn default() -> Self {
		Self {
			seed: 1337,
			frequency: 1.0,
			amplitude: 1.0,
			octaves: 1,
			noise_type: NoiseType::OpenSimplex2,
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
		let generator = Arc::new(params.build_fast_noise());
		Self { generator, params }
	}

	pub fn with_frequency(mut self, frequency: f32) -> Self {
		self.params.frequency = frequency;
		self
	}

	pub fn params(&self) -> &NoiseParams {
		&self.params
	}

	fn gain(&self, raw: f32) -> f32 {
		raw * self.params.amplitude
	}

	/// Direct FastNoise Lite output: no coordinate frequency or amplitude from [`NoiseParams`].
	pub fn raw_3d(&self, x: f32, y: f32, z: f32) -> f32 {
		self.generator.get_noise_3d(x, y, z)
	}

	// --- Sampled (frequency on coordinates, then amplitude on output) ---

	fn sample_1d(&self, x: f32) -> f32 {
		let f = self.params.frequency;
		self.gain(self.generator.get_noise_2d(x * f, 0.0))
	}

	pub fn sample_2d(&self, position: Vec2) -> f32 {
		let f = self.params.frequency;
		self.gain(self.generator.get_noise_2d(position.x * f, position.y * f))
	}

	pub fn sample_3d(&self, position: Vec3) -> f32 {
		let f = self.params.frequency;
		self.gain(self.generator.get_noise_3d(position.x * f, position.y * f, position.z * f))
	}

	// --- 4D: FastNoise Lite is 3D max, so the fourth coordinate **w** shears **x** before
	// sampling. Use **w** as a *salt lane*: keep `(x, y, z)` spatial and bump `w` by a constant
	// per logical channel (e.g. per ring, per attribute) to decorrelate samples at the same
	// position without a second generator. ---

	pub fn sample_4d(&self, x: f32, y: f32, z: f32, w: f32) -> f32 {
		let f = self.params.frequency;
		self.gain(self.generator.get_noise_3d((x + w) * f, y * f, z * f))
	}

	/// Sample 4D, remap from `[-1, 1]` to `[0, 1]`, and clamp (amplitude can overshoot).
	pub fn sample_unit_4d(&self, x: f32, y: f32, z: f32, w: f32) -> f32 {
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
		let u = self.sample_unit_4d(x, y, z, w);
		let span = hi - lo;
		lo + ((u * span as f32).floor() as usize).min(span - 1)
	}

	/// Deterministically map a 4D sample to a float range.
	pub fn sample_range_f32_4d(&self, lo: f32, hi: f32, x: f32, y: f32, z: f32, w: f32) -> f32 {
		if hi <= lo {
			return lo;
		}
		let u = self.sample_unit_4d(x, y, z, w);
		lo + u * (hi - lo)
	}

	// --- Unit-range mappings `[-1, 1] → [0, 1]` (unclamped; assume amplitude ≤ 1) ---

	pub fn sample_unit_1d(&self, x: f32) -> f32 {
		(self.sample_1d(x) + 1.0) * 0.5
	}

	pub fn sample_unit_3d(&self, x: f32, y: f32, z: f32) -> f32 {
		(self.sample_3d(Vec3::new(x, y, z)) + 1.0) * 0.5
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

impl FromScalarNoise for NoiseConfig {
	fn from_scalar(noise: NoiseParams) -> Self {
		Self::new(noise)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	fn example_params() -> NoiseParams {
		NoiseParams {
			seed: 7,
			frequency: 5.0,
			amplitude: 0.05,
			octaves: 1,
			noise_type: NoiseType::Perlin,
		}
	}

	#[test]
	fn config_samples_are_deterministic() -> Result<()> {
		let p = example_params();
		let a = NoiseConfig::new(p);
		let b = NoiseConfig::new(p);
		let q = Vec3::splat(0.2);
		assert_eq!(a.sample_3d(q), b.sample_3d(q));
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
	fn salt_lane_decorrelates_samples_at_same_position() -> Result<()> {
		let n = NoiseConfig::new(example_params());
		let a = n.sample_4d(0.3, 0.4, 0.5, 0.0);
		let b = n.sample_4d(0.3, 0.4, 0.5, 17.0);
		assert_ne!(a, b);
		Ok(())
	}

	#[test]
	fn sample_unit_4d_is_clamped_unit_range() -> Result<()> {
		let mut p = example_params();
		p.amplitude = 10.0;
		let n = NoiseConfig::new(p);
		for i in 0..32 {
			let x = i as f32 * 0.37;
			let u = n.sample_unit_4d(x, x * 0.5, x * 0.25, 3.0);
			assert!((0.0..=1.0).contains(&u));
		}
		Ok(())
	}
}
