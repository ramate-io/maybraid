//! Iteration over leaf hyper-rectangles from per-axis cut lists.

use crate::guillotine::bounds::Bounds;

/// Result of a guillotine cut pass: root bounds plus interior cuts per axis.
#[derive(Debug, Clone, PartialEq)]
pub struct GuillotineCuts<const D: usize> {
	pub root: Bounds<D>,
	/// Strictly interior cut positions per axis, sorted ascending.
	pub cuts: [Vec<f32>; D],
}

impl<const D: usize> GuillotineCuts<D> {
	pub fn new(root: Bounds<D>, cuts: [Vec<f32>; D]) -> Self {
		Self { root, cuts }
	}

	/// Number of leaf regions (= product of per-axis interval counts).
	pub fn region_count(&self) -> usize {
		let mut n = 1usize;
		for axis in 0..D {
			n = n.saturating_mul(self.cuts[axis].len() + 1);
		}
		n
	}

	/// Iterate leaf regions that tile [`Self::root`] with no gaps or interior overlap.
	pub fn regions(&self) -> Regions<'_, D> {
		Regions::new(&self.root, &self.cuts)
	}

	/// Collect all leaf regions into a `Vec`.
	pub fn regions_vec(&self) -> Vec<Bounds<D>> {
		self.regions().collect()
	}
}

/// Odometer-style iterator over the cartesian product of per-axis intervals.
#[derive(Debug, Clone)]
pub struct Regions<'a, const D: usize> {
	root: &'a Bounds<D>,
	cuts: &'a [Vec<f32>; D],
	/// Interval count per axis (`cuts[i].len() + 1`).
	counts: [usize; D],
	/// Current interval index per axis.
	cursor: [usize; D],
	finished: bool,
}

impl<'a, const D: usize> Regions<'a, D> {
	pub fn new(root: &'a Bounds<D>, cuts: &'a [Vec<f32>; D]) -> Self {
		let mut counts = [0usize; D];
		for i in 0..D {
			counts[i] = cuts[i].len() + 1;
		}
		let finished = counts.iter().any(|&c| c == 0);
		Self { root, cuts, counts, cursor: [0; D], finished }
	}

	fn bounds_at_cursor(&self) -> Bounds<D> {
		let mut min = [0.0; D];
		let mut max = [0.0; D];
		for axis in 0..D {
			let i = self.cursor[axis];
			min[axis] = if i == 0 { self.root.min[axis] } else { self.cuts[axis][i - 1] };
			max[axis] =
				if i + 1 >= self.counts[axis] { self.root.max[axis] } else { self.cuts[axis][i] };
		}
		Bounds::new(min, max)
	}

	fn advance(&mut self) {
		for axis in 0..D {
			self.cursor[axis] += 1;
			if self.cursor[axis] < self.counts[axis] {
				return;
			}
			self.cursor[axis] = 0;
		}
		self.finished = true;
	}
}

impl<'a, const D: usize> Iterator for Regions<'a, D> {
	type Item = Bounds<D>;

	fn next(&mut self) -> Option<Self::Item> {
		if self.finished {
			return None;
		}
		let item = self.bounds_at_cursor();
		self.advance();
		Some(item)
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		if self.finished {
			return (0, Some(0));
		}
		let mut linear = 0usize;
		let mut stride = 1usize;
		for axis in 0..D {
			linear += self.cursor[axis] * stride;
			stride = stride.saturating_mul(self.counts[axis]);
		}
		let mut total = 1usize;
		for axis in 0..D {
			total = total.saturating_mul(self.counts[axis]);
		}
		let left = total.saturating_sub(linear);
		(left, Some(left))
	}
}

impl<'a, const D: usize> ExactSizeIterator for Regions<'a, D> {}
