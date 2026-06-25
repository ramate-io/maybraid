//! **Ball** — SDF sphere centered at the origin ([RFC-183 §3.1.2.2](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/02-ball-components/02-noisy-ball/README.md), [#213](https://github.com/ramate-io/maybraid/issues/213)).
//!
//! Pair with [`crate::noisy::NoisySurface`] for the RFC **noisy ball** composition ([`NoisyBall`](crate::noisy::NoisyBall)).

pub mod render_item_plugin;

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use procedural_common::NUMERIC_SURFACE_EPSILON;
use render_item::{
	mesh::{IdentifiedMesh, MeshId},
	NormalizeChunk,
};
use sdf::{Bounds, Sdf};

/// Half-edge of [`CascadeChunk::unit_3d_center_chunk`] before [`CascadeChunk::with_mu`].
const UNIT_3D_CENTER_HALF: f32 = 0.5;

/// Solid sphere **centered at the origin**: inside is negative distance.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Ball {
	/// Radius (world units).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.5))]
	pub radius: f32,
	/// Extra padding on the axis-aligned bounds cube (e.g. match chunk μ or mesh band).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.0))]
	pub bounds_margin: f32,
}

impl Default for Ball {
	fn default() -> Self {
		Self { radius: 0.5, bounds_margin: 0.0 }
	}
}

impl Ball {
	/// Sphere of radius `0.5` at the origin (inscribes the default [`CascadeChunk::unit_3d_center_chunk`] cube).
	pub fn unit_sphere() -> Self {
		Self { radius: 0.5, bounds_margin: 0.0 }
	}
}

impl Sdf for Ball {
	fn distance(&self, p: Vec3) -> f32 {
		p.length() - self.radius
	}

	fn bounds(&self) -> Bounds {
		let h = self.radius + self.bounds_margin;
		let min = Vec3::splat(-h);
		let max = Vec3::splat(h);
		Bounds::Cuboid(Aabb3d::from_min_max(min, max))
	}
}

impl NormalizeChunk for Ball {
	fn normalize_chunk(&self, cascade_chunk: &CascadeChunk) -> CascadeChunk {
		let h = self.radius + self.bounds_margin;
		let mu = (h - UNIT_3D_CENTER_HALF).max(0.0) + NUMERIC_SURFACE_EPSILON;
		CascadeChunk::unit_3d_center_chunk().with_res_2(cascade_chunk.res_2).with_mu(mu)
	}
}

impl IdentifiedMesh for Ball {
	fn id(&self) -> MeshId {
		MeshId::new(format!("{:?}", self))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn unit_sphere_inside_negative() -> Result<()> {
		let b = Ball::unit_sphere();
		assert!(b.distance(Vec3::ZERO) < 0.0);
		assert!(b.distance(Vec3::new(0.5, 0.0, 0.0)).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn bounds_cube_half_size() -> Result<()> {
		let b = Ball { radius: 0.4, bounds_margin: 0.02 };
		if let Bounds::Cuboid(a) = b.bounds() {
			let e = 0.42;
			assert!((a.min.x + e).abs() < 1e-5);
			assert!((a.max.x - e).abs() < 1e-5);
		} else {
			panic!("expected cuboid bounds");
		}
		Ok(())
	}
}
