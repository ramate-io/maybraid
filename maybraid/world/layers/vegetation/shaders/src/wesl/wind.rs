//! CPU mirror of `wind.wesl`.

use bevy::math::{Vec2, Vec3};

/// Explicit wind field inputs (world metres, seconds).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindInputs {
	pub direction_xz: Vec2,
	pub speed: f32,
	pub strength_m: f32,
	pub time_s: f32,
}

impl WindInputs {
	pub fn gentle_botw(time_s: f32) -> Self {
		Self { direction_xz: Vec2::new(0.92, 0.38), speed: 0.85, strength_m: 0.22, time_s }
	}

	/// Stronger travelling gust for a still-frame capture.
	pub fn botw_gust(time_s: f32) -> Self {
		Self { direction_xz: Vec2::new(0.92, 0.38), speed: 1.15, strength_m: 0.58, time_s }
	}
}

/// Sampled XZ displacement in metres (pre-stiffness).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindField {
	pub displacement_xz: Vec2,
}

impl WindField {
	pub fn sample(root_xz: Vec2, inputs: WindInputs) -> Self {
		Self { displacement_xz: wind_field(root_xz, inputs) }
	}
}

/// Deformed world-space point after root-anchored wind.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindDeform {
	pub position: Vec3,
}

impl WindDeform {
	pub fn along_curve(rest: Vec3, root: Vec3, t: f32, wind_xz: Vec2, stiffness: f32) -> Self {
		Self { position: deform_along_curve(rest, root, t, wind_xz, stiffness) }
	}
}

pub fn wind_field(root_xz: Vec2, inputs: WindInputs) -> Vec2 {
	let mut dir = inputs.direction_xz;
	if dir.length() < 1e-5 {
		dir = Vec2::new(0.92, 0.38);
	}
	let dir = dir.normalize_or_zero();
	let travelling = root_xz.dot(Vec2::new(0.058, 0.034)) - inputs.time_s * inputs.speed;
	let gust = 0.72 * travelling.sin() + 0.28 * (travelling * 2.3 - inputs.time_s * 1.7).sin();
	let wave = (root_xz.dot(dir) * 0.08 - inputs.time_s * 0.55).sin();
	dir * (inputs.strength_m * (0.65 * gust + 0.35 * wave))
}

pub fn max_wind_displacement_m(height_m: f32, strength_m: f32, stiffness: f32) -> f32 {
	height_m * strength_m * (1.0 - stiffness).clamp(0.0, 1.0)
}

pub fn deform_along_curve(rest: Vec3, root: Vec3, t: f32, wind_xz: Vec2, stiffness: f32) -> Vec3 {
	let along = t.clamp(0.0, 1.0);
	if along <= 1e-4 {
		return root;
	}
	let weight = along * along * (1.0 - stiffness).clamp(0.0, 1.0);
	rest + Vec3::new(wind_xz.x * weight, -wind_xz.length() * weight * 0.12, wind_xz.y * weight)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn root_stays_pinned() -> anyhow::Result<()> {
		let root = Vec3::new(3.0, 1.5, -8.0);
		let rest = root;
		let deformed = deform_along_curve(rest, root, 0.0, Vec2::new(4.0, 1.0), 0.0);
		assert_eq!(deformed, root);
		Ok(())
	}

	#[test]
	fn conservative_bound_covers_tip_displacement() -> anyhow::Result<()> {
		let height = 0.7_f32;
		let inputs = WindInputs::gentle_botw(1.7);
		let wind = wind_field(Vec2::new(12.0, -3.0), inputs);
		let root = Vec3::new(12.0, 0.0, -3.0);
		let rest_tip = root + Vec3::Y * height;
		let tip = deform_along_curve(rest_tip, root, 1.0, wind, 0.0);
		let bound = max_wind_displacement_m(height, inputs.strength_m, 0.0);
		let lateral = Vec2::new(tip.x - root.x, tip.z - root.z).length();
		assert!(lateral <= bound + 1e-4, "lateral {lateral} bound {bound}");
		Ok(())
	}

	#[test]
	fn wood_stiffness_reduces_motion() -> anyhow::Result<()> {
		let root = Vec3::ZERO;
		let rest = Vec3::Y;
		let grass = deform_along_curve(rest, root, 1.0, Vec2::new(0.4, 0.0), 0.0);
		let wood = deform_along_curve(rest, root, 1.0, Vec2::new(0.4, 0.0), 0.75);
		assert!((wood - rest).length() < (grass - rest).length());
		Ok(())
	}
}
