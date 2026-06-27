use std::marker::PhantomData;

use crozon_rigs::humanoid::LegSegmentLengths;

use crate::animations::{
	Land, Squat, DEFAULT_RECOVERY_SPEED, DEFAULT_WINDUP_DESCENT_SPEED,
};

pub const DEFAULT_GRAVITY: f32 = 9.8;
pub const DEFAULT_JUMP_HEIGHT: f32 = 1.5;
pub const DEFAULT_SPRING_DURATION: f32 = 0.15;

/// Fraction of the airborne segment used to blend from spring into fall spread.
pub const FALL_BLEND_FRACTION: f32 = 0.25;
/// Fraction of the landing segment used to blend from fall spread into absorption.
pub const LAND_BLEND_FRACTION: f32 = 0.25;

const MIN_SEGMENT_DURATION: f32 = 1e-3;
const MIN_SPEED: f32 = 1e-3;

/// Artist multipliers applied on top of [`TwoFootedJump::auto_scale`] baselines.
#[derive(Debug, Clone, Copy)]
pub struct JumpSquatTuning {
	/// >1 slows windup descent; <1 speeds it up.
	pub windup_descent_scale: f32,
	/// >1 slows landing recovery; <1 speeds it up.
	pub recovery_scale: f32,
}

impl Default for JumpSquatTuning {
	fn default() -> Self {
		Self { windup_descent_scale: 1.0, recovery_scale: 1.0 }
	}
}

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
	pub elapsed: f32,
	pub gravity: f32,
	pub jump_height: f32,
	pub windup: Squat<Rig>,
	pub landing: Land<Rig>,
	pub spring_duration: f32,
	pub tuning: JumpSquatTuning,
	_rig: PhantomData<Rig>,
}

