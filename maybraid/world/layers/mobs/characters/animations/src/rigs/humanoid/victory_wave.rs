//! Humanoid mapping for [`VictoryWave`](crate::animations::VictoryWave).
//!
//! The waving humerus aims out and up on its own side. Elbow flexion folds the
//! forearm back toward the midline. Roll is the clip's measured wave roll, not
//! the jab punch plane.

use character_rigs::authoring::{ArmAim, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::VictoryWave;
use crate::Animation;

impl Animation<HumanoidV0Rig> for VictoryWave {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let mut pose = HumanoidPose::default();
		let wave_side = self.side;
		let other = wave_side.opposite();

		apply_wave_arm(&mut pose, wave_side, progress, self);
		pose.arm_mut(other).shoulder_forward += self.counter_shoulder(progress);

		let bounce = self.knee_bounce(progress);
		pose.apply_leg(Side::Left, bounce, bounce * 0.55);
		pose.apply_leg(Side::Right, bounce, bounce * 0.55);

		pose.spine.add_turn(-self.torso_turn(progress));
		let nod = self.neck_nod(progress);
		let turn = self.torso_turn(progress) * 0.35;
		pose.apply_neck_twisted(turn, 0.0, nod, turn, 0.0, nod);
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
	use bevy::prelude::Vec3;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	use super::*;
	use crate::Animation;

	const FOREARM_SEGMENT: f32 = 0.48;
	const RAISE_END: f32 = 0.28;
	const WAVE_END: f32 = 0.82;
	const WAVE_HOLD: f32 = (RAISE_END + WAVE_END) * 0.5;
	/// `sin(2π · 2.5 · u)` extrema at `u = 0.3` and `u = 0.7` (full envelope).
	const WAVE_EXTREME_A: f32 = RAISE_END + 0.3 * (WAVE_END - RAISE_END);
	const WAVE_EXTREME_B: f32 = RAISE_END + 0.7 * (WAVE_END - RAISE_END);

	#[derive(Debug)]
	struct ArmJoints {
		shoulder: Vec3,
		elbow: Vec3,
		tip: Vec3,
		humerus: Vec3,
		forearm: Vec3,
		elbow_flex: f32,
	}

	fn bone(side: Side, name: &str) -> String {
		format!("{name}.{}", side.suffix())
	}

	fn arm_joints(rig: &HumanoidV0Rig, side: Side) -> ArmJoints {
		let shoulder = rig.character_point(&bone(side, "humerus"));
		let elbow = rig.character_point(&bone(side, "forearm"));
		let forearm_dir = rig.character_length(&bone(side, "forearm"));
		let tip = elbow + forearm_dir * FOREARM_SEGMENT;
		ArmJoints {
			shoulder,
			elbow,
			tip,
			humerus: elbow - shoulder,
			forearm: tip - elbow,
			elbow_flex: rig.posed_angle(&bone(side, "forearm")),
		}
	}

	fn sample(side: Side, progress: f32) -> ArmJoints {
		let wave = VictoryWave::new(side);
		let mut rig = HumanoidV0Rig::for_clip_test();
		wave.apply(&mut rig, progress);
		arm_joints(&rig, side)
	}

	fn assert_raised_wave_pose(side: Side, progress: f32) {
		let j = sample(side, progress);
		let own = side.sign();
		assert!(
			j.shoulder.x * own > 0.2,
			"{side:?} @{progress} shoulder should stay on its own side, sh={:?}",
			j.shoulder
		);
		assert!(
			j.elbow.x * own > 0.2,
			"{side:?} @{progress} elbow should stay outboard of the midline, el={:?}",
			j.elbow
		);
		assert!(
			j.elbow.y > j.shoulder.y + 0.15,
			"{side:?} @{progress} elbow should lift above the shoulder, el={:?} sh={:?}",
			j.elbow,
			j.shoulder
		);
		let hum_len = j.humerus.length();
		assert!(hum_len > 0.3, "{side:?} @{progress} missing humerus length {j:?}");
		let lateral = j.humerus.x * own;
		assert!(
			lateral > 0.18,
			"{side:?} @{progress} humerus must leave the shoulder outboard, hum={:?}",
			j.humerus
		);
		assert!(
			j.humerus.y / hum_len < 0.88,
			"{side:?} @{progress} humerus is too vertical, hum={:?}",
			j.humerus
		);
		assert!(
			j.humerus.y / hum_len > 0.45,
			"{side:?} @{progress} humerus should still lift, hum={:?}",
			j.humerus
		);
		let toward_mid = -j.forearm.x * own;
		assert!(
			toward_mid > 0.08,
			"{side:?} @{progress} forearm must point medially, fa={:?}",
			j.forearm
		);
		assert!(
			j.forearm.y > 0.18,
			"{side:?} @{progress} forearm must point upward, fa={:?}",
			j.forearm
		);
		assert!(
			j.elbow_flex > 0.9,
			"{side:?} @{progress} elbow should stay bent, flex={}",
			j.elbow_flex
		);
		assert!(
			j.tip.x * own > 0.05,
			"{side:?} @{progress} tip must stay on its own side of the midline, tip={:?}",
			j.tip
		);
	}

	#[test]
	fn victory_wave_raised_arm_is_outboard_with_medial_forearm() {
		for side in [Side::Right, Side::Left] {
			assert_raised_wave_pose(side, RAISE_END);
			assert_raised_wave_pose(side, WAVE_HOLD);
			assert_raised_wave_pose(side, WAVE_EXTREME_A);
			assert_raised_wave_pose(side, WAVE_EXTREME_B);
		}
	}

	#[test]
	fn victory_wave_raises_the_waving_forearm() {
		let rest = sample(Side::Right, 0.0);
		let peak = sample(Side::Right, WAVE_HOLD);
		assert!(
			peak.tip.y > rest.tip.y + 0.12,
			"right hand lifts for the wave, {:?} vs {:?}",
			peak.tip,
			rest.tip
		);
		assert_raised_wave_pose(Side::Right, WAVE_HOLD);
	}

	#[test]
	fn victory_wave_sweeps_laterally_during_hold() {
		let a = sample(Side::Right, WAVE_EXTREME_A);
		let b = sample(Side::Right, WAVE_HOLD);
		assert!(
			(a.tip.x - b.tip.x).abs() > 0.06,
			"wave sweeps the forearm in character X, {:?} vs {:?}",
			a.tip,
			b.tip
		);
	}

	#[test]
	fn victory_wave_left_arm_mirrors_the_raised_pose() {
		let rest = sample(Side::Left, 0.0);
		let peak = sample(Side::Left, WAVE_HOLD);
		assert!(
			peak.tip.y > rest.tip.y + 0.12,
			"left hand lifts for the wave, {:?} vs {:?}",
			peak.tip,
			rest.tip
		);
		assert_raised_wave_pose(Side::Left, WAVE_HOLD);
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
		for side in [Side::Right, Side::Left] {
			let rest = sample(side, 0.0);
			let end = sample(side, 1.0);
			assert!(
				(end.tip - rest.tip).length() < 0.12,
				"{side:?} forearm returns near rest, {:?} vs {:?}",
				end.tip,
				rest.tip
			);
		}
	}

	#[test]
	fn victory_wave_oscillation_mirrors_left_and_right() {
		let right_a = sample(Side::Right, WAVE_EXTREME_A);
		let right_b = sample(Side::Right, WAVE_HOLD);
		let left_a = sample(Side::Left, WAVE_EXTREME_A);
		let left_b = sample(Side::Left, WAVE_HOLD);
		let right_dx = right_b.tip.x - right_a.tip.x;
		let left_dx = left_b.tip.x - left_a.tip.x;
		assert!(
			right_dx * left_dx < 0.0,
			"left/right tip sweeps must oppose, right {right_dx} left {left_dx}"
		);
		assert!(
			(right_dx.abs() - left_dx.abs()).abs() < 0.08,
			"mirrored sweep magnitudes, right {right_dx} left {left_dx}"
		);
	}

	#[test]
	fn victory_wave_recovery_blends_without_a_cutoff() {
		const FRAME: f32 = 1.0 / 60.0;
		let wave = VictoryWave::default();
		for side in [Side::Right, Side::Left] {
			let rest = sample(side, 0.0);
			let mut prev = sample(side, WAVE_END);
			let mut prev_raise = wave.raise_amount(WAVE_END);
			let mut max_step = 0.0_f32;
			let mut t = WAVE_END + FRAME;
			while t < 1.0 - 1e-4 {
				let now = sample(side, t);
				let raise = wave.raise_amount(t);
				assert!(
					raise <= prev_raise + 1e-4,
					"{side:?} raise must fall through recovery, {prev_raise} -> {raise} at {t}"
				);
				let step = (now.tip - prev.tip).length();
				max_step = max_step.max(step);
				assert!(
					step < 0.22,
					"{side:?} recovery jumped {step} at t={t:.3} {:?} -> {:?}",
					prev.tip,
					now.tip
				);
				prev = now;
				prev_raise = raise;
				t += FRAME;
			}
			let end = sample(side, 1.0);
			assert!(
				(end.tip - rest.tip).length() < 0.12,
				"{side:?} end must land on rest, {:?} vs {:?}",
				end.tip,
				rest.tip
			);
			assert!(
				(end.tip - prev.tip).length() < 0.12,
				"{side:?} last frame must not snap to rest, {:?} -> {:?}",
				prev.tip,
				end.tip
			);
			assert!(max_step > 0.02, "{side:?} recovery should keep moving, max={max_step}");
		}
	}
}
