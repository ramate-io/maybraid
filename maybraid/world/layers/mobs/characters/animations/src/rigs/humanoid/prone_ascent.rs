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
	use crate::animations::{Idle, Prone, Transition, TwoFootedJump};

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
		let transition = Transition::from_visible(
			TwoFootedJump::default(),
			from_pose,
			ArmatureOffset::IDENTITY,
		);
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
}
