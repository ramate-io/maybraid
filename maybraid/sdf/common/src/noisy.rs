//! Compositional **surface noise** for any [`Sdf`]: `distance(p) + noise.sample_3d_world(p)`.
//!
//! [`NoisySurface`] stores authoring [`NoiseParams`] (CLI / serde friendly) and keeps a private
//! [`NoiseConfig`] built from those params for sampling.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3A;
use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use procedural_common::{
	sdf_band_margin, NoiseConfig, NoiseParams, NoiseType, NUMERIC_SURFACE_EPSILON,
};
use render_item::{
	mesh::{IdentifiedMesh, MeshId},
	NormalizeChunk,
};
use sdf::Bounds;
use sdf::Sdf;

use crate::ball::Ball;
use crate::crook_cylinder::CrookCylinder;
use crate::cylinder::TaperedCylinder;

pub mod unit_ball;
pub mod unit_cylinder;

pub use unit_ball::UnitBallNoiseParams;
pub use unit_cylinder::UnitCylinderNoiseParams;

fn inflate_cuboid_bounds(aabb: Aabb3d, margin: f32) -> Aabb3d {
	let pad = Vec3A::splat(margin);
	Aabb3d::from_min_max(aabb.min - pad, aabb.max + pad)
}

/// Wraps an inner SDF and offsets its field using [`NoiseConfig::sample_3d_world`] built from [`NoiseParams`].
pub struct NoisySurface<S> {
	pub inner: S,
	pub noise: NoiseParams,
	config: NoiseConfig,
}

#[cfg(feature = "serde")]
impl<S: serde::Serialize> serde::Serialize for NoisySurface<S> {
	fn serialize<Ser: serde::Serializer>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error> {
		use serde::Serialize;
		#[derive(Serialize)]
		struct NoisySurfaceRef<'a, T: serde::Serialize> {
			inner: &'a T,
			noise: &'a NoiseParams,
		}
		NoisySurfaceRef { inner: &self.inner, noise: &self.noise }.serialize(serializer)
	}
}

#[cfg(feature = "serde")]
impl<'de, S: serde::Deserialize<'de>> serde::Deserialize<'de> for NoisySurface<S> {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		#[derive(serde::Deserialize)]
		struct NoisySurfaceData<S> {
			inner: S,
			noise: NoiseParams,
		}
		let NoisySurfaceData { inner, noise } = NoisySurfaceData::deserialize(deserializer)?;
		Ok(Self::new(inner, noise))
	}
}

impl<S> NoisySurface<S> {
	pub fn new(inner: S, noise: NoiseParams) -> Self {
		let config = NoiseConfig::new(noise);
		Self { inner, noise, config }
	}

	pub fn from_params(inner: S, params: NoiseParams) -> Self {
		Self::new(inner, params)
	}

	/// Convenience for tests and legacy call sites: Perlin, single octave, given seed / frequency / amplitude.
	pub fn new_perlin(inner: S, seed: i32, frequency: f32, amplitude: f32) -> Self {
		Self::from_params(
			inner,
			NoiseParams {
				seed,
				frequency,
				amplitude,
				octaves: 1,
				noise_type: NoiseType::Perlin,
				..Default::default()
			},
		)
	}

	pub fn set_noise(&mut self, params: NoiseParams) {
		self.noise = params;
		self.config = NoiseConfig::new(self.noise);
	}
}

impl<S: Sdf> Sdf for NoisySurface<S> {
	fn distance(&self, p: Vec3) -> f32 {
		let base = self.inner.distance(p);
		let n = self.config.sample_3d_world(p);
		base + n
	}

	fn bounds(&self) -> Bounds {
		let inner = self.inner.bounds();
		let m = sdf_band_margin(&self.noise);
		match inner {
			Bounds::Cuboid(a) => Bounds::Cuboid(inflate_cuboid_bounds(a, m)),
			Bounds::Unbounded => inner,
		}
	}
}

impl<S: Clone> Clone for NoisySurface<S> {
	fn clone(&self) -> Self {
		Self::new(self.inner.clone(), self.noise)
	}
}

impl<S: IdentifiedMesh + Clone> IdentifiedMesh for NoisySurface<S> {
	fn id(&self) -> MeshId {
		self.inner.id().with_suffix(&format!("|noise:{:?}", self.noise))
	}
}

/// [`NoisySurface`] over [`TaperedCylinder`] — RFC-183 **noisy cylinder** primitive ([#210](https://github.com/ramate-io/maybraid/issues/210)).
pub type NoisyCylinder = NoisySurface<TaperedCylinder>;

impl NormalizeChunk for NoisyCylinder {
	fn normalize_chunk(&self, cascade_chunk: &CascadeChunk) -> CascadeChunk {
		let mu = sdf_band_margin(&self.noise);
		CascadeChunk::unit_center_chunk().with_res_2(cascade_chunk.res_2).with_mu(mu)
	}
}