impl<Rig> Default for TwoFootedJump<Rig> {
	fn default() -> Self {
		Self {
			elapsed: 0.0,
			gravity: DEFAULT_GRAVITY,
			jump_height: DEFAULT_JUMP_HEIGHT,
			windup: Squat::default(),
			landing: Land::default(),
			spring_duration: DEFAULT_SPRING_DURATION,
			tuning: JumpSquatTuning::default(),
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

	/// Build a jump with speeds scaled from height and impact speed.
	pub fn auto_scale(
		elapsed: f32,
		gravity: f32,
		jump_height: f32,
		lengths: LegSegmentLengths,
	) -> Self {
		Self::auto_scale_tuned(elapsed, gravity, jump_height, lengths, JumpSquatTuning::default())
	}

	pub fn auto_scale_tuned(
		elapsed: f32,
		gravity: f32,
		jump_height: f32,
		lengths: LegSegmentLengths,
		tuning: JumpSquatTuning,
	) -> Self {
		let (windup, landing) = Self::squat_configs(gravity, jump_height, lengths, tuning);
		Self {
			elapsed,
			gravity,
			jump_height,
			windup,
			landing,
			spring_duration: DEFAULT_SPRING_DURATION,
			tuning,
			_rig: PhantomData,
		}
	}

	fn squat_configs(
		gravity: f32,
		jump_height: f32,
		lengths: LegSegmentLengths,
		tuning: JumpSquatTuning,
	) -> (Squat<Rig>, Land<Rig>) {
		let impact = launch_speed(gravity, jump_height);
		let squat_peak = Squat::<Rig>::default().peak_vertical_drop(lengths);
		let land_peak = Land::<Rig>::default().peak_vertical_drop(lengths);

		let windup_descent = suggest_windup_descent(gravity, jump_height) / tuning.windup_descent_scale;
		let windup_ascent = if squat_peak > f32::EPSILON {
			impact / squat_peak
		} else {
			DEFAULT_WINDUP_DESCENT_SPEED
		};

		let land_descent = if land_peak > f32::EPSILON {
			impact / land_peak
		} else {
			DEFAULT_WINDUP_DESCENT_SPEED
		};
		let land_recovery =
			suggest_recovery(gravity, jump_height) / tuning.recovery_scale;

		let template = Squat::<Rig>::default();
		let windup = Squat::with_speeds(windup_descent.max(MIN_SPEED), windup_ascent.max(MIN_SPEED));
		let landing = Land::with_speeds(
			land_descent.max(MIN_SPEED),
			land_recovery.max(MIN_SPEED),
			template,
		);
		(windup, landing)
	}

	fn resolved_squats(&self, lengths: LegSegmentLengths) -> (Squat<Rig>, Land<Rig>) {
		Self::squat_configs(self.gravity, self.jump_height, lengths, self.tuning)
	}

	pub fn with_gravity(mut self, gravity: f32) -> Self {
		self.gravity = gravity;
		self
	}

	pub fn with_jump_height(mut self, jump_height: f32) -> Self {
		self.jump_height = jump_height;
		self
	}

	pub fn with_spring_duration(mut self, spring_duration: f32) -> Self {
		self.spring_duration = spring_duration;
		self
	}

	/// Factor >1 lengthens windup descent (slower down); <1 shortens it.
	pub fn with_slower_initial_squat_down_by(mut self, factor: f32) -> Self {
		self.tuning.windup_descent_scale *= factor.max(f32::EPSILON);
		self
	}

	/// Factor >1 lengthens landing recovery (slower stand-up); <1 shortens it.
	pub fn with_slower_landing_recovery_by(mut self, factor: f32) -> Self {
		self.tuning.recovery_scale *= factor.max(f32::EPSILON);
		self
	}

	pub fn timings(&self, lengths: LegSegmentLengths) -> JumpTiming {
		let (windup, landing) = self.resolved_squats(lengths);
		JumpTiming {
			squat_descent_duration: windup.descent_duration().max(MIN_SEGMENT_DURATION),
			squat_ascent_duration: windup.ascent_duration().max(MIN_SEGMENT_DURATION),
			spring_duration: self.spring_duration,
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
		let (windup, _) = self.resolved_squats(lengths);
		let (segment, time) = self.segment(lengths);
		if segment != JumpSegment::Squat {
			return windup.at_segment_time(0.0);
		}
		windup.at_segment_time(time)
	}

	pub fn landing_squat(&self, lengths: LegSegmentLengths) -> Land<Rig> {
		let (_, landing) = self.resolved_squats(lengths);
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

/// Scales artist-tuned windup descent with impact speed relative to the default jump.
pub fn suggest_windup_descent(gravity: f32, jump_height: f32) -> f32 {
	let impact = launch_speed(gravity, jump_height);
	let ref_impact = launch_speed(DEFAULT_GRAVITY, DEFAULT_JUMP_HEIGHT);
	let scale = (impact / ref_impact).clamp(0.5, 2.0);
	DEFAULT_WINDUP_DESCENT_SPEED * scale
}

/// Scales landing recovery with impact speed relative to the default jump.
pub fn suggest_recovery(gravity: f32, jump_height: f32) -> f32 {
	let impact = launch_speed(gravity, jump_height);
	let ref_impact = launch_speed(DEFAULT_GRAVITY, DEFAULT_JUMP_HEIGHT);
	let scale = (impact / ref_impact).clamp(0.5, 2.0);
	DEFAULT_RECOVERY_SPEED * scale
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

	fn auto_jump() -> TwoFootedJump<()> {
		TwoFootedJump::auto_scale(0.0, DEFAULT_GRAVITY, DEFAULT_JUMP_HEIGHT, LegSegmentLengths::default())
	}

	#[test]
	fn segment_routing_at_boundaries() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = auto_jump();
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
	fn auto_scale_uses_physics_for_ascent_and_land_compression() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = auto_jump();
		let timings = jump.timings(lengths);
		let impact = launch_speed(DEFAULT_GRAVITY, DEFAULT_JUMP_HEIGHT);
		let squat_peak = Squat::<()>::default().peak_vertical_drop(lengths);
		let land_peak = Land::<()>::default().peak_vertical_drop(lengths);

		assert!((timings.squat_ascent_duration - squat_peak / impact).abs() < 1e-3);
		assert!((timings.land_descent_duration - land_peak / impact).abs() < 1e-3);
		Ok(())
	}

	#[test]
	fn higher_jump_shortens_land_compression() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let low = TwoFootedJump::<()>::auto_scale(0.0, DEFAULT_GRAVITY, 1.0, lengths);
		let high = TwoFootedJump::<()>::auto_scale(0.0, DEFAULT_GRAVITY, 3.0, lengths);
		assert!(high.timings(lengths).land_descent_duration < low.timings(lengths).land_descent_duration);
		Ok(())
	}

	#[test]
	fn physics_ascent_faster_than_slow_uniform() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = auto_jump();
		let (windup, _) = jump.resolved_squats(lengths);
		let slow_uniform = Squat::<()>::with_speeds(0.25, 0.25);
		assert!(windup.ascent_duration() < slow_uniform.ascent_duration());
		Ok(())
	}

	#[test]
	fn landing_descent_faster_than_recovery() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = auto_jump();
		let timings = jump.timings(lengths);
		assert!(timings.land_descent_duration < timings.land_ascent_duration);
		Ok(())
	}

	#[test]
	fn land_starts_at_stand() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = TwoFootedJump::<()>::auto_scale(
			auto_jump().timings(lengths).air_end(),
			DEFAULT_GRAVITY,
			DEFAULT_JUMP_HEIGHT,
			lengths,
		);
		assert!(jump.landing_squat(lengths).depth().abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn slower_windup_down_by_extends_descent() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let base = auto_jump();
		let slower = TwoFootedJump::<()>::auto_scale(0.0, DEFAULT_GRAVITY, DEFAULT_JUMP_HEIGHT, lengths)
			.with_slower_initial_squat_down_by(2.0);
		let (base_windup, _) = base.resolved_squats(lengths);
		let (slow_windup, _) = slower.resolved_squats(lengths);
		assert!(slow_windup.descent_duration() > base_windup.descent_duration());
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
		let jump = auto_jump();
		let elapsed = jump.timings(lengths).squat_end() + DEFAULT_SPRING_DURATION * 0.5;
		let jump = TwoFootedJump::<()>::auto_scale(
			elapsed,
			DEFAULT_GRAVITY,
			DEFAULT_JUMP_HEIGHT,
			lengths,
		);
		assert!(jump.vertical_offset(lengths) > 0.0);
		Ok(())
	}

	#[test]
	fn vertical_profile_endpoints_and_peak() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = auto_jump();
		assert!(jump.vertical_offset(lengths).abs() < 1e-5);

		let timings = jump.timings(lengths);
		let squat_bottom = TwoFootedJump::<()>::auto_scale(
			timings.squat_descent_duration * 0.99,
			DEFAULT_GRAVITY,
			DEFAULT_JUMP_HEIGHT,
			lengths,
		);
		assert!(squat_bottom.vertical_offset(lengths) < 0.0);

		let squat_end = TwoFootedJump::<()>::auto_scale(
			timings.squat_end() - 0.001,
			DEFAULT_GRAVITY,
			DEFAULT_JUMP_HEIGHT,
			lengths,
		);
		assert!(squat_end.vertical_offset(lengths).abs() < 0.08);

		let apex_time = timings.squat_end()
			+ launch_speed(DEFAULT_GRAVITY, DEFAULT_JUMP_HEIGHT) / DEFAULT_GRAVITY;
		let apex = TwoFootedJump::<()>::auto_scale(
			apex_time,
			DEFAULT_GRAVITY,
			DEFAULT_JUMP_HEIGHT,
			lengths,
		);
		assert!((apex.vertical_offset(lengths) - DEFAULT_JUMP_HEIGHT).abs() < 0.05);

		Ok(())
	}
}
