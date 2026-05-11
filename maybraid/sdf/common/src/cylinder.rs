//! Axis-aligned **tapered cylinder** segment (stick / trunk primitive).
//!
//! Local space: **Y** is the axis from [`TaperedCylinder::y_min`] through height [`TaperedCylinder::height`].
//! Radius varies linearly from [`TaperedCylinder::base_radius`] at the bottom of the segment to
//! [`TaperedCylinder::top_radius`] at the top. Flat caps close the ends (same construction as the
//! legacy [`SimpleTrunkSegment`](https://github.com/ramate-io/maybraid) trunk mesh, without noise).
//!
//! Pair with [`NoisySurface`](crate::noisy::NoisySurface) and [`NoiseParams`](procedural_common::NoiseParams)
//! from **`procedural-common`** for the RFC-183 noisy-cylinder composition ([#210](https://github.com/ramate-io/maybraid/issues/210)).

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

/// Tapered cylinder segment aligned with **+Y**, capped at both ends.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TaperedCylinder {
	/// Radius at `y = y_min` (bottom of the segment).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.5))]
	pub base_radius: f32,
	/// Radius at `y = y_min + height` (top of the segment).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.4))]
	pub top_radius: f32,
	/// Lower extent of the finite cylinder along Y.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.0))]
	pub y_min: f32,
	/// Segment length along Y (must be positive).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1.0))]
	pub height: f32,
	/// Extra **uniform** padding on the axis-aligned bounds (e.g. match chunk **mu** or mesh band).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.0))]
	pub bounds_margin: f32,
}

impl Default for TaperedCylinder {
	fn default() -> Self {
		Self { base_radius: 0.5, top_radius: 0.4, y_min: 0.0, height: 1.0, bounds_margin: 0.0 }
	}
}

impl TaperedCylinder {
	/// Unit-height segment `[y_min, y_min + 1]` matching legacy trunk **unit-space** conventions.
	pub fn unit_segment(base_radius: f32, top_radius: f32) -> Self {
		Self { base_radius, top_radius, y_min: 0.0, height: 1.0, bounds_margin: 0.0 }
	}

	fn lateral_shell_distance(&self, p: Vec3) -> f32 {
		let h = self.height.max(1e-6);
		let t = ((p.y - self.y_min) / h).clamp(0.0, 1.0);
		let radius = self.base_radius * (1.0 - t) + self.top_radius * t;
		let xz_dist = (p.x * p.x + p.z * p.z).sqrt();
		xz_dist - radius
	}
}

impl Sdf for TaperedCylinder {
	fn distance(&self, p: Vec3) -> f32 {
		let y = p.y;
		let y0 = self.y_min;
		let y1 = self.y_min + self.height;

		let mut dist = self.lateral_shell_distance(p);

		if y < y0 {
			dist = dist.max(y0 - y);
		} else if y > y1 {
			dist = dist.max(y - y1);
		}

		dist
	}

	fn bounds(&self) -> Bounds {
		let r = self.base_radius.max(self.top_radius);
		let m = self.bounds_margin;
		let min = Vec3::new(-r - m, self.y_min - m, -r - m);
		let max = Vec3::new(r + m, self.y_min + self.height + m, r + m);
		Bounds::Cuboid(Aabb3d::from_min_max(min, max))
	}
}

impl NormalizeChunk for TaperedCylinder {
	fn normalize_chunk(&self, cascade_chunk: &CascadeChunk) -> CascadeChunk {
		let mu = self.bounds_margin + NUMERIC_SURFACE_EPSILON;
		CascadeChunk::unit_center_chunk().with_res_2(cascade_chunk.res_2).with_mu(mu)
	}
}

impl IdentifiedMesh for TaperedCylinder {
	fn id(&self) -> MeshId {
		MeshId::new(format!("{self:?}"))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use bevy::math::bounding::BoundingVolume;

	#[test]
	fn axis_point_inside_leaves_negative_distance() -> Result<()> {
		let c = TaperedCylinder::default();
		let d = c.distance(Vec3::new(0.0, 0.5, 0.0));
		assert!(d < 0.0, "on-axis mid segment should be inside, got {d}");
		Ok(())
	}

	#[test]
	fn far_point_positive_distance() -> Result<()> {
		let c = TaperedCylinder::unit_segment(0.5, 0.4);
		let d = c.distance(Vec3::new(100.0, 0.5, 0.0));
		assert!(d > 0.0);
		Ok(())
	}

	#[test]
	fn bounds_cover_extents() -> Result<()> {
		let c = TaperedCylinder::unit_segment(1.0, 0.5);
		if let Bounds::Cuboid(a) = c.bounds() {
			let hs = a.half_size();
			assert!((hs.x - 1.0).abs() < 1e-5);
			assert!((hs.y - 0.5).abs() < 1e-5);
		} else {
			panic!("expected cuboid bounds");
		}
		Ok(())
	}

	#[test]
	fn bounds_margin_inflates_extents() -> Result<()> {
		let mut c = TaperedCylinder::unit_segment(1.0, 0.5);
		c.bounds_margin = 0.1;
		if let Bounds::Cuboid(a) = c.bounds() {
			let hs = a.half_size();
			assert!((hs.x - 1.1).abs() < 1e-4);
			assert!((hs.y - 0.6).abs() < 1e-4);
		} else {
			panic!("expected cuboid bounds");
		}
		Ok(())
	}
}
