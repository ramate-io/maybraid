//! Host-owned bounded world XZ height coverage. No game terrain dependency.
use bevy::{
	math::{DVec2, DVec3},
	prelude::*,
};
use std::sync::Arc;
#[derive(Clone, Copy, Debug)]
pub struct GroundSample {
	pub point: DVec3,
	pub normal: DVec3,
}
#[derive(Clone, Debug)]
pub struct Heightfield {
	pub origin: DVec2,
	pub step: f64,
	pub dimensions: [usize; 2],
	pub version: u64,
	pub heights: Vec<f64>,
	pub valid: Vec<bool>,
}
impl Heightfield {
	pub fn new(
		origin: DVec2,
		step: f64,
		dimensions: [usize; 2],
		version: u64,
		heights: Vec<f64>,
		valid: Vec<bool>,
	) -> Result<Self, &'static str> {
		let n = dimensions[0].checked_mul(dimensions[1]).ok_or("terrain dimensions overflow")?;
		if !origin.is_finite()
			|| !step.is_finite()
			|| step <= 0.
			|| dimensions.iter().any(|n| !(2..=65).contains(n))
			|| heights.len() != n
			|| valid.len() != n
			|| heights.iter().any(|h| !h.is_finite())
		{
			return Err("invalid terrain heightfield (2..65 nodes per axis)");
		}
		Ok(Self { origin, step, dimensions, version, heights, valid })
	}
	pub fn sample(&self, p: DVec3) -> Option<GroundSample> {
		if !p.is_finite() {
			return None;
		}
		let q = (DVec2::new(p.x, p.z) - self.origin) / self.step;
		if q.cmplt(DVec2::ZERO).any()
			|| q.cmpgt(DVec2::new((self.dimensions[0] - 1) as f64, (self.dimensions[1] - 1) as f64))
				.any()
		{
			return None;
		}
		let x = (q.x.floor() as usize).min(self.dimensions[0] - 2);
		let z = (q.y.floor() as usize).min(self.dimensions[1] - 2);
		let u = q.x - x as f64;
		let v = q.y - z as f64;
		let i = x + self.dimensions[0] * z;
		let ids = [i, i + 1, i + self.dimensions[0], i + self.dimensions[0] + 1];
		if ids.iter().any(|i| !self.valid[*i]) {
			return None;
		}
		let [a, b, c, d] = ids.map(|i| self.heights[i]);
		let h = (a * (1. - u) + b * u) * (1. - v) + (c * (1. - u) + d * u) * v;
		let dx = ((b - a) * (1. - v) + (d - c) * v) / self.step;
		let dz = ((c - a) * (1. - u) + (d - b) * u) / self.step;
		Some(GroundSample {
			point: DVec3::new(p.x, h, p.z),
			normal: DVec3::new(-dx, 1., -dz).normalize(),
		})
	}
}
#[derive(Component, Clone)]
pub struct HerdTerrain(pub Arc<Heightfield>);
#[derive(Resource)]
pub struct HerdBudget {
	pub maximum: usize,
}
impl Default for HerdBudget {
	fn default() -> Self {
		Self { maximum: 2 }
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn world_height_normal_validity_and_missing_are_explicit() -> Result<(), String> {
		let h = Heightfield::new(
			DVec2::new(20., -15.),
			0.5,
			[3, 3],
			7,
			(0..9).map(|i| 4. + (i % 3) as f64 * 0.1 + (i / 3) as f64 * 0.2).collect(),
			vec![true; 9],
		)?;
		let g = h.sample(DVec3::new(20.25, 30., -14.75)).ok_or("ground")?;
		assert!((g.point.y - 4.15).abs() < 1e-12);
		assert!((g.normal - DVec3::new(-0.2, 1., -0.4).normalize()).length() < 1e-12);
		assert!(h.sample(DVec3::ZERO).is_none());
		let mut invalid = h.clone();
		invalid.valid[0] = false;
		assert!(invalid.sample(g.point).is_none());
		Ok(())
	}
}
