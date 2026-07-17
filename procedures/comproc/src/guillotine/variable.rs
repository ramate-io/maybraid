//! Higher-order guillotine: sample a depth in a range, then cut.

use crate::guillotine::bounds::Bounds;
use crate::guillotine::config::GuillotineConfig;
use crate::guillotine::cutter::{Guillotine, RegionsOwned};
use crate::guillotine::regions::GuillotineCuts;
use crate::noise::config::NoiseConfig;
use noise::{NoiseFn, Seedable};

/// Inclusive depth range for [`VariableGuillotine`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DepthRange {
	pub min: u8,
	pub max: u8,
}

impl DepthRange {
	pub const fn new(min: u8, max: u8) -> Self {
		Self { min, max }
	}

	/// Half-open interpretation used when sampling: `[min, max]` inclusive on both ends.
	pub fn sample_hi_exclusive(self) -> usize {
		self.max as usize + 1
	}
}

impl Default for DepthRange {
	fn default() -> Self {
		Self { min: 1, max: 4 }
	}
}

/// Samples a cut depth in [`DepthRange`], then runs a fixed-depth [`Guillotine`].
///
/// Useful when neighboring roots should vary how finely they subdivide while sharing
/// the same step window and noise family.
#[derive(Debug, Clone, PartialEq)]
pub struct VariableGuillotine<const D: usize, N: NoiseFn<f64, D> + Seedable> {
	noise: NoiseConfig<D, N>,
	config: GuillotineConfig,
	depth_range: DepthRange,
}

impl<const D: usize, N: NoiseFn<f64, D> + Seedable> VariableGuillotine<D, N> {
	pub fn new(
		noise: NoiseConfig<D, N>,
		config: GuillotineConfig,
		depth_range: DepthRange,
	) -> Self {
		Self {
			noise,
			config,
			depth_range,
		}
	}

	pub fn with_noise(noise: NoiseConfig<D, N>) -> Self {
		Self::new(noise, GuillotineConfig::default(), DepthRange::default())
	}

	pub fn with_depth_range(mut self, depth_range: DepthRange) -> Self {
		self.depth_range = depth_range;
		self
	}

	pub fn with_config(mut self, config: GuillotineConfig) -> Self {
		self.config = config;
		self
	}

	pub fn depth_range(&self) -> DepthRange {
		self.depth_range
	}

	pub fn set_depth_range(&mut self, depth_range: DepthRange) {
		self.depth_range = depth_range;
	}

	pub fn config(&self) -> &GuillotineConfig {
		&self.config
	}

	pub fn config_mut(&mut self) -> &mut GuillotineConfig {
		&mut self.config
	}

	pub fn set_config(&mut self, config: GuillotineConfig) {
		self.config = config;
	}

	pub fn noise(&self) -> &NoiseConfig<D, N> {
		&self.noise
	}

	pub fn noise_mut(&mut self) -> &mut NoiseConfig<D, N> {
		&mut self.noise
	}

	pub fn set_noise(&mut self, noise: NoiseConfig<D, N>) {
		self.noise = noise;
	}

	pub fn with_seed(mut self, seed: u32) -> Self {
		self.noise = self.noise.with_seed(seed);
		self
	}

	pub fn with_frequency(mut self, frequency: f32) -> Self {
		self.noise = self.noise.with_frequency(frequency);
		self
	}

	/// Sample depth for `root`, build a fixed [`Guillotine`], and return it (with depth applied).
	pub fn guillotine_for(&self, root: Bounds<D>) -> Guillotine<D, N>
	where
		N: Clone,
	{
		let depth = self.sample_depth(root);
		Guillotine::new(self.noise.clone(), self.config, depth)
	}

	/// Sample depth and run [`Guillotine::cut`].
	pub fn cut(&self, root: Bounds<D>) -> GuillotineCuts<D>
	where
		N: Clone,
	{
		self.guillotine_for(root).cut(root)
	}

	/// Sample depth and iterate leaf regions.
	pub fn regions(&self, root: Bounds<D>) -> RegionsOwned<D>
	where
		N: Clone,
	{
		self.guillotine_for(root).regions(root)
	}

	/// Sample depth and collect leaf regions.
	pub fn regions_vec(&self, root: Bounds<D>) -> Vec<Bounds<D>>
	where
		N: Clone,
	{
		self.guillotine_for(root).regions_vec(root)
	}

	fn sample_depth(&self, root: Bounds<D>) -> u8 {
		let lo = self.depth_range.min as usize;
		let hi = self.depth_range.sample_hi_exclusive();
		self.noise
			.sample_range_usize(lo, hi, sample_point(root.lower_left(), 0, SALT_DEPTH))
			as u8
	}
}

/// Noise query coordinate only (decorrelates the depth draw from cut-channel salts).
/// Not part of the geometric packing rule—see [`crate::guillotine::cutter`].
fn sample_point<const D: usize>(anchor: [f32; D], attempt: u8, salt: f32) -> [f32; D] {
	let mut p = anchor;
	p[0] += salt + attempt as f32 * 101.0;
	if D > 1 {
		p[1] += salt * 0.37;
	}
	p
}

const SALT_DEPTH: f32 = 19.9;
