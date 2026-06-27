//! Two-footed jump timing and vertical motion.
//!
//! The jump is parameterized by four values:
//!
//! - **`jump_height`** — apex height above the take-off point (world units).
//! - **`gravity`** — constant downward acceleration (units/s²).
//! - **`pre_squat_speed`** — how fast the character squats down before take-off
//!   (stand to bottom in `1/pre_squat_speed` seconds). The return to stand before
//!   spring is matched to launch speed from `jump_height` and `gravity`.
//! - **`landing_squat_speed`** — scales landing compression (matched to impact speed)
//!   and sets recovery rate (stand-up in `1/landing_squat_speed` seconds).

use std::marker::PhantomData;

use crozon_rigs::humanoid::LegSegmentLengths;

use crate::animations::{Land, Squat};

pub const DEFAULT_GRAVITY: f32 = 9.8;
pub const DEFAULT_JUMP_HEIGHT: f32 = 1.5;
/// Default pre-jump squat-down rate (bottom in ~0.7 s).
pub const DEFAULT_PRE_SQUAT_SPEED: f32 = 1.4;
/// Default landing recovery rate; also scales impact-matched compression.
pub const DEFAULT_LANDING_SQUAT_SPEED: f32 = 1.2;
pub const DEFAULT_SPRING_DURATION: f32 = 0.15;

/// Fraction of the airborne segment used to blend from spring into fall spread.
pub const FALL_BLEND_FRACTION: f32 = 0.25;
/// Fraction of the **compression** half used to blend fall arms into landing pose.
pub const LAND_BLEND_FRACTION: f32 = 0.25;
/// Upper cap on fall-to-land pose blend so compression is not delayed by recovery timing.
pub const LAND_POSE_BLEND_MAX_SECS: f32 = 0.06;

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
	/// Ballistic air time from take-off back to launch height (seconds).
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

#[derive(Debug, Clone)]
pub struct TwoFootedJump<Rig> {
	/// Elapsed time in seconds (loops over the full jump cycle).
	pub elapsed: f32,
	/// Downward acceleration for ballistic motion (units/s²).
	pub gravity: f32,
	/// Apex height above the take-off point (world units).
	pub jump_height: f32,
	/// Stand-to-bottom rate before take-off (`1/speed` = descent seconds).
	pub pre_squat_speed: f32,
	/// Landing compression scale and recovery rate after touch-down.
	pub landing_squat_speed: f32,
	_rig: PhantomData<Rig>,
}

impl<Rig> Default for TwoFootedJump<Rig> {
	fn default() -> Self {
		Self {
			elapsed: 0.0,
			gravity: DEFAULT_GRAVITY,
			jump_height: DEFAULT_JUMP_HEIGHT,
			pre_squat_speed: DEFAULT_PRE_SQUAT_SPEED,
			landing_squat_speed: DEFAULT_LANDING_SQUAT_SPEED,
			_rig: PhantomData,
		}
	}
}

impl<Rig> TwoFootedJump<Rig> {
	pub fn new(elapsed: f32) -> Self {
		Self { elapsed, ..Self::default() }
	}

	pub fn from_time(elapsed: f32) -> Self {
		Self::new(elapsed)
	}

	pub fn with_gravity(mut self, gravity: f32) -> Self {
		self.gravity = gravity;
		self
	}

	pub fn with_jump_height(mut self, jump_height: f32) -> Self {
		self.jump_height = jump_height;
		self
	}

	pub fn with_pre_squat_speed(mut self, pre_squat_speed: f32) -> Self {
		self.pre_squat_speed = pre_squat_speed;
		self
	}

	pub fn with_landing_squat_speed(mut self, landing_squat_speed: f32) -> Self {
		self.landing_squat_speed = landing_squat_speed;
		self
	}

	fn squat_configs(&self, lengths: LegSegmentLengths) -> (Squat<Rig>, Land<Rig>) {
		let impact = launch_speed(self.gravity, self.jump_height);
		let squat_peak = Squat::<Rig>::default().peak_vertical_drop(lengths);
		let land_peak = Land::<Rig>::default().peak_vertical_drop(lengths);

		let windup_descent = self.pre_squat_speed.max(MIN_SPEED);
		let windup_ascent = if squat_peak > f32::EPSILON {
			impact / squat_peak
		} else {
			windup_descent
		};

		let land_compression = if land_peak > f32::EPSILON {
			(impact / land_peak) * self.landing_squat_speed.max(MIN_SPEED)
		} else {
			self.landing_squat_speed.max(MIN_SPEED)
		};
		let land_recovery = self.landing_squat_speed.max(MIN_SPEED);

		let windup = Squat::with_speeds(windup_descent, windup_ascent.max(MIN_SPEED));
		let landing = Land::with_speeds(
			land_compression,
			land_recovery,
			Squat::<Rig>::default(),
		);
		(windup, landing)
	}

	pub fn timings(&self, lengths: LegSegmentLengths) -> JumpTiming {
		let (windup, landing) = self.squat_configs(lengths);
		JumpTiming {
			squat_descent_duration: windup.descent_duration().max(MIN_SEGMENT_DURATION),
			squat_ascent_duration: windup.ascent_duration().max(MIN_SEGMENT_DURATION),
			spring_duration: DEFAULT_SPRING_DURATION,
			air_duration: air_duration(self.gravity, self.jump_height),
			land_descent_duration: landing.descent_duration().max(MIN_SEGMENT_DURATION),
			land_ascent_duration: landing.ascent_duration().max(MIN_SEGMENT_DURATION),
		}
	}

