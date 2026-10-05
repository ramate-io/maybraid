//! Humanoid mapping for [`VictoryWave`](crate::animations::VictoryWave).

use character_rigs::authoring::{ArmAim, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::VictoryWave;
use crate::rigs::humanoid::apply::{apply_arm, apply_leg, apply_neck_twisted};
use crate::Animation;

impl Animation<HumanoidV0Rig> for VictoryWave {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let mut pose = HumanoidPose::default();
		let wave_side = self.side;
		let other = wave_side.opposite();

		apply_wave_arm(&mut pose, wave_side, progress, self);
		apply_arm(&mut pose, other, self.counter_shoulder(progress), 0.0, 0.0, 0.0, 0.0);

		let bounce = self.knee_bounce(progress);
		apply_leg(&mut pose, Side::Left, bounce, bounce * 0.55);
		apply_leg(&mut pose, Side::Right, bounce, bounce * 0.55);

		pose.spine.add_turn(-self.torso_turn(progress));
		apply_neck_twisted(
			&mut pose,
			self.torso_turn(progress) * 0.35,
			0.0,
			self.neck_nod(progress),
			0.0,
			0.0,
			0.0,
		);
		rig.write_pose(&pose);
	}
}

fn apply_wave_arm(pose: &mut HumanoidPose, side: Side, progress: f32, wave: &VictoryWave) {
	let raise = wave.raise_amount(progress);
	if raise < 1e-4 {
		return;
	}
	let arm = pose.arm_mut(side);
	arm.shoulder_forward += wave.shoulder_carry(progress);
	arm.elbow_flexion += wave.elbow_flex(progress);
	arm.aim = Some(ArmAim {
		along: wave.humerus_along(side, progress),
		roll: wave.humerus_roll(side, progress),
	});
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn victory_wave_raises_the_waving_forearm() {
		let wave = VictoryWave::default();
		let rest = HumanoidV0Rig::for_clip_test();
		let mut peak = HumanoidV0Rig::for_clip_test();
		wave.apply(&mut peak, 0.55);

		let rest_tip = rest.character_point("forearm.R");
		let peak_tip = peak.character_point("forearm.R");
		assert!(peak_tip.y > rest_tip.y + 0.15, "arm lifts overhead, {peak_tip:?} vs {rest_tip:?}");
	}

	#[test]
	fn victory_wave_sweeps_laterally_during_hold() {
		let wave = VictoryWave::default();
		let mut a = HumanoidV0Rig::for_clip_test();
		let mut b = HumanoidV0Rig::for_clip_test();
		wave.apply(&mut a, 0.42);
		wave.apply(&mut b, 0.58);

		let tip_a = a.character_point("forearm.R");
		let tip_b = b.character_point("forearm.R");
		assert!(
			(tip_a.x - tip_b.x).abs() > 0.06,
			"wave sweeps in character X, {tip_a:?} vs {tip_b:?}"
		);
	}

	#[test]
	fn victory_wave_left_arm_raises_overhead() {
		let wave = VictoryWave::new(Side::Left);
		let rest = HumanoidV0Rig::for_clip_test();
		let mut peak = HumanoidV0Rig::for_clip_test();
		wave.apply(&mut peak, 0.55);

		let rest_tip = rest.character_point("forearm.L");
		let peak_tip = peak.character_point("forearm.L");
		assert!(
			peak_tip.y > rest_tip.y + 0.15,
			"left wave lifts overhead, {peak_tip:?} vs {rest_tip:?}"
		);
	}

	#[test]
	fn victory_wave_roll_mirrors_by_side() {
		let wave = VictoryWave::default();
		assert!(
			(wave.humerus_roll(Side::Right, 0.5) + wave.humerus_roll(Side::Left, 0.5)).abs() < 1e-4
		);
	}

	#[test]
	fn victory_wave_unwritten_bones_match_rest() {
		let wave = VictoryWave::default();
		let rest = HumanoidV0Rig::for_clip_test();
		let mut posed = HumanoidV0Rig::for_clip_test();
		wave.apply(&mut posed, 0.0);
		for name in ["pelvis.L", "pelvis.R", "shin.L", "shin.R"] {
			assert!(
				rest.rotation(name).angle_between(posed.rotation(name)) < 1e-4,
				"{name} should stay at rest at progress 0"
			);
		}
	}

	#[test]
	fn victory_wave_recovers_to_rest() {
		let wave = VictoryWave::default();
		let rest = HumanoidV0Rig::for_clip_test();
		let mut end = HumanoidV0Rig::for_clip_test();
		wave.apply(&mut end, 1.0);
		let rest_tip = rest.character_point("forearm.R");
		let end_tip = end.character_point("forearm.R");
		assert!(
			(end_tip - rest_tip).length() < 0.12,
			"forearm returns near rest, {end_tip:?} vs {rest_tip:?}"
		);
	}
}
