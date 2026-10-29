//! Quadruped lie-down: sphinx or curled rest, with a closed get-up.
//!
//! Clip knobs live here. The quadruped resolver maps [`crate::rigs::quadruped`]
//! samples onto imported swing / flex / twist. [`Self::held`] treats progress as
//! settle depth. The rest default stays down and uses unwrapped time for a
//! quiet breath. [`Self::for_cycle`] lies down, rests, and stands so a period
//! wrap is closed. `curl` mixes sphinx (0) toward a tucked side rest.

use crate::animations::smoothstep;

const DEFAULT_FRONT_REACH: f32 = 0.80;
const DEFAULT_FRONT_HINGE: f32 = 0.40;
const DEFAULT_HIND_TUCK: f32 = 1.60;
const DEFAULT_HIND_HINGE: f32 = 1.20;
const DEFAULT_HIP_SPLAY: f32 = 0.20;
const DEFAULT_SPINE_SETTLE: f32 = 0.40;
const DEFAULT_CURL: f32 = 0.0;
const DEFAULT_NECK_REST: f32 = 0.10;
const DEFAULT_BREATH: f32 = 0.028;
const CURLED_CURL: f32 = 0.42;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuadrupedLieDown {
	/// Front thigh reach along the ground (radians).
	pub front_reach: f32,
	/// Front shin / elbow fold (radians).
	pub front_hinge: f32,
	/// Hind thigh tuck (radians).
	pub hind_tuck: f32,
	/// Hind shin fold (radians).
	pub hind_hinge: f32,
	/// Hip splay so the haunches settle beside the body (radians).
	pub hip_splay: f32,
	/// Lumbar sagittal flatten (radians).
	pub spine_settle: f32,
	/// Lateral curl amount. `0` is sphinx; `1` is a full side tuck.
	pub curl: f32,
	/// Soft neck nod while resting (radians).
	pub neck_rest: f32,
	/// Quiet breathing amplitude (radians).
	pub breath: f32,
	/// When true, `progress` is settle depth (0 = stand, 1 = lie).
	pub held: bool,
	/// When true, progress is one lie-down / rest / get-up; otherwise rest.
	pub one_shot: bool,
}

impl Default for QuadrupedLieDown {
	fn default() -> Self {
		Self::for_rest()
	}
}

impl QuadrupedLieDown {
	/// Sphinx rest. Progress is unwrapped time for breathing.
	pub fn for_rest() -> Self {
		Self {
			front_reach: DEFAULT_FRONT_REACH,
			front_hinge: DEFAULT_FRONT_HINGE,
			hind_tuck: DEFAULT_HIND_TUCK,
			hind_hinge: DEFAULT_HIND_HINGE,
			hip_splay: DEFAULT_HIP_SPLAY,
			spine_settle: DEFAULT_SPINE_SETTLE,
			curl: DEFAULT_CURL,
			neck_rest: DEFAULT_NECK_REST,
			breath: DEFAULT_BREATH,
			held: false,
			one_shot: false,
		}
	}

	/// Stance lie. `progress` is depth; no breath oscillator.
	pub fn held() -> Self {
		Self { held: true, one_shot: false, breath: 0.0, ..Self::for_rest() }
	}

	/// Held curl toward the left haunch.
	pub fn curled() -> Self {
		Self { curl: CURLED_CURL, ..Self::held() }
	}

	/// Looping curled rest with breathing.
	pub fn curled_rest() -> Self {
		Self { curl: CURLED_CURL, ..Self::for_rest() }
	}

	/// Closed lie-down, rest, and get-up on [`Self::PERIOD`].
	pub fn for_cycle() -> Self {
		Self { held: false, one_shot: true, ..Self::for_rest() }
	}

	/// One lie-down / rest / stand, in progress units (~21 s at idle speed 0.2).
	pub const PERIOD: f32 = 4.2;
	pub const REST_END: f32 = 0.14;
	pub const DOWN_END: f32 = 0.95;
	pub const HOLD_END: f32 = 3.35;
	pub const UP_END: f32 = 4.05;

	pub const BREATH_FREQ: f32 = 0.85;

	/// Mid-lie hold on the cycle. Useful for tests and a readable still frame.
	pub fn rest_peak() -> f32 {
		(Self::DOWN_END + Self::HOLD_END) * 0.5
	}

	/// Mid lie-down ramp.
	pub fn down_mid() -> f32 {
		(Self::REST_END + Self::DOWN_END) * 0.5
	}

