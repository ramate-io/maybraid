use crate::noise::{BuildWithNoise, NoiseConfig, NoiseParams};
use bevy_math::{Vec2, Vec3};

#[derive(Debug, Clone)]
pub struct Bucket {
	weight: f32,
}

/// A bucket in a bucket throw distribution.
impl Bucket {
	pub fn new(weight: f32) -> Self {
		Self { weight }
	}

	pub fn weight(&self) -> f32 {
		self.weight
	}
}

// A bucket throw distribution.
#[derive(Debug, Clone)]
pub struct BucketThrow {
	/// Mostly, distributions are small, so it is cheaper to just iterate over a Vec.
	buckets: Vec<Bucket>,
	/// The mean anchor is the center of the throw.
	mean_anchor: f32,
	/// The total weight of the distribution.
	total_weight: f32,
}

impl BucketThrow {
	pub fn new() -> Self {
		Self { buckets: vec![], mean_anchor: 0.0, total_weight: 0.0 }
	}

	/// The total weight of the distribution.
	pub fn total_weight(&self) -> f32 {
		self.total_weight
	}

	/// Adds a bucket to the distribution.
	pub fn add(&mut self, weight: f32) {
		self.buckets.push(Bucket::new(weight));
		self.total_weight += weight;
	}

	/// Anchors the throw within the distribution according to the mean anchor.
	pub fn anchored_throw(&self, throw: f32) -> f32 {
		(self.mean_anchor + throw).rem_euclid(self.total_weight())
	}

	/// Selects a bucket from the distribution.
	pub fn select(&self, throw: f32) -> Option<usize> {
		if self.buckets.is_empty() {
			return None;
		}

		let total = self.total_weight();
		if total <= 0.0 || !total.is_finite() {
			return None;
		}

		let mut throw = self.anchored_throw(throw);

		for (index, bucket) in self.buckets.iter().enumerate() {
			throw -= bucket.weight();
			if throw <= 0.0 {
				return Some(index);
			}
		}

		Some(self.buckets.len() - 1)
	}
}

/// The underlying implementation of a bucket distribution simply selects
/// an instance stored in the distribution.
#[derive(Debug, Clone)]
pub struct TypedBucketThrow<T>
where
	T: Sized,
{
	distribution: BucketThrow,
	items: Vec<T>,
}

impl<T> TypedBucketThrow<T>
where
	T: Sized,
{
	pub fn new() -> Self {
		Self { distribution: BucketThrow::new(), items: vec![] }
	}

	/// Adds an item to the distribution.
	pub fn add(&mut self, item: T, weight: f32) {
		self.distribution.add(weight);
		self.items.push(item);
	}

	/// Selects an item from the distribution.
	pub fn select(&self, throw: f32) -> Option<&T> {
		self.distribution.select(throw).map(|index| &self.items[index])
	}

	/// Builds a resultant type from the noise params.
	pub fn build<S>(&self, throw: f32, noise: NoiseParams) -> Option<S>
	where
		T: BuildWithNoise<S>,
	{
		let builder = self.select(throw);
		builder.map(|item| item.build_with_noise(noise))
	}

	/// Selects from noise params in 2d
	pub fn select_from_noise_2d(&self, noise: NoiseParams, position: Vec2) -> Option<&T> {
		let total_weight = self.distribution.total_weight();
		let half_weight = total_weight / 2.0;
		// Sample is centered around 0 in the distribution, so we need to scale it by half the total weight to get a value between -half_weight and half_weight
		let throw = NoiseConfig::new(noise).sample_2d_world(position) * half_weight;
		self.select(throw)
	}

	/// Selects from noise params in 3d
	pub fn select_from_noise_3d(&self, noise: NoiseParams, position: Vec3) -> Option<&T> {
		let total_weight = self.distribution.total_weight();
		let half_weight = total_weight / 2.0;
		// Sample is centered around 0 in the distribution, so we need to scale it by half the total weight to get a value between -half_weight and half_weight
		let throw = NoiseConfig::new(noise).sample_3d_world(position) * half_weight;
		self.select(throw)
	}

	/// Builds from noise params in 2d
	pub fn build_from_noise_2d<S>(&self, noise: NoiseParams, position: Vec2) -> Option<S>
	where
		T: BuildWithNoise<S>,
	{
		let builder = self.select_from_noise_2d(noise, position);
		builder.map(|item| item.build_with_noise(noise))
	}

	/// Builds from noise params in 3d
	pub fn build_from_noise_3d<S>(&self, noise: NoiseParams, position: Vec3) -> Option<S>
	where
		T: BuildWithNoise<S>,
	{
		let builder = self.select_from_noise_3d(noise, position);
		builder.map(|item| item.build_with_noise(noise))
	}
}
