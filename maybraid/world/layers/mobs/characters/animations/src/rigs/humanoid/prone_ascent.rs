use character_rigs::authoring::{ArmatureOffset, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::ProneAscent;
use crate::{Animation, Effects};

impl Animation<HumanoidV0Rig> for ProneAscent {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let mut pose = HumanoidPose::default();
		let femur = self.femur_swing(progress);
		let shin = self.shin_flex(progress);
		pose.apply_leg(Side::Left, femur, shin);
		pose.apply_leg(Side::Right, femur, shin);
		pose.apply_hip_fold(Side::Left, femur * 0.35);
		pose.apply_hip_fold(Side::Right, femur * 0.35);
		pose.apply_spine_pitch(self.spine_pitch(progress));
		let neck = self.neck_swing(progress);
		pose.apply_neck_twisted(0.0, 0.0, neck, 0.0, 0.0, neck);
		let hold = self.arm_hold(progress);
		let push = self.arm_push(progress);
		for side in [Side::Left, Side::Right] {
			pose.apply_arm(side, 0.0, 0.0, 0.0, hold + push, hold);
		}
		rig.write_pose(&pose);
	}

	fn effects_for(&self, _rig: &HumanoidV0Rig, _progress: f32) -> Effects {
		ArmatureOffset::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;

	use super::*;
	use crate::animations::{Idle, Prone, Squat, Transition, TwoFootedJump};

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	#[test]
	fn start_matches_held_prone() -> anyhow::Result<()> {
		let mut ascent_rig = HumanoidV0Rig::for_clip_test();
		let mut held_rig = HumanoidV0Rig::for_clip_test();
		ProneAscent::default().apply(&mut ascent_rig, 0.0);
		Prone::default().apply(&mut held_rig, 1.0);

		for name in ascent_rig.animation_bone_names() {
			let a = ascent_rig.rotation(name);
			let b = held_rig.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-4, "rotation mismatch on {name}");
		}
		Ok(())
	}

	#[test]
	fn end_matches_upright() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		ProneAscent::default().apply(&mut rig, 1.0);
		let rest = HumanoidV0Rig::for_clip_test();
		for name in ["femur.L", "shin.L", "root", "lumbar"] {
			let posed = tip(&rig, name);
			let rest_tip = tip(&rest, name);
			assert!((posed - rest_tip).length() < 1e-3, "{name} should return to rest");
		}
		Ok(())
	}

	#[test]
	fn mid_ascent_unfolds_spine_and_legs() -> anyhow::Result<()> {
		let mut mid = HumanoidV0Rig::for_clip_test();
		let mut prone = HumanoidV0Rig::for_clip_test();
		let mut upright = HumanoidV0Rig::for_clip_test();
		ProneAscent::default().apply(&mut mid, 0.5);
		Prone::default().apply(&mut prone, 1.0);
		ProneAscent::default().apply(&mut upright, 1.0);
		let mid_root = tip(&mid, "root");
		let prone_root = tip(&prone, "root");
		let upright_root = tip(&upright, "root");
		assert!(
			mid_root.z < prone_root.z - 0.02 && mid_root.z > upright_root.z + 0.02,
			"mid ascent root {mid_root:?} should sit between prone {prone_root:?} and upright {upright_root:?}"
		);
		assert!(
			(mid.posed_angle("femur.L") - mid.posed_angle("femur.R")).abs() < 1e-4,
			"both legs share flexion"
		);
		Ok(())
	}

	#[test]
	fn resampling_does_not_accumulate() -> anyhow::Result<()> {
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		let ascent = ProneAscent::default();
		ascent.apply(&mut once, 0.4);
		ascent.apply(&mut twice, 0.4);
		ascent.apply(&mut twice, 0.4);
		for name in once.animation_bone_names() {
			let a = once.rotation(name);
			let b = twice.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "re-sample must not drift on {name}");
		}
		Ok(())
	}

	#[test]
	fn transition_from_visible_prone_blends_mid_ascent() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Prone::default().apply(&mut rig, 1.0);
		let from_pose = rig.pose.clone();
		let transition =
			Transition::from_visible(ProneAscent::default(), from_pose, ArmatureOffset::IDENTITY);
		transition.apply(&mut rig, 0.5, 0.5);
		let root = tip(&rig, "root");
		assert!(root.z > 0.15 && root.z < 0.85, "blend should sit between prone and upright");
		Ok(())
	}

	#[test]
	fn jump_interrupt_pose_stays_finite() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		ProneAscent::default().apply(&mut rig, 0.35);
		let from_pose = rig.pose.clone();
		let transition =
			Transition::from_visible(TwoFootedJump::default(), from_pose, ArmatureOffset::IDENTITY);
		transition.apply(&mut rig, 0.2, 0.4);
		for name in rig.animation_bone_names() {
			let q = rig.rotation(name);
			assert!(q.length_squared().is_finite(), "{name} rotation must stay finite");
		}
		Ok(())
	}

	#[test]
	fn body_proportions_preserve_rotation_endpoints() -> anyhow::Result<()> {
		use character_rigs::humanoid::LegSegmentLengths;

		let mut default_rig = HumanoidV0Rig::for_clip_test();
		let mut long_rig = HumanoidV0Rig::for_clip_test();
		long_rig.segment_lengths = LegSegmentLengths { femur: 0.625, shin: 0.625 };
		let ascent = ProneAscent::default();
		for progress in [0.0, 1.0] {
			ascent.apply(&mut default_rig, progress);
			ascent.apply(&mut long_rig, progress);
			for name in ["femur.L", "femur.R", "shin.L", "shin.R"] {
				let a = default_rig.rotation(name);
				let b = long_rig.rotation(name);
				assert!(a.dot(b).abs() > 1.0 - 1e-5, "{name} at progress {progress}");
			}
		}
		Ok(())
	}

	#[test]
	fn upright_endpoint_matches_idle_neighbour() -> anyhow::Result<()> {
		let mut ascent_rig = HumanoidV0Rig::for_clip_test();
		let mut idle_rig = HumanoidV0Rig::for_clip_test();
		ProneAscent::default().apply(&mut ascent_rig, 1.0);
		Idle::default().apply(&mut idle_rig, 0.0);
		for name in ["femur.L", "femur.R", "root", "lumbar"] {
			let a = ascent_rig.rotation(name);
			let b = idle_rig.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-4, "ascent end should match idle on {name}");
		}
		Ok(())
	}

	#[derive(Debug, Clone, Copy)]
	struct Landmarks {
		root: Vec3,
		pelvis: Vec3,
		head: Vec3,
		foot_l: Vec3,
		foot_r: Vec3,
		hand_l: Vec3,
		hand_r: Vec3,
	}

	fn end_effector(rig: &HumanoidV0Rig, bone: &str) -> Vec3 {
		rig.character_point(bone) + rig.character_length(bone)
	}

	fn landmarks(rig: &HumanoidV0Rig) -> Landmarks {
		Landmarks {
			root: rig.character_point("root"),
			pelvis: rig.character_point("buttocks"),
			head: end_effector(rig, "upper_neck"),
			foot_l: end_effector(rig, "shin.L"),
			foot_r: end_effector(rig, "shin.R"),
			hand_l: end_effector(rig, "forearm.L"),
			hand_r: end_effector(rig, "forearm.R"),
		}
	}

	fn max_joint_delta(a: &HumanoidV0Rig, b: &HumanoidV0Rig) -> f32 {
		a.animation_bone_names()
			.map(|name| a.rotation(name).angle_between(b.rotation(name)))
			.fold(0.0_f32, f32::max)
	}

	fn sample_ascent(progress: f32) -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::for_clip_test();
		ProneAscent::default().apply(&mut rig, progress);
		rig
	}

	#[test]
	fn progress_zero_matches_held_prone_landmarks() -> anyhow::Result<()> {
		let ascent = landmarks(&sample_ascent(0.0));
		let mut held_rig = HumanoidV0Rig::for_clip_test();
		Prone::default().apply(&mut held_rig, 1.0);
		let held = landmarks(&held_rig);
		for (label, a, b) in [
			("root", ascent.root, held.root),
			("pelvis", ascent.pelvis, held.pelvis),
			("head", ascent.head, held.head),
			("foot_l", ascent.foot_l, held.foot_l),
			("foot_r", ascent.foot_r, held.foot_r),
			("hand_l", ascent.hand_l, held.hand_l),
			("hand_r", ascent.hand_r, held.hand_r),
		] {
			assert!((a - b).length() < 2e-3, "{label} entry mismatch: {a:?} vs {b:?}");
		}
		let ascent_rig = sample_ascent(0.0);
		assert!(
			max_joint_delta(&ascent_rig, &held_rig) < 1e-3,
			"entry max joint Δ = {}",
			max_joint_delta(&ascent_rig, &held_rig),
		);
		Ok(())
	}

	#[test]
	fn progress_one_matches_idle_for_stand_exit() -> anyhow::Result<()> {
		let ascent_rig = sample_ascent(1.0);
		let mut idle_rig = HumanoidV0Rig::for_clip_test();
		Idle::default().apply(&mut idle_rig, 0.0);
		let ascent = landmarks(&ascent_rig);
		let idle = landmarks(&idle_rig);
		for (label, a, b) in
			[("root", ascent.root, idle.root), ("pelvis", ascent.pelvis, idle.pelvis)]
		{
			assert!((a - b).length() < 5e-3, "{label} stand-exit mismatch: {a:?} vs {b:?}");
		}
		for (label, a, b) in
			[("foot_l", ascent.foot_l, idle.foot_l), ("foot_r", ascent.foot_r, idle.foot_r)]
		{
			assert!((a - b).length() < 0.04, "{label} stand-exit mismatch: {a:?} vs {b:?}");
		}
		assert!(
			(ascent.head - idle.head).length() < 0.08,
			"head may differ slightly from idle hang: {:?} vs {:?}",
			ascent.head,
			idle.head,
		);
		for name in ["femur.L", "femur.R", "root", "lumbar"] {
			let delta = ascent_rig.rotation(name).angle_between(idle_rig.rotation(name));
			assert!(delta < 1e-3, "stand-exit max joint Δ on {name} = {delta}");
		}
		Ok(())
	}

	/// Gameplay squat exit keeps the motor squat hull while the clip rises to idle; held squat
	/// follows via mailbox crossfade once ascent settles.
	#[test]
	fn progress_one_is_idle_before_squat_clip_handoff() -> anyhow::Result<()> {
		let ascent_rig = sample_ascent(1.0);
		let mut squat_rig = HumanoidV0Rig::for_clip_test();
		Squat::held().apply(&mut squat_rig, 1.0);
		let delta = max_joint_delta(&ascent_rig, &squat_rig);
		assert!(
			delta > 0.15,
			"squat handoff should differ from ascent end (Δ={delta}); mailbox crossfade covers it",
		);
		assert!(
			ascent_rig.posed_angle("femur.L") < squat_rig.posed_angle("femur.L") - 0.1,
			"ascent end legs stay straighter than held squat",
		);
		Ok(())
	}

	#[test]
	fn posed_landmarks_rise_and_feet_stay_planted() -> anyhow::Result<()> {
		let samples = [0.0, 0.25, 0.5, 0.75, 1.0].map(|p| (p, landmarks(&sample_ascent(p))));
		for (progress, pose) in &samples {
			assert!(pose.root.y > -0.05, "root at {progress} should stay above floor");
			assert!(
				pose.foot_l.y > -1.55 && pose.foot_r.y > -1.55,
				"feet at {progress} should not penetrate the floor plane",
			);
			assert!(
				(pose.foot_l.x + pose.foot_r.x).abs() < 0.12,
				"feet stay roughly symmetric at {progress}: L={:?} R={:?}",
				pose.foot_l,
				pose.foot_r,
			);
		}
		for window in samples.windows(2) {
			let (_, earlier) = window[0];
			let (_, later) = window[1];
			assert!(
				later.root.y >= earlier.root.y - 1e-3,
				"root height should not drop during ascent",
			);
			assert!(
				later.head.y >= earlier.head.y - 1e-3,
				"head height should not drop during ascent",
			);
			assert!(
				later.foot_l.z >= earlier.foot_l.z - 0.05
					&& later.foot_r.z >= earlier.foot_r.z - 0.05,
				"feet should sweep forward toward standing, not drift backward",
			);
		}
		let (_, start) = samples[0];
		let (_, end) = samples[4];
		assert!(end.head.y > start.head.y + 0.4, "head lifts from prone");
		assert!(end.head.z < start.head.z - 0.8, "spine unfolds from prone pitch");
		Ok(())
	}

	#[test]
	fn mid_ascent_interrupt_blends_toward_jump() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		ProneAscent::default().apply(&mut rig, 0.5);
		let mid = landmarks(&rig);
		let from_pose = rig.pose.clone();
		let transition =
			Transition::from_visible(TwoFootedJump::default(), from_pose, ArmatureOffset::IDENTITY);
		transition.apply(&mut rig, 0.2, 0.4);
		let interrupted = landmarks(&rig);
		assert!(
			interrupted.root.y > mid.root.y - 0.05 && interrupted.head.y > mid.head.y - 0.2,
			"interrupt should stay in a plausible rising band",
		);
		for name in rig.animation_bone_names() {
			let q = rig.rotation(name);
			assert!(q.length_squared().is_finite(), "{name} rotation must stay finite");
		}
		Ok(())
	}

	#[test]
	fn mailbox_visible_entry_matches_prone_at_blend_start() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Prone::default().apply(&mut rig, 1.0);
		let prone_landmarks = landmarks(&rig);
		let from_pose = rig.pose.clone();
		let transition =
			Transition::from_visible(ProneAscent::default(), from_pose, ArmatureOffset::IDENTITY);
		transition.apply(&mut rig, 0.0, 0.0);
		let entry = landmarks(&rig);
		assert!(
			(entry.root - prone_landmarks.root).length() < 2e-3,
			"blend weight 0 should preserve prone root",
		);
		transition.apply(&mut rig, 0.0, 0.5);
		let mid_blend = landmarks(&rig);
		assert!(
			mid_blend.root.y > prone_landmarks.root.y - 0.02,
			"mid crossfade should not pop below prone",
		);
		Ok(())
	}

	#[test]
	fn landmark_snapshot_for_review() {
		for progress in [0.0, 0.25, 0.5, 0.75, 1.0] {
			let pose = landmarks(&sample_ascent(progress));
			eprintln!(
				"prone_ascent progress={progress:.2} root_y={:.4} pelvis_y={:.4} head={:?} foot_L={:?} foot_R={:?} hand_L={:?} hand_R={:?}",
				pose.root.y,
				pose.pelvis.y,
				pose.head,
				pose.foot_l,
				pose.foot_r,
				pose.hand_l,
				pose.hand_r,
			);
		}
	}
}
