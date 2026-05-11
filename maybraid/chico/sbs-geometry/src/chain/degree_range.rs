//! Direction blending and angular perturbation (degrees of freedom around a mean ray).

use bevy_math::Vec3;

/// Blend incoming growth direction toward `bias_ray` with weight `t` in `[0, 1]` (`1` = use bias only).
pub fn blend_direction(incoming_ray: Vec3, bias_ray: Vec3, t: f32) -> Vec3 {
	let t = t.clamp(0.0, 1.0);
	let bias = bias_ray.normalize_or_zero();
	let bias = if bias.length_squared() < 1e-12 { Vec3::Y } else { bias };
	if t >= 1.0 - 1e-6 {
		return bias;
	}
	let prev = incoming_ray.normalize_or_zero();
	let prev = if prev.length_squared() < 1e-12 { Vec3::Y } else { prev };
	prev.slerp(bias, t).normalize_or_zero()
}

/// Small tangent-space jitter in radians around `mean` using noise samples `u`, `v` in roughly `[-1, 1]`.
pub fn perturb_direction(mean: Vec3, dof_rad: f32, u: f32, v: f32) -> Vec3 {
	let mean = mean.normalize_or_zero();
	let mean = if mean.length_squared() < 1e-12 { Vec3::Y } else { mean };
	let up = if mean.y.abs() < 0.99 { Vec3::Y } else { Vec3::X };
	let mut tangent = mean.cross(up);
	if tangent.length_squared() < 1e-12 {
		tangent = mean.cross(Vec3::Z);
	}
	tangent = tangent.normalize_or_zero();
	let bitangent = mean.cross(tangent).normalize_or_zero();
	let d = dof_rad.max(0.0);
	(mean + tangent * (d * u) + bitangent * (d * v)).normalize_or_zero()
}
