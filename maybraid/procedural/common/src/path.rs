//! Noisy 3D polylines with per-axis allowed angles.
//!
//! Heading is tracked as world **yaw** (about \(+Y\)) and **pitch** (elevation from the
//! horizontal \(XZ\) plane). [`AllowedAngles::x`] is the max absolute pitch from
//! horizontal (the vertical angle you can set); [`AllowedAngles::y`] / [`z`] bound
//! per-step yaw / roll deltas. Each step also samples a length from
//! [`NoisyPathParams::step_len`].

use bevy_math::Vec3;

use crate::noise::{NoiseConfig, NoiseParams};

/// Inclusive segment-length range sampled each step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepLenRange {
	pub min: f32,
	pub max: f32,
}

impl StepLenRange {
	pub fn new(min: f32, max: f32) -> Self {
		let min = min.max(1e-4);
		let max = max.max(min);
		Self { min, max }
	}

	pub fn exact(len: f32) -> Self {
		Self::new(len, len)
	}

	fn sample(self, noise: &NoiseConfig, pos: Vec3, step_i: u32) -> f32 {
		if (self.max - self.min).abs() < 1e-8 {
			return self.min;
		}
		noise.sample_range_f32_4d(self.min, self.max, pos.x, pos.y, pos.z, step_i as f32 + 71.0)
	}
}

impl Default for StepLenRange {
	fn default() -> Self {
		Self::exact(1.0)
	}
}

/// Allowed angles (radians) for the noisy walk.
///
/// - [`Self::x`]: max **absolute pitch** from horizontal (vertical angle), and max
///   per-step \(|\Delta\mathrm{pitch}|\)
/// - [`Self::y`]: max per-step \(|\Delta\mathrm{yaw}|\) about world \(+Y\)
/// - [`Self::z`]: max per-step \(|\Delta\mathrm{roll}|\) (sampled; unused for positions)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AllowedAngles {
	pub x: f32,
	pub y: f32,
	pub z: f32,
}

impl AllowedAngles {
	pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };

	pub fn new(x: f32, y: f32, z: f32) -> Self {
		Self { x: x.max(0.0), y: y.max(0.0), z: z.max(0.0) }
	}

	/// Plan-turn only (yaw), useful for flat façades.
	pub fn yaw_only(max_yaw: f32) -> Self {
		Self::new(0.0, max_yaw.max(0.0), 0.0)
	}

	/// Yaw + pitch (no roll), useful for vertical joinery tests.
	pub fn yaw_pitch(max_yaw: f32, max_pitch: f32) -> Self {
		Self::new(max_pitch.max(0.0), max_yaw.max(0.0), 0.0)
	}

	/// Sample per-step \((\Delta\mathrm{pitch}, \Delta\mathrm{yaw}, \Delta\mathrm{roll})\).
	pub fn sample_turn_deltas(
		&self,
		noise: &NoiseConfig,
		pos: Vec3,
		step_i: u32,
	) -> (f32, f32, f32) {
		let s = step_i as f32;
		// Distinct w-salts so axes are independent under the same seed.
		let d_pitch = if self.x > 1e-8 {
			noise.sample_range_f32_4d(-self.x, self.x, pos.x, pos.y, pos.z, s + 11.0)
		} else {
			0.0
		};
		let d_yaw = if self.y > 1e-8 {
			noise.sample_range_f32_4d(-self.y, self.y, pos.x, pos.y, pos.z, s + 29.0)
		} else {
			0.0
		};
		let d_roll = if self.z > 1e-8 {
			noise.sample_range_f32_4d(-self.z, self.z, pos.x, pos.y, pos.z, s + 47.0)
		} else {
			0.0
		};
		(d_pitch, d_yaw, d_roll)
	}
}

impl Default for AllowedAngles {
	fn default() -> Self {
		Self::yaw_pitch(std::f32::consts::FRAC_PI_6, std::f32::consts::FRAC_PI_8)
	}
}

/// Parameters for [`noisy_path`].
#[derive(Debug, Clone, PartialEq)]
pub struct NoisyPathParams {
	pub start: Vec3,
	/// Initial tangent (normalized internally). Zero falls back to \(+Z\).
	pub initial_dir: Vec3,
	/// Total arc-length budget to spend.
	pub distance: f32,
	/// Inclusive segment length range sampled each step (last step may be shorter).
	pub step_len: StepLenRange,
	/// Allowed pitch / yaw / roll (see [`AllowedAngles`]).
	pub allowed_angles: AllowedAngles,
	/// Noise used to pick turns and step lengths (seed lives here).
	pub noise: NoiseParams,
}

impl Default for NoisyPathParams {
	fn default() -> Self {
		Self {
			start: Vec3::ZERO,
			initial_dir: Vec3::Z,
			distance: 12.0,
			step_len: StepLenRange::default(),
			allowed_angles: AllowedAngles::default(),
			noise: NoiseParams::default(),
		}
	}
}

