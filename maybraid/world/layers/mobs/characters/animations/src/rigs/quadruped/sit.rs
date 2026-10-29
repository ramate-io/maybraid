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
		// Quadruped V0's imported sagittal axis is the inverse of the animation
		// author's intuitive "upright" direction: positive bend lifts the lumbar
		// and hind joints. A sit settles the rump, so this channel is negative.
		pose.spine_sagittal = -self.spine_upright * settle;
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
				-self.hind_stride * settle,
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
	use crate::rigs::quadruped::production_fixture::{point, rig as epiphant_rig};
	use bevy::prelude::Vec3;

	fn hind_tip(rig: &QuadrupedV0Rig, side: &str) -> anyhow::Result<Vec3> {
		point(rig, &format!("posterior_shin.{side}"), Vec3::Y * 0.5)
	}

	#[test]
	fn held_sit_lowers_the_rump_and_keeps_hind_support_near_the_stand_plane() -> anyhow::Result<()>
	{
		let sit = QuadrupedSit::held();
		let mut rest = epiphant_rig()?;
		let mut rig = epiphant_rig()?;
		sit.apply(&mut rest, 0.0);
		let effects = sit.apply(&mut rig, 1.0);
		for side in ["L", "R"] {
			let resting = hind_tip(&rest, side)?;
			let seated = hind_tip(&rig, side)?;
			assert!(
				(seated.y - resting.y).abs() < 0.12,
				"rear foot stays near support plane: {side} {seated:?}/{resting:?}"
			);
			let hip = format!("hip.{side}");
			assert!(
				point(&rig, &hip, Vec3::ZERO)?.y < point(&rest, &hip, Vec3::ZERO)?.y - 0.40,
				"rump lowers: {side}"
			);
			assert!(
				seated.z > resting.z + 0.15,
				"hind foot tucks forward beneath the rump: {side}"
			);
			let front = format!("anterior_shin.{side}");
			let front_before = point(&rest, &front, Vec3::Y * 0.5)?;
			let front_after = point(&rig, &front, Vec3::Y * 0.5)?;
			assert!(
				(front_after.y - front_before.y).abs() < 0.08,
				"front support height remains coherent: {side}"
			);
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
