use crate::Sdf;
use bevy::prelude::*;

/// A sphere SDF
#[derive(Clone, Copy, Debug)]
pub struct SphereSdf {
	pub center: Vec3,
	pub radius: f32,
}

impl SphereSdf {
	pub fn new(center: Vec3, radius: f32) -> Self {
		Self { center, radius }
	}
}

impl Sdf for SphereSdf {
	fn distance(&self, p: Vec3) -> f32 {
		(p - self.center).length() - self.radius
	}
}
