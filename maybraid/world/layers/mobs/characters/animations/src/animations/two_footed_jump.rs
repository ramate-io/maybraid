//! Two-footed jump timing and vertical motion.
//!
//! The jump is parameterized by four values:
//!
//! - **`jump_height`** — apex height above the take-off point (world units).
//! - **`gravity`** — constant downward acceleration (units/s²).
//! - **`pre_squat_speed`** — how fast the character squats down before take-off
//!   (stand to bottom in `1/pre_squat_speed` seconds). The return to stand before
//!   spring is matched to launch speed from `jump_height` and `gravity`.
//! - **`landing_squat_speed`** — landing compression and recovery rate (each half-cycle
//!   takes `1/landing_squat_speed` seconds).

use std::cell::RefCell;
use std::marker::PhantomData;

use character_rigs::humanoid::LegSegmentLengths;

use crate::animations::{Land, Squat};

pub const DEFAULT_GRAVITY: f32 = 9.8;
pub const DEFAULT_JUMP_HEIGHT: f32 = 1.5;
/// Default pre-jump squat-down rate (bottom in ~0.33 s).
pub const DEFAULT_PRE_SQUAT_SPEED: f32 = 3.0;
/// Default landing recovery rate; also scales impact-matched compression.
pub const DEFAULT_LANDING_SQUAT_SPEED: f32 = 3.0;
pub const DEFAULT_SPRING_DURATION: f32 = 0.15;

/// Fraction of the airborne segment used to blend from spring into fall spread.
pub const FALL_BLEND_FRACTION: f32 = 0.25;
/// Fraction of the **compression** half used to blend fall arms into landing pose.
pub const LAND_BLEND_FRACTION: f32 = 0.25;
/// Upper cap on fall-to-land pose blend so compression is not delayed by recovery timing.
pub const LAND_POSE_BLEND_MAX_SECS: f32 = 0.5;

const MIN_SEGMENT_DURATION: f32 = 1e-3;
const MIN_SPEED: f32 = 1e-3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JumpTiming {
	/// Pre-jump stand-to-bottom duration (seconds).
	pub squat_descent_duration: f32,
	/// Pre-jump bottom-to-stand duration before spring (seconds).
	pub squat_ascent_duration: f32,
	/// Leg extension / push-off window (seconds).
	pub spring_duration: f32,
	/// Fall segment duration: spring end until ballistic touchdown (seconds).
	pub air_duration: f32,
	/// Landing compression duration (seconds).
	pub land_descent_duration: f32,
	/// Stand-up after landing compression (seconds).
	pub land_ascent_duration: f32,
}

impl JumpTiming {
	pub fn squat_duration(&self) -> f32 {
		self.squat_descent_duration + self.squat_ascent_duration
	}

	pub fn land_duration(&self) -> f32 {
		self.land_descent_duration + self.land_ascent_duration
	}

	pub fn cycle_duration(&self) -> f32 {
		self.squat_duration() + self.spring_duration + self.air_duration + self.land_duration()
	}

	pub fn squat_end(&self) -> f32 {
		self.squat_duration()
	}

	pub fn spring_end(&self) -> f32 {
		self.squat_end() + self.spring_duration
	}

	pub fn air_end(&self) -> f32 {
		self.spring_end() + self.air_duration
	}

