//! Dance stomp beat: lift a knee, drive the foot down, and whip the opposite arm.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::Vec3;
use character_rigs::Side;

use crate::animations::smoothstep;
use crate::Progress;

pub const LIFT_CENTER: f32 = 0.26;
const LIFT_HALF_WIDTH: f32 = 0.11;
pub const IMPACT_CENTER: f32 = 0.40;
const IMPACT_HALF_WIDTH: f32 = 0.07;
const RECOVER_START: f32 = 0.58;
const LIFT_HIP: f32 = -0.90;
const LIFT_KNEE: f32 = 0.55;
const STRIKE_HIP: f32 = 0.18;
const STRIKE_KNEE: f32 = 0.12;
const SUPPORT_KNEE: f32 = 0.22;
const TORSO_DIP: f32 = 0.12;
const PUMP_ELBOW: f32 = 0.35;
const PUMP_ROLL: f32 = FRAC_PI_2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
	pub fn pump_side(&self) -> Side {
		self.side.opposite()
	}

	fn pulse(t: f32, center: f32, half_width: f32) -> f32 {
		if half_width <= 0.0 {
			return 0.0;
		}
		let x = ((t - center) / half_width).abs();
		if x >= 1.0 {
			0.0
		} else {
			(1.0 - x).powi(2)
		}
	}

	pub fn lift_amount(&self, progress: f32) -> f32 {
		Self::pulse(Progress(progress).clamp(), LIFT_CENTER, LIFT_HALF_WIDTH)
	}

	pub fn impact_amount(&self, progress: f32) -> f32 {
		Self::pulse(Progress(progress).clamp(), IMPACT_CENTER, IMPACT_HALF_WIDTH)
	}

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
		(LIFT_HIP * lift - STRIKE_HIP * impact) * (1.0 - self.recover_blend(progress))
	}

	pub fn stomping_knee_flexion(&self, progress: f32) -> f32 {
		let lift = self.lift_amount(progress);
		let impact = self.impact_amount(progress);
		let strike = STRIKE_KNEE * impact;
		(lift * LIFT_KNEE * (1.0 - impact) + strike * impact) * (1.0 - self.recover_blend(progress))
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

	pub fn pump_humerus_along(&self, _progress: f32) -> Vec3 {
		Vec3::new(0.0, -0.995, 0.10).normalize()
	}

	pub fn pump_roll(&self) -> f32 {
		PUMP_ROLL
	}

	pub fn motion_amount(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t <= 0.0 || t >= 1.0 {
			0.0
		} else {
			self.lift_amount(t).max(self.impact_amount(t))
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn stomp_clamps_and_separates_lift_from_impact() -> anyhow::Result<()> {
		let stomp = StompBeat::default();
		assert_eq!(stomp.lift_amount(-1.0), 0.0);
		assert_eq!(stomp.impact_amount(2.0), 0.0);
		assert!(stomp.lift_amount(LIFT_CENTER) > 0.99);
		assert!(stomp.impact_amount(IMPACT_CENTER) > 0.99);
		assert!(stomp.impact_amount(LIFT_CENTER) < 0.05);
		assert!(stomp.lift_amount(IMPACT_CENTER) < 0.2);
		assert!(stomp.pump_humerus_along(IMPACT_CENTER).y < 0.0);
		Ok(())
	}
}
