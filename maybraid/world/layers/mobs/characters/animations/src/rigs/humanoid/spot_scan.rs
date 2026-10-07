//! Humanoid mapping for [`SpotScan`](crate::animations::SpotScan).

use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::SpotScan;
use crate::rigs::humanoid::apply::{apply_leg, apply_neck_twisted};
use crate::Animation;

impl Animation<HumanoidV0Rig> for SpotScan {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let mut pose = HumanoidPose::default();
		let torso = self.torso_turn(progress);
		let neck = self.neck_turn(progress);
		let knee = self.knee_flex(progress);
		let shift = self.pelvis_shift(progress);

		pose.spine.add_turn(torso);
		pose.spine.add_root_forward(self.forward_lean(progress));
		apply_neck_twisted(
			&mut pose,
			neck * 0.6,
			0.0,
			self.head_dip(progress),
			neck * 0.4,
			0.0,
			0.0,
		);

		apply_leg(&mut pose, Side::Left, knee * 0.35, knee);
		apply_leg(&mut pose, Side::Right, knee * 0.35, knee);
		pose.leg_mut(Side::Left).pelvis_lateral += shift;
		pose.leg_mut(Side::Right).pelvis_lateral -= shift;

		rig.write_pose(&pose);
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;

	use super::*;

	fn forward(rig: &HumanoidV0Rig, bone: &str) -> Vec3 {
		rig.rotation(bone) * Vec3::Z
	}

	#[test]
	fn spot_scan_turns_the_neck_left_then_right() {
		let scan = SpotScan;
		let rest = HumanoidV0Rig::for_clip_test();
		let mut left = HumanoidV0Rig::for_clip_test();
		let mut right = HumanoidV0Rig::for_clip_test();
		scan.apply(&mut left, SpotScan::left_peak());
		scan.apply(&mut right, SpotScan::right_peak());

		let rest_fwd = forward(&rest, "upper_neck");
		let left_fwd = forward(&left, "upper_neck");
		let right_fwd = forward(&right, "upper_neck");
		assert!(
			left_fwd.x < rest_fwd.x - 0.08,
			"neck yaws left (+X), {left_fwd:?} vs {rest_fwd:?}"
		);
		assert!(
			right_fwd.x > rest_fwd.x + 0.08,
			"neck yaws right (-X), {right_fwd:?} vs {rest_fwd:?}"
		);
	}

	#[test]
	fn spot_scan_torso_follows_the_neck() {
		let scan = SpotScan;
		let mut rig = HumanoidV0Rig::for_clip_test();
		scan.apply(&mut rig, SpotScan::left_peak());

		let upper = forward(&rig, "upper_back");
		let neck = forward(&rig, "upper_neck");
		assert!(upper.x.abs() > 0.02, "torso participates, {upper:?}");
		assert!(neck.x.abs() > upper.x.abs(), "neck leads torso, {neck:?} vs {upper:?}");
	}

	#[test]
	fn spot_scan_bends_knees_during_the_sweep() {
		let scan = SpotScan;
		let rest = HumanoidV0Rig::for_clip_test();
		let mut peak = HumanoidV0Rig::for_clip_test();
		scan.apply(&mut peak, SpotScan::right_peak());

		assert!(
			peak.posed_angle("shin.L") > rest.posed_angle("shin.L") + 0.04,
			"knees flex during alert scan"
		);
		assert!(
			(peak.posed_angle("shin.L") - peak.posed_angle("shin.R")).abs() < 1e-4,
			"both knees share flexion"
		);
	}

	#[test]
	fn spot_scan_unwritten_arms_stay_at_rest() {
		let scan = SpotScan;
		let rest = HumanoidV0Rig::for_clip_test();
		let mut posed = HumanoidV0Rig::for_clip_test();
		scan.apply(&mut posed, SpotScan::right_peak());
		for name in ["humerus.L", "humerus.R", "forearm.L", "forearm.R", "shoulder.L", "shoulder.R"] {
			let a = rest.rotation(name);
			let b = posed.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "{name} should stay at rest");
		}
	}

	#[test]
	fn spot_scan_recovers_to_rest() {
		let scan = SpotScan;
		let rest = HumanoidV0Rig::for_clip_test();
		let mut end = HumanoidV0Rig::for_clip_test();
		scan.apply(&mut end, 1.0);
		let rest_fwd = forward(&rest, "upper_neck");
		let end_fwd = forward(&end, "upper_neck");
		assert!(
			(end_fwd - rest_fwd).length() < 0.08,
			"neck returns near rest, {end_fwd:?} vs {rest_fwd:?}"
		);
	}

	#[test]
	fn spot_scan_resampling_does_not_accumulate() {
		let scan = SpotScan;
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		scan.apply(&mut once, 0.5);
		scan.apply(&mut twice, 0.5);
		scan.apply(&mut twice, 0.5);
		for name in once.animation_bone_names() {
			let a = once.rotation(name);
			let b = twice.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "re-sample must not drift on {name}");
		}
	}

	#[test]
	fn spot_scan_transition_from_idle_blends_mid_sweep() {
		use crate::animations::{Idle, Transition};
		use character_rigs::authoring::ArmatureOffset;

		let scan = SpotScan;
		let mut idle_rig = HumanoidV0Rig::for_clip_test();
		let mut full_rig = HumanoidV0Rig::for_clip_test();
		let mut blended_rig = HumanoidV0Rig::for_clip_test();
		Idle::default().apply(&mut idle_rig, 0.0);
		scan.apply(&mut full_rig, SpotScan::left_peak());
		let from_pose = idle_rig.pose.clone();
		let transition = Transition::from_visible(scan, from_pose, ArmatureOffset::IDENTITY);
		transition.apply(&mut blended_rig, SpotScan::left_peak(), 0.5);

		let idle_fwd = forward(&idle_rig, "upper_neck");
		let full_fwd = forward(&full_rig, "upper_neck");
		let blended_fwd = forward(&blended_rig, "upper_neck");
		assert!(
			blended_fwd.x < idle_fwd.x - 0.02 && blended_fwd.x.abs() < full_fwd.x.abs(),
			"blend lands between idle and left scan, {blended_fwd:?} vs {idle_fwd:?} {full_fwd:?}"
		);
	}

	#[test]
	fn spot_scan_interruption_holds_partial_turn() {
		let scan = SpotScan;
		let mut rig = HumanoidV0Rig::for_clip_test();
		scan.apply(&mut rig, 0.2);
		let partial = forward(&rig, "upper_neck");
		scan.apply(&mut rig, 0.0);
		let reset = forward(&rig, "upper_neck");
		assert!(
			(partial.x.abs() - reset.x.abs()) > 0.03,
			"mid-scan differs from rest, {partial:?} vs {reset:?}"
		);
	}
}