impl NoisyPathParams {
	/// Build a noisy polyline through 3D space.
	///
	/// Returns at least two points when `distance > 0`; otherwise a single point at `start`.
	pub fn generate(&self) -> Vec<Vec3> {
		let distance = self.distance.max(0.0);
		let step_len = StepLenRange::new(self.step_len.min, self.step_len.max);
		let start = self.start;
		if distance < 1e-6 {
			return vec![start];
		}

		let initial = if self.initial_dir.length_squared() > 1e-12 {
			self.initial_dir.normalize()
		} else {
			Vec3::Z
		};

		let noise = NoiseConfig::new(self.noise);
		let allowed =
			AllowedAngles::new(self.allowed_angles.x, self.allowed_angles.y, self.allowed_angles.z);

		let (mut yaw, mut pitch) = yaw_pitch_from_dir(initial);
		pitch = pitch.clamp(-allowed.x, allowed.x);

		let mut pos = start;
		let mut points = vec![start];
		let mut spent = 0.0;
		let mut step_i = 0u32;

		while spent + 1e-5 < distance {
			let remaining = distance - spent;
			let advance = remaining.min(step_len.sample(&noise, pos, step_i));

			let (d_pitch, d_yaw, _d_roll) = allowed.sample_turn_deltas(&noise, pos, step_i);
			yaw += d_yaw;
			pitch = (pitch + d_pitch).clamp(-allowed.x, allowed.x);

			let dir = dir_from_yaw_pitch(yaw, pitch);
			pos += dir * advance;
			points.push(pos);
			spent += advance;
			step_i = step_i.saturating_add(1);

			if step_i > 10_000 {
				break;
			}
		}

		points
	}
}

/// Convenience wrapper for [`NoisyPathParams::generate`].
pub fn noisy_path(params: NoisyPathParams) -> Vec<Vec3> {
	params.generate()
}

/// Yaw about \(+Y\) (\(0\) → \(+Z\)) and pitch from the horizontal plane.
fn yaw_pitch_from_dir(dir: Vec3) -> (f32, f32) {
	let horiz = (dir.x * dir.x + dir.z * dir.z).sqrt();
	let pitch = dir.y.atan2(horiz.max(1e-8));
	let yaw = dir.x.atan2(dir.z);
	(yaw, pitch)
}

fn dir_from_yaw_pitch(yaw: f32, pitch: f32) -> Vec3 {
	let (sy, cy) = yaw.sin_cos();
	let (sp, cp) = pitch.sin_cos();
	Vec3::new(sy * cp, sp, cy * cp)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn path_length(points: &[Vec3]) -> f32 {
		points.windows(2).map(|w| w[0].distance(w[1])).sum()
	}

	#[test]
	fn respects_distance_budget() -> anyhow::Result<()> {
		let points = noisy_path(NoisyPathParams {
			distance: 10.0,
			step_len: StepLenRange::exact(1.0),
			allowed_angles: AllowedAngles::yaw_only(0.2),
			noise: NoiseParams { seed: 7, ..NoiseParams::default() },
			..NoisyPathParams::default()
		});
		assert!(points.len() >= 2);
		let len = path_length(&points);
		assert!((len - 10.0).abs() < 1e-2, "path length {len} should match budget");
		Ok(())
	}

	#[test]
	fn zero_pitch_stays_level() -> anyhow::Result<()> {
		let points = noisy_path(NoisyPathParams {
			start: Vec3::ZERO,
			initial_dir: Vec3::Z,
			distance: 8.0,
			step_len: StepLenRange::new(0.5, 1.5),
			allowed_angles: AllowedAngles::yaw_only(0.5),
			noise: NoiseParams { seed: 99, ..NoiseParams::default() },
		});
		assert!(points.len() >= 2);
		for p in &points {
			assert!(p.y.abs() < 1e-3, "expected flat path when max pitch is 0, got {p:?}");
		}
		Ok(())
	}

	#[test]
	fn same_seed_is_deterministic() -> anyhow::Result<()> {
		let params = NoisyPathParams {
			distance: 8.0,
			step_len: StepLenRange::new(0.5, 1.0),
			allowed_angles: AllowedAngles::yaw_pitch(0.4, 0.25),
			noise: NoiseParams { seed: 1234, frequency: 0.5, ..NoiseParams::default() },
			..NoisyPathParams::default()
		};
		let a = noisy_path(params.clone());
		let b = noisy_path(params);
		assert_eq!(a.len(), b.len());
		for (p, q) in a.iter().zip(b.iter()) {
			assert!(p.distance(*q) < 1e-5);
		}
		Ok(())
	}

	#[test]
	fn pitch_limit_bounds_elevation_angle() -> anyhow::Result<()> {
		let max_pitch = 0.4_f32;
		let points = noisy_path(NoisyPathParams {
			distance: 16.0,
			step_len: StepLenRange::exact(1.0),
			allowed_angles: AllowedAngles::new(max_pitch, 0.0, 0.0),
			noise: NoiseParams { seed: 42, ..NoiseParams::default() },
			..NoisyPathParams::default()
		});
		assert!(points.len() >= 2);
		for w in points.windows(2) {
			let d = (w[1] - w[0]).normalize_or_zero();
			let horiz = (d.x * d.x + d.z * d.z).sqrt();
			let pitch = d.y.atan2(horiz.max(1e-8));
			assert!(
				pitch.abs() <= max_pitch + 1e-3,
				"segment pitch {pitch} exceeds max {max_pitch}"
			);
		}
		let y_span = points
			.iter()
			.map(|p| p.y)
			.fold(f32::INFINITY, f32::min)
			.abs()
			.max(points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max).abs());
		assert!(y_span > 0.05, "expected elevation change with pitch allowance, span={y_span}");
		Ok(())
	}
}
