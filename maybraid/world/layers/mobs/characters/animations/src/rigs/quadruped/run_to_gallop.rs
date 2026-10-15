use bevy::prelude::Vec3;
use character_rigs::rigs::quadruped_v0::QuadrupedV0Rig;

use crate::animations::{Gallop, QuadrupedRun, QuadrupedRunToGallop};
use crate::rigs::quadruped::mix::blend_clips;
use crate::{Animation, Effects};

impl Animation<QuadrupedV0Rig> for QuadrupedRunToGallop {
	fn apply_for(&self, rig: &mut QuadrupedV0Rig, progress: f32) {
		let weight = self.weight(progress);
		let run_phase = self.run_phase_at_progress_for(progress);
		let gallop_phase = self.gallop_phase_at_progress_for(progress);
		blend_clips(rig, &self.run, run_phase, &self.gallop, gallop_phase, weight);
	}

	fn effects_for(&self, _rig: &QuadrupedV0Rig, _progress: f32) -> Effects {
		Effects::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use std::fmt::Write as _;

	use super::*;

	fn tip(rig: &QuadrupedV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	#[test]
	fn start_matches_run_at_gait_phase() -> anyhow::Result<()> {
		let shift = QuadrupedRunToGallop { gait_phase: 0.25, ..QuadrupedRunToGallop::default() };
		let phase = shift.gait_phase();
		let mut shift_rig = QuadrupedV0Rig::for_clip_test();
		let mut run_rig = QuadrupedV0Rig::for_clip_test();
		shift.apply(&mut shift_rig, 0.0);
		QuadrupedRun::default().apply(&mut run_rig, phase);

		for name in shift_rig.animation_bone_names() {
			let a = shift_rig.rotation(name);
			let b = run_rig.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "rotation mismatch on {name}");
		}
		Ok(())
	}

	#[test]
	fn end_matches_gallop_at_handoff_phase() -> anyhow::Result<()> {
		let shift = QuadrupedRunToGallop { gait_phase: 0.25, ..QuadrupedRunToGallop::default() };
		let handoff = shift.handoff_gallop_phase_for();
		let mut shift_rig = QuadrupedV0Rig::for_clip_test();
		let mut gallop_rig = QuadrupedV0Rig::for_clip_test();
		shift.apply(&mut shift_rig, 1.0);
		Gallop::default().apply(&mut gallop_rig, handoff);

		for name in shift_rig.animation_bone_names() {
			let a = shift_rig.rotation(name);
			let b = gallop_rig.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "rotation mismatch on {name}");
		}
		Ok(())
	}

	#[test]
	fn midpoint_differs_from_either_endpoint() -> anyhow::Result<()> {
		let shift = QuadrupedRunToGallop::default();
		let progress = 0.5;
		let run_phase = shift.run_phase_at_progress_for(progress);
		let gallop_phase = shift.gallop_phase_at_progress_for(progress);
		let mut mid = QuadrupedV0Rig::for_clip_test();
		let mut run = QuadrupedV0Rig::for_clip_test();
		let mut gallop = QuadrupedV0Rig::for_clip_test();
		shift.apply(&mut mid, progress);
		QuadrupedRun::default().apply(&mut run, run_phase);
		Gallop::default().apply(&mut gallop, gallop_phase);

		let mid_tip = tip(&mid, "anterior_thigh.L");
		let run_tip = tip(&run, "anterior_thigh.L");
		let gallop_tip = tip(&gallop, "anterior_thigh.L");
		assert!(
			(mid_tip - run_tip).length() > 0.02 && (mid_tip - gallop_tip).length() > 0.02,
			"mid blend should differ from both endpoints at weight 0.5"
		);
		Ok(())
	}

	#[test]
	fn phase_alignment_numbers_at_half_weight() -> anyhow::Result<()> {
		let gait = 0.2;
		let shift = QuadrupedRunToGallop { gait_phase: gait, ..QuadrupedRunToGallop::default() };
		let progress = 0.5;
		let weight = shift.weight(progress);
		let run_phase = shift.run_phase_at_progress_for(progress);
		let gallop_phase = shift.gallop_phase_at_progress_for(progress);
		assert!((weight - 0.5).abs() < 0.02, "weight {weight}");
		let elapsed = QuadrupedRunToGallop::blend_elapsed(progress);
		let expected_run =
			(gait + elapsed * crate::animations::QUADRUPED_RUN_CYCLE_SPEED).rem_euclid(1.0);
		let expected_gallop =
			(gait + elapsed * crate::animations::GALLOP_CYCLE_SPEED).rem_euclid(1.0);
		assert!((run_phase - expected_run).abs() < 1e-5);
		assert!((gallop_phase - expected_gallop).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn resampling_does_not_accumulate() -> anyhow::Result<()> {
		let mut once = QuadrupedV0Rig::for_clip_test();
		let mut twice = QuadrupedV0Rig::for_clip_test();
		let shift = QuadrupedRunToGallop::default();
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
	fn handoff_gallop_phase_advances_past_entry_gait() -> anyhow::Result<()> {
		let gait = 0.25;
		let shift = QuadrupedRunToGallop { gait_phase: gait, ..QuadrupedRunToGallop::default() };
		let handoff = shift.handoff_gallop_phase_for();
		assert!((handoff - gait).abs() > 0.02, "gallop phase should advance during the shift");

		let mut end = QuadrupedV0Rig::for_clip_test();
		shift.apply(&mut end, 1.0);
		let mut gallop_loop = QuadrupedV0Rig::for_clip_test();
		Gallop::default().apply(&mut gallop_loop, handoff);
		for name in end.animation_bone_names() {
			let a = end.rotation(name);
			let b = gallop_loop.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "handoff mismatch on {name}");
		}
		Ok(())
	}

	#[test]
	fn posed_joint_evidence_tables() -> anyhow::Result<()> {
		const SAMPLES: [f32; 5] = [0.0, 0.25, 0.5, 0.75, 1.0];
		for gait in [0.0, 0.5] {
			let shift =
				QuadrupedRunToGallop { gait_phase: gait, ..QuadrupedRunToGallop::default() };
			eprintln!(
				"\nQuadrupedRunToGallop gait_phase={gait} (for_clip_test; thigh tips in bone-local +Y)"
			);
			eprintln!("| progress | weight | run_phase | gallop_phase | ant_thigh.L tip |");
			eprintln!("| --- | --- | --- | --- | --- |");
			for progress in SAMPLES {
				let mut rig = QuadrupedV0Rig::for_clip_test();
				shift.apply(&mut rig, progress);
				let mut row = String::new();
				write!(
					&mut row,
					"| {progress:.2} | {:.3} | {:.3} | {:.3} | ({:.3}, {:.3}, {:.3}) |",
					shift.weight(progress),
					shift.run_phase_at_progress_for(progress),
					shift.gallop_phase_at_progress_for(progress),
					tip(&rig, "anterior_thigh.L").x,
					tip(&rig, "anterior_thigh.L").y,
					tip(&rig, "anterior_thigh.L").z,
				)?;
				eprintln!("{row}");
			}
		}
		Ok(())
	}
}
