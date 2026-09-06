//! Approximate terrain pitch and roll from a rest support span. No IK.
//!
//! Sample ground at front / hind / left / right, take `atan(Δh / run)`, and
//! apply separate pitch and roll weights. Mesh faces `+Z`; positive local `X`
//! dips the nose, so sagittal slope is negated. Family roll weight is 0
//! (stand upright); set [`TerrainPitch::roll_weight`] to bank. The capsule
//! stays upright and owns world Y. Quadruped rays follow the live shoulder–hip
//! axis when girdles exist, not Bevy `+Z`, so a long body still measures the
//! slope it is standing on. Sample the **pitched** footprint so the visual
//! chord matches that slope without max-lifting the rear into the air.

use bevy::prelude::*;

use crate::rig::RigSkeletonKind;

/// Last probe locations for gizmos. Written by [`crate::elevation::apply_terrain_pitch`].
#[derive(Clone, Copy, Debug, Default)]
pub struct TerrainPitchProbe {
	pub origin: Vec3,
	pub front: Vec3,
	pub hind: Vec3,
	pub front_hit: bool,
	pub hind_hit: bool,
	/// Flattened Bevy mesh `+Z` (`-forward`).
	pub visual_facing: Vec3,
	/// Axis actually used for front/hind rays.
	pub sample_facing: Vec3,
}

/// Live girdle world points from prepare. `None` means the bone was not in the map.
#[derive(Clone, Copy, Debug, Default)]
pub struct TerrainPitchGirdles {
	pub shoulder_l: Option<Vec3>,
	pub shoulder_r: Option<Vec3>,
	pub hip_l: Option<Vec3>,
	pub hip_r: Option<Vec3>,
	pub front: Option<Vec3>,
	pub hind: Option<Vec3>,
	/// True when [`sagittal_axis`] accepted the front/hind midpoints.
	pub sagittal_ok: bool,
}

/// Visual tilt plus the rest support span used to sample terrain.
#[derive(Component, Clone, Copy, Debug)]
pub struct TerrainPitch {
	pub half_span: f32,
	pub half_width: f32,
	pub pitch_weight: f32,
	/// Fraction of observed side slope. Family default is 0; set per species to bank.
	pub roll_weight: f32,
	/// Local-X radians (nose down is positive).
	pub pitch: f32,
	/// Local-Z radians (right side up is positive).
	pub roll: f32,
	/// Smoothed local Y for a capsule child. Ignored for world-placed hosts.
	pub support: f32,
	/// Last probe target that passed the tilt deadband.
	pub accepted_pitch: f32,
	/// Last probe target that passed the tilt deadband.
	pub accepted_roll: f32,
	/// Last probe target that passed the support deadband.
	pub accepted_support: f32,
	/// Unit XZ hind→front from live girdles. [`Vec3::ZERO`] until measured.
	pub sagittal: Vec3,
	pub probe: TerrainPitchProbe,
	pub girdles: TerrainPitchGirdles,
}

impl TerrainPitch {
	pub fn new(kind: RigSkeletonKind, half_span: f32, half_width: f32) -> Self {
		Self {
			half_span,
			half_width,
			pitch_weight: pitch_weight(kind),
			roll_weight: roll_weight(kind),
			pitch: 0.0,
			roll: 0.0,
			support: 0.0,
			accepted_pitch: 0.0,
			accepted_roll: 0.0,
			accepted_support: 0.0,
			sagittal: Vec3::ZERO,
			probe: TerrainPitchProbe::default(),
			girdles: TerrainPitchGirdles::default(),
		}
	}

