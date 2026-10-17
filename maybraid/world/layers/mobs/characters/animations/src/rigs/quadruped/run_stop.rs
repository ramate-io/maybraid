use character_rigs::rigs::quadruped_v0::QuadrupedV0Rig;

use crate::animations::QuadrupedRunStop;
use crate::rigs::quadruped::blend::blend_clips;
use crate::{Animation, Effects};

impl Animation<QuadrupedV0Rig> for QuadrupedRunStop {
	fn apply_for(&self, rig: &mut QuadrupedV0Rig, progress: f32) {
		let weight = self.weight(progress);
		blend_clips(rig, &self.run, 0.0, &self.idle, 0.0, weight);
	}

	fn effects_for(&self, _rig: &QuadrupedV0Rig, _progress: f32) -> Effects {
		Effects::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;
	use character_rigs::authoring::ArmatureOffset;

	use super::*;
	use crate::animations::{QuadrupedIdle, QuadrupedRun, Transition};

	const FOOT_SHINS: [&str; 4] =
		["anterior_shin.L", "anterior_shin.R", "posterior_shin.L", "posterior_shin.R"];

	fn shin_tip(rig: &QuadrupedV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	#[test]
	fn start_matches_run_phase_zero() -> anyhow::Result<()> {
		let stop = QuadrupedRunStop::default();
		let mut stop_rig = QuadrupedV0Rig::for_clip_test();
		let mut run_rig = QuadrupedV0Rig::for_clip_test();
		stop.apply(&mut stop_rig, 0.0);
		QuadrupedRun::default().apply(&mut run_rig, 0.0);

		for name in stop_rig.animation_bone_names() {
			let a = stop_rig.rotation(name);
			let b = run_rig.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "rotation mismatch on {name}");
		}
		Ok(())
	}

	#[test]
	fn end_matches_idle_neutral() -> anyhow::Result<()> {
		let stop = QuadrupedRunStop::default();
		let mut stop_rig = QuadrupedV0Rig::for_clip_test();
		let mut idle_rig = QuadrupedV0Rig::for_clip_test();
		stop.apply(&mut stop_rig, 1.0);
		QuadrupedIdle::default().apply(&mut idle_rig, 0.0);

		for name in stop_rig.animation_bone_names() {
			let a = stop_rig.rotation(name);
			let b = idle_rig.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "rotation mismatch on {name}");
		}
		Ok(())
	}

	#[test]
	fn mid_settle_advances_left_front_thigh() -> anyhow::Result<()> {
		let rest = QuadrupedV0Rig::for_clip_test();
		let mut rig = QuadrupedV0Rig::for_clip_test();
		QuadrupedRunStop::default().apply(&mut rig, 0.5);
		let run_angle = {
			let mut run = QuadrupedV0Rig::for_clip_test();
			QuadrupedRun::default().apply(&mut run, 0.0);
			run.posed_angle("anterior_thigh.L")
		};
		let idle_angle = rest.posed_angle("anterior_thigh.L");
		let mid = rig.posed_angle("anterior_thigh.L");
		assert!(mid < run_angle + 0.02, "mid should ease toward idle");
		assert!(mid > idle_angle - 0.02, "mid should stay above full rest");
		Ok(())
	}

