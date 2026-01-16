use bevy::prelude::*;
use noise::{NoiseFn, Seedable};
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

#[derive(Clone)]
pub struct NoiseConfig<const D: usize, N: NoiseFn<f64, D> + Seedable> {
	pub noise: N,
	pub frequency: f32,
	pub amplitude: f32,
	pub octaves: u32,
}

impl<const D: usize, N: NoiseFn<f64, D> + Seedable> Debug for NoiseConfig<D, N> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"NoiseConfig<{}, {}> {{ frequency: {}, amplitude: {}, octaves: {} }}",
			D,
			std::any::type_name::<N>(),
			self.frequency,
			self.amplitude,
			self.octaves
		)
	}
}

impl<const D: usize, N: NoiseFn<f64, D> + Seedable + Default> Default for NoiseConfig<D, N> {
	fn default() -> Self {
		Self { frequency: 0.1, amplitude: 1.0, octaves: 3, noise: N::default() }
	}
}

impl<const D: usize, N: NoiseFn<f64, D> + Seedable> PartialEq for NoiseConfig<D, N> {
	fn eq(&self, other: &Self) -> bool {
		self.frequency == other.frequency
			&& self.amplitude == other.amplitude
			&& self.octaves == other.octaves
			&& self.noise.seed() == other.noise.seed()
	}
}

impl<const D: usize, N: NoiseFn<f64, D> + Seedable> Eq for NoiseConfig<D, N> {}

impl<const D: usize, N: NoiseFn<f64, D> + Seedable> Hash for NoiseConfig<D, N> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.frequency.to_bits().hash(state);
		self.amplitude.to_bits().hash(state);
		self.octaves.hash(state);
		self.noise.seed().hash(state);
	}
}

impl<const D: usize, N: NoiseFn<f64, D> + Seedable> NoiseConfig<D, N> {
	pub fn new(noise: N) -> Self {
		Self { frequency: 0.1, amplitude: 1.0, octaves: 3, noise }
	}

	pub fn with_frequency(mut self, frequency: f32) -> Self {
		self.frequency = frequency;
		self
	}

	pub fn with_amplitude(mut self, amplitude: f32) -> Self {
		self.amplitude = amplitude;
		self
	}

	pub fn with_octaves(mut self, octaves: u32) -> Self {
		self.octaves = octaves;
		self
	}

	pub fn with_seed(mut self, seed: u32) -> Self {
		self.noise = self.noise.set_seed(seed);
		self
	}
}

impl<N: NoiseFn<f64, 2> + Seedable> NoiseConfig<2, N> {
	pub fn vec2_freqo(&self, position: Vec2) -> f64 {
		self.noise.get([
			position.x as f64 * self.frequency as f64,
			position.y as f64 * self.frequency as f64,
		])
	}
}

impl<N: NoiseFn<f64, 3> + Seedable> NoiseConfig<3, N> {
	/// Gets on vec3 only applies frequency
	pub fn vec3_freqo(&self, position: Vec3) -> f64 {
		self.noise.get([
			position.x as f64 * self.frequency as f64,
			position.y as f64 * self.frequency as f64,
			position.z as f64 * self.frequency as f64,
		])
	}

	/// Gets on vec3 only applies frequency to obtain a value on the unit interval
	pub fn vec3_on_unit(&self, position: Vec3) -> f64 {
		let noise = self.vec3_freqo(position);
		noise * 0.5 + 0.5
	}

	/// Gets the vec3 and applies the amplitude
	pub fn vec3_amp(&self, position: Vec3) -> f64 {
		let noise = self.vec3_freqo(position);
		noise * self.amplitude as f64
	}
}

impl<N: NoiseFn<f64, 4> + Seedable> NoiseConfig<4, N> {
	/// Gets the vec4 and applies the frequency to obtain a value
	pub fn vec4_freqo(&self, position: Vec4) -> f64 {
		self.noise.get([
			position.x as f64 * self.frequency as f64,
			position.y as f64 * self.frequency as f64,
			position.z as f64 * self.frequency as f64,
			position.w as f64 * self.frequency as f64,
		])
	}

	/// Gets the vec4 and applies the frequency to obtain a value on the unit interval
	pub fn vec4_on_unit(&self, position: Vec4) -> f64 {
		let noise = self.vec4_freqo(position);
		noise * 0.5 + 0.5
	}

	/// Gets the vec4 and applies the amplitude
	pub fn vec4_amp(&self, position: Vec4) -> f64 {
		let noise = self.vec4_freqo(position);
		noise * self.amplitude as f64
	}
}

pub trait InternalNoise<const D: usize, N: NoiseFn<f64, D> + Seedable> {
	fn set_internal_noise(&mut self, noise: NoiseConfig<D, N>);
}
