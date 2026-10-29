//! Alert crouch with a neck-led left/right scan and a smooth settle.

use character_rigs::humanoid::LegSegmentLengths;

use crate::animations::{smoothstep, vertical_drop};
use crate::Progress;

const PREPARE_END: f32 = 0.10;
const LEFT_PEAK: f32 = 0.28;
const CROSS_LEFT: f32 = 0.42;
const RIGHT_PEAK: f32 = 0.60;
const CROSS_RIGHT: f32 = 0.78;
const SCAN_TURN: f32 = 0.52;
const TORSO_FRACTION: f32 = 0.42;
const NECK_LEAD: f32 = 1.35;
const CROUCH_KNEE: f32 = 0.14;
const FORWARD_LEAN: f32 = 0.07;
const HEAD_DIP: f32 = 0.06;
const PELVIS_SHIFT: f32 = 0.022;

pub const DEFAULT_SPOT_SCAN_SPEED: f32 = 0.87;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpotScan;

impl SpotScan {
	pub fn scan_yaw(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t <= PREPARE_END {
			return 0.0;
		}
		if t <= LEFT_PEAK {
			return SCAN_TURN * smoothstep((t - PREPARE_END) / (LEFT_PEAK - PREPARE_END));
		}
		if t <= CROSS_LEFT {
			return SCAN_TURN * (1.0 - smoothstep((t - LEFT_PEAK) / (CROSS_LEFT - LEFT_PEAK)));
		}
		if t <= RIGHT_PEAK {
			return -SCAN_TURN * smoothstep((t - CROSS_LEFT) / (RIGHT_PEAK - CROSS_LEFT));
		}
		if t <= CROSS_RIGHT {
			return -SCAN_TURN * (1.0 - smoothstep((t - RIGHT_PEAK) / (CROSS_RIGHT - RIGHT_PEAK)));
		}
		0.0
	}

	pub fn torso_turn(&self, progress: f32) -> f32 {
		self.scan_yaw(progress) * TORSO_FRACTION
	}

	pub fn neck_turn(&self, progress: f32) -> f32 {
		self.scan_yaw(progress) * NECK_LEAD
	}

	pub fn settle_weight(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t <= CROSS_RIGHT {
			1.0
		} else {
			1.0 - smoothstep((t - CROSS_RIGHT) / (1.0 - CROSS_RIGHT))
		}
	}

	pub fn knee_flex(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		let alert = smoothstep((t - PREPARE_END) / (CROSS_RIGHT - PREPARE_END));
		let turn = (self.scan_yaw(progress).abs() / SCAN_TURN).clamp(0.0, 1.0);
		CROUCH_KNEE * alert * (0.35 + 0.65 * turn) * self.settle_weight(progress)
	}

	pub fn forward_lean(&self, progress: f32) -> f32 {
		FORWARD_LEAN
			* smoothstep((Progress(progress).clamp() - PREPARE_END) / (RIGHT_PEAK - PREPARE_END))
			* self.settle_weight(progress)
	}

	pub fn head_dip(&self, progress: f32) -> f32 {
		HEAD_DIP * (self.scan_yaw(progress).abs() / SCAN_TURN).clamp(0.0, 1.0)
	}

	pub fn pelvis_shift(&self, progress: f32) -> f32 {
		self.scan_yaw(progress) * PELVIS_SHIFT
	}

	pub fn femur_swing(&self, progress: f32) -> f32 {
		self.knee_flex(progress) * 0.35
	}

	pub fn shin_flex(&self, progress: f32) -> f32 {
		self.knee_flex(progress)
	}

	pub fn vertical_drop(&self, progress: f32, lengths: LegSegmentLengths) -> f32 {
		vertical_drop(self.femur_swing(progress), self.shin_flex(progress), lengths)
	}

	pub fn left_peak() -> f32 {
		LEFT_PEAK
	}
	pub fn right_peak() -> f32 {
		RIGHT_PEAK
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn spot_scan_clamps_and_swings_left_then_right() -> anyhow::Result<()> {
		let scan = SpotScan;
		assert_eq!(scan.scan_yaw(-1.0), scan.scan_yaw(0.0));
		assert_eq!(scan.scan_yaw(2.0), scan.scan_yaw(1.0));
		let left = scan.scan_yaw(SpotScan::left_peak());
		let right = scan.scan_yaw(SpotScan::right_peak());
		assert!(left > 0.35 && right < -0.35);
		assert!((left + right).abs() < 0.08);
		assert!(scan.scan_yaw(CROSS_LEFT).abs() < 1e-5);
		assert!(scan.scan_yaw(0.0).abs() < 1e-5 && scan.scan_yaw(1.0).abs() < 1e-5);
		Ok(())
	}
}
