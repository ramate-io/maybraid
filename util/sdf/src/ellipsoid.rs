use crate::Sdf;
use bevy::prelude::*;

/// An ellipsoid SDF with arbitrary radii along each axis
pub struct EllipsoidSdf {
	pub center: Vec3,
	pub radii: Vec3,
}

impl EllipsoidSdf {
	pub fn new(center: Vec3, radii: Vec3) -> Self {
		Self { center, radii }
	}
}

impl Sdf for EllipsoidSdf {
	fn distance(&self, p: Vec3) -> f32 {
		let local = (p - self.center) / self.radii;
		let d = local.length();
		if d > 0.0 {
			(d - 1.0) * self.radii.min_element()
		} else {
			-self.radii.min_element()
		}
	}
}
