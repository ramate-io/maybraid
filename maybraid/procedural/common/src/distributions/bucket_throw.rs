use crate::noise::{BuildWithNoise, NoiseParams};
use std::collections::BTreeSet;
use std::hash::Hash;
use std::ops::Add;
use std::ops::Deref;

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

#[derive(Debug, Clone)]
pub struct BucketThrow {
	/// Mostl, distributions are small, so it is cheaper to just iterate over a Vec.
	buckets: Vec<Bucket>,
	/// The mean anchor is the center of the throw.
	mean_anchor: f32,
}

impl BucketThrow {
	pub fn new() -> Self {
		Self { buckets: vec![], mean_anchor: 0.0 }
	}

	pub fn total_weight(&self) -> f32 {
		self.buckets.iter().map(|b| b.weight()).sum()
	}

	pub fn add(&mut self, weight: f32) {
		self.buckets.push(Bucket::new(weight));
	}

	pub fn anchored_throw(&self, throw: f32) -> f32 {
		(self.mean_anchor + throw).rem_euclid(self.total_weight())
	}

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

	pub fn add(&mut self, item: T, weight: f32) {
		self.distribution.add(weight);
		self.items.push(item);
	}

	pub fn select(&self, throw: f32) -> Option<&T> {
		self.distribution.select(throw).map(|index| &self.items[index])
	}
}