	/// Mid get-up ramp.
	pub fn up_mid() -> f32 {
		(Self::HOLD_END + Self::UP_END) * 0.5
	}

	/// `1` while down, `0` at stand. Closed at the period wrap for a cycle.
	pub fn settle_weight(&self, progress: f32) -> f32 {
		if self.held {
			return progress.clamp(0.0, 1.0);
		}
		if !self.one_shot {
			return 1.0;
		}
		// Cycle time is in progress units on [`Self::PERIOD`], not 0..1 clip time.
		let t = progress.clamp(0.0, Self::PERIOD);
		if t < Self::REST_END || t >= Self::UP_END {
			0.0
		} else if t < Self::DOWN_END {
			smoothstep((t - Self::REST_END) / (Self::DOWN_END - Self::REST_END))
		} else if t < Self::HOLD_END {
			1.0
		} else {
			1.0 - smoothstep((t - Self::HOLD_END) / (Self::UP_END - Self::HOLD_END))
		}
	}

	/// Body lowering as a fraction of the measured combined leg length.
	/// Follow the front-chain compression so entry/exit do not sink before folding.
	pub fn body_lowering(&self, progress: f32) -> f32 {
		let depth = self.settle_weight(progress);
		let compression = |depth: f32| {
			2.0 - (self.front_reach * depth).cos()
				- ((self.front_reach + self.front_hinge) * depth).cos()
		};
		let full = compression(1.0);
		if full <= f32::EPSILON {
			0.0
		} else {
			0.65 * compression(depth) / full
		}
	}

	/// `1` while the rest hold can breathe. Zero on the stand and ramps.
	pub fn breath_weight(&self, progress: f32) -> f32 {
		if self.held || self.breath.abs() <= f32::EPSILON {
			return 0.0;
		}
		if !self.one_shot {
			return 1.0;
		}
		let t = progress.clamp(0.0, Self::PERIOD);
		if t < Self::DOWN_END || t >= Self::HOLD_END {
			0.0
		} else {
			1.0
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::animations::QuadrupedSit;

	#[test]
	fn held_depth_is_progress() -> anyhow::Result<()> {
		let lie = QuadrupedLieDown::held();
		assert!((lie.settle_weight(0.0)).abs() < 1e-6);
		assert!((lie.settle_weight(1.0) - 1.0).abs() < 1e-6);
		assert!(lie.breath_weight(1.0) < 1e-6);
		Ok(())
	}

	#[test]
	fn rest_stays_down_and_breathes() -> anyhow::Result<()> {
		let lie = QuadrupedLieDown::for_rest();
		assert!((lie.settle_weight(0.0) - 1.0).abs() < 1e-6);
		assert!((lie.breath_weight(0.4) - 1.0).abs() < 1e-6);
		assert!(lie.curl.abs() < 1e-6);
		Ok(())
	}

	#[test]
	fn curled_is_a_side_tuck() -> anyhow::Result<()> {
		let sphinx = QuadrupedLieDown::held();
		let curled = QuadrupedLieDown::curled();
		assert!(curled.curl > sphinx.curl);
		assert!(curled.curl > 0.2);
		Ok(())
	}

	#[test]
	fn cycle_lies_down_then_gets_up() -> anyhow::Result<()> {
		let lie = QuadrupedLieDown::for_cycle();
		assert!(lie.settle_weight(0.0) < 1e-5);
		assert!(lie.settle_weight(QuadrupedLieDown::down_mid()) > 0.2);
		assert!(lie.settle_weight(QuadrupedLieDown::down_mid()) < 0.95);
		assert!(lie.settle_weight(QuadrupedLieDown::rest_peak()) > 0.9);
		assert!(lie.settle_weight(QuadrupedLieDown::up_mid()) < 0.95);
		assert!(lie.settle_weight(QuadrupedLieDown::PERIOD) < 1e-5);
		assert!(lie.breath_weight(QuadrupedLieDown::rest_peak()) > 0.9);
		assert!(lie.breath_weight(0.0) < 1e-5);
		Ok(())
	}

	#[test]
	fn sphinx_reaches_more_in_front_than_it_plants_in_a_sit() -> anyhow::Result<()> {
		let lie = QuadrupedLieDown::default();
		let sit = QuadrupedSit::default();
		assert!(lie.front_reach > sit.front_plant);
		assert!(lie.spine_settle > lie.neck_rest);
		assert!(lie.breath < lie.neck_rest);
		Ok(())
	}
}
