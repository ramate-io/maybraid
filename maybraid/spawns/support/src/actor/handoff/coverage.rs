//! Exact union coverage for the presented terrain under a capsule footprint.

use bevy::prelude::*;
use chunk::cascade::CascadeChunk;

#[derive(Clone, Copy)]
pub(super) struct Footprint {
	min: Vec2,
	max: Vec2,
}

impl Footprint {
	pub(super) fn capsule(position: Vec3, radius: f32) -> Self {
		Self { min: position.xz() - Vec2::splat(radius), max: position.xz() + Vec2::splat(radius) }
	}

	pub(super) fn chunk(chunk: &CascadeChunk) -> [Option<Self>; 4] {
		let min = chunk.origin.xz();
		let max = min + chunk.extent_vec().xz();
		if !min.is_finite() || !max.is_finite() || !min.cmplt(max).all() {
			return [None; 4];
		}
		let bounds = Self { min, max };
		let Some(omit) = chunk.omit else { return [Some(bounds), None, None, None] };
		let omit = Self { min: Vec3::from(omit.min).xz(), max: Vec3::from(omit.max).xz() };
		if !omit.min.is_finite() || !omit.max.is_finite() {
			return [None; 4];
		}
		bounds.subtract(omit)
	}

	fn subtract(self, covered: Self) -> [Option<Self>; 4] {
		let min = self.min.max(covered.min);
		let max = self.max.min(covered.max);
		if min.cmpge(max).any() {
			return [Some(self), None, None, None];
		}
		[
			(self.min, Vec2::new(min.x, self.max.y)),
			(Vec2::new(max.x, self.min.y), self.max),
			(Vec2::new(min.x, self.min.y), Vec2::new(max.x, min.y)),
			(Vec2::new(min.x, max.y), Vec2::new(max.x, self.max.y)),
		]
		.map(|(min, max)| min.cmplt(max).all().then_some(Self { min, max }))
	}

	pub(super) fn covered_by(self, chunks: impl IntoIterator<Item = Self>) -> bool {
		let mut uncovered = vec![self];
		let mut remaining = Vec::new();
		for chunk in chunks {
			remaining.clear();
			for rectangle in uncovered.drain(..) {
				remaining.extend(rectangle.subtract(chunk).into_iter().flatten());
			}
			if remaining.is_empty() {
				return true;
			}
			std::mem::swap(&mut uncovered, &mut remaining);
		}
		false
	}
}

#[cfg(test)]
mod tests {
	use super::Footprint;
	use bevy::prelude::*;

	#[test]
	fn large_capsule_union_keeps_an_interior_hole_uncovered_until_filled() {
		let footprint = Footprint::capsule(Vec3::ZERO, 4.0);
		let ring = [
			Footprint { min: Vec2::splat(-4.0), max: Vec2::new(0.8, 4.0) },
			Footprint { min: Vec2::new(1.2, -4.0), max: Vec2::splat(4.0) },
			Footprint { min: Vec2::new(0.8, -4.0), max: Vec2::new(1.2, 0.8) },
			Footprint { min: Vec2::new(0.8, 1.2), max: Vec2::new(1.2, 4.0) },
		];
		assert!(!footprint.covered_by(ring));
		assert!(!footprint.covered_by(ring.into_iter().rev()));
		assert!(footprint.covered_by(
			ring.into_iter()
				.chain([Footprint { min: Vec2::splat(0.8), max: Vec2::splat(1.2) },])
		));
	}
}
