//! Temporal stick-flick classifier. A flick is out → peak → home; a hold is not.
//!
//! Shared by the skill-map steer (RB + either stick) and third-person zoom
//! (right stick alone). Gameplay systems should sample [`crate::VirtualPad::look_analog`]
//! / [`crate::VirtualPad::move_stick`], not the look-delta [`crate::VirtualPad::look_stick`].

use bevy::prelude::*;

/// Rest nest. Below this the stick has come home.
pub const FLICK_REST: f32 = 0.28;
/// Peak must clear this or it was a nudge.
pub const FLICK_MIN_PEAK: f32 = 0.28;
/// Faster than a hold, slower than a one-frame tap.
pub const FLICK_MIN_SECS: f32 = 0.02;
pub const FLICK_MAX_SECS: f32 = 0.32;

/// One analog stick's flick state machine.
#[derive(Clone, Debug, Default)]
pub struct FlickLane {
	active: bool,
	t0: f32,
	peak: Vec2,
}

impl FlickLane {
	pub fn reset(&mut self) {
		self.active = false;
		self.peak = Vec2::ZERO;
	}

	/// Feed the current analog stick. Returns the peak throw when a flick completes.
	pub fn sample(&mut self, t: f32, stick: Vec2) -> Option<Vec2> {
		let mag = stick.length();
		if !self.active {
			if mag >= FLICK_REST {
				self.active = true;
				self.t0 = t;
				self.peak = stick;
			}
			return None;
		}
		if mag > self.peak.length() {
			self.peak = stick;
		}
		let elapsed = t - self.t0;
		if elapsed > FLICK_MAX_SECS {
			if mag < FLICK_REST {
				self.reset();
			}
			return None;
		}
		if mag >= FLICK_REST {
			return None;
		}
		self.active = false;
		let peak = self.peak;
		self.peak = Vec2::ZERO;
		if elapsed >= FLICK_MIN_SECS && peak.length() >= FLICK_MIN_PEAK {
			Some(peak)
		} else {
			None
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn throw(lane: &mut FlickLane, t0: f32, stick: Vec2) -> Option<Vec2> {
		let mut last = None;
		for (i, mag) in [0.0, 0.75, 1.0, 0.4, 0.0].into_iter().enumerate() {
			last = lane.sample(t0 + i as f32 * 0.04, stick * mag);
		}
		last
	}

	#[test]
	fn a_fast_throw_is_a_flick() {
		let mut lane = FlickLane::default();
		let peak = throw(&mut lane, 0.0, Vec2::X).expect("flick");
		assert!(peak.x > 0.9);
	}

	#[test]
	fn a_held_stick_is_not_a_flick() {
		let mut lane = FlickLane::default();
		assert!(lane.sample(0.0, Vec2::X).is_none());
		assert!(lane.sample(0.3, Vec2::X).is_none());
		assert!(lane.sample(0.35, Vec2::ZERO).is_none());
	}

	#[test]
	fn a_nudge_is_not_a_flick() {
		let mut lane = FlickLane::default();
		assert!(lane.sample(0.0, Vec2::X * 0.15).is_none());
		assert!(lane.sample(0.08, Vec2::X * 0.2).is_none());
		assert!(lane.sample(0.12, Vec2::ZERO).is_none());
	}
}