	/// Store live girdle world points and set [`Self::sagittal`] when the XZ run is long enough.
	pub fn record_girdles(
		&mut self,
		shoulder_l: Option<Vec3>,
		shoulder_r: Option<Vec3>,
		hip_l: Option<Vec3>,
		hip_r: Option<Vec3>,
	) {
		let front = girdle_midpoint([shoulder_l, shoulder_r].into_iter().flatten());
		let hind = girdle_midpoint([hip_l, hip_r].into_iter().flatten());
		let left = girdle_midpoint([shoulder_l, hip_l].into_iter().flatten());
		let right = girdle_midpoint([shoulder_r, hip_r].into_iter().flatten());
		let sagittal_ok = match (front, hind) {
			(Some(f), Some(h)) => {
				if let Some(axis) = sagittal_axis(f, h) {
					self.sagittal = axis;
					true
				} else {
					false
				}
			}
			_ => false,
		};
		if let Some(span) = measured_support_half(front, hind) {
			self.half_span = span;
		}
		if let Some(width) = measured_support_half(left, right) {
			self.half_width = width;
		}
		self.girdles = TerrainPitchGirdles {
			shoulder_l,
			shoulder_r,
			hip_l,
			hip_r,
			front,
			hind,
			sagittal_ok,
		};
	}
}

/// Match Durham / playground walkable slopes so long bodies can follow the mesh.
pub const MAX_TILT: f32 = 80.0_f32.to_radians();
/// Max rad/s toward the accepted tilt. Large snaps still take several frames.
pub const TILT_RATE: f32 = 3.0;
/// Exponential follow rate (1/s) so sub-rate-cap noise is low-passed, not copied.
pub const TILT_SMOOTH: f32 = 10.0;
/// Ignore new pitch/roll samples closer than this to the accepted target.
pub const MIN_TILT_CHANGE: f32 = 2.5_f32.to_radians();
/// Max m/s toward the accepted support offset.
pub const SUPPORT_RATE: f32 = 2.0;
/// Ignore new support samples closer than this to the accepted offset.
pub const MIN_SUPPORT_CHANGE: f32 = 0.04;

const HUMANOID_HALF_SPAN: f32 = 0.22;
const QUADRUPED_HALF_SPAN: f32 = 1.2;
const FORELIMBED_HALF_SPAN: f32 = 0.4;
const HUMANOID_HALF_WIDTH: f32 = 0.18;
const QUADRUPED_HALF_WIDTH: f32 = 0.45;
const FORELIMBED_HALF_WIDTH: f32 = 0.25;
const MIN_MEASURED: f32 = 0.12;
/// Cap on signed visual Y so a bad ray cannot yank the mesh through the world.
const MAX_SUPPORT_OFFSET: f32 = 2.0;

pub const QUADRUPED_FRONT: &[&str] = &["shoulder.L", "shoulder.R"];
pub const QUADRUPED_HIND: &[&str] = &["hip.L", "hip.R"];
pub const QUADRUPED_LEFT: &[&str] = &["shoulder.L", "hip.L"];
pub const QUADRUPED_RIGHT: &[&str] = &["shoulder.R", "hip.R"];

pub fn default_half_span(kind: RigSkeletonKind) -> f32 {
	match kind {
		RigSkeletonKind::Humanoid | RigSkeletonKind::Neck => HUMANOID_HALF_SPAN,
		RigSkeletonKind::Quadruped => QUADRUPED_HALF_SPAN,
		RigSkeletonKind::Forelimbed => FORELIMBED_HALF_SPAN,
	}
}

pub fn default_half_width(kind: RigSkeletonKind) -> f32 {
	match kind {
		RigSkeletonKind::Humanoid | RigSkeletonKind::Neck => HUMANOID_HALF_WIDTH,
		RigSkeletonKind::Quadruped => QUADRUPED_HALF_WIDTH,
		RigSkeletonKind::Forelimbed => FORELIMBED_HALF_WIDTH,
	}
}

/// How much of the front/hind slope to apply. Long bodies need more or they sink.
pub fn pitch_weight(kind: RigSkeletonKind) -> f32 {
	match kind {
		RigSkeletonKind::Humanoid | RigSkeletonKind::Neck => 0.4,
		RigSkeletonKind::Quadruped => 1.0,
		RigSkeletonKind::Forelimbed => 0.7,
	}
}

