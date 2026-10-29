//! Humanoid mapping for [`VictoryWave`](crate::animations::VictoryWave).

use character_rigs::authoring::{ArmAim, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::{Idle, VictoryWave};
use crate::Animation;

impl Animation<HumanoidV0Rig> for VictoryWave {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let idle = Idle::default().sample_pose(0.0);
		let mut target = idle;

		apply_wave_arm(&mut target, self.side, progress, self);

		let bounce = self.knee_bounce(progress);
		target.apply_leg(Side::Left, bounce, bounce * 0.55);
		target.apply_leg(Side::Right, bounce, bounce * 0.55);

		target.spine.add_turn(-self.torso_turn(progress));
		let nod = self.neck_nod(progress);
		let turn = self.torso_turn(progress) * 0.35;
		target.apply_neck_twisted(turn, 0.0, nod, turn, 0.0, nod);

		let weight = self.raise_amount(progress);
		if weight <= 0.0 {
			rig.write_pose(&idle);
			return;
		}
		let depth = rig.scratch.depth;
		rig.write_pose(&idle);
		rig.scratch.capture_from(depth, &rig.pose);
		rig.write_pose(&target);
		rig.scratch.capture_to(depth, &rig.pose);
		rig.scratch.blend_saved(depth, weight, &mut rig.pose);
	}
}

fn apply_wave_arm(pose: &mut HumanoidPose, side: Side, progress: f32, wave: &VictoryWave) {
	let arm = pose.arm_mut(side);
	arm.shoulder_forward += wave.shoulder_carry(progress);
	arm.elbow_flexion = wave.elbow_flex(progress);
	arm.aim = Some(ArmAim {
		along: wave.humerus_along(side, progress),
		roll: wave.humerus_roll(side, progress),
	});
}

#[cfg(test)]
mod tests {
	use bevy::prelude::{Transform, Vec3};
	use character_rigs::authoring::ArmatureOffset;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
	use character_rigs::Side;

	use super::*;
	use crate::animations::{Idle, Transition, TransitionCurve};
	use crate::Animation;

	#[derive(Debug, Clone, Copy)]
	struct ArmJoints {
		shoulder: Vec3,
		elbow: Vec3,
		tip: Vec3,
		humerus: Vec3,
		forearm: Vec3,
		elbow_flex: f32,
	}

	const DISTAL_FOREARM_LENGTH: f32 = 0.5;

	fn bone(prefix: &str, side: Side) -> String {
		format!("{prefix}.{}", side.suffix())
	}

	fn segment_tip(rig: &HumanoidV0Rig, name: &str) -> anyhow::Result<Vec3> {
		let Some(_id) = rig.binding.definition.id(name) else {
			anyhow::bail!("missing bind bone {name}");
		};
		Ok(rig.character_point(name) + rig.character_length(name) * DISTAL_FOREARM_LENGTH)
	}

	fn max_pose_rotation_delta(left: &HumanoidV0Rig, right: &HumanoidV0Rig) -> f32 {
		left.animation_bone_names()
			.map(|name| left.rotation(name).angle_between(right.rotation(name)))
			.fold(0.0_f32, f32::max)
	}

	fn arm_joints(rig: &HumanoidV0Rig, side: Side) -> anyhow::Result<ArmJoints> {
		let humerus = bone("humerus", side);
		let forearm = bone("forearm", side);
		let shoulder = rig.character_point(&humerus);
		let elbow = rig.character_point(&forearm);
		let tip = segment_tip(rig, &forearm)?;
		Ok(ArmJoints {
			shoulder,
			elbow,
			tip,
			humerus: elbow - shoulder,
			forearm: tip - elbow,
			elbow_flex: rig.posed_angle(&forearm),
		})
	}

