//! **Crook cylinder** — tapered segment with a smooth sinusoidal centerline ([RFC-183 §3.1.1.2](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/01-stick-and-stalk-components/02-crook-cylinder/README.md), [#211](https://github.com/ramate-io/maybraid/issues/211)).
//!
//! Local space matches [`crate::cylinder::TaperedCylinder`]: **Y** runs along the nominal axis from [`CrookCylinder::y_min`] through [`CrookCylinder::height`]. The spine is offset in **X** and **Z** by
//! `bend_x * sin(π t + φ_x)` and `bend_z * sin(π t + φ_z)` with `t ∈ [0,1]` along the segment.
//!
//! Pair with [`crate::noisy::NoisySurface`] for the RFC **noisy crook cylinder** composition ([`NoisyCrookCylinder`](crate::noisy::NoisyCrookCylinder)).

use std::f32::consts::PI;

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use procedural_common::NUMERIC_SURFACE_EPSILON;
use render_item::NormalizeChunk;
use sdf::{Bounds, Sdf};

use crate::cylinder::TaperedCylinder;

/// Extra horizontal half-extent (beyond `|bend| + max_radius`) so tilted tube cross-sections and the
/// approximate closest-spine field do not clip marching-cubes meshes.
const BOUNDS_XZ_SLACK_PER_RADIUS: f32 = 0.55;
/// Additional slack on **each** of X and Z tied to total bend magnitude (coupled bulge).
const BOUNDS_XZ_SLACK_PER_BEND_SUM: f32 = 0.2;

/// Tapered cylinder segment with a smooth bent centerline (capped in **world Y** like [`TaperedCylinder`]).
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CrookCylinder {
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
	/// Extra **uniform** padding on the axis-aligned bounds.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.0))]
	pub bounds_margin: f32,
	/// Bend amplitude in **X** (same units as radius; multiplies `sin(π t + phase_x)`).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.0))]
	pub bend_x: f32,
	/// Bend amplitude in **Z**.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.0))]
	pub bend_z: f32,
	/// Phase offset for the X bend (radians).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.0))]
	pub phase_x: f32,
	/// Phase offset for the Z bend (radians).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.0))]
	pub phase_z: f32,
}

impl Default for CrookCylinder {
	fn default() -> Self {
		Self {
			base_radius: 0.5,
			top_radius: 0.4,
			y_min: 0.0,
			height: 1.0,
			bounds_margin: 0.0,
			bend_x: 0.0,
			bend_z: 0.0,
			phase_x: 0.0,
			phase_z: 0.0,
		}
	}
}

impl CrookCylinder {
	/// Unit-height segment `[y_min, y_min + 1]` with straight spine (same as [`TaperedCylinder::unit_segment`] when bends are zero).
	pub fn unit_segment(base_radius: f32, top_radius: f32) -> Self {
		Self {
			base_radius,
			top_radius,
			y_min: 0.0,
			height: 1.0,
			bounds_margin: 0.0,
			bend_x: 0.0,
			bend_z: 0.0,
			phase_x: 0.0,
			phase_z: 0.0,
		}
	}

	/// Same radii / height / `y_min` / margin as `taper`, with explicit bend parameters.
	pub fn from_tapered(taper: TaperedCylinder, bend_x: f32, bend_z: f32, phase_x: f32, phase_z: f32) -> Self {
		Self {
			base_radius: taper.base_radius,
			top_radius: taper.top_radius,
			y_min: taper.y_min,
			height: taper.height,
			bounds_margin: taper.bounds_margin,
			bend_x,
			bend_z,
			phase_x,
			phase_z,
		}
	}

	/// Axis-aligned half-width in **X** and **Z** for [`Sdf::bounds`] (liberal envelope around the bent tube).
	fn bounds_xz_half_extents(&self) -> (f32, f32) {
		let r = self.base_radius.max(self.top_radius);
		let m = self.bounds_margin;
		let bx = self.bend_x.abs();
		let bz = self.bend_z.abs();
		let slack_r = r * BOUNDS_XZ_SLACK_PER_RADIUS;
		let slack_bend = BOUNDS_XZ_SLACK_PER_BEND_SUM * (bx + bz);
		let half_x = bx + r + slack_r + slack_bend + m;
		let half_z = bz + r + slack_r + slack_bend + m;
		(half_x, half_z)
	}

	/// Centerline position in world space for normalized parameter `t ∈ [0, 1]`.
	#[inline]
	pub fn centerline(&self, t: f32) -> Vec3 {
		let u = t.clamp(0.0, 1.0);
		let y = self.y_min + u * self.height;
		let a = PI * u;
		Vec3::new(
			self.bend_x * (a + self.phase_x).sin(),
			y,
			self.bend_z * (a + self.phase_z).sin(),
		)
	}

