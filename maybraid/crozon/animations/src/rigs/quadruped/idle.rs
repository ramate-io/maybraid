use std::f32::consts::TAU;

use crozon_rigs::{quadruped::QuadrupedRig, Side};

use crate::animations::{Idle, QuadrupedIdle};
use crate::rigs::quadruped::apply::{
	apply_front_leg, apply_hind_leg, apply_neck_axes, apply_spine,
};
use crate::Animation;

impl<R: QuadrupedRig> Animation<R> for QuadrupedIdle {
	fn apply_for(&self, rig: &mut R, progress: f32) {
		let graze = QuadrupedIdle::graze_weight(progress);
		let look = QuadrupedIdle::look_weight(progress);
		let shake = QuadrupedIdle::shake_weight(progress);
		let sway = (TAU * (progress * QuadrupedIdle::SWAY_FREQ)).sin();
		let bob = (TAU * (progress * QuadrupedIdle::BOB_FREQ)).sin();
		let glance = Idle::look_wave(progress, QuadrupedIdle::GLANCE_FREQ, 0.21);
		let rattle = (TAU * (progress * QuadrupedIdle::SHAKE_FREQ)).sin();

		// Flex = side-to-side, twist = up/down, swing = roll about +Y (along the bone).
		let neck_flex = -self.graze_neck * graze
			+ self.look_neck * look
			+ self.look_neck * 0.55 * glance * (look + graze * 0.35)
			+ self.sway * 0.35 * sway;
		let neck_twist = -self.graze_pitch * graze
			+ self.look_pitch * look
			+ self.graze_pitch * 0.06 * bob * graze
			+ self.sway * 0.35 * sway;
		let neck_swing = self.shake * rattle * shake;
		apply_neck_axes(rig, neck_swing, neck_flex, neck_twist);

		let lumbar = self.lumbar * graze - self.lumbar * 0.35 * look + self.sway * sway;
		apply_spine(rig, lumbar * 0.35, lumbar);

		for side in [Side::Left, Side::Right] {
			let plant = graze * 0.08;
			apply_front_leg(rig, side, 0.0, 0.0, -plant, plant * 0.7);
			apply_hind_leg(
				rig,
				side,
				0.0,
				self.sway * sway * side.sign(),
				plant * 0.5,
				plant * 0.4,
			);
		}
	}
}

#[cfg(test)]
mod tests {
	use crozon_rigs::rigs::quadruped_v0::QuadrupedV0Rig;
	use crozon_rigs::Name;

	use super::*;
	use crate::Animation;

	fn neck_swing(rig: &QuadrupedV0Rig) -> f32 {
		rig.pose().get(&Name::from("neck")).expect("neck").swing
	}

	fn neck_flex(rig: &QuadrupedV0Rig) -> f32 {
		rig.pose().get(&Name::from("neck")).expect("neck").flex
	}

	fn neck_twist(rig: &QuadrupedV0Rig) -> f32 {
		rig.pose().get(&Name::from("neck")).expect("neck").twist
	}

	fn lumbar(rig: &QuadrupedV0Rig) -> f32 {
		rig.pose().get(&Name::from("lumbar")).expect("lumbar").flex
	}

	#[test]
	fn graze_bows_the_neck_and_gathers_the_spine() {
		let mut rig = QuadrupedV0Rig::imported();
		QuadrupedIdle::default().apply(&mut rig, QuadrupedIdle::graze_peak());

		assert!(neck_flex(&rig) < -0.5, "graze should keep the side-to-side");
		assert!(neck_twist(&rig) < -0.3, "graze should also nod down");
		assert!(lumbar(&rig) > 0.1);
	}

	#[test]
	fn look_raises_the_neck_above_rest() {
		let mut graze = QuadrupedV0Rig::imported();
		let mut look = QuadrupedV0Rig::imported();
		QuadrupedIdle::default().apply(&mut graze, QuadrupedIdle::graze_peak());
		QuadrupedIdle::default().apply(&mut look, QuadrupedIdle::look_peak());

		assert!(neck_twist(&look) > 0.12);
		assert!(neck_twist(&look) > neck_twist(&graze) + 0.4);
		assert!(lumbar(&look) < lumbar(&graze));
	}

	#[test]
	fn shake_rotates_the_neck_after_the_look() {
		let mut shake = QuadrupedV0Rig::imported();
		QuadrupedIdle::default().apply(&mut shake, QuadrupedIdle::shake_peak());

		assert!(neck_swing(&shake).abs() > 0.03);
		assert!(QuadrupedIdle::shake_weight(QuadrupedIdle::shake_peak()) > 0.9);
	}

	#[test]
	fn rest_and_period_wrap_stay_quiet() {
		let mut rest = QuadrupedV0Rig::imported();
		let mut wrap = QuadrupedV0Rig::imported();
		QuadrupedIdle::default().apply(&mut rest, 0.0);
		QuadrupedIdle::default().apply(&mut wrap, QuadrupedIdle::PERIOD);

		assert!(neck_swing(&rest).abs() < 0.02);
		assert!(neck_flex(&rest).abs() < 0.02);
		assert!(neck_twist(&rest).abs() < 0.02);
		assert!(neck_swing(&wrap).abs() < 0.03);
		assert!(neck_flex(&wrap).abs() < 0.03);
		assert!(neck_twist(&wrap).abs() < 0.03);
	}

	#[test]
	fn envelopes_do_not_snap_across_unit_progress() {
		let idle = QuadrupedIdle::default();
		let mut before = QuadrupedV0Rig::imported();
		let mut after = QuadrupedV0Rig::imported();
		idle.apply(&mut before, 0.999);
		idle.apply(&mut after, 1.001);

		assert!((neck_swing(&before) - neck_swing(&after)).abs() < 0.05);
		assert!((neck_flex(&before) - neck_flex(&after)).abs() < 0.05);
		assert!((neck_twist(&before) - neck_twist(&after)).abs() < 0.05);
	}

	#[test]
	fn phase_offset_changes_pose() {
		let idle = QuadrupedIdle::default();
		let mut a = QuadrupedV0Rig::imported();
		let mut b = QuadrupedV0Rig::imported();
		idle.apply(&mut a, QuadrupedIdle::graze_peak());
		idle.apply(&mut b, QuadrupedIdle::graze_peak() + Idle::phase_from_entity_bits(7));

		assert_ne!(neck_twist(&a), neck_twist(&b));
	}
}