	fn sample(side: Side, progress: f32) -> anyhow::Result<(HumanoidV0Rig, ArmJoints)> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		VictoryWave::new(side).apply_for(&mut rig, progress);
		let joints = arm_joints(&rig, side)?;
		Ok((rig, joints))
	}

	fn idle_sample(side: Side) -> anyhow::Result<(HumanoidV0Rig, ArmJoints)> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Idle::default().apply_for(&mut rig, 0.0);
		let joints = arm_joints(&rig, side)?;
		Ok((rig, joints))
	}

	fn assert_opposite_arm_relaxed(wave_side: Side, progress: f32) -> anyhow::Result<()> {
		let opposite = wave_side.opposite();
		let (rig, _) = sample(wave_side, progress)?;
		let joints = arm_joints(&rig, opposite)?;
		let (_, idle) = idle_sample(opposite)?;
		assert!(
			joints.elbow.y < joints.shoulder.y - 0.08,
			"{opposite:?} elbow remains below shoulder: {joints:?}"
		);
		assert!(
			joints.tip.y < joints.shoulder.y - 0.05,
			"{opposite:?} hand remains below shoulder: {joints:?}"
		);
		assert!(
			(rig.posed_angle(&bone("forearm", opposite)) - idle.elbow_flex).abs() < 0.05,
			"{opposite:?} elbow stays relaxed"
		);
		Ok(())
	}

	fn assert_wave_arm(side: Side, progress: f32) -> anyhow::Result<()> {
		let (_, joints) = sample(side, progress)?;
		let own = side.sign();
		assert!(joints.shoulder.x * own > 0.2, "{side:?} shoulder {joints:?}");
		assert!(joints.elbow.x * own > 0.2, "{side:?} elbow {joints:?}");
		assert!(joints.elbow.y > joints.shoulder.y + 0.15, "elbow should lift {joints:?}");
		assert!(joints.humerus.x * own > 0.18, "humerus should leave shoulder outboard {joints:?}");
		let humerus_length = joints.humerus.length();
		assert!(humerus_length > 0.3, "missing humerus length {joints:?}");
		assert!(joints.humerus.y / humerus_length > 0.45, "humerus should lift {joints:?}");
		assert!(
			joints.humerus.y / humerus_length < 0.88,
			"humerus should not be vertical {joints:?}"
		);
		assert!(-joints.forearm.x * own > 0.08, "forearm should point medially {joints:?}");
		assert!(joints.forearm.y > 0.18, "forearm should point upward {joints:?}");
		assert!(joints.tip.x * own > 0.05, "hand should stay on its side {joints:?}");
		assert!(joints.elbow_flex > 0.9, "elbow should remain bent {joints:?}");
		Ok(())
	}

	#[test]
	fn victory_wave_production_bind_pose_has_bent_medial_upward_arms() -> anyhow::Result<()> {
		for side in [Side::Left, Side::Right] {
			for progress in [crate::animations::victory_wave::RAISE_END, 0.45, 0.65] {
				assert_wave_arm(side, progress)?;
			}
		}
		Ok(())
	}

	#[test]
	fn victory_wave_moves_both_actual_endpoints_and_mirrors() -> anyhow::Result<()> {
		for side in [Side::Left, Side::Right] {
			let (_, rest) = idle_sample(side)?;
			let (_, raised) = sample(side, 0.5)?;
			assert!(
				raised.tip.y > rest.tip.y + 0.12,
				"{side:?} hand should rise {rest:?} -> {raised:?}"
			);
			assert_wave_arm(side, 0.5)?;
		}
		let (_, right_a) = sample(Side::Right, 0.442)?;
		let (_, right_b) = sample(Side::Right, 0.55)?;
		let (_, left_a) = sample(Side::Left, 0.442)?;
		let (_, left_b) = sample(Side::Left, 0.55)?;
		let right_dx = right_b.tip.x - right_a.tip.x;
		let left_dx = left_b.tip.x - left_a.tip.x;
		assert!(right_dx * left_dx < 0.0, "mirrored lateral sweep {right_dx} / {left_dx}");
		assert!((right_dx.abs() - left_dx.abs()).abs() < 0.08);
		for side in [Side::Left, Side::Right] {
			for progress in [0.0, 0.15, 0.3, 0.45, 0.6, 0.78, 0.9, 1.0] {
				assert_opposite_arm_relaxed(side, progress)?;
			}
		}
		Ok(())
	}

	#[test]
	fn victory_wave_recovers_continuously_at_sixty_hz_and_finer_boundaries() -> anyhow::Result<()> {
		const FRAME: f32 = 1.0 / 60.0;
		for side in [Side::Left, Side::Right] {
			let (_, rest) = idle_sample(side)?;
			let (mut previous_rig, mut previous) =
				sample(side, crate::animations::victory_wave::WAVE_END)?;
			let mut time = crate::animations::victory_wave::WAVE_END + FRAME;
			let mut max_step = 0.0_f32;
			let mut max_rotation_step = 0.0_f32;
			while time < 1.0 {
				let (current_rig, current) = sample(side, time)?;
				let step = (current.tip - previous.tip).length();
				assert!(step.is_finite(), "{side:?} nonfinite recovery at {time}");
				max_step = max_step.max(step);
				max_rotation_step =
					max_rotation_step.max(max_pose_rotation_delta(&previous_rig, &current_rig));
				previous_rig = current_rig;
				previous = current;
				time += FRAME;
			}
			let (mut fine_rig, mut fine_joints) =
				sample(side, crate::animations::victory_wave::WAVE_END)?;
			let mut fine_time = crate::animations::victory_wave::WAVE_END + FRAME / 4.0;
			let mut fine_max_step = 0.0_f32;
			let mut fine_max_rotation = 0.0_f32;
			while fine_time < 1.0 {
				let (next_rig, next_joints) = sample(side, fine_time)?;
				fine_max_step = fine_max_step.max(next_joints.tip.distance(fine_joints.tip));
				fine_max_rotation =
					fine_max_rotation.max(max_pose_rotation_delta(&fine_rig, &next_rig));
				fine_rig = next_rig;
				fine_joints = next_joints;
				fine_time += FRAME / 4.0;
			}
			assert!(max_step > 0.01 && max_rotation_step > 0.01, "recovery must move");
			assert!(
				fine_max_step < max_step * 0.3,
				"{side:?} endpoint refinement {fine_max_step} vs {max_step}"
			);
			assert!(
				fine_max_rotation < max_rotation_step * 0.3,
				"{side:?} rotation refinement {fine_max_rotation} vs {max_rotation_step}"
			);
			let end = sample(side, 1.0)?.1;
			assert!(
				(end.tip - rest.tip).length() < 0.12,
				"end should land at rest {end:?} {rest:?}"
			);
			assert!(
				(end.tip - previous.tip).length() < 0.12,
				"last frame should blend {previous:?} {end:?}"
			);

			for boundary in [
				0.0,
				crate::animations::victory_wave::RAISE_END,
				crate::animations::victory_wave::WAVE_END,
				1.0,
			] {
				let epsilon = 1e-4;
				let (before_rig, before) = sample(side, (boundary - epsilon).max(0.0))?;
				let (at_rig, at) = sample(side, boundary)?;
				let (after_rig, after) = sample(side, (boundary + epsilon).min(1.0))?;
				assert!(
					(at.tip - before.tip).length() < 0.002,
					"{side:?} before {boundary}: {before:?} {at:?}"
				);
				assert!(max_pose_rotation_delta(&before_rig, &at_rig) < 0.01);
				assert!(
					(after.tip - at.tip).length() < 0.002,
					"{side:?} after {boundary}: {at:?} {after:?}"
				);
				assert!(max_pose_rotation_delta(&at_rig, &after_rig) < 0.01);
			}
		}
		Ok(())
	}

	#[test]
	fn victory_wave_clamps_and_preserves_unwritten_bones_and_root_effects() -> anyhow::Result<()> {
		let wave = VictoryWave::default();
		let (rest, _) = idle_sample(Side::Right)?;
		let mut start = HumanoidV0Rig::for_clip_test();
		assert_eq!(wave.apply(&mut start, 0.0), ArmatureOffset::IDENTITY);
		assert_eq!(start.pose, rest.pose, "start matches neutral Idle at phase zero");

		let mut end = HumanoidV0Rig::for_clip_test();
		assert_eq!(wave.apply(&mut end, 1.0), ArmatureOffset::IDENTITY);
		assert_eq!(end.pose, rest.pose, "end recovers to neutral Idle at phase zero");

		let mut below = HumanoidV0Rig::for_clip_test();
		let mut above = HumanoidV0Rig::for_clip_test();
		wave.apply_for(&mut below, -2.0);
		wave.apply_for(&mut above, 3.0);
		assert_eq!(below.pose, start.pose);
		assert_eq!(above.pose, end.pose);

		let mut posed = HumanoidV0Rig::for_clip_test();
		wave.apply_for(&mut posed, 0.5);
		for name in ["pelvis.L", "pelvis.R", "upper_belly", "lat.L", "lat.R"] {
			assert!(
				posed.rotation(name).normalize().dot(rest.rotation(name).normalize()).abs()
					> 1.0 - 1e-6,
				"{name} remains at rest"
			);
		}
		assert!(
			posed.rotation("root").normalize().dot(rest.rotation("root").normalize()).abs()
				> 1.0 - 1e-6
		);
		for name in ["femur.L", "femur.R", "shin.L", "shin.R"] {
			assert!(posed.posed_angle(name) > 1e-3, "{name} receives the intentional bounce");
		}
		Ok(())
	}

	#[test]
	fn victory_wave_is_repeatable_out_of_order_and_finite_for_changed_limb_lengths(
	) -> anyhow::Result<()> {
		let wave = VictoryWave::new(Side::Right);
		let mut long_forearm = HumanoidV0Rig::for_clip_test();
		let forearm = long_forearm.binding.definition.id("forearm.R");
		let Some(forearm) = forearm else {
			anyhow::bail!("missing right forearm");
		};
		let Some(rest) = long_forearm.binding.effective_rest.local.get(forearm.index()).copied()
		else {
			anyhow::bail!("missing right forearm rest transform");
		};
		long_forearm.seed_rest(
			"forearm.R",
			Transform {
				translation: rest.translation * 1.35,
				rotation: rest.rotation,
				scale: rest.scale,
			},
		);
		wave.apply_for(&mut long_forearm, 0.7);
		let first = arm_joints(&long_forearm, Side::Right)?;
		wave.apply_for(&mut long_forearm, 0.3);
		wave.apply_for(&mut long_forearm, 0.7);
		let repeated = arm_joints(&long_forearm, Side::Right)?;
		assert!((first.tip - repeated.tip).length() < 1e-5);
		assert!(first.tip.is_finite() && first.elbow.is_finite());
		for name in ["humerus.R", "forearm.R", "lumbar", "midback", "upper_back"] {
			let rotation = long_forearm.rotation(name);
			assert!(rotation.is_finite(), "non-finite {name}: {rotation:?}");
		}
		Ok(())
	}

	#[test]
	fn victory_wave_transition_entry_and_interrupt_keep_visible_pose() -> anyhow::Result<()> {
		let mut visible = HumanoidV0Rig::for_clip_test();
		VictoryWave::new(Side::Right).apply_for(&mut visible, 0.5);
		let captured = visible.pose.clone();
		let transition = Transition::from_visible(
			VictoryWave::new(Side::Left),
			captured.clone(),
			ArmatureOffset::IDENTITY,
		)
		.with_curve(TransitionCurve::SmoothStep);
		let mut at_entry = HumanoidV0Rig::for_clip_test();
		transition.apply(&mut at_entry, 0.4, 0.0);
		assert_eq!(at_entry.pose, captured, "transition weight zero preserves visible pose");

		let mut at_target = HumanoidV0Rig::for_clip_test();
		transition.apply(&mut at_target, 0.4, 1.0);
		let mut direct_target = HumanoidV0Rig::for_clip_test();
		VictoryWave::new(Side::Left).apply_for(&mut direct_target, 0.4);
		assert_eq!(at_target.pose, direct_target.pose, "transition weight one reaches target");

		let mut interrupted = HumanoidV0Rig::for_clip_test();
		transition.apply(&mut interrupted, 0.4, 0.5);
		let interrupted_visible = interrupted.pose.clone();
		let latest = Transition::from_visible(
			VictoryWave::new(Side::Right),
			interrupted_visible.clone(),
			ArmatureOffset::IDENTITY,
		);
		latest.apply(&mut interrupted, 0.7, 0.0);
		assert_eq!(
			interrupted.pose, interrupted_visible,
			"interruption starts at last visible pose"
		);
		Ok(())
	}
	#[test]
	fn victory_wave_nested_mix_preserves_the_outer_visible_source() {
		use crate::animations::Mix;
		use character_rigs::authoring::PoseBuffer;

		let wave = VictoryWave::new(Side::Left);
		let progress = 0.14;
		let mut source = HumanoidV0Rig::for_clip_test();
		Idle::default().apply_for(&mut source, progress);
		let mut target = HumanoidV0Rig::for_clip_test();
		wave.apply_for(&mut target, progress);
		let mut expected = source.pose.clone();
		PoseBuffer::blend_into(&source.pose, &target.pose, 0.35, &mut expected);

		let mut actual = HumanoidV0Rig::for_clip_test();
		Mix::new(Idle::default(), wave, 0.35).apply_for(&mut actual, progress);
		assert_eq!(actual.scratch.depth, 0);
		for (index, (a, b)) in actual.pose.local.iter().zip(&expected.local).enumerate() {
			assert!(a.translation.distance(b.translation) < 1e-6, "bone {index} translation");
			assert!(
				a.rotation.normalize().dot(b.rotation.normalize()).abs() > 1.0 - 1e-6,
				"bone {index} rotation"
			);
			assert!(a.scale.distance(b.scale) < 1e-6, "bone {index} scale");
		}
	}
}
