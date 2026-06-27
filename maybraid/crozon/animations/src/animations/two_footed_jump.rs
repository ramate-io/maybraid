use std::marker::PhantomData;

use crozon_rigs::humanoid::LegSegmentLengths;

use crate::animations::{Land, Squat, descent_ascent_amount};

pub const DEFAULT_GRAVITY: f32 = 9.8;
pub const DEFAULT_JUMP_HEIGHT: f32 = 1.5;
/// Descents from stand to full squat per second.
pub const DEFAULT_SQUAT_DESCENT_SPEED: f32 = 0.6;
/// Returns to stand after landing per second.
pub const DEFAULT_LAND_RECOVERY_SPEED: f32 = 0.4;
pub const DEFAULT_SPRING_DURATION: f32 = 0.15;

/// Fraction of the airborne segment used to blend from spring into fall spread.
pub const FALL_BLEND_FRACTION: f32 = 0.25;

const MIN_SEGMENT_DURATION: f32 = 1e-3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JumpTiming {
	pub squat_descent_duration: f32,
	pub squat_ascent_duration: f32,
	pub spring_duration: f32,
	pub air_duration: f32,
	pub land_descent_duration: f32,
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
	/// Elapsed time in seconds (need not be normalized).
	pub elapsed: f32,
	pub gravity: f32,
	pub jump_height: f32,
	pub squat_descent_speed: f32,
	pub land_recovery_speed: f32,
	pub spring_duration: f32,
	_rig: PhantomData<Rig>,
}

