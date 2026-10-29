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
				self.hip_splay * settle * side.sign(),
				-self.hind_tuck * settle,
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

	fn effects_for(&self, rig: &QuadrupedV0Rig, progress: f32) -> Effects {
		let drop =
			self.body_lowering(progress) * (rig.segment_lengths.upper + rig.segment_lengths.lower);
		Effects::from_translation(bevy::prelude::Vec3::NEG_Y * drop)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::rigs::quadruped::production_fixture::{point, rig as epiphant_rig};
	use bevy::prelude::Vec3;

	#[test]
	fn sphinx_lowers_the_hindquarters_and_keeps_rear_support_near_standing_plane(
	) -> anyhow::Result<()> {
		let standing = epiphant_rig()?;
		let mut sphinx = epiphant_rig()?;
		let mut curled = epiphant_rig()?;
		let sphinx_fx = QuadrupedLieDown::held().apply(&mut sphinx, 1.0);
		let curled_fx = QuadrupedLieDown::curled().apply(&mut curled, 1.0);
		for (posed, effect, label) in [(&sphinx, sphinx_fx, "sphinx"), (&curled, curled_fx, "curl")]
		{
			assert!(effect.0.translation.y < -0.55, "{label} lowers the whole trunk");
			for side in ["L", "R"] {
				for prefix in ["anterior", "posterior"] {
					let bone = format!("{prefix}_shin.{side}");
					let before = point(&standing, &bone, Vec3::Y * 0.5)?;
					let after = point(posed, &bone, Vec3::Y * 0.5)? + effect.0.translation;
					assert!(
						(after.y - before.y).abs() < 0.20,
						"{label} {bone} remains near support: {after:?}/{before:?}"
					);
				}
				let hip = format!("hip.{side}");
				assert!(
					point(posed, &hip, Vec3::ZERO)?.y + effect.0.translation.y
						< point(&standing, &hip, Vec3::ZERO)?.y - 0.70,
					"{label} rearquarter settles deeply"
				);
			}
		}

		if (sphinx.rotation("lumbar") * bevy::prelude::Vec3::Y
			- curled.rotation("lumbar") * bevy::prelude::Vec3::Y)
			.length() <= 0.05
		{
			anyhow::bail!("curl should change lumbar articulation");
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
	#[test]
	fn rest_offset_closes_its_cycle_and_scales_with_measured_legs() -> anyhow::Result<()> {
		let mut rig = epiphant_rig()?;
		let cycle = QuadrupedLieDown::for_cycle();
		assert!(cycle.effects_for(&rig, 0.0).is_identity());
		assert!(cycle.effects_for(&rig, QuadrupedLieDown::PERIOD).is_identity());
		let depth = QuadrupedLieDown::held();
		let baseline = depth.effects_for(&rig, 1.0).0.translation.y;
		rig.segment_lengths.upper *= 1.2;
		rig.segment_lengths.lower *= 1.2;
		assert!((depth.effects_for(&rig, 1.0).0.translation.y - baseline * 1.2).abs() < 1e-6);
		Ok(())
	}
}
