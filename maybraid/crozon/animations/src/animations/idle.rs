//! Humanoid idle: a few degrees of arm sway, neck roll, and hip shift.
//!
//! Progress is owned by the controller; [`Idle`] is a pure sampler at a
//! normalized cycle phase in `[0, 1)`. Amplitude stays tiny so a freeze
//! mid-cycle is still a rest pose, not a T-pose or a flung arm.

/// Golden-ratio odd constant; same mix family as POI discovery.
const PHASE_GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;

/// A few degrees in radians. Large hip / spine travel fights terrain pitch.
const DEFAULT_ARM_SWAY: f32 = 0.06;
const DEFAULT_NECK_ROLL: f32 = 0.05;
const DEFAULT_HIP_SHIFT: f32 = 0.025;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Idle {
	/// Shoulder / humerus swing amplitude (radians).
	pub arm_sway: f32,
	/// Neck yaw / roll amplitude (radians).
	pub neck_roll: f32,
	/// Pelvis weight-shift amplitude (radians). Keep smaller than the arms.
	pub hip_shift: f32,
}

impl Default for Idle {
	fn default() -> Self {
		Self {
			arm_sway: DEFAULT_ARM_SWAY,
			neck_roll: DEFAULT_NECK_ROLL,
			hip_shift: DEFAULT_HIP_SHIFT,
		}
	}
}

impl Idle {
	/// Deterministic `[0, 1)` phase from entity bits so a crowd does not lock step.
	pub fn phase_from_entity_bits(bits: u64) -> f32 {
		let mixed = bits.wrapping_mul(PHASE_GOLDEN);
		((mixed >> 32) as u32 as f32) * (1.0 / (u32::MAX as f32))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_amplitudes_stay_a_few_degrees() {
		let idle = Idle::default();
		assert!(idle.arm_sway < 0.1);
		assert!(idle.neck_roll < 0.1);
		assert!(idle.hip_shift < idle.arm_sway);
	}

	#[test]
	fn entity_bits_desync_phase() {
		let a = Idle::phase_from_entity_bits(1);
		let b = Idle::phase_from_entity_bits(2);
		assert!(a >= 0.0 && a < 1.0);
		assert!(b >= 0.0 && b < 1.0);
		assert_ne!(a, b);
	}
}
