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
}