/// [`NoisySurface`] over [`CrookCylinder`] — RFC-183 **noisy crook cylinder** ([#211](https://github.com/ramate-io/maybraid/issues/211)).
pub type NoisyCrookCylinder = NoisySurface<CrookCylinder>;

impl NormalizeChunk for NoisyCrookCylinder {
	fn normalize_chunk(&self, cascade_chunk: &CascadeChunk) -> CascadeChunk {
		let m = sdf_band_margin(&self.noise);
		let mu_xz = self.inner.chunk_mu_xz_pad() + m;
		let mu_y = m + self.inner.bounds_margin + NUMERIC_SURFACE_EPSILON;
		CascadeChunk::unit_center_chunk_with_mu_xz_y(mu_xz, mu_y).with_res_2(cascade_chunk.res_2)
	}
}

/// [`NoisySurface`] over [`Ball`] — RFC-183 **noisy ball** primitive ([#213](https://github.com/ramate-io/maybraid/issues/213)).
pub type NoisyBall = NoisySurface<Ball>;

impl NormalizeChunk for NoisyBall {
	fn normalize_chunk(&self, cascade_chunk: &CascadeChunk) -> CascadeChunk {
		let m = sdf_band_margin(&self.noise);
		let h = self.inner.radius + self.inner.bounds_margin + m;
		let half = 0.5_f32;
		let mu = (h - half).max(0.0) + NUMERIC_SURFACE_EPSILON;
		CascadeChunk::unit_3d_center_chunk().with_res_2(cascade_chunk.res_2).with_mu(mu)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use bevy::math::bounding::BoundingVolume;
	use procedural_common::NUMERIC_SURFACE_EPSILON;

	#[test]
	fn unit_cylinder_noise_preset() -> Result<()> {
		let p: NoiseParams = UnitCylinderNoiseParams.into();
		assert_eq!(p.amplitude, 0.05);
		assert_eq!(p.frequency, 5.0);
		assert_eq!(p.octaves, 1);
		assert_eq!(p.noise_type, NoiseType::Perlin);
		Ok(())
	}

	#[test]
	fn unit_ball_noise_preset() -> Result<()> {
		let p: NoiseParams = UnitBallNoiseParams.into();
		assert_eq!(p.amplitude, 0.05);
		assert_eq!(p.frequency, 5.0);
		assert_eq!(p.octaves, 1);
		assert_eq!(p.noise_type, NoiseType::Perlin);
		Ok(())
	}

	#[test]
	fn same_seed_same_distance() -> Result<()> {
		let c = TaperedCylinder::default();
		let params = NoiseParams {
			seed: 64,
			frequency: 5.0,
			amplitude: 0.05,
			octaves: 1,
			noise_type: NoiseType::Perlin,
			..Default::default()
		};
		let a = NoisySurface::from_params(c, params);
		let b = NoisySurface::from_params(c, params);
		let p = Vec3::new(0.12, 0.34, 0.56);
		assert_eq!(a.distance(p), b.distance(p));
		Ok(())
	}

	#[test]
	fn different_seed_can_differ() -> Result<()> {
		let c = TaperedCylinder::default();
		let a = NoisySurface::new_perlin(c, 1, 5.0, 0.05);
		let b = NoisySurface::new_perlin(c, 2, 5.0, 0.05);
		let p = Vec3::new(0.11, 0.22, 0.33);
		assert_ne!(a.distance(p), b.distance(p));
		Ok(())
	}

	#[test]
	fn domain_weight_zero_on_z_decouples_z_in_noise_only() -> Result<()> {
		let c = TaperedCylinder::default();
		let params = NoiseParams {
			seed: 7,
			frequency: 5.0,
			amplitude: 0.05,
			octaves: 1,
			noise_type: NoiseType::Perlin,
			domain_weights: Vec3::new(1.0, 1.0, 0.0),
		};
		let n = NoisySurface::from_params(c, params);
		let p0 = Vec3::new(0.2, 0.3, 0.0);
		let p1 = Vec3::new(0.0, 0.3, 0.2);
		assert_eq!(n.distance(p0), n.distance(p1));
		Ok(())
	}

	#[test]
	fn bounds_inflate_by_noise_margin() -> Result<()> {
		let c = TaperedCylinder::unit_segment(1.0, 0.5);
		let params = NoiseParams {
			seed: 3,
			frequency: 2.0,
			amplitude: 0.12,
			octaves: 1,
			noise_type: NoiseType::Perlin,
			..Default::default()
		};
		let noisy = NoisySurface::from_params(c, params);
		let m = procedural_common::sdf_band_margin(&params);
		if let (Bounds::Cuboid(inner), Bounds::Cuboid(out)) = (c.bounds(), noisy.bounds()) {
			let d = out.half_size() - inner.half_size();
			assert!((d.x - m).abs() < 1e-4);
			assert!((d.y - m).abs() < 1e-4);
			assert!((d.z - m).abs() < 1e-4);
		} else {
			panic!("expected cuboid bounds");
		}
		assert!(m > NUMERIC_SURFACE_EPSILON);
		Ok(())
	}
}