	#[test]
	fn resampling_does_not_accumulate() -> anyhow::Result<()> {
		let mut once = QuadrupedV0Rig::for_clip_test();
		let mut twice = QuadrupedV0Rig::for_clip_test();
		let stop = QuadrupedRunStop::default();
		stop.apply(&mut once, 0.4);
		stop.apply(&mut twice, 0.4);
		stop.apply(&mut twice, 0.4);
		for name in once.animation_bone_names() {
			let a = once.rotation(name);
			let b = twice.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "re-sample must not drift on {name}");
		}
		Ok(())
	}

	#[test]
	fn transition_from_mid_trot_blends_into_mid_settle() -> anyhow::Result<()> {
		let mut rig = QuadrupedV0Rig::for_clip_test();
		QuadrupedRun::default().apply(&mut rig, 0.25);
		let from_pose = rig.pose.clone();
		let transition = Transition::from_visible(
			QuadrupedRunStop::default(),
			from_pose,
			ArmatureOffset::IDENTITY,
		);
		transition.apply_quadruped(&mut rig, 0.5, 0.5);

		let mut full = QuadrupedV0Rig::for_clip_test();
		QuadrupedRunStop::default().apply(&mut full, 0.5);
		assert!(
			rig.posed_angle("anterior_thigh.L") < full.posed_angle("anterior_thigh.L") + 0.02,
			"blend should stay below full settle"
		);
		let mut run = QuadrupedV0Rig::for_clip_test();
		QuadrupedRun::default().apply(&mut run, 0.25);
		assert!(
			rig.posed_angle("anterior_thigh.L") > run.posed_angle("anterior_thigh.L") - 0.02,
			"blend should stay near captured trot pose"
		);
		Ok(())
	}

	#[test]
	fn leap_interrupt_from_mid_settle_preserves_visible_pose() -> anyhow::Result<()> {
		let mut rig = QuadrupedV0Rig::for_clip_test();
		QuadrupedRunStop::default().apply(&mut rig, 0.4);
		let from_pose = rig.pose.clone();
		let transition = Transition::from_visible(
			QuadrupedRun::default(),
			from_pose.clone(),
			ArmatureOffset::IDENTITY,
		);
		transition.apply_quadruped(&mut rig, 0.0, 0.0);

		let mut captured = QuadrupedV0Rig::for_clip_test();
		captured.pose.copy_from(&from_pose);
		for name in ["anterior_thigh.L", "posterior_thigh.R"] {
			let a = rig.rotation(name);
			let b = captured.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "interrupt should keep visible pose on {name}");
		}
		Ok(())
	}

	#[test]
	fn midpoint_sits_between_run_contact_and_idle() -> anyhow::Result<()> {
		let mut mid = QuadrupedV0Rig::for_clip_test();
		let mut run = QuadrupedV0Rig::for_clip_test();
		let mut idle = QuadrupedV0Rig::for_clip_test();
		QuadrupedRunStop::default().apply(&mut mid, 0.5);
		QuadrupedRun::default().apply(&mut run, 0.0);
		QuadrupedIdle::default().apply(&mut idle, 0.0);

		assert!(mid.posed_angle("anterior_thigh.L") < run.posed_angle("anterior_thigh.L") + 0.02);
		assert!(mid.posed_angle("anterior_thigh.L") > idle.posed_angle("anterior_thigh.L") - 0.02);
		Ok(())
	}

	#[test]
	fn idle_endpoint_mirrors_diagonal_pairs() -> anyhow::Result<()> {
		let mut rig = QuadrupedV0Rig::for_clip_test();
		QuadrupedRunStop::default().apply(&mut rig, 1.0);
		assert!(
			(rig.posed_angle("anterior_thigh.L") - rig.posed_angle("anterior_thigh.R")).abs()
				< 1e-4,
			"idle end should mirror front thighs"
		);
		assert!(
			(rig.posed_angle("posterior_thigh.L") - rig.posed_angle("posterior_thigh.R")).abs()
				< 1e-4,
			"idle end should mirror hind thighs"
		);
		Ok(())
	}

	#[test]
	fn run_endpoint_diagonal_contact_offsets_feet() -> anyhow::Result<()> {
		let mut rig = QuadrupedV0Rig::for_clip_test();
		QuadrupedRunStop::default().apply(&mut rig, 0.0);
		let max_delta = FOOT_SHINS
			.iter()
			.map(|shin| {
				let mut idle = QuadrupedV0Rig::for_clip_test();
				QuadrupedIdle::default().apply(&mut idle, 0.0);
				(shin_tip(&rig, shin) - shin_tip(&idle, shin)).length()
			})
			.fold(0.0_f32, f32::max);
		assert!(max_delta > 0.02, "run@0 should offset feet from idle, delta {max_delta}");
		Ok(())
	}

	#[test]
	fn scaled_limb_proportions_stay_finite() -> anyhow::Result<()> {
		let mut rig = QuadrupedV0Rig::for_clip_test();
		rig.binding.effective_rest.local[1].translation *= Vec3::splat(1.25);
		QuadrupedRunStop::default().apply(&mut rig, 0.5);
		for name in rig.animation_bone_names() {
			let q = rig.rotation(name);
			assert!(q.x.is_finite() && q.y.is_finite() && q.z.is_finite() && q.w.is_finite());
		}
		Ok(())
	}

	#[test]
	fn unwritten_bones_stay_finite_at_mid_settle() -> anyhow::Result<()> {
		let mut rig = QuadrupedV0Rig::for_clip_test();
		QuadrupedRunStop::default().apply(&mut rig, 0.5);
		for name in rig.animation_bone_names() {
			let q = rig.rotation(name);
			assert!(q.x.is_finite() && q.y.is_finite() && q.z.is_finite() && q.w.is_finite());
		}
		Ok(())
	}
}
