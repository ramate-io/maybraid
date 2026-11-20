//! Conservative world-space bounds including wood, foliage, and wind.

use bevy::camera::primitives::Aabb;
use bevy::math::{Vec3, Vec3A};
use vegetation_shaders::wesl::max_wind_displacement_m;

use crate::layout::PlantDescriptor;

/// Inclusive AABB in world metres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BushBounds {
	pub min: Vec3,
	pub max: Vec3,
}

impl Default for BushBounds {
	fn default() -> Self {
		Self::empty()
	}
}

impl BushBounds {
	pub fn empty() -> Self {
		Self { min: Vec3::splat(f32::MAX), max: Vec3::splat(f32::MIN) }
	}

	pub fn from_plant(plant: &PlantDescriptor, wind_strength_m: f32) -> Self {
		let pad = plant.conservative_extent_m(wind_strength_m);
		Self {
			min: plant.root - Vec3::new(pad, 0.05, pad),
			max: plant.root + Vec3::new(pad, plant.height_m + pad * 0.25, pad),
		}
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

	pub fn covers_wind(
		plant: &PlantDescriptor,
		wind_strength_m: f32,
		stiffness: f32,
	) -> bool {
		let pad = max_wind_displacement_m(plant.height_m, wind_strength_m, stiffness);
		pad <= plant.conservative_extent_m(wind_strength_m) + 1e-4
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::recipe::BushRecipe;
	use crate::topology::BushPlant;
	use bevy::math::Vec3Swizzles;
	use vegetation_shaders::wesl::{deform_along_curve, wind_field, WindInputs};

	#[test]
	fn bounds_cover_deformed_tip() -> anyhow::Result<()> {
		let plant = BushPlant::generate(
			2,
			0,
			Vec3::new(1.0, 0.0, -3.0),
			Vec3::Y,
			1.0,
			&BushRecipe::small_scrub(),
		);
		let strength = 0.58;
		let bounds = BushBounds::from_plant(&plant.descriptor, strength);
		let wind = wind_field(plant.descriptor.root.xz(), WindInputs::botw_gust(1.85));
		let tip = deform_along_curve(
			plant.descriptor.root + plant.descriptor.up * plant.descriptor.height_m,
			plant.descriptor.root,
			1.0,
			wind,
			plant.descriptor.wood_stiffness,
		);
		assert!(bounds.contains_deformed(tip), "tip {tip:?} not in {bounds:?}");
		assert!(bounds.contains_deformed(plant.descriptor.root));
		Ok(())
	}
}
