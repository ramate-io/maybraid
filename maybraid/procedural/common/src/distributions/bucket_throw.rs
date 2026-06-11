//! Ordered weighted **bucket throw** selection ([RFC-183 3.4.2.1]).
//!
//! Variants sit in contiguous buckets on a wrapped number line; each bucket's **weight** sets its
//! span and **order** is the vector index. Selection evaluates
//!
//! ```text
//! idx = bucket_lookup(wrap(mean_anchor + shift + sample, total_weight))
//! ```
//!
//! where `sample` is an independent noise draw mapped to `[-total_weight/2, total_weight/2]`
//! (half span is enough to cover the full wrapped line when the draw is centered at zero).
//! [`BucketThrow::select`] takes the combined `shift + sample`; [`BucketThrow::mean_anchor`]
//! holds the authored center (default `0.0`). Parent **perturbation** of bucket sizes should
//! produce a new distribution with reweighted buckets before lookup—this module does not mutate
//! weights in place.
//!
//! [RFC-183 3.4.2.1]: https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-04-cellular-groves/02-selection-and-placement/01-bucket-throw/README.md

#[macro_use]
mod r#macro;

use crate::noise::{BuildWithNoise, NoiseConfig, NoiseParams};
use bevy_math::{Vec2, Vec3};

/// One ordered bucket in a [`BucketThrow`] distribution.
#[derive(Debug, Clone)]
pub struct Bucket {
	weight: f32,
}

impl Bucket {
	pub fn new(weight: f32) -> Self {
		Self { weight }
	}

	pub fn weight(&self) -> f32 {
		self.weight
	}
}

/// Ordered weighted buckets on a wrapped line for locally coherent variant selection.
#[derive(Debug, Clone)]
pub struct BucketThrow {
	/// Mostly, distributions are small, so it is cheaper to just iterate over a Vec.
	buckets: Vec<Bucket>,
	/// Center of the throw in bucket space (default `0.0`; see [RFC-183 3.4.2.1.1]).
	mean_anchor: f32,
	/// Sum of bucket weights; also the wrap modulus.
	total_weight: f32,
}

impl Default for BucketThrow {
	fn default() -> Self {
		Self::new()
	}
}

impl BucketThrow {
	pub fn new() -> Self {
		Self { buckets: vec![], mean_anchor: 0.0, total_weight: 0.0 }
	}

	pub fn is_empty(&self) -> bool {
		self.buckets.is_empty()
	}

	pub fn len(&self) -> usize {
		self.buckets.len()
	}

	pub fn total_weight(&self) -> f32 {
		self.total_weight
	}

	pub fn mean_anchor(&self) -> f32 {
		self.mean_anchor
	}

	/// Replace the mean anchor (parent **shift** can also be passed into [`Self::select`]).
	pub fn with_mean_anchor(mut self, mean_anchor: f32) -> Self {
		self.mean_anchor = mean_anchor;
		self
	}

	/// Build a distribution from ordered weights.
	pub fn from_weights(weights: impl IntoIterator<Item = f32>, mean_anchor: f32) -> Self {
		let mut distribution = Self::new().with_mean_anchor(mean_anchor);
		for weight in weights {
			distribution.add(weight);
		}
		distribution
	}

	pub fn weight_at(&self, index: usize) -> Option<f32> {
		self.buckets.get(index).map(Bucket::weight)
	}

	/// Append a bucket in order. Non-finite and non-positive weights are ignored.
	pub fn add(&mut self, weight: f32) -> bool {
		if !weight.is_finite() || weight <= 0.0 {
			return false;
		}
		self.buckets.push(Bucket::new(weight));
		self.total_weight += weight;
		true
	}

	/// Map `shift + sample` into `[0, total_weight)`.
	pub fn anchored_throw(&self, throw: f32) -> f32 {
		let total = self.total_weight();
		if total <= 0.0 || !total.is_finite() {
			return 0.0;
		}
		(self.mean_anchor + throw).rem_euclid(total)
	}

