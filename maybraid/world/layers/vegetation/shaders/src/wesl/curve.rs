//! CPU mirror of `curve.wesl`.

use bevy::math::Vec3;

/// Right-handed local frame: X across, Y along, Z facing. World metres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocalFrame {
	pub across: Vec3,
	pub along: Vec3,
	pub facing: Vec3,
}

impl LocalFrame {
	pub fn as_matrix_columns(self) -> [Vec3; 3] {
		[self.across, self.along, self.facing]
	}
}

/// Sampled curve point used by grass ribbons and later bush attachments.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurveSample {
	pub position: Vec3,
	pub tangent: Vec3,
	pub frame: LocalFrame,
}

impl LocalFrame {
	/// Right-handed frame from a tangent: X across, Y along, Z facing.
	pub fn from_tangent(tangent: Vec3, up_hint: Vec3) -> Self {
		let mut along = tangent;
		if along.length() < 1e-5 {
			along = up_hint;
		}
		let along = along.normalize_or_zero();
		let along = if along.length_squared() < 1e-8 { Vec3::Y } else { along };
		let mut reference = up_hint;
		if along.dot(reference).abs() > 0.94 {
			reference = Vec3::X;
			if along.dot(reference).abs() > 0.94 {
				reference = Vec3::Z;
			}
		}
		let across = reference.cross(along).normalize_or_zero();
		let across = if across.length_squared() < 1e-8 { Vec3::X } else { across };
		let facing = across.cross(along);
		Self { across, along, facing }
	}
}

pub fn stable_frame(tangent: Vec3, up_hint: Vec3) -> LocalFrame {
	LocalFrame::from_tangent(tangent, up_hint)
}

pub fn attachment_frame(sample: CurveSample, twist_radians: f32) -> LocalFrame {
	let (s, c) = twist_radians.sin_cos();
	let across = sample.frame.across * c + sample.frame.facing * s;
	let facing = -sample.frame.across * s + sample.frame.facing * c;
	LocalFrame {
		across: across.normalize_or_zero(),
		along: sample.frame.along,
		facing: facing.normalize_or_zero(),
	}
}

pub fn blade_width_at(width_m: f32, taper: f32, t: f32) -> f32 {
	let tt = t.clamp(0.0, 1.0);
	let profile = (1.0 - tt).powf(taper.max(0.15));
	(width_m * profile).max(1e-4)
}

pub fn blade_rest_sample(
	root: Vec3,
	up: Vec3,
	bend_dir: Vec3,
	height_m: f32,
	bend: f32,
	t: f32,
) -> CurveSample {
	let tt = t.clamp(0.0, 1.0);
	let upn = up.normalize_or_zero();
	let upn = if upn.length_squared() < 1e-8 { Vec3::Y } else { upn };
	let mut sideways = bend_dir - upn * bend_dir.dot(upn);
	if sideways.length() < 1e-5 {
		sideways = Vec3::new(upn.z, 0.0, -upn.x);
		if sideways.length() < 1e-5 {
			sideways = Vec3::X;
		}
	}
	let sideways = sideways.normalize_or_zero();
	let sag_k = 0.16 + 0.58 * (bend * 0.72).clamp(0.0, 1.0);
	let rise = tt - sag_k * tt * tt * tt;
	let sweep = tt * tt * (0.42 + 0.58 * tt);
	let position = root + upn * (height_m * rise) + sideways * (bend * height_m * sweep);
	let d_rise = 1.0 - 3.0 * sag_k * tt * tt;
	let d_sweep = 2.0 * 0.42 * tt + 3.0 * 0.58 * tt * tt;
	let tangent = upn * (height_m * d_rise) + sideways * (bend * height_m * d_sweep);
	let frame = stable_frame(tangent, upn);
	CurveSample { position, tangent: frame.along, frame }
}

pub fn ribbon_vertex(sample: CurveSample, side: f32, half_width_m: f32) -> Vec3 {
	sample.position + sample.frame.across * (side * half_width_m)
}

pub fn ribbon_normal(sample: CurveSample, side: f32) -> Vec3 {
	(sample.frame.facing * 0.45 + sample.frame.across * (side * 0.10) + Vec3::Y * 0.85)
		.normalize_or_zero()
}