	/// Seconds spent blending fall spread into landing at touch-down.
	pub fn land_pose_blend_duration(&self) -> f32 {
		(self.land_descent_duration * LAND_BLEND_FRACTION).min(LAND_POSE_BLEND_MAX_SECS)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpSegment {
	Squat,
	Spring,
	Fall,
	Land,
}

/// Rig-derived jump timings and squat envelopes, reused across pose and root-motion sampling.
#[derive(Debug, Clone, Copy, PartialEq)]
struct JumpConfigFingerprint {
	gravity: f32,
	jump_height: f32,
	pre_squat_speed: f32,
	landing_squat_speed: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JumpRigDerived {
	pub lengths: LegSegmentLengths,
	pub timings: JumpTiming,
	windup_descent: f32,
	windup_ascent: f32,
	land_half_speed: f32,
	config: JumpConfigFingerprint,
}

impl JumpRigDerived {
	pub fn prejump_squat<Rig>(&self) -> Squat<Rig> {
		Squat::with_speeds(self.windup_descent, self.windup_ascent)
	}

	pub fn landing_squat<Rig>(&self) -> Land<Rig> {
		Land::with_speeds(self.land_half_speed, self.land_half_speed, Squat::<Rig>::default())
	}
}

/// Shared per-time jump sample for pose transitions and vertical root motion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JumpSample {
	pub segment: JumpSegment,
	pub local_progress: f32,
	/// Normalized blend weight for segment pose transitions (1.0 when not blending).
	pub transition_weight: f32,
	pub vertical_offset: f32,
}

#[derive(Debug, Clone)]
struct JumpSampleCache {
	lengths: LegSegmentLengths,
	elapsed_bits: u32,
	sample: JumpSample,
}

#[derive(Debug)]
pub struct TwoFootedJump<Rig> {
	/// Downward acceleration for ballistic motion (units/s²).
	pub gravity: f32,
	/// Apex height above the take-off point (world units).
	pub jump_height: f32,
	/// Stand-to-bottom rate before take-off (`1/speed` = descent seconds).
	pub pre_squat_speed: f32,
	/// Landing compression scale and recovery rate after touch-down.
	pub landing_squat_speed: f32,
	_rig: PhantomData<Rig>,
	rig_cache: RefCell<Option<JumpRigDerived>>,
	sample_cache: RefCell<Option<JumpSampleCache>>,
}

impl<Rig> Default for TwoFootedJump<Rig> {
	fn default() -> Self {
		Self {
			gravity: DEFAULT_GRAVITY,
			jump_height: DEFAULT_JUMP_HEIGHT,
			pre_squat_speed: DEFAULT_PRE_SQUAT_SPEED,
			landing_squat_speed: DEFAULT_LANDING_SQUAT_SPEED,
			_rig: PhantomData,
			rig_cache: RefCell::new(None),
			sample_cache: RefCell::new(None),
		}
	}
}

impl<Rig> Clone for TwoFootedJump<Rig> {
	fn clone(&self) -> Self {
		Self {
			gravity: self.gravity,
			jump_height: self.jump_height,
			pre_squat_speed: self.pre_squat_speed,
			landing_squat_speed: self.landing_squat_speed,
			_rig: PhantomData,
			rig_cache: RefCell::new(None),
			sample_cache: RefCell::new(None),
		}
	}
}

impl<Rig> TwoFootedJump<Rig> {
	pub fn with_gravity(mut self, gravity: f32) -> Self {
		self.gravity = gravity;
		self.invalidate_caches();
		self
	}

	pub fn with_jump_height(mut self, jump_height: f32) -> Self {
		self.jump_height = jump_height;
		self.invalidate_caches();
		self
	}

	pub fn with_pre_squat_speed(mut self, pre_squat_speed: f32) -> Self {
		self.pre_squat_speed = pre_squat_speed;
		self.invalidate_caches();
		self
	}

	pub fn with_landing_squat_speed(mut self, landing_squat_speed: f32) -> Self {
		self.landing_squat_speed = landing_squat_speed;
		self.invalidate_caches();
		self
	}

	fn invalidate_caches(&self) {
		*self.rig_cache.borrow_mut() = None;
		*self.sample_cache.borrow_mut() = None;
	}

	fn config_fingerprint(&self) -> JumpConfigFingerprint {
		JumpConfigFingerprint {
			gravity: self.gravity,
			jump_height: self.jump_height,
			pre_squat_speed: self.pre_squat_speed,
			landing_squat_speed: self.landing_squat_speed,
		}
	}

	fn build_rig_derived(&self, lengths: LegSegmentLengths) -> JumpRigDerived {
		let (prejump_squat, landing_squat) = self.squat_configs(lengths);
		let touchdown = touchdown_time_since_launch(self.gravity, self.jump_height);
		let fall_duration = (touchdown - DEFAULT_SPRING_DURATION).max(MIN_SEGMENT_DURATION);
		let timings = JumpTiming {
			squat_descent_duration: prejump_squat.descent_duration().max(MIN_SEGMENT_DURATION),
			squat_ascent_duration: prejump_squat.ascent_duration().max(MIN_SEGMENT_DURATION),
			spring_duration: DEFAULT_SPRING_DURATION,
			air_duration: fall_duration,
			land_descent_duration: landing_squat.descent_duration().max(MIN_SEGMENT_DURATION),
			land_ascent_duration: landing_squat.ascent_duration().max(MIN_SEGMENT_DURATION),
		};
		JumpRigDerived {
			lengths,
			timings,
			windup_descent: prejump_squat.descent_speed,
			windup_ascent: prejump_squat.ascent_speed,
			land_half_speed: landing_squat.squat.descent_speed,
			config: self.config_fingerprint(),
		}
	}

	/// Cached rig-derived lengths, timings, and squat envelopes.
	///
	/// Invalidated when leg segment lengths or jump configuration change.
	pub fn rig_derived(&self, lengths: LegSegmentLengths) -> JumpRigDerived {
		let config = self.config_fingerprint();
		if let Some(cached) = self.rig_cache.borrow().clone() {
			if cached.lengths == lengths && cached.config == config {
				return cached;
			}
		}
		let derived = self.build_rig_derived(lengths);
		*self.rig_cache.borrow_mut() = Some(derived);
		derived
	}

	pub fn sample(&self, lengths: LegSegmentLengths, elapsed: f32) -> (JumpRigDerived, JumpSample) {
		let derived = self.rig_derived(lengths);
		let sample = self.cached_sample(lengths, elapsed, &derived);
		(derived, sample)
	}

	pub(crate) fn cached_sample(
		&self,
		lengths: LegSegmentLengths,
		elapsed: f32,
		derived: &JumpRigDerived,
	) -> JumpSample {
		let elapsed_bits = elapsed.to_bits();
		if let Some(cached) = self.sample_cache.borrow().as_ref() {
			if cached.lengths == lengths && cached.elapsed_bits == elapsed_bits {
				return cached.sample;
			}
		}
		let sample = self.sample_at(derived, elapsed);
		*self.sample_cache.borrow_mut() = Some(JumpSampleCache { lengths, elapsed_bits, sample });
		sample
	}

	pub fn sample_at(&self, derived: &JumpRigDerived, elapsed: f32) -> JumpSample {
		let time_in_cycle = self.time_in_cycle(derived.lengths, elapsed);
		let (segment, local) = Self::segment_at_time(time_in_cycle, &derived.timings);
		let transition_weight = match segment {
			JumpSegment::Fall => {
				let blend_end = FALL_BLEND_FRACTION;
				if local < blend_end {
					(local / blend_end).clamp(0.0, 1.0)
				} else {
					1.0
				}
			}
			JumpSegment::Land => {
				let blend_window = derived.timings.land_pose_blend_duration();
				if blend_window > f32::EPSILON {
					(local / blend_window).clamp(0.0, 1.0)
				} else {
					1.0
				}
			}
			_ => 1.0,
		};
		let vertical_offset =
			Self::vertical_offset_for_segment::<Rig>(self, derived, segment, local, time_in_cycle);
		JumpSample { segment, local_progress: local, transition_weight, vertical_offset }
	}

	fn vertical_offset_for_segment<R>(
		jump: &Self,
		derived: &JumpRigDerived,
		segment: JumpSegment,
		local: f32,
		time_in_cycle: f32,
	) -> f32 {
		match segment {
			JumpSegment::Squat => {
				let p = local / derived.timings.squat_duration().max(f32::EPSILON);
				-derived.prejump_squat::<R>().vertical_drop(p, derived.lengths)
			}
			JumpSegment::Spring | JumpSegment::Fall => {
				let since_launch = (time_in_cycle - derived.timings.squat_end()).max(0.0);
				jump.ballistic_height(since_launch.min(jump.touchdown_time_since_launch()))
			}
			JumpSegment::Land => {
				let p = local / derived.timings.land_duration().max(f32::EPSILON);
				-derived.landing_squat::<R>().vertical_drop(p, derived.lengths)
			}
		}
	}

	fn squat_configs(&self, lengths: LegSegmentLengths) -> (Squat<Rig>, Land<Rig>) {
		let impact = launch_speed(self.gravity, self.jump_height);
		let squat_peak = Squat::<Rig>::default().peak_vertical_drop(lengths);

		let windup_descent = self.pre_squat_speed.max(MIN_SPEED);
		let windup_ascent =
			if squat_peak > f32::EPSILON { impact / squat_peak } else { windup_descent };

		let land_half_speed = self.landing_squat_speed.max(MIN_SPEED);

		let windup = Squat::with_speeds(windup_descent, windup_ascent.max(MIN_SPEED));
		let landing = Land::with_speeds(land_half_speed, land_half_speed, Squat::<Rig>::default());
		(windup, landing)
	}

	pub fn timings(&self, lengths: LegSegmentLengths) -> JumpTiming {
		self.rig_derived(lengths).timings
	}

	/// Seconds after take-off when [`ballistic_height`] returns to zero on descent.
	pub fn touchdown_time_since_launch(&self) -> f32 {
		touchdown_time_since_launch(self.gravity, self.jump_height)
	}

	pub fn cycle_duration(&self, lengths: LegSegmentLengths) -> f32 {
		self.timings(lengths).cycle_duration()
	}

	pub fn time_in_cycle(&self, lengths: LegSegmentLengths, elapsed: f32) -> f32 {
		let cycle = self.cycle_duration(lengths);
		if cycle <= f32::EPSILON {
			return 0.0;
		}
		elapsed % cycle
	}

	pub fn launch_speed(&self) -> f32 {
		launch_speed(self.gravity, self.jump_height)
	}

	pub fn ballistic_height(&self, time_since_launch: f32) -> f32 {
		ballistic_height(time_since_launch, self.gravity, self.jump_height)
	}

	pub fn segment_at_time(time: f32, timings: &JumpTiming) -> (JumpSegment, f32) {
		let mut t = time;
		if t < timings.squat_duration() {
			return (JumpSegment::Squat, t);
		}
		t -= timings.squat_duration();
		if t < timings.spring_duration {
			return (JumpSegment::Spring, t / timings.spring_duration);
		}
		t -= timings.spring_duration;
		if t < timings.air_duration {
			return (JumpSegment::Fall, t / timings.air_duration.max(f32::EPSILON));
		}
		t -= timings.air_duration;
		(JumpSegment::Land, t)
	}

	pub fn segment(&self, lengths: LegSegmentLengths, elapsed: f32) -> (JumpSegment, f32) {
		Self::segment_at_time(self.time_in_cycle(lengths, elapsed), &self.timings(lengths))
	}

	pub fn time_since_launch(&self, lengths: LegSegmentLengths, elapsed: f32) -> f32 {
		(self.time_in_cycle(lengths, elapsed) - self.timings(lengths).squat_end()).max(0.0)
	}

	pub fn prejump_squat(&self, lengths: LegSegmentLengths) -> Squat<Rig> {
		self.rig_derived(lengths).prejump_squat()
	}

	pub fn landing_squat(&self, lengths: LegSegmentLengths) -> Land<Rig> {
		self.rig_derived(lengths).landing_squat()
	}

	pub fn landing_depth(&self, lengths: LegSegmentLengths, elapsed: f32) -> f32 {
		let land = self.landing_squat(lengths);
		let (segment, local) = self.segment(lengths, elapsed);
		if segment != JumpSegment::Land {
			return 0.0;
		}
		let duration = land.cycle_duration().max(f32::EPSILON);
		land.depth(local / duration)
	}

	pub fn vertical_offset(&self, lengths: LegSegmentLengths, elapsed: f32) -> f32 {
		let derived = self.rig_derived(lengths);
		self.cached_sample(lengths, elapsed, &derived).vertical_offset
	}
}

/// Impact speed at touch-down for a jump of the given height under constant gravity.
pub fn launch_speed(gravity: f32, jump_height: f32) -> f32 {
	(2.0 * gravity * jump_height).sqrt()
}

/// Ballistic air time from take-off back to launch height.
pub fn air_duration(gravity: f32, jump_height: f32) -> f32 {
	2.0 * launch_speed(gravity, jump_height) / gravity
}

/// Time after take-off when [`ballistic_height`] returns to zero on descent.
pub fn touchdown_time_since_launch(gravity: f32, jump_height: f32) -> f32 {
	air_duration(gravity, jump_height)
}

/// Height above launch point `t` seconds after take-off.
pub fn ballistic_height(time_since_launch: f32, gravity: f32, jump_height: f32) -> f32 {
	if time_since_launch <= 0.0 {
		return 0.0;
	}
	let v0 = launch_speed(gravity, jump_height);
	(v0 * time_since_launch - 0.5 * gravity * time_since_launch * time_since_launch).max(0.0)
}

#[cfg(test)]
mod tests {
	use super::*;

	mod legacy {
		use super::*;

		pub(super) fn squat_configs<Rig>(
			jump: &TwoFootedJump<Rig>,
			lengths: LegSegmentLengths,
		) -> (Squat<Rig>, Land<Rig>) {
			let impact = launch_speed(jump.gravity, jump.jump_height);
			let squat_peak = Squat::<Rig>::default().peak_vertical_drop(lengths);
			let windup_descent = jump.pre_squat_speed.max(MIN_SPEED);
			let windup_ascent =
				if squat_peak > f32::EPSILON { impact / squat_peak } else { windup_descent };
			let land_half_speed = jump.landing_squat_speed.max(MIN_SPEED);
			let windup = Squat::with_speeds(windup_descent, windup_ascent.max(MIN_SPEED));
			let landing =
				Land::with_speeds(land_half_speed, land_half_speed, Squat::<Rig>::default());
			(windup, landing)
		}

		pub(super) fn timings(jump: &TwoFootedJump<()>, lengths: LegSegmentLengths) -> JumpTiming {
			let (windup, landing) = squat_configs(jump, lengths);
			let touchdown = touchdown_time_since_launch(jump.gravity, jump.jump_height);
			let fall_duration = (touchdown - DEFAULT_SPRING_DURATION).max(MIN_SEGMENT_DURATION);
			JumpTiming {
				squat_descent_duration: windup.descent_duration().max(MIN_SEGMENT_DURATION),
				squat_ascent_duration: windup.ascent_duration().max(MIN_SEGMENT_DURATION),
				spring_duration: DEFAULT_SPRING_DURATION,
				air_duration: fall_duration,
				land_descent_duration: landing.descent_duration().max(MIN_SEGMENT_DURATION),
				land_ascent_duration: landing.ascent_duration().max(MIN_SEGMENT_DURATION),
			}
		}

		pub(super) fn segment(
			jump: &TwoFootedJump<()>,
			lengths: LegSegmentLengths,
			elapsed: f32,
		) -> (JumpSegment, f32) {
			let timings = timings(jump, lengths);
			let cycle = timings.cycle_duration();
			let time_in_cycle = if cycle <= f32::EPSILON { 0.0 } else { elapsed % cycle };
			TwoFootedJump::<()>::segment_at_time(time_in_cycle, &timings)
		}

		pub(super) fn vertical_offset(
			jump: &TwoFootedJump<()>,
			lengths: LegSegmentLengths,
			elapsed: f32,
		) -> f32 {
			let (segment, local) = segment(jump, lengths, elapsed);
			let timings = timings(jump, lengths);
			let (prejump_squat, landing_squat) = squat_configs(jump, lengths);
			match segment {
				JumpSegment::Squat => {
					let p = local / timings.squat_duration().max(f32::EPSILON);
					-prejump_squat.vertical_drop(p, lengths)
				}
				JumpSegment::Spring | JumpSegment::Fall => {
					let cycle = timings.cycle_duration();
					let time_in_cycle = if cycle <= f32::EPSILON { 0.0 } else { elapsed % cycle };
					let since_launch = (time_in_cycle - timings.squat_end()).max(0.0);
					jump.ballistic_height(since_launch.min(jump.touchdown_time_since_launch()))
				}
				JumpSegment::Land => {
					let p = local / timings.land_duration().max(f32::EPSILON);
					-landing_squat.vertical_drop(p, lengths)
				}
			}
		}

		pub(super) fn transition_weight(
			segment: JumpSegment,
			local: f32,
			timings: &JumpTiming,
		) -> f32 {
			match segment {
				JumpSegment::Fall => {
					let blend_end = FALL_BLEND_FRACTION;
					if local < blend_end {
						(local / blend_end).clamp(0.0, 1.0)
					} else {
						1.0
					}
				}
				JumpSegment::Land => {
					let blend_window = timings.land_pose_blend_duration();
					if blend_window > f32::EPSILON {
						(local / blend_window).clamp(0.0, 1.0)
					} else {
						1.0
					}
				}
				_ => 1.0,
			}
		}
	}

	fn default_jump() -> TwoFootedJump<()> {
		TwoFootedJump::default()
	}

	#[test]
	fn fall_duration_ends_at_ballistic_touchdown() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = default_jump();
		let timings = jump.timings(lengths);
		let touchdown = jump.touchdown_time_since_launch();
		assert!(
			(timings.spring_duration + timings.air_duration - touchdown).abs() < 1e-3,
			"spring + fall should equal ballistic touchdown"
		);
		Ok(())
	}

	#[test]
	fn vertical_offset_near_zero_at_touchdown() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = default_jump();
		let timings = jump.timings(lengths);
		let y = jump.vertical_offset(lengths, timings.air_end());
		assert!(y.abs() < 0.05, "expected near-ground at land start, y={y}");
		Ok(())
	}

