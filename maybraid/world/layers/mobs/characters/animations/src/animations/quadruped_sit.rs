//! Quadruped sit: dog-style haunch settle, or a closed sit-down / get-up.
//!
//! Clip knobs live here. The quadruped resolver maps [`crate::rigs::quadruped`]
//! samples onto imported swing / flex / twist. Progress is owned by the
//! controller. [`Self::held`] treats it as settle depth; the idle default stays
//! seated and uses unwrapped time for a quiet sway; [`Self::for_cycle`] sits,
//! holds, and stands so a period wrap is closed.

use crate::animations::smoothstep;

const DEFAULT_HIND_STRIDE: f32 = 0.72;
const DEFAULT_HIND_HINGE: f32 = 0.98;
const DEFAULT_HIP_SPLAY: f32 = 0.16;
const DEFAULT_FRONT_PLANT: f32 = 0.10;
const DEFAULT_FRONT_HINGE: f32 = 0.14;
const DEFAULT_SPINE_UPRIGHT: f32 = 0.28;
const DEFAULT_SPINE_GATHER: f32 = 0.12;
const DEFAULT_NECK_ALERT: f32 = 0.16;
const DEFAULT_SWAY: f32 = 0.016;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuadrupedSit {
	/// Hind thigh tuck under the rump (radians).
	pub hind_stride: f32,
	/// Hind shin fold (radians).
	pub hind_hinge: f32,
	/// Symmetric hip splay so the sit reads as settled haunches (radians).
	pub hip_splay: f32,
	/// Slight front-leg plant (radians).
	pub front_plant: f32,
	/// Residual front shin fold (radians).
	pub front_hinge: f32,
	/// Lumbar sagittal lift so the chest stays more upright than a lie (radians).
	pub spine_upright: f32,
	/// Lumbar lateral gather (radians).
	pub spine_gather: f32,
	/// Alert neck nod (radians).
	pub neck_alert: f32,
	/// Quiet weight-shift amplitude (radians).
	pub sway: f32,
	/// When true, `progress` is settle depth (0 = stand, 1 = sit).
	pub held: bool,
	/// When true, progress is one sit-down / hold / get-up; otherwise idle sit.
	pub one_shot: bool,
}

impl Default for QuadrupedSit {
	fn default() -> Self {
		Self::for_idle()
	}
}

impl QuadrupedSit {
	/// Quiet seated hold. Progress is unwrapped time for sway.
	pub fn for_idle() -> Self {
		Self {
			hind_stride: DEFAULT_HIND_STRIDE,
			hind_hinge: DEFAULT_HIND_HINGE,
			hip_splay: DEFAULT_HIP_SPLAY,
			front_plant: DEFAULT_FRONT_PLANT,
			front_hinge: DEFAULT_FRONT_HINGE,
			spine_upright: DEFAULT_SPINE_UPRIGHT,
			spine_gather: DEFAULT_SPINE_GATHER,
			neck_alert: DEFAULT_NECK_ALERT,
			sway: DEFAULT_SWAY,
			held: false,
			one_shot: false,
		}
	}

	/// Stance sit. `progress` is depth; no sway.
	pub fn held() -> Self {
		Self { held: true, one_shot: false, sway: 0.0, ..Self::for_idle() }
	}

	/// Closed sit-down, hold, and get-up on [`Self::PERIOD`].
	pub fn for_cycle() -> Self {
		Self { held: false, one_shot: true, ..Self::for_idle() }
	}

	/// One sit-down / hold / stand, in progress units (~17 s at idle speed 0.2).
	pub const PERIOD: f32 = 3.4;
	pub const REST_END: f32 = 0.12;
	pub const DOWN_END: f32 = 0.70;
	pub const HOLD_END: f32 = 2.70;
	pub const UP_END: f32 = 3.28;

	pub const SWAY_FREQ: f32 = 0.55;

	/// Mid-sit hold on the cycle. Useful for tests and a readable still frame.
	pub fn sit_peak() -> f32 {
		(Self::DOWN_END + Self::HOLD_END) * 0.5
	}

	/// `1` while seated, `0` at stand. Closed at the period wrap for a cycle.
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
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn held_depth_is_progress() -> anyhow::Result<()> {
		let sit = QuadrupedSit::held();
		assert!((sit.settle_weight(0.0)).abs() < 1e-6);
		assert!((sit.settle_weight(1.0) - 1.0).abs() < 1e-6);
		assert!((sit.settle_weight(2.0) - 1.0).abs() < 1e-6);
		Ok(())
	}

	#[test]
	fn idle_stays_seated() -> anyhow::Result<()> {
		let sit = QuadrupedSit::for_idle();
		assert!((sit.settle_weight(0.0) - 1.0).abs() < 1e-6);
		assert!((sit.settle_weight(1.7) - 1.0).abs() < 1e-6);
		Ok(())
	}

	#[test]
	fn cycle_is_quiet_outside_the_sit() -> anyhow::Result<()> {
		let sit = QuadrupedSit::for_cycle();
		assert!(sit.settle_weight(0.0) < 1e-5);
		assert!(sit.settle_weight(QuadrupedSit::REST_END * 0.5) < 1e-5);
		assert!(sit.settle_weight(QuadrupedSit::sit_peak()) > 0.9);
		assert!(sit.settle_weight(1.0) > 0.9);
		assert!(sit.settle_weight(QuadrupedSit::PERIOD) < 1e-5);
		Ok(())
	}

	#[test]
	fn sit_folds_harder_than_a_front_plant() -> anyhow::Result<()> {
		let sit = QuadrupedSit::default();
		assert!(sit.hind_hinge > sit.front_hinge);
		assert!(sit.hind_stride > sit.front_plant);
		assert!(sit.spine_upright > sit.spine_gather);
		assert!(sit.sway < sit.neck_alert);
		Ok(())
	}
}
