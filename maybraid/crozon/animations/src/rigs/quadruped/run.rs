use crozon_rigs::{quadruped::QuadrupedRig, Side};

use crate::animations::{QuadrupedRun, QuadrupedRunPose};
use crate::rigs::quadruped::apply::{apply_neck, apply_spine};
use crate::rigs::quadruped::gait::{
	apply_front_leg_at_strike, apply_hind_leg_at_strike, thigh_swing, KneeTuning, LegStrideTuning,
};
use crate::{Animation, Progress};

impl<R: QuadrupedRig> Animation<R> for QuadrupedRun {
	fn apply_for(&self, rig: &mut R, progress: f32) {
		QuadrupedRunPose::from_run(self).apply_for(rig, progress)
	}
}

impl<R: QuadrupedRig> Animation<R> for QuadrupedRunPose<R> {
	fn apply_for(&self, rig: &mut R, progress: f32) {
		let cycle = Progress(progress).cycle();
		let tuning = leg_tuning(self);

		// Diagonal trot: FL, HR, FR, HL.
		apply_front_leg_at_strike(rig, Side::Left, cycle, 0.0, tuning);
		apply_hind_leg_at_strike(rig, Side::Right, cycle, 0.25, tuning);
		apply_front_leg_at_strike(rig, Side::Right, cycle, 0.5, tuning);
		apply_hind_leg_at_strike(rig, Side::Left, cycle, 0.75, tuning);

		let spine_swing = thigh_swing(cycle) * self.spine_swing;
		apply_spine(rig, spine_swing, -spine_swing * 0.5);
		apply_neck(rig, -spine_swing * self.neck_swing / self.spine_swing.max(1e-4));
	}
}

fn leg_tuning<Rig>(run: &QuadrupedRunPose<Rig>) -> LegStrideTuning {
	LegStrideTuning {
		shoulder_swing: run.shoulder_swing,
		shoulder_lift: run.shoulder_lift,
		hip_swing: run.hip_swing,
		hip_lift: run.hip_lift,
		stride: run.stride,
		knee: KneeTuning {
			knee_neutral: run.knee_neutral,
			knee_contracted: run.knee_contracted,
			knee_extended: run.knee_extended,
		},
	}
}

#[cfg(test)]
mod tests {
	use crozon_rigs::{rigs::quadruped_v0::QuadrupedV0Rig, Side};

	use super::*;

	fn assert_pose_matches_at_phases(phases: &[f32]) {
		for &phase in phases {
			let mut from_run = QuadrupedV0Rig::imported();
			let mut from_pose = QuadrupedV0Rig::imported();
			QuadrupedRun::default().apply(&mut from_run, phase);
			QuadrupedRunPose::<QuadrupedV0Rig>::default().apply(&mut from_pose, phase);

			for bone in from_run.animation_bones() {
				let Some(run_pose) = from_run.pose().get(&bone) else {
					continue;
				};
				let pose = from_pose.pose().get(&bone).expect("pose");
				assert!(
					(run_pose.swing - pose.swing).abs() < 1e-5,
					"swing mismatch on {bone} at {phase}"
				);
				assert!(
					(run_pose.flex - pose.flex).abs() < 1e-5,
					"flex mismatch on {bone} at {phase}"
				);
			}
		}
	}

	#[test]
	fn quadruped_run_delegates_to_pose_default() {
		assert_pose_matches_at_phases(&[0.0, 0.25, 0.5, 0.75]);
	}

	#[test]
	fn quadruped_run_writes_swing_flex_for_left_front_thigh() {
		let mut rig = QuadrupedV0Rig::imported();
		QuadrupedRunPose::<QuadrupedV0Rig>::default().apply(&mut rig, 0.0);

		let thigh =
			rig.pose().get(&rig.front_leg(Side::Left).thigh.name).expect("front thigh pose");
		assert!(thigh.swing.abs() > 0.0);
	}

	#[test]
	fn quadruped_run_offsets_legs_across_stride() {
		let mut rig = QuadrupedV0Rig::imported();
		QuadrupedRunPose::<QuadrupedV0Rig>::default().apply(&mut rig, 0.0);

		let front_left =
			rig.pose().get(&rig.front_leg(Side::Left).thigh.name).expect("front left thigh");
		let hind_right =
			rig.pose().get(&rig.hind_leg(Side::Right).thigh.name).expect("hind right thigh");
		assert_ne!(front_left.swing, hind_right.swing);
	}
}