	pub fn cycle_duration(&self, lengths: LegSegmentLengths) -> f32 {
		self.timings(lengths).cycle_duration()
	}

	pub fn time_in_cycle(&self, lengths: LegSegmentLengths) -> f32 {
		let cycle = self.cycle_duration(lengths);
		if cycle <= f32::EPSILON {
			return 0.0;
		}
		self.elapsed % cycle
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
			return (JumpSegment::Fall, t / timings.air_duration);
		}
		t -= timings.air_duration;
		(JumpSegment::Land, t)
	}

	pub fn segment(&self, lengths: LegSegmentLengths) -> (JumpSegment, f32) {
		Self::segment_at_time(self.time_in_cycle(lengths), &self.timings(lengths))
	}

	pub fn time_since_launch(&self, lengths: LegSegmentLengths) -> f32 {
		(self.time_in_cycle(lengths) - self.timings(lengths).squat_end()).max(0.0)
	}

	pub fn prejump_squat(&self, lengths: LegSegmentLengths) -> Squat<Rig> {
		let (windup, _) = self.squat_configs(lengths);
		let (segment, time) = self.segment(lengths);
		if segment != JumpSegment::Squat {
			return windup.at_segment_time(0.0);
		}
		windup.at_segment_time(time)
	}

	pub fn landing_squat(&self, lengths: LegSegmentLengths) -> Land<Rig> {
		let (_, landing) = self.squat_configs(lengths);
		let (segment, time) = self.segment(lengths);
		if segment != JumpSegment::Land {
			return landing.at_segment_time(0.0);
		}
		landing.at_segment_time(time)
	}

	pub fn vertical_offset(&self, lengths: LegSegmentLengths) -> f32 {
		let (segment, _) = self.segment(lengths);

		match segment {
			JumpSegment::Squat => -self.prejump_squat(lengths).vertical_drop(lengths),
			JumpSegment::Spring | JumpSegment::Fall => {
				self.ballistic_height(self.time_since_launch(lengths))
			}
			JumpSegment::Land => -self.landing_squat(lengths).vertical_drop(lengths),
		}
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

	fn default_jump() -> TwoFootedJump<()> {
		TwoFootedJump::from_time(0.0)
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
	fn ascent_and_compression_match_impact_speed() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = default_jump();
		let timings = jump.timings(lengths);
		let impact = launch_speed(DEFAULT_GRAVITY, DEFAULT_JUMP_HEIGHT);
		let squat_peak = Squat::<()>::default().peak_vertical_drop(lengths);
		let land_peak = Land::<()>::default().peak_vertical_drop(lengths);

		assert!((timings.squat_ascent_duration - squat_peak / impact).abs() < 1e-3);
		let expected_land_descent = land_peak
			/ (impact * DEFAULT_LANDING_SQUAT_SPEED.max(f32::EPSILON));
		assert!((timings.land_descent_duration - expected_land_descent).abs() < 1e-3);
		Ok(())
	}

	#[test]
	fn higher_jump_shortens_land_compression() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let low = TwoFootedJump::<()>::from_time(0.0).with_jump_height(1.0);
		let high = TwoFootedJump::<()>::from_time(0.0).with_jump_height(3.0);
		assert!(high.timings(lengths).land_descent_duration < low.timings(lengths).land_descent_duration);
		Ok(())
	}

	#[test]
	fn landing_descent_faster_than_recovery() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = default_jump();
		let timings = jump.timings(lengths);
		assert!(timings.land_descent_duration < timings.land_ascent_duration);
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
		let jump = TwoFootedJump::<()>::from_time(default_jump().timings(lengths).air_end());
		assert!(jump.landing_squat(lengths).depth().abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn compression_visible_shortly_after_touchdown() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let timings = default_jump().timings(lengths);
		let jump = TwoFootedJump::<()>::from_time(timings.air_end() + 0.02);
		assert!(jump.landing_squat(lengths).depth() > 0.1);
		Ok(())
	}

	#[test]
	fn pre_squat_speed_controls_windup_descent() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let slow = TwoFootedJump::<()>::from_time(0.0).with_pre_squat_speed(0.5);
		let fast = TwoFootedJump::<()>::from_time(0.0).with_pre_squat_speed(2.0);
		assert!(slow.timings(lengths).squat_descent_duration > fast.timings(lengths).squat_descent_duration);
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
		assert!(jump.vertical_offset(lengths).abs() < 1e-5);

		let timings = jump.timings(lengths);
		let squat_bottom =
			TwoFootedJump::<()>::from_time(timings.squat_descent_duration * 0.99);
		assert!(squat_bottom.vertical_offset(lengths) < 0.0);

		let squat_end = TwoFootedJump::<()>::from_time(timings.squat_end() - 0.001);
		assert!(squat_end.vertical_offset(lengths).abs() < 0.08);

		let apex_time = timings.squat_end()
			+ launch_speed(DEFAULT_GRAVITY, DEFAULT_JUMP_HEIGHT) / DEFAULT_GRAVITY;
		let apex = TwoFootedJump::<()>::from_time(apex_time);
		assert!((apex.vertical_offset(lengths) - DEFAULT_JUMP_HEIGHT).abs() < 0.05);

		Ok(())
	}
}