	/// Derivative `dγ/du` for `u ∈ [0, 1]`.
	#[inline]
	fn centerline_dt(&self, u: f32) -> Vec3 {
		let u = u.clamp(0.0, 1.0);
		let a = PI * u;
		let ca = (a + self.phase_x).cos() * PI;
		let cz = (a + self.phase_z).cos() * PI;
		Vec3::new(self.bend_x * ca, self.height, self.bend_z * cz)
	}

	/// Radius at normalized height `u ∈ [0, 1]`.
	#[inline]
	fn radius_at(&self, u: f32) -> f32 {
		let u = u.clamp(0.0, 1.0);
		self.base_radius * (1.0 - u) + self.top_radius * u
	}

	/// Approximate closest `u ∈ [0,1]` minimizing `‖p − γ(u)‖²` (fixed iterations, deterministic).
	fn closest_u(&self, p: Vec3) -> f32 {
		let h = self.height.max(1e-6);
		let mut u = ((p.y - self.y_min) / h).clamp(0.0, 1.0);
		for _ in 0..12 {
			let c = self.centerline(u);
			let g = self.centerline_dt(u);
			let denom = g.dot(g).max(1e-20);
			u = (u + (p - c).dot(g) / denom).clamp(0.0, 1.0);
		}
		u
	}

	fn lateral_shell_distance(&self, p: Vec3) -> f32 {
		let u = self.closest_u(p);
		let c = self.centerline(u);
		let g = self.centerline_dt(u);
		let gl = g.length();
		let tangent = if gl > 1e-8 { g / gl } else { Vec3::Y };
		let w = p - c;
		let along = w.dot(tangent);
		let perp = (w - tangent * along).length();
		perp - self.radius_at(u)
	}
}

impl Sdf for CrookCylinder {
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
		let m = self.bounds_margin;
		let (half_x, half_z) = self.bounds_xz_half_extents();
		let min = Vec3::new(-half_x, self.y_min - m, -half_z);
		let max = Vec3::new(half_x, self.y_min + self.height + m, half_z);
		Bounds::Cuboid(Aabb3d::from_min_max(min, max))
	}
}

impl NormalizeChunk for CrookCylinder {
	fn normalize_chunk(&self, cascade_chunk: &CascadeChunk) -> CascadeChunk {
		let mu = self.bounds_margin + NUMERIC_SURFACE_EPSILON;
		CascadeChunk::unit_center_chunk().with_res_2(cascade_chunk.res_2).with_mu(mu)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn zero_bend_matches_tapered_on_axis() -> Result<()> {
		let taper = TaperedCylinder::unit_segment(0.5, 0.4);
		let crook = CrookCylinder::from_tapered(taper, 0.0, 0.0, 0.0, 0.0);
		let p = Vec3::new(0.0, 0.5, 0.0);
		let dt = (crook.distance(p) - taper.distance(p)).abs();
		assert!(dt < 5e-3, "expected near match with straight taper, got delta {dt}");
		Ok(())
	}

	#[test]
	fn bent_spine_offset_inside() -> Result<()> {
		let c = CrookCylinder {
			bend_x: 0.1,
			bend_z: -0.05,
			phase_x: 0.3,
			phase_z: -0.7,
			..CrookCylinder::unit_segment(0.5, 0.4)
		};
		let u = 0.5_f32;
		let spine = c.centerline(u);
		let g = c.centerline_dt(u);
		let t = if g.length_squared() > 1e-12 {
			g.normalize()
		} else {
			Vec3::Y
		};
		let mut perp = t.cross(Vec3::Y);
		if perp.length_squared() < 1e-12 {
			perp = t.cross(Vec3::X);
		}
		let perp = perp.normalize();
		let r = c.base_radius * 0.5 + c.top_radius * 0.5;
		let p = spine + perp * (r * 0.5);
		let d = c.distance(p);
		assert!(d < 0.0, "point offset from spine inside radius should be negative, got {d}");
		Ok(())
	}

	#[test]
	fn far_point_outside() -> Result<()> {
		let c = CrookCylinder::unit_segment(0.5, 0.4);
		let d = c.distance(Vec3::new(50.0, 0.5, 0.0));
		assert!(d > 0.0);
		Ok(())
	}

	#[test]
	fn bounds_xz_liberal_vs_tight_envelope() -> Result<()> {
		let c = CrookCylinder {
			bend_x: 0.15,
			bend_z: 0.1,
			..CrookCylinder::unit_segment(0.5, 0.4)
		};
		let r = c.base_radius.max(c.top_radius);
		let tight = r + c.bend_x.abs() + c.bounds_margin;
		let (half_x, _) = c.bounds_xz_half_extents();
		assert!(
			half_x > tight + r * 0.4,
			"expected liberal XZ bounds (got {half_x}, tight spine+radius {tight})"
		);
		Ok(())
	}
}
