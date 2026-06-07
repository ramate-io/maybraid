//! Ordered **first-fit** walk over bucket indices ([RFC-183 3.4.2.5]).

use super::bucket_throw::BucketThrow;

/// Yields bucket indices starting at `start`, then each adjacent bucket in order, wrapping once.
pub struct FirstFitIndices {
	len: usize,
	start: usize,
	step: usize,
}

impl FirstFitIndices {
	pub fn new(len: usize, start: usize) -> Self {
		Self { len, start: start % len.max(1), step: 0 }
	}
}

impl Iterator for FirstFitIndices {
	type Item = usize;

	fn next(&mut self) -> Option<Self::Item> {
		if self.len == 0 {
			return None;
		}
		if self.step >= self.len {
			return None;
		}
		let index = (self.start + self.step) % self.len;
		self.step += 1;
		Some(index)
	}
}

impl BucketThrow {
	/// Iterate bucket indices for first-fit selection starting at `start`.
	pub fn first_fit_from(&self, start: usize) -> FirstFitIndices {
		FirstFitIndices::new(self.len(), start)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn walks_in_order_and_wraps_once() -> Result<()> {
		let mut d = BucketThrow::new();
		d.add(1.0);
		d.add(1.0);
		d.add(1.0);
		let indices: Vec<_> = d.first_fit_from(1).collect();
		assert_eq!(indices, vec![1, 2, 0]);
		Ok(())
	}

	#[test]
	fn empty_distribution_yields_nothing() -> Result<()> {
		let d = BucketThrow::new();
		assert_eq!(d.first_fit_from(0).count(), 0);
		Ok(())
	}
}
