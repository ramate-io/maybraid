//! One-shot spot scan: alert crouch, sweep the head and torso left then right, settle.
//!
//! Progress is clamped to `[0.0, 1.0]`. Arms stay at rest; the read is neck- and
//! spine-led. Sampling is repeatable from effective rest, clip parameters, and
//! progress only.

use crate::animations::smoothstep;
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

/// Default playback: full scan in ~1.15 s at speed `1.0`.
pub const DEFAULT_SPOT_SCAN_SPEED: f32 = 0.87;

/// Short torso-led search sweep without raising the arms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpotScan;

impl SpotScan {
	/// Character yaw in radians. Negative turns left (+X), positive turns right.
	pub fn scan_yaw(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t <= PREPARE_END {
			return 0.0;
		}
		if t <= LEFT_PEAK {
			let u = (t - PREPARE_END) / (LEFT_PEAK - PREPARE_END);
			return -SCAN_TURN * smoothstep(u);
		}
		if t <= CROSS_LEFT {
			let u = (t - LEFT_PEAK) / (CROSS_LEFT - LEFT_PEAK);
			return -SCAN_TURN * (1.0 - smoothstep(u));
		}
		if t <= RIGHT_PEAK {
			let u = (t - CROSS_LEFT) / (RIGHT_PEAK - CROSS_LEFT);
			return SCAN_TURN * smoothstep(u);
		}
		if t <= CROSS_RIGHT {
			let u = (t - RIGHT_PEAK) / (CROSS_RIGHT - RIGHT_PEAK);
			return SCAN_TURN * (1.0 - smoothstep(u));
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
			return 1.0;
		}
		1.0 - smoothstep((t - CROSS_RIGHT) / (1.0 - CROSS_RIGHT))
	}

	pub fn knee_flex(&self, progress: f32) -> f32 {
		let alert = smoothstep((Progress(progress).clamp() - PREPARE_END) / (CROSS_RIGHT - PREPARE_END));
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

	/// Full left sweep. Useful for tests and a readable still frame.
	pub fn left_peak() -> f32 {
		LEFT_PEAK
	}

	/// Full right sweep.
	pub fn right_peak() -> f32 {
		RIGHT_PEAK
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn spot_scan_is_quiet_at_start_and_end() -> anyhow::Result<()> {
		let scan = SpotScan;
		assert!(scan.scan_yaw(0.0).abs() < 1e-4);
		assert!(scan.scan_yaw(1.0).abs() < 1e-4);
		assert!(scan.knee_flex(0.0).abs() < 1e-4);
		assert!(scan.knee_flex(1.0).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn spot_scan_sweeps_left_then_right() -> anyhow::Result<()> {
		let scan = SpotScan;
		let left = scan.scan_yaw(SpotScan::left_peak());
		let right = scan.scan_yaw(SpotScan::right_peak());
		assert!(left < -0.35, "left sweep, got {left}");
		assert!(right > 0.35, "right sweep, got {right}");
		assert!((left + right).abs() < 0.08, "left and right mirror, {left} {right}");
		Ok(())
	}

	#[test]
	fn spot_scan_crosses_center_between_peaks() -> anyhow::Result<()> {
		let scan = SpotScan;
		assert!(scan.scan_yaw(CROSS_LEFT).abs() < 0.05, "returns through center");
		Ok(())
	}

	#[test]
	fn spot_scan_is_repeatable() -> anyhow::Result<()> {
		let scan = SpotScan;
		let progress = 0.47;
		assert!((scan.scan_yaw(progress) - scan.scan_yaw(progress)).abs() < 1e-6);
		assert!((scan.neck_turn(progress) - scan.neck_turn(progress)).abs() < 1e-6);
		Ok(())
	}

	#[test]
	fn spot_scan_clamps_past_one() -> anyhow::Result<()> {
		let scan = SpotScan;
		assert!(scan.scan_yaw(1.5).abs() < 1e-4);
		Ok(())
	}
}
