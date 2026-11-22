//! Conservative world-space bounds including generated geometry and wind.

use bevy::camera::primitives::Aabb;
use bevy::math::{Vec3, Vec3A};
use vegetation_shaders::wesl::max_wind_displacement_m;

use crate::patch::SurfaceSample;

/// Inclusive AABB in world metres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrassBounds {
	pub min: Vec3,
	pub max: Vec3,
}

impl Default for GrassBounds {
	fn default() -> Self {
		Self::empty()
	}
}

impl GrassBounds {
	pub fn empty() -> Self {
		Self { min: Vec3::splat(f32::MAX), max: Vec3::splat(f32::MIN) }
	}

	pub fn from_samples(
		samples: &[SurfaceSample],
		height_m: f32,
		wind_strength_m: f32,
		stiffness: f32,
	) -> Self {
		let mut bounds = Self::empty();
		for sample in samples {
			let blade = sample.blade_height_m(height_m);
			let pad = max_wind_displacement_m(blade, wind_strength_m, stiffness) + blade + 0.08;
			bounds.include_point(sample.root, pad, blade);
		}
		if samples.is_empty() {
			Self { min: Vec3::ZERO, max: Vec3::ZERO }
		} else {
			bounds
		}
	}

	pub fn include_point(&mut self, root: Vec3, pad_xz: f32, height_m: f32) {
		self.min = self.min.min(root - Vec3::new(pad_xz, 0.05, pad_xz));
		self.max = self.max.max(root + Vec3::new(pad_xz, height_m + pad_xz * 0.2, pad_xz));
	}

	pub fn union(self, other: Self) -> Self {
		if self.is_empty() {
			return other;
		}
		if other.is_empty() {
			return self;
		}
		Self { min: self.min.min(other.min), max: self.max.max(other.max) }
	}

	pub fn is_empty(self) -> bool {
		self.min.cmpgt(self.max).any()
	}

	pub fn aabb(self) -> Aabb {
		let center = (self.min + self.max) * 0.5;
		let half = (self.max - self.min) * 0.5;
		Aabb { center: Vec3A::from(center), half_extents: Vec3A::from(half.max(Vec3::splat(0.05))) }
	}

	pub fn contains_deformed(self, point: Vec3) -> bool {
		point.cmpge(self.min).all() && point.cmple(self.max).all()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::math::Vec3Swizzles;
	use vegetation_shaders::wesl::{deform_along_curve, wind_field, WindInputs};

	#[test]
	fn bounds_cover_maximum_wind_tip() -> anyhow::Result<()> {
		let root = Vec3::new(2.0, 1.0, -4.0);
		let height = 0.7;
		let samples = [SurfaceSample::planar(root)];
		let bounds = GrassBounds::from_samples(&samples, height, 0.22, 0.0);
		let wind = wind_field(root.xz(), WindInputs::gentle_botw(3.4));
		let tip = deform_along_curve(root + Vec3::Y * height, root, 1.0, wind, 0.0);
		assert!(bounds.contains_deformed(tip), "tip {tip:?} not in {bounds:?}");
		assert!(bounds.contains_deformed(root));
		Ok(())
	}
}