/// How much of the left/right slope to apply. Zero: stand upright; opt in later.
pub fn roll_weight(_kind: RigSkeletonKind) -> f32 {
	0.0
}

/// Midpoint of named bones in XZ, if at least one exists.
pub fn girdle_midpoint(positions: impl IntoIterator<Item = Vec3>) -> Option<Vec3> {
	let mut sum = Vec3::ZERO;
	let mut n = 0u32;
	for p in positions {
		sum += p;
		n += 1;
	}
	(n > 0).then(|| sum / n as f32)
}

fn measured_half(a: Option<Vec3>, b: Option<Vec3>, fallback: f32) -> f32 {
	measured_support_half(a, b).unwrap_or(fallback)
}

/// XZ half-distance between two support samples, if they are far enough apart.
pub fn measured_support_half(a: Option<Vec3>, b: Option<Vec3>) -> Option<f32> {
	let (Some(a), Some(b)) = (a, b) else {
		return None;
	};
	let delta = Vec2::new(a.x - b.x, a.z - b.z).length() * 0.5;
	(delta >= MIN_MEASURED).then_some(delta)
}

/// Rest wheelbase from girdle world positions, or the family default.
pub fn half_span_from_girdles(
	kind: RigSkeletonKind,
	front: Option<Vec3>,
	hind: Option<Vec3>,
) -> f32 {
	measured_half(front, hind, default_half_span(kind))
}

/// Rest stance width from left/right world positions, or the family default.
pub fn half_width_from_sides(
	kind: RigSkeletonKind,
	left: Option<Vec3>,
	right: Option<Vec3>,
) -> f32 {
	measured_half(left, right, default_half_width(kind))
}

/// Flattened unit XZ, or `None` if the vector has no ground-plane direction.
pub fn xz_dir(v: Vec3) -> Option<Vec3> {
	let xz = Vec3::new(v.x, 0.0, v.z);
	(xz.length_squared() >= 1e-6).then(|| xz.normalize())
}

/// Flattened unit XZ, or `None` if the vector is too short to be a support axis.
pub fn xz_unit(v: Vec3) -> Option<Vec3> {
	let xz = Vec3::new(v.x, 0.0, v.z);
	(xz.length() >= 2.0 * MIN_MEASURED).then(|| xz.normalize())
}

/// Hind→front on the ground plane from live girdle midpoints.
pub fn sagittal_axis(front: Vec3, hind: Vec3) -> Option<Vec3> {
	xz_unit(front - hind)
}

/// Front/hind sample direction: live spine when girdles exist, else mesh `+Z`.
pub fn sample_facing(sagittal: Vec3, visual_facing: Vec3) -> Vec3 {
	xz_unit(sagittal).unwrap_or(visual_facing)
}

fn slope_angle(high_side: f32, low_side: f32, half_run: f32) -> f32 {
	let run = (2.0 * half_run).max(1e-3);
	((high_side - low_side) / run).atan().clamp(-MAX_TILT, MAX_TILT)
}

/// Local-X angle: nose up when the front sample is higher (`+Z` mesh).
pub fn observed_pitch(front_height: f32, hind_height: f32, half_span: f32) -> f32 {
	-slope_angle(front_height, hind_height, half_span)
}

/// Local-Z angle: right side up when the right sample is higher.
pub fn observed_roll(left_height: f32, right_height: f32, half_width: f32) -> f32 {
	slope_angle(right_height, left_height, half_width)
}

pub fn step_toward(current: f32, target: f32, dt: f32) -> f32 {
	step_toward_rate(current, target, dt, TILT_RATE)
}

pub fn step_toward_rate(current: f32, target: f32, dt: f32, rate: f32) -> f32 {
	let max_step = rate * dt;
	current + (target - current).clamp(-max_step, max_step)
}