pub fn branch_radius_at(root_radius_m: f32, tip_radius_m: f32, t: f32) -> f32 {
	let tt = t.clamp(0.0, 1.0);
	(root_radius_m + (tip_radius_m - root_radius_m) * tt).max(1e-4)
}

pub fn branch_rest_sample(p0: Vec3, p1: Vec3, p2: Vec3, t: f32) -> CurveSample {
	let tt = t.clamp(0.0, 1.0);
	let u = 1.0 - tt;
	let position = p0 * (u * u) + p1 * (2.0 * u * tt) + p2 * (tt * tt);
	let mut tangent = (p1 - p0) * (2.0 * u) + (p2 - p1) * (2.0 * tt);
	if tangent.length() < 1e-5 {
		tangent = p2 - p0;
	}
	let frame = stable_frame(tangent, Vec3::Y);
	CurveSample { position, tangent: frame.along, frame }
}

pub fn woody_surface_vertex(sample: CurveSample, radius_m: f32, radial: f32) -> Vec3 {
	let angle = radial * std::f32::consts::TAU;
	let (sin_a, cos_a) = angle.sin_cos();
	let dir = sample.frame.across * cos_a + sample.frame.facing * sin_a;
	sample.position + dir * radius_m
}

pub fn woody_surface_normal(sample: CurveSample, radial: f32) -> Vec3 {
	let angle = radial * std::f32::consts::TAU;
	let (sin_a, cos_a) = angle.sin_cos();
	(sample.frame.across * cos_a + sample.frame.facing * sin_a).normalize_or_zero()
}

/// Ovate / teardrop width: narrow petiole, fullest near `v = 0.38`, rounded tip.
pub fn leaf_teardrop_width(v: f32) -> f32 {
	let vv = v.clamp(0.0, 1.0);
	let lower = smoothstep(0.0, 0.18, vv);
	let upper = 1.0 - 0.70 * smoothstep(0.48, 0.98, vv);
	(lower * upper).max(0.16 * smoothstep(0.90, 0.98, vv))
}

pub fn leaf_surface_vertex(
	origin: Vec3,
	frame: LocalFrame,
	size_m: f32,
	u: f32,
	v: f32,
	fold: f32,
) -> Vec3 {
	let vv = v.clamp(0.0, 1.0);
	let half = size_m * 0.50 * leaf_teardrop_width(vv);
	let across = frame.across * (u * half);
	let along = frame.along * (vv * size_m);
	let cup = frame.facing * (fold * u * u * size_m * 0.14 + fold * vv * (1.0 - vv) * size_m * 0.26);
	origin + across + along + cup
}

