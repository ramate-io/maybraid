use std::f32::consts::TAU;

use character_rigs::authoring::QuadrupedPose;
use character_rigs::rigs::quadruped_v0::QuadrupedV0Rig;
use character_rigs::Side;

use crate::animations::QuadrupedSit;
use crate::rigs::quadruped::apply::{
	apply_front_leg, apply_hind_leg, apply_neck_axes, apply_spine,
};
use crate::{Animation, Effects};

impl QuadrupedSit {
	pub fn sample_pose(&self, progress: f32) -> QuadrupedPose {
		let mut pose = QuadrupedPose::default();
		let settle = self.settle_weight(progress);
		let sway = if self.sway.abs() > f32::EPSILON {
			(TAU * (progress * Self::SWAY_FREQ)).sin() * self.sway
		} else {
			0.0
		};
		apply_neck_axes(
			&mut pose,
			sway * 0.45,
			sway * 0.55,
			self.neck_alert * settle + sway * 0.25,
		);
		apply_spine(&mut pose, sway * 0.35, self.spine_gather * settle + sway * 0.4);
		pose.spine_sagittal = self.spine_upright * settle;
		for side in [Side::Left, Side::Right] {
			apply_front_leg(
				&mut pose,
				side,
				0.0,
				0.0,
				-self.front_plant * settle,
				self.front_hinge * settle,
			);
			apply_hind_leg(
				&mut pose,
				side,
				0.0,
				self.hip_splay * settle * side.sign(),
				self.hind_stride * settle,
				self.hind_hinge * settle,
			);
		}
		pose
	}
}

impl Animation<QuadrupedV0Rig> for QuadrupedSit {
	fn apply_for(&self, rig: &mut QuadrupedV0Rig, progress: f32) {
		rig.write_pose(&self.sample_pose(progress));
	}

	fn effects_for(&self, _rig: &QuadrupedV0Rig, _progress: f32) -> Effects {
		Effects::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn held_sit_tucks_haunches_and_preserves_a_planted_front() -> anyhow::Result<()> {
		let sit = QuadrupedSit::held();
		let mut rig = QuadrupedV0Rig::for_clip_test();
		let effects = sit.apply(&mut rig, 1.0);
		if rig.posed_angle("posterior_thigh.L") <= rig.posed_angle("anterior_thigh.L") + 0.35 {
			anyhow::bail!("sit must tuck the hind thigh more than the front");
		}
		if rig.posed_angle("posterior_shin.L") <= rig.posed_angle("anterior_shin.L") + 0.45 {
			anyhow::bail!("sit must fold the hind shin more than the front");
		}
		if self::QuadrupedSit::sample_pose(&sit, 1.0).spine_sagittal <= 0.15 {
			anyhow::bail!("seated chest should stay lifted");
		}
		if !effects.is_identity() {
			anyhow::bail!("quadruped sit articulation should not offset the root");
		}
		Ok(())
	}

	#[test]
	fn sit_down_cycle_reaches_a_hold_and_returns_to_stand() -> anyhow::Result<()> {
		let sit = QuadrupedSit::for_cycle();
		let mut stand = QuadrupedV0Rig::for_clip_test();
		let mut held = QuadrupedV0Rig::for_clip_test();
		sit.apply(&mut stand, 0.0);
		sit.apply(&mut held, QuadrupedSit::sit_peak());
		if stand.posed_angle("posterior_shin.L") > 0.08
			|| held.posed_angle("posterior_shin.L") <= 0.55
		{
			anyhow::bail!("cycle should open at stand and reach a seated hind fold");
		}
		let mut end = QuadrupedV0Rig::for_clip_test();
		sit.apply(&mut end, QuadrupedSit::PERIOD);
		if end.posed_angle("posterior_shin.L") > 0.08 {
			anyhow::bail!("cycle should close at stand");
		}
		Ok(())
	}
}
