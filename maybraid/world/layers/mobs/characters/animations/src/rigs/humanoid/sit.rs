use bevy::prelude::Vec3;
use character_rigs::authoring::{ArmatureOffset, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::Sit;
use crate::{Animation, Effects};

impl Sit {
	pub fn sample_pose(&self, progress: f32) -> HumanoidPose {
		let mut pose = HumanoidPose::default();
		for side in [Side::Left, Side::Right] {
			pose.apply_leg(side, self.femur_swing(), self.shin_flex());
			pose.apply_hip_fold(side, self.hip_peak);
			pose.leg_mut(side).hip_abduction = self.hip_spread * side.sign();
			apply_sit_arm(&mut pose, side, self);
		}
		pose.apply_spine_pitch(self.spine_pitch(progress));
		let breath = Self::breath(progress) * self.breath_amp;
		pose.apply_neck_twisted(0.0, 0.0, 0.35 * breath, 0.0, 0.0, 0.65 * breath);
		pose
	}
}

impl Animation<HumanoidV0Rig> for Sit {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		rig.write_pose(&self.sample_pose(progress));
	}

	fn effects_for(&self, rig: &HumanoidV0Rig, progress: f32) -> Effects {
		if self.bones_only {
			return ArmatureOffset::IDENTITY;
		}
		let drop = self.vertical_drop(progress, rig.segment_lengths);
		if drop.abs() > f32::EPSILON {
			ArmatureOffset::from_translation(Vec3::new(0.0, -drop, 0.0))
		} else {
			ArmatureOffset::IDENTITY
		}
	}
}

fn apply_sit_arm(pose: &mut HumanoidPose, side: Side, sit: &Sit) {
	let hang = -side.sign() * sit.arm_hang;
	let inward = -side.sign() * 0.12;
	pose.apply_arm(
		side,
		sit.shoulder_forward,
		inward,
		sit.shoulder_forward * 0.45,
		hang,
		-sit.elbow_hang,
	);
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::animations::sit::SitSurface;

	fn visual_point(rig: &HumanoidV0Rig, effects: Effects, name: &str) -> Vec3 {
		effects.0.rotation * rig.character_point(name) + effects.0.translation
	}

	#[test]
	fn ground_sit_folds_and_lowers_more_than_a_seat() -> anyhow::Result<()> {
		let mut ground = HumanoidV0Rig::for_clip_test();
		let mut seat = HumanoidV0Rig::for_clip_test();
		let ground_fx = Sit::ground().apply(&mut ground, 0.0);
		let seat_fx = Sit::seat().apply(&mut seat, 0.0);
		if ground.posed_angle("femur.L") <= 0.8 || ground.posed_angle("shin.L") <= 0.8 {
			anyhow::bail!("ground sit must visibly fold thigh and shin");
		}
		let ground_knee = visual_point(&ground, ground_fx, "shin.L");
		let seat_knee = visual_point(&seat, seat_fx, "shin.L");
		if ground_knee.y >= seat_knee.y - 0.04 {
			anyhow::bail!(
				"floor sit should lower the knee below chair sit: {ground_knee:?} {seat_knee:?}"
			);
		}
		if ground_fx.0.translation.y >= seat_fx.0.translation.y - 0.04 {
			anyhow::bail!("floor sit should use a larger visual drop");
		}
		if Sit::ground().surface != SitSurface::Ground || Sit::seat().surface != SitSurface::Seat {
			anyhow::bail!("surface parameter should match the selected sit");
		}
		Ok(())
	}

	#[test]
	fn seated_breath_moves_the_chest_without_changing_leg_fold() -> anyhow::Result<()> {
		let sit = Sit::ground();
		let mut start = HumanoidV0Rig::for_clip_test();
		let mut peak = HumanoidV0Rig::for_clip_test();
		sit.apply(&mut start, 0.0);
		sit.apply(&mut peak, 0.25 / crate::animations::sit::SIT_BREATH_FREQ);
		let start_chest = start.character_length("upper_back");
		let peak_chest = peak.character_length("upper_back");
		if (start_chest - peak_chest).length() < 0.02 {
			anyhow::bail!("breath should visibly change the composed chest axis");
		}
		if sit.sample_pose(0.0).legs
			!= sit.sample_pose(0.25 / crate::animations::sit::SIT_BREATH_FREQ).legs
		{
			anyhow::bail!("breathing must keep the seated leg fold fixed");
		}
		Ok(())
	}
}