	/// Select a bucket index from `throw` (`shift + sample` in bucket space).
	///
	/// Buckets form half-open intervals `[0, w0), [w0, w0+w1), …` along the wrapped line.
	pub fn select(&self, throw: f32) -> Option<usize> {
		if self.buckets.is_empty() {
			return None;
		}

		let total = self.total_weight();
		if total <= 0.0 || !total.is_finite() {
			return None;
		}

		let mut cursor = self.anchored_throw(throw);

		for (index, bucket) in self.buckets.iter().enumerate() {
			cursor -= bucket.weight();
			if cursor < 0.0 {
				return Some(index);
			}
		}

		// Floating-point residue at the last bucket boundary.
		Some(self.buckets.len() - 1)
	}
}

/// [`BucketThrow`] plus stored variant values.
#[derive(Debug, Clone)]
pub struct TypedBucketThrow<T>
where
	T: Sized,
{
	distribution: BucketThrow,
	items: Vec<T>,
}

impl<T> Default for TypedBucketThrow<T>
where
	T: Sized,
{
	fn default() -> Self {
		Self::new()
	}
}

impl<T> TypedBucketThrow<T>
where
	T: Sized,
{
	pub fn new() -> Self {
		Self { distribution: BucketThrow::new(), items: vec![] }
	}

	pub fn len(&self) -> usize {
		self.items.len()
	}

	pub fn is_empty(&self) -> bool {
		self.items.is_empty()
	}

	pub fn with_mean_anchor(mut self, mean_anchor: f32) -> Self {
		self.distribution = self.distribution.with_mean_anchor(mean_anchor);
		self
	}

	pub fn add(&mut self, item: T, weight: f32) {
		if self.distribution.add(weight) {
			self.items.push(item);
		}
	}

	pub fn select(&self, throw: f32) -> Option<&T> {
		self.distribution.select(throw).map(|index| &self.items[index])
	}

	pub fn build<S>(&self, throw: f32, noise: NoiseParams) -> Option<S>
	where
		T: BuildWithNoise<S>,
	{
		self.select(throw).map(|item| item.build_with_noise(noise))
	}

	/// Independent centered noise sample scaled to `[-total_weight/2, total_weight/2]`.
	fn selection_throw(&self, sample: f32) -> f32 {
		sample * self.distribution.total_weight() * 0.5
	}

	pub fn select_from_noise_2d(&self, noise: NoiseParams, position: Vec2) -> Option<&T> {
		let sample = NoiseConfig::new(noise).sample_2d(position);
		self.select(self.selection_throw(sample))
	}

	pub fn select_from_noise_3d(&self, noise: NoiseParams, position: Vec3) -> Option<&T> {
		let sample = NoiseConfig::new(noise).sample_3d(position);
		self.select(self.selection_throw(sample))
	}

	pub fn build_from_noise_2d<S>(&self, noise: NoiseParams, position: Vec2) -> Option<S>
	where
		T: BuildWithNoise<S>,
	{
		self.select_from_noise_2d(noise, position)
			.map(|item| item.build_with_noise(noise))
	}

	pub fn build_from_noise_3d<S>(&self, noise: NoiseParams, position: Vec3) -> Option<S>
	where
		T: BuildWithNoise<S>,
	{
		self.select_from_noise_3d(noise, position)
			.map(|item| item.build_with_noise(noise))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	fn three_equal() -> BucketThrow {
		let mut d = BucketThrow::new();
		d.add(1.0);
		d.add(1.0);
		d.add(1.0);
		d
	}

	#[test]
	fn empty_distribution_returns_none() -> Result<()> {
		let d = BucketThrow::new();
		assert!(d.select(0.0).is_none());
		Ok(())
	}

	#[test]
	fn non_positive_total_weight_returns_none() -> Result<()> {
		let mut d = BucketThrow::new();
		d.add(-1.0);
		assert_eq!(d.len(), 0);
		assert!(d.select(0.0).is_none());
		Ok(())
	}

	#[test]
	fn equal_weights_select_by_half_open_intervals() -> Result<()> {
		let d = three_equal();
		assert_eq!(d.select(0.0), Some(0));
		assert_eq!(d.select(0.999), Some(0));
		assert_eq!(d.select(1.0), Some(1));
		assert_eq!(d.select(2.0), Some(2));
		assert_eq!(d.select(2.999), Some(2));
		Ok(())
	}

	#[test]
	fn weighted_buckets_scale_interval_width() -> Result<()> {
		let mut d = BucketThrow::new();
		d.add(3.0);
		d.add(1.0);
		assert_eq!(d.select(0.0), Some(0));
		assert_eq!(d.select(2.999), Some(0));
		assert_eq!(d.select(3.0), Some(1));
		Ok(())
	}

	#[test]
	fn anchored_throw_wraps() -> Result<()> {
		let d = three_equal();
		assert!((d.anchored_throw(3.5) - 0.5).abs() < 1e-5);
		assert!((d.anchored_throw(-0.5) - 2.5).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn mean_anchor_shifts_selection() -> Result<()> {
		let d = three_equal().with_mean_anchor(1.0);
		assert_eq!(d.select(0.0), Some(1));
		assert_eq!(d.select(-1.0), Some(0));
		Ok(())
	}

	#[test]
	fn typed_select_returns_matching_item() -> Result<()> {
		let mut d = TypedBucketThrow::new();
		d.add('a', 1.0);
		d.add('b', 2.0);
		assert_eq!(d.select(0.0), Some(&'a'));
		assert_eq!(d.select(1.5), Some(&'b'));
		Ok(())
	}

	#[test]
	fn noise_selection_is_deterministic() -> Result<()> {
		let mut d = TypedBucketThrow::new();
		d.add(10_u32, 1.0);
		d.add(20_u32, 1.0);
		let noise = NoiseParams { seed: 99, frequency: 0.25, ..Default::default() };
		let pos = Vec2::new(4.0, 7.0);
		let a = d.select_from_noise_2d(noise, pos);
		let b = d.select_from_noise_2d(noise, pos);
		assert_eq!(a, b);
		Ok(())
	}

	mod macro_tests {
		use super::*;
		use crate::FromScalarNoise;

		#[derive(Debug, Clone, PartialEq)]
		struct Oak {
			seed: i32,
		}

		#[derive(Debug, Clone, PartialEq)]
		struct Pine {
			seed: i32,
		}

		impl FromScalarNoise for Oak {
			fn from_scalar(noise: NoiseParams) -> Self {
				Self { seed: noise.seed }
			}
		}

		impl FromScalarNoise for Pine {
			fn from_scalar(noise: NoiseParams) -> Self {
				Self { seed: noise.seed + 1 }
			}
		}

		crate::bucket_throw! {
			#[derive(Debug, PartialEq)]
			enum SampleTree {
				Oak(Oak) => 1.0,
				Pine(Pine) => 2.0,
			}
		}

		#[test]
		fn macro_declares_distribution_and_builds() -> Result<()> {
			let dist = SampleTree::bucket_throw();
			assert_eq!(dist.len(), 2);
			let noise = NoiseParams { seed: 7, ..Default::default() };
			let tree = dist.build(0.0, noise).expect("selection");
			assert_eq!(tree, SampleTree::Oak(Oak { seed: 7 }));
			Ok(())
		}

		#[test]
		fn macro_respects_weights() -> Result<()> {
			let dist = SampleTree::bucket_throw();
			assert_eq!(dist.select(0.0), Some(&SampleTreeBuilder::Oak));
			assert_eq!(dist.select(1.5), Some(&SampleTreeBuilder::Pine));
			Ok(())
		}
	}
}
