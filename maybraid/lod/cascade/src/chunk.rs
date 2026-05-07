use std::hash::{Hash, Hasher};
use std::ops::Sub;

use bevy_math::bounding::Aabb3d;
use bevy_math::{Vec3, Vec3A};

/// Geometric footprint for cascade work (RFC §3.1): solid axis-aligned bounds \(B\) with optional
/// omission \(O\) so the effective region is \(B \setminus O\). \(B\) need not be a cube.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chunk {
	bounds: Aabb3d,
	omit: Option<Aabb3d>,
}

impl Eq for Chunk {}

impl Hash for Chunk {
	fn hash<H: Hasher>(&self, state: &mut H) {
		hash_aabb(self.bounds, state);
		match self.omit {
			None => 0u8.hash(state),
			Some(o) => {
				1u8.hash(state);
				hash_aabb(o, state);
			}
		}
	}
}

impl PartialOrd for Chunk {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp_key().cmp(&other.cmp_key()))
	}
}

impl Ord for Chunk {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.cmp_key().cmp(&other.cmp_key())
	}
}

impl Chunk {
	/// Solid bounds \(B\) and optional omission \(O\).
	pub fn new(bounds: Aabb3d, omit: Option<Aabb3d>) -> Self {
		Self { bounds, omit }
	}

	/// Axis-aligned footprint from inclusive min/max corners.
	pub fn from_min_max(min: Vec3, max: Vec3, omit: Option<Aabb3d>) -> Self {
		Self::new(Aabb3d::from_min_max(min, max), omit)
	}

	/// Convenience when \(B\) is a cube \([\mathrm{min}, \mathrm{min} + (s,s,s)]\).
	pub fn cube(min_corner: Vec3, edge: f32, omit: Option<Aabb3d>) -> Self {
		let max = min_corner + Vec3::splat(edge);
		Self::from_min_max(min_corner, max, omit)
	}

	#[inline]
	pub fn bounds(&self) -> Aabb3d {
		self.bounds
	}

	#[inline]
	pub fn omit(&self) -> Option<Aabb3d> {
		self.omit
	}

	#[inline]
	pub fn bounds_min(&self) -> Vec3 {
		self.bounds.min.into()
	}

	#[inline]
	pub fn bounds_max(&self) -> Vec3 {
		self.bounds.max.into()
	}

	/// Edge lengths \((\max_x-\min_x, \ldots)\); use this when comparing LOD “level” without assuming a cube.
	#[inline]
	pub fn extent(&self) -> Vec3 {
		Vec3::from(self.bounds.max) - Vec3::from(self.bounds.min)
	}

	/// Largest edge length (useful when you only need a scalar comparable to the old uniform `size`).
	#[inline]
	pub fn max_extent_component(&self) -> f32 {
		let e = self.extent();
		e.x.max(e.y).max(e.z)
	}

	/// Overlap volume between this footprint and `query`, subtracting the omission wedge clipped to \(B\).
	pub fn overlap_volume(&self, query: &Aabb3d) -> f32 {
		let v = intersection_volume(&self.bounds, query);
		if let Some(omit) = self.omit {
			if let Some(omit_in_outer) = non_empty_aabb_intersect(self.bounds, omit) {
				let v_omit = intersection_volume(&omit_in_outer, query);
				return (v - v_omit).max(0.0);
			}
		}
		v
	}

	fn cmp_key(self) -> impl Ord {
		let b = self.bounds;
		let omit = self.omit.map(|o| (o.min, o.max));
		(
			vec3a_bits_tuple(b.min),
			vec3a_bits_tuple(b.max),
			omit.map(|(a, b)| (vec3a_bits_tuple(a), vec3a_bits_tuple(b))),
		)
	}
}

fn vec3a_bits_tuple(v: Vec3A) -> (u32, u32, u32) {
	(v.x.to_bits(), v.y.to_bits(), v.z.to_bits())
}

fn hash_vec3a(v: Vec3A, state: &mut impl Hasher) {
	vec3a_bits_tuple(v).hash(state);
}

fn hash_aabb(a: Aabb3d, state: &mut impl Hasher) {
	hash_vec3a(a.min, state);
	hash_vec3a(a.max, state);
}

/// Non-empty intersection of two axis-aligned boxes, if any.
fn non_empty_aabb_intersect(a: Aabb3d, b: Aabb3d) -> Option<Aabb3d> {
	let min = a.min.max(b.min);
	let max = a.max.min(b.max);
	if min.x <= max.x && min.y <= max.y && min.z <= max.z {
		Some(Aabb3d::from_min_max(min, max))
	} else {
		None
	}
}

fn intersection_volume(a: &Aabb3d, b: &Aabb3d) -> f32 {
	let min = a.min.max(b.min);
	let max = a.max.min(b.max);
	let d: Vec3A = max.sub(min);
	if d.x <= 0.0 || d.y <= 0.0 || d.z <= 0.0 {
		return 0.0;
	}
	d.x * d.y * d.z
}