/// Keep `accepted` unless `observed` moved by at least `min_change`.
pub fn accept_target(accepted: f32, observed: f32, min_change: f32) -> f32 {
	if (observed - accepted).abs() >= min_change {
		observed
	} else {
		accepted
	}
}

/// Exponential blend toward `accepted`, then clamp to `rate`.
pub fn follow_target(current: f32, accepted: f32, dt: f32, rate: f32) -> f32 {
	let alpha = 1.0 - (-TILT_SMOOTH * dt.max(0.0)).exp();
	let blended = current + (accepted - current) * alpha;
	step_toward_rate(current, blended, dt, rate)
}

/// Deadband the probe target, then follow it. `force` skips the deadband (jump).
pub fn smooth_toward(
	current: f32,
	accepted: &mut f32,
	observed: f32,
	dt: f32,
	min_change: f32,
	rate: f32,
	force: bool,
) -> f32 {
	*accepted = if force {
		observed
	} else {
		accept_target(*accepted, observed, min_change)
	};
	follow_target(current, *accepted, dt, rate)
}

/// Yaw from flattened facing (`look_to(-facing)`), then local pitch and roll.
pub fn facing_with_tilt(facing_xz: Vec3, pitch: f32, roll: f32) -> Quat {
	facing_with_support_tilt(facing_xz, facing_xz, pitch, roll)
}

/// Keep locomotion yaw on `yaw_facing`, pitch about the axis perpendicular to `sagittal`.
///
/// When the shoulder–hip axis is not mesh `+Z`, local `X` would bank the spine
/// instead of planting it. World lateral is `Y × sagittal`, brought into the
/// yawed frame so a +X spine still pitches while wish `+Z` stays level.
pub fn facing_with_support_tilt(yaw_facing: Vec3, sagittal: Vec3, pitch: f32, roll: f32) -> Quat {
	let yaw_facing = Vec3::new(yaw_facing.x, 0.0, yaw_facing.z);
	let yaw = if yaw_facing.length_squared() < 1e-6 {
		Quat::IDENTITY
	} else {
		Transform::IDENTITY.looking_to(-yaw_facing, Vec3::Y).rotation
	};
	let spine = Vec3::new(sagittal.x, 0.0, sagittal.z);
	let spine = if spine.length_squared() < 1e-6 { yaw_facing } else { spine };
	let lateral = Vec3::Y.cross(spine);
	let pitch_q = if lateral.length_squared() < 1e-6 {
		Quat::IDENTITY
	} else {
		Quat::from_axis_angle(yaw.inverse() * lateral.normalize(), pitch)
	};
	yaw * pitch_q * Quat::from_rotation_z(roll)
}

/// Horizontal half-run of the rest chord after pitch (girdles move closer in XZ).
pub fn pitched_half_run(half_span: f32, pitch: f32) -> f32 {
	(half_span * pitch.cos().abs()).max(MIN_MEASURED)
}