impl<Rig> Default for TwoFootedJump<Rig> {
	fn default() -> Self {
		Self {
			elapsed: 0.0,
			gravity: DEFAULT_GRAVITY,
			jump_height: DEFAULT_JUMP_HEIGHT,
			squat_descent_speed: DEFAULT_SQUAT_DESCENT_SPEED,
			land_recovery_speed: DEFAULT_LAND_RECOVERY_SPEED,
			spring_duration: DEFAULT_SPRING_DURATION,
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

	pub fn with_squat_descent_speed(mut self, squat_descent_speed: f32) -> Self {
		self.squat_descent_speed = squat_descent_speed;
		self
	}

	pub fn with_land_recovery_speed(mut self, land_recovery_speed: f32) -> Self {
		self.land_recovery_speed = land_recovery_speed;
		self
	}

	pub fn with_spring_duration(mut self, spring_duration: f32) -> Self {
		self.spring_duration = spring_duration;
		self
	}

	pub fn timings(&self, lengths: LegSegmentLengths) -> JumpTiming {
		let impact_speed = launch_speed(self.gravity, self.jump_height);
		let squat_peak = Squat::<Rig>::default().peak_vertical_drop(lengths);
		let land_peak = Land::<Rig>::default().peak_vertical_drop(lengths);

		JumpTiming {
			squat_descent_duration: (1.0 / self.squat_descent_speed).max(MIN_SEGMENT_DURATION),
			squat_ascent_duration: (squat_peak / impact_speed).max(MIN_SEGMENT_DURATION),
			spring_duration: self.spring_duration,
			air_duration: air_duration(self.gravity, self.jump_height),
			land_descent_duration: (land_peak / impact_speed).max(MIN_SEGMENT_DURATION),
			land_ascent_duration: (1.0 / self.land_recovery_speed).max(MIN_SEGMENT_DURATION),
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

	pub fn prejump_squat_amount(&self, lengths: LegSegmentLengths) -> f32 {
		let timings = self.timings(lengths);
		let (segment, time) = self.segment(lengths);
		if segment != JumpSegment::Squat {
			return 0.0;
		}
		descent_ascent_amount(
			time,
			timings.squat_descent_duration,
			timings.squat_ascent_duration,
		)
	}

	pub fn land_squat_amount(&self, lengths: LegSegmentLengths) -> f32 {
		let timings = self.timings(lengths);
		let (segment, time) = self.segment(lengths);
		if segment != JumpSegment::Land {
			return 0.0;
		}
		descent_ascent_amount(
			time,
			timings.land_descent_duration,
			timings.land_ascent_duration,
		)
	}

	pub fn vertical_offset(&self, lengths: LegSegmentLengths) -> f32 {
		let (segment, _) = self.segment(lengths);

		match segment {
			JumpSegment::Squat => {
				-Squat::<Rig>::with_amount(self.prejump_squat_amount(lengths)).vertical_drop(lengths)
			}
			JumpSegment::Spring | JumpSegment::Fall => {
				self.ballistic_height(self.time_since_launch(lengths))
			}
			JumpSegment::Land => {
				-Land::<Rig>::with_amount(self.land_squat_amount(lengths), Squat::<Rig>::default())
					.vertical_drop(lengths)
			}
		}
	}
}

/// Initial launch speed for a peak height under constant gravity.
pub fn launch_speed(gravity: f32, jump_height: f32) -> f32 {
	(2.0 * gravity * jump_height).sqrt()
}

/// Time from launch to apex and back to the launch height.
pub fn air_duration(gravity: f32, jump_height: f32) -> f32 {
	2.0 * launch_speed(gravity, jump_height) / gravity
}

/// Height above launch point `t` seconds after takeoff.
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

	#[test]
	fn segment_routing_at_boundaries() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = TwoFootedJump::<()>::default();
		let timings = jump.timings(lengths);

		let (seg, _) = TwoFootedJump::<()>::segment_at_time(0.0, &timings);
		assert_eq!(seg, JumpSegment::Squat);

		let (seg, _) = TwoFootedJump::<()>::segment_at_time(timings.squat_end() + 1e-4, &timings);
		assert_eq!(seg, JumpSegment::Spring);

		let (seg, _) =
			TwoFootedJump::<()>::segment_at_time(timings.spring_end() + 1e-4, &timings);
		assert_eq!(seg, JumpSegment::Fall);

		let (seg, _) =
			TwoFootedJump::<()>::segment_at_time(timings.air_end() + 1e-4, &timings);
		assert_eq!(seg, JumpSegment::Land);

		Ok(())
	}

	#[test]
	fn landing_descent_faster_than_recovery() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = TwoFootedJump::<()>::default();
		let timings = jump.timings(lengths);
		assert!(timings.land_descent_duration < timings.land_ascent_duration);
		Ok(())
	}

	#[test]
	fn land_starts_with_visible_flex() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = TwoFootedJump::<()>::new(TwoFootedJump::<()>::default().timings(lengths).air_end() + 0.01);
		assert!(jump.land_squat_amount(lengths) > 0.0);
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
	fn spring_phase_rises_with_extension() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = TwoFootedJump::<()>::default();
		let elapsed = jump.timings(lengths).squat_end() + DEFAULT_SPRING_DURATION * 0.5;
		let jump = TwoFootedJump::<()>::new(elapsed);
		assert!(jump.vertical_offset(lengths) > 0.0);
		Ok(())
	}

	#[test]
	fn vertical_profile_endpoints_and_peak() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = TwoFootedJump::<()>::new(0.0);
		assert!(jump.vertical_offset(lengths).abs() < 1e-5);

		let timings = jump.timings(lengths);
		let squat_bottom = TwoFootedJump::<()>::new(timings.squat_descent_duration * 0.99);
		assert!(squat_bottom.vertical_offset(lengths) < 0.0);

		let squat_end = TwoFootedJump::<()>::new(timings.squat_end() - 0.001);
		assert!(squat_end.vertical_offset(lengths).abs() < 0.08);

		let apex_time = timings.squat_end() + launch_speed(DEFAULT_GRAVITY, DEFAULT_JUMP_HEIGHT) / DEFAULT_GRAVITY;
		let apex = TwoFootedJump::<()>::new(apex_time);
		assert!((apex.vertical_offset(lengths) - DEFAULT_JUMP_HEIGHT).abs() < 0.05);

		Ok(())
	}
}
