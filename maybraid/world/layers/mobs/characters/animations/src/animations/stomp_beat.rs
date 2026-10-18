//! Dance stomp beat: lift one knee, slam the foot down with a torso dip and opposite-arm whip.
//!
//! Character space is +X right, +Y up, +Z fight-forward. Progress is a one-shot in `[0, 1]`.
//! Distinct from locomotion, jabs, and upper-body-only celebration clips.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::Vec3;
use character_rigs::Side;

use crate::animations::smoothstep;
use crate::Progress;

pub(crate) const LIFT_CENTER: f32 = 0.26;
const LIFT_HALF_WIDTH: f32 = 0.11;
pub(crate) const IMPACT_CENTER: f32 = 0.40;
const IMPACT_HALF_WIDTH: f32 = 0.07;
const RECOVER_START: f32 = 0.58;

/// Hip flexion while the stomping thigh lifts.
const LIFT_HIP: f32 = 0.62;
/// Knee flex at peak lift (shin folded under the thigh).
const LIFT_KNEE: f32 = 1.15;
/// Hip extension impulse on impact (leg drives down).
const STRIKE_HIP: f32 = 0.18;
/// Knee stays nearly straight at the slam.
const STRIKE_KNEE: f32 = 0.12;
/// Support knee softens on impact.
const SUPPORT_KNEE: f32 = 0.22;
/// Sagittal torso dip on impact.
const TORSO_DIP: f32 = 0.12;
/// Opposite-arm elbow at the whip.
const PUMP_ELBOW: f32 = 0.35;
const PUMP_ROLL: f32 = FRAC_PI_2;

/// One-shot stomp knobs. The rig resolver maps these onto V0 channels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StompBeat {
	pub side: Side,
}

impl Default for StompBeat {
	fn default() -> Self {
		Self { side: Side::Right }
	}
}

impl StompBeat {
	pub fn new(side: Side) -> Self {
		Self { side }
	}

	pub fn with_side(mut self, side: Side) -> Self {
		self.side = side;
		self
	}

	pub fn stomping_side(&self) -> Side {
		self.side
	}

	/// Arm that whips down on impact (opposite the stomping leg).
	pub fn pump_side(&self) -> Side {
		self.side.opposite()
	}

	fn parabolic_pulse(t: f32, center: f32, half_width: f32) -> f32 {
		if half_width <= 0.0 {
			return 0.0;
		}
		let x = ((t - center) / half_width).abs();
		if x >= 1.0 {
			0.0
		} else {
			let w = 1.0 - x;
			w * w
		}
	}

	pub fn lift_amount(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		Self::parabolic_pulse(t, LIFT_CENTER, LIFT_HALF_WIDTH)
	}

	pub fn impact_amount(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		Self::parabolic_pulse(t, IMPACT_CENTER, IMPACT_HALF_WIDTH)
	}

	/// Envelope for returning to rest after the hit.
	pub fn recover_blend(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t < RECOVER_START {
			0.0
		} else {
			smoothstep((t - RECOVER_START) / (1.0 - RECOVER_START))
		}
	}

	pub fn stomping_hip_flexion(&self, progress: f32) -> f32 {
		let lift = self.lift_amount(progress);
		let impact = self.impact_amount(progress);
		let recover = self.recover_blend(progress);
		let cocked = LIFT_HIP * lift;
		let strike = STRIKE_HIP * impact;
		(cocked - strike) * (1.0 - recover)
	}

	pub fn stomping_knee_flexion(&self, progress: f32) -> f32 {
		let lift = self.lift_amount(progress);
		let impact = self.impact_amount(progress);
		let recover = self.recover_blend(progress);
		let cocked = LIFT_KNEE * lift;
		let strike = STRIKE_KNEE * impact;
		let blended = cocked * (1.0 - impact) + strike * impact;
		blended * (1.0 - recover)
	}

	pub fn support_knee_flexion(&self, progress: f32) -> f32 {
		SUPPORT_KNEE * self.impact_amount(progress) * (1.0 - self.recover_blend(progress))
	}

	pub fn torso_dip(&self, progress: f32) -> f32 {
		TORSO_DIP * self.impact_amount(progress) * (1.0 - self.recover_blend(progress))
	}

	pub fn pump_elbow(&self, progress: f32) -> f32 {
		PUMP_ELBOW * self.impact_amount(progress) * (1.0 - self.recover_blend(progress))
	}

	/// Down-and-forward whip on the opposite arm at impact.
	pub fn pump_humerus_along(&self, progress: f32) -> Vec3 {
		let amount = self.impact_amount(progress) * (1.0 - self.recover_blend(progress));
		if amount < 1e-5 {
			return Vec3::ZERO;
		}
		Vec3::new(0.0, -0.88 * amount, 0.42 * amount).normalize()
	}

	pub fn pump_roll(&self) -> f32 {
		PUMP_ROLL
	}

	/// Whole-clip activity for early-out in the rig adapter.
	pub fn motion_amount(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t <= 0.0 || t >= 1.0 {
			return 0.0;
		}
		self.lift_amount(t).max(self.impact_amount(t))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn stomp_beat_starts_and_ends_at_rest() -> anyhow::Result<()> {
		let stomp = StompBeat::default();
		assert!(stomp.motion_amount(0.0) < 1e-4);
		assert!(stomp.motion_amount(1.0) < 1e-4);
		assert!(stomp.stomping_knee_flexion(0.0) < 1e-4);
		assert!(stomp.stomping_knee_flexion(1.0) < 0.05);
		Ok(())
	}

	#[test]
	fn stomp_beat_lift_and_impact_are_separated() -> anyhow::Result<()> {
		let stomp = StompBeat::default();
		let lift = stomp.lift_amount(LIFT_CENTER);
		let impact_at_lift = stomp.impact_amount(LIFT_CENTER);
		let impact = stomp.impact_amount(IMPACT_CENTER);
		let lift_at_impact = stomp.lift_amount(IMPACT_CENTER);
		assert!(lift > 0.85, "lift peak {lift}");
		assert!(impact > 0.85, "impact peak {impact}");
		assert!(impact_at_lift < lift * 0.35, "lift before impact {impact_at_lift}");
		assert!(lift_at_impact < impact * 0.35, "impact after lift {lift_at_impact}");
		Ok(())
	}

	#[test]
	fn stomp_beat_knee_flexes_then_extends() -> anyhow::Result<()> {
		let stomp = StompBeat::default();
		let cocked = stomp.stomping_knee_flexion(LIFT_CENTER);
		let strike = stomp.stomping_knee_flexion(IMPACT_CENTER);
		assert!(cocked > LIFT_KNEE * 0.7, "cocked knee {cocked}");
		assert!(strike < cocked * 0.35, "strike extends knee {strike} vs {cocked}");
		Ok(())
	}

	#[test]
	fn stomp_beat_clamps_beyond_one_shot_end() -> anyhow::Result<()> {
		let stomp = StompBeat::default();
		assert!((stomp.lift_amount(1.5) - stomp.lift_amount(1.0)).abs() < 1e-5);
		Ok(())
	}
}
