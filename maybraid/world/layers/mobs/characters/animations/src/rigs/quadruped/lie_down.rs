use std::f32::consts::TAU;

use character_rigs::authoring::QuadrupedPose;
use character_rigs::rigs::quadruped_v0::QuadrupedV0Rig;
use character_rigs::Side;

use crate::animations::QuadrupedLieDown;
use crate::rigs::quadruped::apply::{
	apply_front_leg, apply_hind_leg, apply_neck_axes, apply_spine,
};
use crate::{Animation, Effects};

impl QuadrupedLieDown {
	pub fn sample_pose(&self, progress: f32) -> QuadrupedPose {
		let mut pose = QuadrupedPose::default();
		let settle = self.settle_weight(progress);
		let curl = self.curl.clamp(0.0, 1.0) * settle;
		let breath = if self.breath.abs() > f32::EPSILON {
			(TAU * (progress * Self::BREATH_FREQ)).sin()
				* self.breath
				* self.breath_weight(progress)
		} else {
			0.0
		};
		apply_neck_axes(
			&mut pose,
			-curl * 0.28 + breath * 0.15,
			curl * 0.18,
			-self.neck_rest * settle + breath * 0.35,
		);
		apply_spine(&mut pose, curl * 0.12, self.spine_settle * 0.18 * settle + curl * 0.55);
		// Positive quadruped sagittal bend raises the lumbar/rear half of the
		// production Epiphant bind. Resting lowers the trunk toward the support
		// plane, and breathing moves around that settled pose.
		pose.spine_sagittal = -self.spine_settle * settle - breath;

		for side in [Side::Left, Side::Right] {
			let curl_side = match side {
				Side::Left => 1.0 - curl * 0.35,
				Side::Right => 1.0 + curl * 0.20,
			};
			apply_front_leg(
				&mut pose,
				side,
				0.0,
				curl * 0.08 * side.sign(),
				self.front_reach * settle,
				self.front_hinge * settle,
			);
			apply_hind_leg(
				&mut pose,
				side,
				0.0,
				self.hip_splay * settle * side.sign() * curl_side,
				-self.hind_tuck * settle * curl_side,
				self.hind_hinge * settle,
			);
		}
		pose
	}
}

impl Animation<QuadrupedV0Rig> for QuadrupedLieDown {
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
	fn sphinx_lowers_the_hindquarters_and_keeps_rear_support_near_standing_plane(
	) -> anyhow::Result<()> {
		let standing = epiphant_rig()?;
		let mut sphinx = epiphant_rig()?;
		let mut curled = epiphant_rig()?;
		let sphinx_fx = QuadrupedLieDown::held().apply(&mut sphinx, 1.0);
		QuadrupedLieDown::curled().apply(&mut curled, 1.0);
		for side in ["L", "R"] {
			let before = hind_tip(&standing, side)?;
			let after = hind_tip(&sphinx, side)?;
			assert!(
				(after.y - before.y).abs() < 0.18,
				"rear support stays near plane: {side} {after:?}/{before:?}"
			);
			assert!(after.z > before.z + 0.15, "rear foot tucks under belly: {side}");
			let hip = format!("hip.{side}");
			assert!(
				point(&sphinx, &hip, Vec3::ZERO)?.y < point(&standing, &hip, Vec3::ZERO)?.y - 0.1,
				"rump settles: {side}"
			);
		}
		if sphinx.posed_angle("anterior_shin.L") <= 0.6 {
			anyhow::bail!("sphinx should visibly fold the front shin");
		}
		if (sphinx.rotation("lumbar") * bevy::prelude::Vec3::Y
			- curled.rotation("lumbar") * bevy::prelude::Vec3::Y)
			.length() <= 0.05
		{
			anyhow::bail!("curl should change lumbar articulation");
		}
		if !sphinx_fx.is_identity() {
			anyhow::bail!("lie-down should keep root translation with physics");
		}
		Ok(())
	}

	#[test]
	fn resting_breath_moves_the_lumbar_at_frame_rate() -> anyhow::Result<()> {
		let lie = QuadrupedLieDown::for_rest();
		let mut start = QuadrupedV0Rig::for_clip_test();
		let mut peak = QuadrupedV0Rig::for_clip_test();
		lie.apply(&mut start, 0.0);
		lie.apply(&mut peak, 0.25 / QuadrupedLieDown::BREATH_FREQ);
		if (start.rotation("lumbar") * bevy::prelude::Vec3::Y
			- peak.rotation("lumbar") * bevy::prelude::Vec3::Y)
			.length() <= 0.01
		{
			anyhow::bail!("rest breath should move the composed lumbar at its peak");
		}
		Ok(())
	}
}
