//! Canonical spatial [`Cell`] region shared by the index and generation APIs.

use std::hash::{Hash, Hasher};
use std::ops::Deref;

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3A;

/// World-space axis-aligned cell region (RFC-142 `(D, Cell)` bucket identity).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell(pub Aabb3d);

impl Eq for Cell {}

impl Hash for Cell {
	fn hash<H: Hasher>(&self, state: &mut H) {
		hash_vec3a(self.0.min, state);
		hash_vec3a(self.0.max, state);
	}
}

impl Deref for Cell {
	type Target = Aabb3d;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Cell {
	pub const fn new(region: Aabb3d) -> Self {
		Self(region)
	}

	pub fn from_min_max(min: impl Into<Vec3A>, max: impl Into<Vec3A>) -> Self {
		Self(Aabb3d::from_min_max(min, max))
	}

	/// Returns the cell as a region.
	pub fn as_region(&self) -> &Aabb3d {
		&self.0
	}

	/// Converts the cell to a region.
	pub fn into_region(self) -> Aabb3d {
		self.0
	}
}

fn hash_vec3a(v: Vec3A, state: &mut impl Hasher) {
	v.x.to_bits().hash(state);
	v.y.to_bits().hash(state);
	v.z.to_bits().hash(state);
}
