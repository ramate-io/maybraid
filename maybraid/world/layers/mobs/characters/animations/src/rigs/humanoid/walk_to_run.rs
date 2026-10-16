use bevy::prelude::Vec3;
use character_rigs::authoring::ArmatureOffset;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::{Prone, Run, Spring, Transition, Walk, WalkToRun};
use crate::rigs::mix::blend_clips;
use crate::{Animation, Effects};

impl Animation<HumanoidV0Rig> for WalkToRun {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let weight = self.weight(progress);
		let walk_phase = self.walk_phase_at_progress_for(progress);
		let run_phase = self.run_phase_at_progress_for(progress);
		blend_clips(rig, &self.walk, walk_phase, &self.run, run_phase, weight);
	}

	fn effects_for(&self, _rig: &HumanoidV0Rig, _progress: f32) -> Effects {
		Effects::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use std::fmt::Write as _;

	use super::*;

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	fn shin_tail(rig: &HumanoidV0Rig, side: &str) -> Vec3 {
		rig.character_point(side) + rig.character_length(side)
	}

	fn format_vec3(v: Vec3) -> String {
		format!("({:.4}, {:.4}, {:.4})", v.x, v.y, v.z)
	}

	/// Character-space foot and pelvis samples for PR evidence (run with `--nocapture`).
	#[test]
	fn posed_joint_evidence_tables() -> anyhow::Result<()> {
		const SAMPLES: [f32; 5] = [0.0, 0.25, 0.5, 0.75, 1.0];
		for gait in [0.0, 0.5] {
			let shift = WalkToRun { gait_phase: gait, ..WalkToRun::default() };
			eprintln!(
				"\nWalkToRun gait_phase={gait} (for_clip_test, character space; hips = femur roots)"
			);
			eprintln!("| progress | femur.L hip | femur.R hip | shin.L tail | shin.R tail |");
			eprintln!("| --- | --- | --- | --- | --- |");
			for progress in SAMPLES {
				let mut rig = HumanoidV0Rig::for_clip_test();
				shift.apply(&mut rig, progress);
				let mut row = String::new();
				write!(
					&mut row,
					"| {progress:.2} | {} | {} | {} | {} |",
					format_vec3(rig.character_point("femur.L")),
					format_vec3(rig.character_point("femur.R")),
					format_vec3(shin_tail(&rig, "shin.L")),
					format_vec3(shin_tail(&rig, "shin.R")),
				)?;
				eprintln!("{row}");
			}
		}
		Ok(())
	}

	#[test]
	fn start_matches_walk_at_gait_phase() -> anyhow::Result<()> {
		let shift = WalkToRun { gait_phase: 0.25, ..WalkToRun::default() };
		let phase = shift.gait_phase();
		let mut shift_rig = HumanoidV0Rig::for_clip_test();
		let mut walk_rig = HumanoidV0Rig::for_clip_test();
		shift.apply(&mut shift_rig, 0.0);
		Walk::default().apply(&mut walk_rig, phase);

		for name in shift_rig.animation_bone_names() {
			let a = shift_rig.rotation(name);
			let b = walk_rig.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "rotation mismatch on {name}");
		}
		Ok(())
	}

	#[test]
	fn end_matches_run_at_handoff_phase() -> anyhow::Result<()> {
		let shift = WalkToRun { gait_phase: 0.25, ..WalkToRun::default() };
		let handoff = shift.handoff_run_phase_for();
		let mut shift_rig = HumanoidV0Rig::for_clip_test();
		let mut run_rig = HumanoidV0Rig::for_clip_test();
		shift.apply(&mut shift_rig, 1.0);
		Run::default().apply(&mut run_rig, handoff);

		for name in shift_rig.animation_bone_names() {
			let a = shift_rig.rotation(name);
			let b = run_rig.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "rotation mismatch on {name}");
		}
		Ok(())
	}

	#[test]
	fn handoff_foot_matches_looping_run_not_frozen_entry_phase() -> anyhow::Result<()> {
		let gait = 0.25;
		let shift = WalkToRun { gait_phase: gait, ..WalkToRun::default() };
		let handoff = shift.handoff_run_phase_for();
		assert!((handoff - gait).abs() > 0.05, "run phase should advance during the shift");

		let mut end = HumanoidV0Rig::for_clip_test();
		shift.apply(&mut end, 1.0);
		let end_left = shin_tail(&end, "shin.L");
		let end_right = shin_tail(&end, "shin.R");

		let mut run_handoff = HumanoidV0Rig::for_clip_test();
		Run::default().apply(&mut run_handoff, handoff);
		let loop_left = shin_tail(&run_handoff, "shin.L");
		let loop_right = shin_tail(&run_handoff, "shin.R");
		assert!(
			(end_left - loop_left).length() < 1e-4,
			"L foot handoff delta {:?}",
			end_left - loop_left
		);
		assert!(
			(end_right - loop_right).length() < 1e-4,
			"R foot handoff delta {:?}",
			end_right - loop_right
		);

		let mut run_frozen = HumanoidV0Rig::for_clip_test();
		Run::default().apply(&mut run_frozen, gait);
		let frozen_left = shin_tail(&run_frozen, "shin.L");
		let frozen_delta = (end_left - frozen_left).length();
		eprintln!(
			"foot L delta WalkToRun@1.0 vs Run@gait={gait} (frozen phase): {frozen_delta:.4} m"
		);
		assert!(frozen_delta > 0.02, "frozen-phase handoff should disagree; got {frozen_delta}");
		Ok(())
	}

	#[test]
	fn midpoint_sits_between_walk_and_run_at_phase() -> anyhow::Result<()> {
		let shift = WalkToRun::default();
		let progress = 0.5;
		let walk_phase = shift.walk_phase_at_progress_for(progress);
		let run_phase = shift.run_phase_at_progress_for(progress);
		let mut mid = HumanoidV0Rig::for_clip_test();
		let mut walk = HumanoidV0Rig::for_clip_test();
		let mut run = HumanoidV0Rig::for_clip_test();
		shift.apply(&mut mid, progress);
		Walk::default().apply(&mut walk, walk_phase);
		Run::default().apply(&mut run, run_phase);

		let mid_femur = mid.posed_angle("femur.L");
		let walk_femur = walk.posed_angle("femur.L");
		let run_femur = run.posed_angle("femur.L");
		let lo = walk_femur.min(run_femur) - 0.02;
		let hi = walk_femur.max(run_femur) + 0.02;
		assert!(mid_femur > lo && mid_femur < hi, "mid femur {mid_femur} not in [{lo}, {hi}]");
		Ok(())
	}

	#[test]
	fn resampling_does_not_accumulate() -> anyhow::Result<()> {
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		let shift = WalkToRun::default();
		shift.apply(&mut once, 0.4);
		shift.apply(&mut twice, 0.4);
		shift.apply(&mut twice, 0.4);
		for name in once.animation_bone_names() {
			let a = once.rotation(name);
			let b = twice.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "re-sample must not drift on {name}");
		}
		Ok(())
	}

	#[test]
	fn samples_at_60hz_are_stable_and_match_endpoints() -> anyhow::Result<()> {
		use crate::animations::DEFAULT_WALK_TO_RUN_SPEED;

		const HZ: f32 = 60.0;
		let shift = WalkToRun { gait_phase: 0.33, ..WalkToRun::default() };
		let wall_duration = 1.0 / DEFAULT_WALK_TO_RUN_SPEED;
		let frame_count = (wall_duration * HZ).ceil() as u32;

		let mut walk_ref = HumanoidV0Rig::for_clip_test();
		Walk::default().apply(&mut walk_ref, shift.gait_phase());
		let mut start = HumanoidV0Rig::for_clip_test();
		shift.apply(&mut start, 0.0);
		for name in start.animation_bone_names() {
			assert!(
				start.rotation(name).dot(walk_ref.rotation(name)).abs() > 1.0 - 1e-5,
				"frame 0 should match walk at gait on {name}"
			);
		}

		let handoff = shift.handoff_run_phase_for();
		let mut run_ref = HumanoidV0Rig::for_clip_test();
		Run::default().apply(&mut run_ref, handoff);

		for frame in 0..=frame_count {
			let progress = ((frame as f32 / HZ) * DEFAULT_WALK_TO_RUN_SPEED).min(1.0);
			let mut once = HumanoidV0Rig::for_clip_test();
			let mut twice = HumanoidV0Rig::for_clip_test();
			shift.apply(&mut once, progress);
			shift.apply(&mut twice, progress);
			shift.apply(&mut twice, progress);
			for name in once.animation_bone_names() {
				assert!(
					once.rotation(name).dot(twice.rotation(name)).abs() > 1.0 - 1e-5,
					"60 Hz frame {frame} must not accumulate on {name}"
				);
			}
		}

		let mut end = HumanoidV0Rig::for_clip_test();
		shift.apply(&mut end, 1.0);
		for name in end.animation_bone_names() {
			assert!(
				end.rotation(name).dot(run_ref.rotation(name)).abs() > 1.0 - 1e-5,
				"final frame should match run handoff on {name}"
			);
		}
		Ok(())
	}

	#[test]
	fn transition_from_walk_blends_into_mid_shift() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Walk::default().apply(&mut rig, 0.35);
		let from_pose = rig.pose.clone();
		let walk_femur = rig.posed_angle("femur.L");

		let mut blended = HumanoidV0Rig::for_clip_test();
		Transition::from_visible(WalkToRun::default(), from_pose.clone(), ArmatureOffset::IDENTITY)
			.apply(&mut blended, 0.35, 0.25);

		assert!(
			(blended.posed_angle("femur.L") - walk_femur).abs() < 0.2,
			"early blend should stay near the visible walk pose"
		);

		let mut entry = HumanoidV0Rig::for_clip_test();
		Transition::from_visible(WalkToRun::default(), from_pose.clone(), ArmatureOffset::IDENTITY)
			.apply(&mut entry, 0.0, 0.0);
		assert!(
			(entry.posed_angle("femur.L") - walk_femur).abs() < 1e-4,
			"transition entry should match captured walk pose"
		);
		Ok(())
	}

	#[test]
	fn jump_spring_interrupt_blends_from_visible_pose() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Spring::default().apply(&mut rig, 0.55);
		let from_pose = rig.pose.clone();
		let spring_femur = rig.posed_angle("femur.L");

		let mut blended = HumanoidV0Rig::for_clip_test();
		Transition::from_visible(WalkToRun::default(), from_pose, ArmatureOffset::IDENTITY).apply(
			&mut blended,
			0.35,
			0.2,
		);

		assert!(
			(blended.posed_angle("femur.L") - spring_femur).abs() < 0.15,
			"early blend should keep jump takeoff leg pose visible"
		);
		Ok(())
	}

	#[test]
	fn prone_interrupt_keeps_torso_near_floor_pose() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Prone::default().apply(&mut rig, 1.0);
		let from_pose = rig.pose.clone();
		let prone_root = tip(&rig, "root");

		let mut blended = HumanoidV0Rig::for_clip_test();
		Transition::from_visible(WalkToRun::default(), from_pose, ArmatureOffset::IDENTITY).apply(
			&mut blended,
			0.0,
			0.0,
		);

		let blended_root = tip(&blended, "root");
		assert!(
			(blended_root - prone_root).length() < 0.05,
			"entry should not pop to upright rest"
		);
		Ok(())
	}

	#[test]
	fn longer_femurs_still_match_run_at_end() -> anyhow::Result<()> {
		let mut shift_rig = HumanoidV0Rig::for_clip_test();
		let mut run_rig = HumanoidV0Rig::for_clip_test();
		shift_rig.seed_rest("femur.L", bevy::prelude::Transform::from_translation(Vec3::Y * 0.8));
		shift_rig.seed_rest("femur.R", bevy::prelude::Transform::from_translation(Vec3::Y * 0.8));
		shift_rig.seed_rest("shin.L", bevy::prelude::Transform::from_translation(Vec3::Y * 0.65));
		shift_rig.seed_rest("shin.R", bevy::prelude::Transform::from_translation(Vec3::Y * 0.65));
		run_rig.seed_rest("femur.L", bevy::prelude::Transform::from_translation(Vec3::Y * 0.8));
		run_rig.seed_rest("femur.R", bevy::prelude::Transform::from_translation(Vec3::Y * 0.8));
		run_rig.seed_rest("shin.L", bevy::prelude::Transform::from_translation(Vec3::Y * 0.65));
		run_rig.seed_rest("shin.R", bevy::prelude::Transform::from_translation(Vec3::Y * 0.65));

		let shift = WalkToRun::default();
		shift.apply(&mut shift_rig, 1.0);
		Run::default().apply(&mut run_rig, shift.handoff_run_phase_for());

		for name in shift_rig.animation_bone_names() {
			let a = shift_rig.rotation(name);
			let b = run_rig.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "rotation mismatch on {name}");
		}
		Ok(())
	}

	#[test]
	fn midpoint_opposes_the_legs() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		WalkToRun::default().apply(&mut rig, 0.5);
		let left = rig.character_length("femur.L");
		let right = rig.character_length("femur.R");
		assert!(
			left.z > 0.02 && right.z < -0.02,
			"mid blend keeps opposing legs L={left:?} R={right:?}"
		);
		Ok(())
	}

	#[test]
	fn both_legs_track_walk_and_run_endpoints() -> anyhow::Result<()> {
		let shift = WalkToRun { gait_phase: 0.4, ..WalkToRun::default() };
		let mut at_start = HumanoidV0Rig::for_clip_test();
		shift.apply(&mut at_start, 0.0);
		for side in ["femur.L", "femur.R"] {
			let angle = at_start.posed_angle(side);
			assert!(angle.abs() > 0.01, "{side} should leave rest at walk entry");
		}

		let mut at_end = HumanoidV0Rig::for_clip_test();
		shift.apply(&mut at_end, 1.0);
		let mut run_ref = HumanoidV0Rig::for_clip_test();
		Run::default().apply(&mut run_ref, shift.handoff_run_phase_for());
		for side in ["femur.L", "femur.R"] {
			assert!(
				at_end.rotation(side).dot(run_ref.rotation(side)).abs() > 1.0 - 1e-5,
				"{side} should match run handoff"
			);
		}
		Ok(())
	}
}