	#[test]
	fn segment_routing_at_boundaries() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = default_jump();
		let timings = jump.timings(lengths);

		let (seg, _) = TwoFootedJump::<()>::segment_at_time(0.0, &timings);
		assert_eq!(seg, JumpSegment::Squat);

		let (seg, _) = TwoFootedJump::<()>::segment_at_time(timings.squat_end() + 1e-4, &timings);
		assert_eq!(seg, JumpSegment::Spring);

		let (seg, _) = TwoFootedJump::<()>::segment_at_time(timings.spring_end() + 1e-4, &timings);
		assert_eq!(seg, JumpSegment::Fall);

		let (seg, _) = TwoFootedJump::<()>::segment_at_time(timings.air_end() + 1e-4, &timings);
		assert_eq!(seg, JumpSegment::Land);

		Ok(())
	}

	#[test]
	fn windup_ascent_matches_impact_speed() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = default_jump();
		let timings = jump.timings(lengths);
		let impact = launch_speed(DEFAULT_GRAVITY, DEFAULT_JUMP_HEIGHT);
		let squat_peak = Squat::<()>::default().peak_vertical_drop(lengths);

		assert!((timings.squat_ascent_duration - squat_peak / impact).abs() < 1e-3);
		Ok(())
	}

	#[test]
	fn landing_squat_speed_controls_compression_and_recovery() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let slow = TwoFootedJump::<()>::default().with_landing_squat_speed(1.0);
		let fast = TwoFootedJump::<()>::default().with_landing_squat_speed(4.0);
		let slow_timings = slow.timings(lengths);
		let fast_timings = fast.timings(lengths);
		assert!((slow_timings.land_descent_duration - 1.0).abs() < 1e-3);
		assert!((slow_timings.land_ascent_duration - 1.0).abs() < 1e-3);
		assert!(fast_timings.land_descent_duration < slow_timings.land_descent_duration);
		assert!(fast_timings.land_ascent_duration < slow_timings.land_ascent_duration);
		Ok(())
	}

	#[test]
	fn land_pose_blend_shorter_than_recovery() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let timings = default_jump().timings(lengths);
		assert!(timings.land_pose_blend_duration() < timings.land_ascent_duration);
		assert!(timings.land_pose_blend_duration() <= LAND_POSE_BLEND_MAX_SECS);
		Ok(())
	}

	#[test]
	fn land_starts_at_stand() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = default_jump();
		let elapsed = jump.timings(lengths).air_end();
		assert!(jump.landing_depth(lengths, elapsed).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn compression_visible_shortly_after_touchdown() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = default_jump();
		let timings = jump.timings(lengths);
		assert!(jump.landing_depth(lengths, timings.air_end() + 0.05) > 0.1);
		Ok(())
	}

	#[test]
	fn compression_does_not_snap_to_bottom_on_first_frame() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = default_jump();
		let timings = jump.timings(lengths);
		let one_frame = 1.0 / 120.0;
		assert!(jump.landing_depth(lengths, timings.air_end() + one_frame) < 0.5);
		Ok(())
	}

	#[test]
	fn pre_squat_speed_controls_windup_descent() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let slow = TwoFootedJump::<()>::default().with_pre_squat_speed(0.5);
		let fast = TwoFootedJump::<()>::default().with_pre_squat_speed(2.0);
		assert!(
			slow.timings(lengths).squat_descent_duration
				> fast.timings(lengths).squat_descent_duration
		);
		Ok(())
	}

	#[test]
	fn ballistic_reaches_configured_apex() -> anyhow::Result<()> {
		let g = DEFAULT_GRAVITY;
		let h = DEFAULT_JUMP_HEIGHT;
		let t_peak = launch_speed(g, h) / g;
		let apex = ballistic_height(t_peak, g, h);
		assert!((apex - h).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn vertical_profile_endpoints_and_peak() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = default_jump();
		assert!(jump.vertical_offset(lengths, 0.0).abs() < 1e-5);

		let timings = jump.timings(lengths);
		assert!(jump.vertical_offset(lengths, timings.squat_descent_duration * 0.99) < 0.0);
		assert!(jump.vertical_offset(lengths, timings.squat_end() - 0.001).abs() < 0.08);

		let apex_time = timings.squat_end()
			+ launch_speed(DEFAULT_GRAVITY, DEFAULT_JUMP_HEIGHT) / DEFAULT_GRAVITY;
		assert!((jump.vertical_offset(lengths, apex_time) - DEFAULT_JUMP_HEIGHT).abs() < 0.05);

		Ok(())
	}

	#[test]
	fn sample_matches_legacy_at_segment_boundaries() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = default_jump();
		let timings = jump.timings(lengths);
		let boundary_times = [
			0.0,
			timings.squat_descent_duration * 0.5,
			timings.squat_end() - 1e-4,
			timings.squat_end(),
			timings.squat_end() + 1e-4,
			timings.spring_end() - 1e-4,
			timings.spring_end(),
			timings.spring_end() + 1e-4,
			timings.air_end() - 1e-4,
			timings.air_end(),
			timings.air_end() + 1e-4,
			timings.air_end() + timings.land_descent_duration * 0.5,
			timings.cycle_duration() - 1e-4,
		];

		for elapsed in boundary_times {
			let derived = jump.rig_derived(lengths);
			let sample = jump.sample_at(&derived, elapsed);
			let (legacy_segment, legacy_local) = legacy::segment(&jump, lengths, elapsed);
			let legacy_y = legacy::vertical_offset(&jump, lengths, elapsed);
			let legacy_transition =
				legacy::transition_weight(legacy_segment, legacy_local, &derived.timings);

			assert_eq!(sample.segment, legacy_segment, "segment at elapsed={elapsed}");
			assert!(
				(sample.local_progress - legacy_local).abs() < 1e-5,
				"local at elapsed={elapsed}"
			);
			assert!(
				(sample.transition_weight - legacy_transition).abs() < 1e-5,
				"transition at elapsed={elapsed}"
			);
			assert!(
				(sample.vertical_offset - legacy_y).abs() < 1e-5,
				"vertical offset at elapsed={elapsed}"
			);
		}
		Ok(())
	}

	#[test]
	fn sample_matches_legacy_across_segments() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths { femur: 0.42, shin: 0.38 };
		let jump = TwoFootedJump::default()
			.with_gravity(8.5)
			.with_jump_height(1.2)
			.with_pre_squat_speed(2.5)
			.with_landing_squat_speed(4.0);
		let derived = jump.rig_derived(lengths);
		let cycle = derived.timings.cycle_duration();
		let steps = 48;

		for step in 0..=steps {
			let elapsed = cycle * step as f32 / steps as f32;
			let sample = jump.sample_at(&derived, elapsed);
			let (legacy_segment, legacy_local) = legacy::segment(&jump, lengths, elapsed);
			let legacy_y = legacy::vertical_offset(&jump, lengths, elapsed);
			let legacy_transition =
				legacy::transition_weight(legacy_segment, legacy_local, &derived.timings);

			assert_eq!(sample.segment, legacy_segment, "segment at elapsed={elapsed}");
			assert!(
				(sample.local_progress - legacy_local).abs() < 1e-5,
				"local at elapsed={elapsed}"
			);
			assert!(
				(sample.transition_weight - legacy_transition).abs() < 1e-5,
				"transition at elapsed={elapsed}"
			);
			assert!(
				(sample.vertical_offset - legacy_y).abs() < 1e-5,
				"vertical offset at elapsed={elapsed}"
			);
		}
		Ok(())
	}

	#[test]
	fn rig_derived_cache_invalidates_on_config_change() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = default_jump();
		let before = jump.rig_derived(lengths).timings;
		let changed = jump.with_gravity(DEFAULT_GRAVITY * 1.5);
		let after = changed.rig_derived(lengths).timings;
		assert_ne!(before.air_duration, after.air_duration);
		Ok(())
	}
}