pub fn leaf_surface_normal(frame: LocalFrame, u: f32, fold: f32) -> Vec3 {
	(frame.facing + frame.across * (u * 0.18) + frame.along * (fold * 0.12) + Vec3::Y * 0.35)
		.normalize_or_zero()
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
	let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
	t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn degenerate_tangent_still_yields_unit_frame() -> anyhow::Result<()> {
		let frame = stable_frame(Vec3::ZERO, Vec3::Y);
		assert!(frame.along.length() > 0.5);
		assert!((frame.across.length() - 1.0).abs() < 1e-3);
		assert!(frame.across.cross(frame.along).dot(frame.facing) > 0.0);
		Ok(())
	}

	#[test]
	fn nearly_straight_up_does_not_twist_wildly() -> anyhow::Result<()> {
		let a = stable_frame(Vec3::new(0.001, 1.0, 0.0), Vec3::Y);
		let b = stable_frame(Vec3::new(-0.001, 1.0, 0.0), Vec3::Y);
		assert!(a.across.dot(b.across) > 0.0);
		Ok(())
	}

	#[test]
	fn ribbon_is_rooted_at_t0() -> anyhow::Result<()> {
		let root = Vec3::new(-4.0, 2.0, 9.0);
		let sample = blade_rest_sample(root, Vec3::Y, Vec3::X, 0.6, 0.25, 0.0);
		let left = ribbon_vertex(sample, -1.0, 0.02);
		let right = ribbon_vertex(sample, 1.0, 0.02);
		assert!((left.y - root.y).abs() < 1e-4);
		assert!((right.y - root.y).abs() < 1e-4);
		assert!((0.5 * (left + right) - root).length() < 1e-3);
		Ok(())
	}

	#[test]
	fn attachment_frame_keeps_tangent() -> anyhow::Result<()> {
		let sample = blade_rest_sample(Vec3::ZERO, Vec3::Y, Vec3::X, 1.0, 0.2, 0.4);
		let twisted = attachment_frame(sample, 0.7);
		assert!((twisted.along - sample.frame.along).length() < 1e-5);
		Ok(())
	}

	#[test]
	fn high_bend_arches_the_tip_over() -> anyhow::Result<()> {
		let mid = blade_rest_sample(Vec3::ZERO, Vec3::Y, Vec3::X, 1.0, 1.15, 0.62);
		let tip = blade_rest_sample(Vec3::ZERO, Vec3::Y, Vec3::X, 1.0, 1.15, 1.0);
		assert!(tip.position.y < mid.position.y);
		assert!(tip.position.x > mid.position.x);
		assert!(tip.frame.along.y < 0.15);
		Ok(())
	}

	#[test]
	fn rest_tangent_at_root_follows_up() -> anyhow::Result<()> {
		let sample = blade_rest_sample(Vec3::ZERO, Vec3::Y, Vec3::X, 0.6, 0.9, 0.0);
		assert!(sample.frame.along.dot(Vec3::Y) > 0.95);
		Ok(())
	}

	#[test]
	fn branch_radius_tapers_toward_the_tip() -> anyhow::Result<()> {
		let root = branch_radius_at(0.04, 0.01, 0.0);
		let tip = branch_radius_at(0.04, 0.01, 1.0);
		assert!((root - 0.04).abs() < 1e-5);
		assert!((tip - 0.01).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn branch_rest_sample_is_rooted_and_reaches_the_tip() -> anyhow::Result<()> {
		let p0 = Vec3::new(1.0, 0.0, 2.0);
		let p2 = Vec3::new(1.2, 0.8, 2.1);
		let p1 = (p0 + p2) * 0.5 + Vec3::X * 0.1;
		let root = branch_rest_sample(p0, p1, p2, 0.0);
		let tip = branch_rest_sample(p0, p1, p2, 1.0);
		assert!((root.position - p0).length() < 1e-4);
		assert!((tip.position - p2).length() < 1e-4);
		Ok(())
	}

	#[test]
	fn woody_tube_sits_on_the_curve() -> anyhow::Result<()> {
		let sample = branch_rest_sample(Vec3::ZERO, Vec3::new(0.0, 0.5, 0.1), Vec3::Y, 0.4);
		let a = woody_surface_vertex(sample, 0.05, 0.0);
		let b = woody_surface_vertex(sample, 0.05, 0.5);
		assert!(((a - sample.position).length() - 0.05).abs() < 1e-3);
		assert!(((b - sample.position).length() - 0.05).abs() < 1e-3);
		assert!(woody_surface_normal(sample, 0.0).length() > 0.5);
		Ok(())
	}

	#[test]
	fn leaf_base_stays_on_the_attachment() -> anyhow::Result<()> {
		let sample = branch_rest_sample(Vec3::ZERO, Vec3::Y * 0.5, Vec3::Y, 0.6);
		let frame = attachment_frame(sample, 0.3);
		let base = leaf_surface_vertex(sample.position, frame, 0.08, 0.0, 0.0, 0.4);
		assert!((base - sample.position).length() < 1e-4);
		Ok(())
	}

	#[test]
	fn teardrop_is_widest_in_the_middle_and_rounded_at_the_tip() -> anyhow::Result<()> {
		assert!(leaf_teardrop_width(0.38) > leaf_teardrop_width(0.08));
		assert!(leaf_teardrop_width(0.38) > leaf_teardrop_width(0.92));
		assert!(leaf_teardrop_width(1.0) > 0.10);
		let sample = branch_rest_sample(Vec3::ZERO, Vec3::Y * 0.5, Vec3::Y, 0.5);
		let frame = sample.frame;
		let across = |u: f32, v: f32| {
			leaf_surface_vertex(sample.position, frame, 1.0, u, v, 0.0)
				.dot(frame.across)
				.abs()
		};
		assert!(across(1.0, 0.38) > across(1.0, 0.12));
		assert!(across(1.0, 0.38) > across(1.0, 0.95));
		Ok(())
	}
}
