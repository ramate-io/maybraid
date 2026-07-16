use crate::Sdf;
use bevy::prelude::*;
use noise::{NoiseFn, Perlin};

/// A 3D ellipse defining the cross-section shape of the tube.
/// The ellipse lies in a local plane spanned by `axes[0]` and `axes[1]`,
/// centered at `center`.
#[derive(Clone, Copy, Debug)]
pub struct Ellipse3d {
	/// Center of the ellipse
	pub center: Vec3,
	/// Two orthonormal vectors defining the ellipse plane
	pub axes: [Vec3; 2],
	/// Radii along each axis
	pub radii: Vec2,
}

impl Ellipse3d {
	/// Computes the signed distance from a point `p` to the ellipse boundary in 3D space.
	/// Negative inside, positive outside.
	pub fn distance(&self, p: Vec3) -> f32 {
		let local = p - self.center;
		let x = local.dot(self.axes[0]) / self.radii.x;
		let y = local.dot(self.axes[1]) / self.radii.y;
		(x * x + y * y).sqrt() - 1.0
	}

	/// Returns the closest point on the ellipse's plane (projected point).
	pub fn project_to_plane(&self, p: Vec3) -> Vec3 {
		let normal = self.axes[0].cross(self.axes[1]).normalize();
		p - normal * normal.dot(p - self.center)
	}
}

/// A tube SDF with elliptical cross-section.
/// Supports end rounding, Perlin noise surface perturbation, and flanging.
#[derive(Clone, Debug)]
pub struct TubeSdf {
	/// Tube start point (axis)
	pub ray_start: Vec3,
	/// Tube end point (axis)
	pub ray_end: Vec3,
	/// Base cross-section shape
	pub ellipse: Ellipse3d,
	/// Optional surface noise
	pub noise: Option<Perlin>,
	/// The noise factor
	pub noise_factor: f32,
	/// Rounding radius at the ends
	pub end_rounding: f32,
	/// Flanging coefficient — how much radii expand or contract near ends.
	/// Positive -> wider at ends, negative -> narrower at ends.
	pub flanging: f32,
}

impl TubeSdf {
	pub fn new(ray_start: Vec3, ray_end: Vec3, ellipse: Ellipse3d) -> Self {
		Self {
			ray_start,
			ray_end,
			ellipse,
			noise: None,
			noise_factor: 0.0,
			end_rounding: 0.0,
			flanging: 0.0,
		}
	}

	pub fn with_noise(mut self, noise: Perlin) -> Self {
		self.noise = Some(noise);
		self
	}

	pub fn with_noise_factor(mut self, noise_factor: f32) -> Self {
		self.noise_factor = noise_factor;
		self
	}

	pub fn with_end_rounding(mut self, rounding: f32) -> Self {
		self.end_rounding = rounding;
		self
	}

	pub fn with_flanging(mut self, flanging: f32) -> Self {
		self.flanging = flanging;
		self
	}

	/// Helper: builds an orthonormal basis perpendicular to the tube axis.
	fn orthonormal_basis(dir: Vec3) -> [Vec3; 2] {
		let right = if dir.x.abs() > dir.z.abs() {
			Vec3::new(-dir.y, dir.x, 0.0).normalize()
		} else {
			Vec3::new(0.0, -dir.z, dir.y).normalize()
		};
		let up = dir.cross(right).normalize();
		[right, up]
	}
}

impl Sdf for TubeSdf {
	fn distance(&self, p: Vec3) -> f32 {
		// Axis and projection
		let ray = self.ray_end - self.ray_start;
		let len = ray.length();
		let dir = ray / len;

		let t = (p - self.ray_start).dot(dir).clamp(0.0, len);
		let normalized_t = t / len; // [0, 1] position along axis
		let point_on_axis = self.ray_start + dir * t;

		// Compute flanging factor: quadratic taper near both ends.
		// Example: if flanging = 0.2 → radii expand by 20% at both ends.
		let flange_factor = 1.0 + self.flanging * (1.0 - (2.0 * normalized_t - 1.0).powi(2));

		// Orthonormal basis for cross-section
		let [right, up] = Self::orthonormal_basis(dir);

		// Apply flanging to radii
		let scaled_radii = self.ellipse.radii * flange_factor;

		// Build local ellipse for this cross-section
		let ellipse = Ellipse3d { center: point_on_axis, axes: [right, up], radii: scaled_radii };

		// Distance to elliptical boundary
		let side_dist = ellipse.distance(p);

		// Distance along axis (for end rounding)
		let cap_dist = (t - len * 0.5).abs() - len * 0.5 + self.end_rounding;

		// Combine distances (elliptical capsule logic)
		let outside_dist = (side_dist.max(0.0).powi(2) + cap_dist.max(0.0).powi(2)).sqrt();
		let inside_dist = side_dist.max(cap_dist);
		let mut sdf = if inside_dist < 0.0 { inside_dist } else { outside_dist };

		// Optional noise perturbation
		if let Some(noise) = &self.noise {
			let nval = noise.get([
				p.x as f64 * self.noise_factor as f64,
				p.y as f64 * self.noise_factor as f64,
				p.z as f64 * self.noise_factor as f64,
			]) as f32;
			sdf += nval * self.noise_factor; // subtle surface variation
		}

		sdf
	}
}
