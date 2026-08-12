//! Bit-stable [`Transform`] equality / hashing for merge cache keys.

use std::hash::{Hash, Hasher};

use bevy::prelude::{Quat, Transform, Vec3};

/// [`PartialEq`] / [`Eq`] / [`Hash`] via IEEE bit patterns (cache-key stable).
#[derive(Debug, Clone, Copy)]
pub struct TransformKey(pub Transform);

impl TransformKey {
	pub const IDENTITY: Self = Self(Transform::IDENTITY);

	pub fn new(transform: Transform) -> Self {
		Self(transform)
	}
}

impl From<Transform> for TransformKey {
	fn from(transform: Transform) -> Self {
		Self(transform)
	}
}

impl PartialEq for TransformKey {
	fn eq(&self, other: &Self) -> bool {
		vec3_bits_eq(self.0.translation, other.0.translation)
			&& quat_bits_eq(self.0.rotation, other.0.rotation)
			&& vec3_bits_eq(self.0.scale, other.0.scale)
	}
}

impl Eq for TransformKey {}

impl Hash for TransformKey {
	fn hash<H: Hasher>(&self, state: &mut H) {
		hash_vec3(self.0.translation, state);
		hash_quat(self.0.rotation, state);
		hash_vec3(self.0.scale, state);
	}
}

fn vec3_bits_eq(a: Vec3, b: Vec3) -> bool {
	a.x.to_bits() == b.x.to_bits()
		&& a.y.to_bits() == b.y.to_bits()
		&& a.z.to_bits() == b.z.to_bits()
}

fn quat_bits_eq(a: Quat, b: Quat) -> bool {
	a.x.to_bits() == b.x.to_bits()
		&& a.y.to_bits() == b.y.to_bits()
		&& a.z.to_bits() == b.z.to_bits()
		&& a.w.to_bits() == b.w.to_bits()
}

fn hash_vec3<H: Hasher>(v: Vec3, state: &mut H) {
	v.x.to_bits().hash(state);
	v.y.to_bits().hash(state);
	v.z.to_bits().hash(state);
}

fn hash_quat<H: Hasher>(q: Quat, state: &mut H) {
	q.x.to_bits().hash(state);
	q.y.to_bits().hash(state);
	q.z.to_bits().hash(state);
	q.w.to_bits().hash(state);
}