/// Signed visual Y so the chord midpoint matches the average of the front/hind
/// samples. Zero on a plane when those samples are taken at [`pitched_half_run`].
pub fn support_offset(center_height: f32, front_height: f32, hind_height: f32) -> f32 {
	((front_height + hind_height) * 0.5 - center_height)
		.clamp(-MAX_SUPPORT_OFFSET, MAX_SUPPORT_OFFSET)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn uphill_front_pitches_nose_up() {
		let pitch = observed_pitch(2.0, 1.0, 0.5);
		assert!(pitch < 0.0);
		assert!(pitch.abs() <= MAX_TILT);
		let nose = facing_with_tilt(Vec3::Z, pitch, 0.0) * Vec3::Z;
		assert!(nose.y > 0.0, "uphill should raise mesh +Z, y={}", nose.y);
	}

	#[test]
	fn high_right_rolls_right_up() {
		let roll = observed_roll(1.0, 2.0, 0.5);
		assert!(roll > 0.0);
		let right = facing_with_tilt(Vec3::Z, 0.0, roll) * Vec3::X;
		assert!(right.y > 0.0, "high right should raise mesh +X, y={}", right.y);
	}

	#[test]
	fn support_tilt_pitches_a_plus_x_spine_without_banking_plus_z() {
		let pitch = -0.6;
		let q = facing_with_support_tilt(Vec3::Z, Vec3::X, pitch, 0.0);
		let spine = q * Vec3::X;
		assert!(spine.y > 0.0, "uphill along +X should raise mesh +X, y={}", spine.y);
		let nose = q * Vec3::Z;
		assert!(nose.y.abs() < 0.05, "wish +Z should stay level, y={}", nose.y);
	}

	#[test]
	fn flat_ground_is_zero() {
		assert_eq!(observed_pitch(3.0, 3.0, 0.9), 0.0);
		assert_eq!(observed_roll(3.0, 3.0, 0.45), 0.0);
	}

	#[test]
	fn missing_girdles_use_family_default() {
		assert_eq!(
			half_span_from_girdles(RigSkeletonKind::Quadruped, None, None),
			QUADRUPED_HALF_SPAN
		);
		assert_eq!(
			half_width_from_sides(RigSkeletonKind::Quadruped, None, None),
			QUADRUPED_HALF_WIDTH
		);
	}

	#[test]
	fn stacked_girdles_use_family_default() {
		let origin = Vec3::new(10.0, 1.0, 4.0);
		assert_eq!(
			half_span_from_girdles(RigSkeletonKind::Quadruped, Some(origin), Some(origin)),
			QUADRUPED_HALF_SPAN
		);
	}

	#[test]
	fn measured_girdles_win_when_separated() {
		let front = Vec3::new(0.0, 1.0, 2.0);
		let hind = Vec3::new(0.0, 1.0, 0.0);
		assert!(
			(half_span_from_girdles(RigSkeletonKind::Quadruped, Some(front), Some(hind)) - 1.0)
				.abs() < 1e-5
		);
	}

	#[test]
	fn offset_is_zero_on_a_plane() {
		assert!(support_offset(0.0, 0.2, -0.2).abs() < 1e-5);
		assert!(support_offset(1.2, 2.4, 0.0).abs() < 1e-5);
	}

	#[test]
	fn pitched_run_shrinks_with_tilt() {
		let span = 1.2;
		assert!((pitched_half_run(span, 0.0) - span).abs() < 1e-5);
		assert!((pitched_half_run(span, 60.0_f32.to_radians()) - span * 0.5).abs() < 1e-5);
	}

	#[test]
	fn plane_slope_survives_resampling_at_pitched_run() {
		let alpha = 40.0_f32.to_radians();
		let span = 1.2;
		let tan = alpha.tan();
		let coarse = observed_pitch(tan * span, -tan * span, span);
		assert!((coarse + alpha).abs() < 1e-4);
		let run = pitched_half_run(span, coarse);
		let refined = observed_pitch(tan * run, -tan * run, run);
		assert!((refined - coarse).abs() < 1e-4);
		assert!(support_offset(0.0, tan * run, -tan * run).abs() < 1e-5);
	}

	#[test]
	fn support_offset_raises_over_a_dip() {
		let offset = support_offset(0.0, 1.0, 1.0);
		assert!((offset - 1.0).abs() < 1e-5);
	}

	#[test]
	fn measured_support_ignores_stacked_girdles() {
		let origin = Vec3::new(10.0, 1.0, 4.0);
		assert_eq!(measured_support_half(Some(origin), Some(origin)), None);
	}

	#[test]
	fn family_roll_weight_defaults_to_zero() {
		for kind in [
			RigSkeletonKind::Humanoid,
			RigSkeletonKind::Quadruped,
			RigSkeletonKind::Forelimbed,
			RigSkeletonKind::Neck,
		] {
			assert_eq!(roll_weight(kind), 0.0);
			assert!(pitch_weight(kind) > 0.0);
		}
	}

	#[test]
	fn sagittal_axis_follows_shoulder_to_hip() {
		let front = Vec3::new(3.0, 2.0, 1.0);
		let hind = Vec3::new(1.0, 0.5, 1.0);
		let axis = sagittal_axis(front, hind).expect("separated girdles");
		assert!((axis - Vec3::X).length() < 1e-5);
		assert!(sagittal_axis(front, front).is_none());
	}

	#[test]
	fn sample_facing_prefers_girdle_axis_over_mesh_plus_z() {
		let visual = Vec3::Z;
		assert_eq!(sample_facing(Vec3::ZERO, visual), visual);
		let spine = sample_facing(Vec3::X, visual);
		assert!((spine - Vec3::X).length() < 1e-5);
	}

	#[test]
	fn record_girdles_accepts_a_long_xz_wheelbase() {
		let mut pitch = TerrainPitch::new(RigSkeletonKind::Quadruped, 1.2, 0.45);
		pitch.record_girdles(
			Some(Vec3::new(0.0, 1.0, 2.0)),
			Some(Vec3::new(0.0, 1.0, 2.0)),
			Some(Vec3::new(0.0, 1.0, 0.0)),
			Some(Vec3::new(0.0, 1.0, 0.0)),
		);
		assert!(pitch.girdles.sagittal_ok);
		assert!((pitch.half_span - 1.0).abs() < 1e-5);
		assert!((pitch.sagittal - Vec3::Z).length() < 1e-5);
	}

	#[test]
	fn record_girdles_rejects_stacked_xz() {
		let mut pitch = TerrainPitch::new(RigSkeletonKind::Quadruped, 1.2, 0.45);
		let a = Vec3::new(1.0, 0.0, 1.0);
		let b = Vec3::new(1.0, 4.0, 1.0);
		pitch.record_girdles(Some(a), Some(a), Some(b), Some(b));
		assert!(pitch.girdles.front.is_some());
		assert!(!pitch.girdles.sagittal_ok);
		assert_eq!(pitch.sagittal, Vec3::ZERO);
		assert!((pitch.half_span - 1.2).abs() < 1e-5);
	}

	#[test]
	fn accept_target_ignores_sub_threshold_noise() {
		let accepted = 0.4;
		assert_eq!(
			accept_target(accepted, accepted + MIN_TILT_CHANGE * 0.5, MIN_TILT_CHANGE),
			accepted
		);
		let next = accepted + MIN_TILT_CHANGE * 1.01;
		assert_eq!(accept_target(accepted, next, MIN_TILT_CHANGE), next);
	}

	#[test]
	fn follow_target_damps_small_error_and_caps_large() {
		let dt = 1.0 / 60.0;
		let small = follow_target(0.0, 0.02, dt, TILT_RATE);
		assert!(small > 0.0);
		assert!(small < 0.02);
		let large = follow_target(0.0, 2.0, dt, TILT_RATE);
		assert!((large - TILT_RATE * dt).abs() < 1e-5);
	}

	#[test]
	fn smooth_toward_forces_zero_below_the_deadband() {
		let dt = 1.0 / 60.0;
		let mut accepted = 0.02;
		let held = smooth_toward(
			0.02,
			&mut accepted,
			0.0,
			dt,
			MIN_TILT_CHANGE,
			TILT_RATE,
			false,
		);
		assert_eq!(accepted, 0.02);
		assert!((held - 0.02).abs() < 1e-6);
		let out = smooth_toward(
			0.02,
			&mut accepted,
			0.0,
			dt,
			MIN_TILT_CHANGE,
			TILT_RATE,
			true,
		);
		assert_eq!(accepted, 0.0);
		assert!(out < 0.02);
	}
}
