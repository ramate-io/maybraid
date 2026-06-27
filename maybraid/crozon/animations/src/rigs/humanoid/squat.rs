use bevy::prelude::Quat;
use crozon_rigs::{humanoid::HumanoidRig, BonePose, Side};

use crate::{animations::Squat, Animation};

impl<R: HumanoidRig> Animation<R> for Squat<R> {
	fn apply(&self, rig: &mut R) {
		let femur_swing = self.femur_swing();
		let shin_flex = self.shin_flex();

		apply_leg(rig, Side::Left, femur_swing, shin_flex);
		apply_leg(rig, Side::Right, femur_swing, shin_flex);
		apply_root(rig, self.root_swing());
	}
}

fn apply_leg<R: HumanoidRig>(rig: &mut R, side: Side, femur_swing: f32, shin_flex: f32) {
	let mut leg = rig.leg_pose(side);

	leg.femur = articulated(leg.femur, femur_swing, 0.0);
	leg.shin = articulated(leg.shin, 0.0, shin_flex);
	rig.pose_leg(leg);
}

fn apply_root<R: HumanoidRig>(rig: &mut R, root_swing: f32) {
	let mut spine = rig.spine_pose();
	spine.root = articulated(spine.root, root_swing, 0.0);
	rig.pose_spine(spine);
}

fn articulated(mut bone: BonePose, swing: f32, flex: f32) -> BonePose {
	bone.swing = swing;
	bone.flex = flex;
	bone.transform.rotation = Quat::from_rotation_x(swing + flex) * bone.transform.rotation;
	bone
}

#[cfg(test)]
mod tests {
	use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

	use bevy::prelude::Vec3;
	use crozon_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	use super::*;

	#[test]
	fn stand_phase_keeps_pose_neutral() {
		let mut rig = HumanoidV0Rig::imported();
		Squat::<HumanoidV0Rig>::new(0.0).apply(&mut rig);

		let femur = rig.pose().get(&rig.leg(Side::Left).femur.name).expect("left femur pose");
		let shin = rig.pose().get(&rig.leg(Side::Left).shin.name).expect("left shin pose");
		let root = rig.pose().get(&rig.spine().root.name).expect("root pose");
		assert_eq!(femur.swing, 0.0);
		assert_eq!(shin.flex, 0.0);
		assert_eq!(root.swing, 0.0);
		assert_eq!(femur.transform.translation, Vec3::ZERO);
	}

	#[test]
	fn deepest_squat_matches_blender_reference_angles() {
		let mut rig = HumanoidV0Rig::imported();
		Squat::<HumanoidV0Rig>::new(0.5).apply(&mut rig);

		let femur = rig.pose().get(&rig.leg(Side::Left).femur.name).expect("left femur pose");
		let shin = rig.pose().get(&rig.leg(Side::Left).shin.name).expect("left shin pose");
		let root = rig.pose().get(&rig.spine().root.name).expect("root pose");

		assert!((femur.swing + FRAC_PI_4).abs() < 1e-5);
		assert!((shin.flex - FRAC_PI_2).abs() < 1e-5);
		assert!((root.swing - 15.0_f32.to_radians()).abs() < 1e-5);
	}

	#[test]
	fn deepest_squat_leaves_translations_neutral_for_now() {
		let mut rig = HumanoidV0Rig::imported();
		Squat::<HumanoidV0Rig>::new(0.5).apply(&mut rig);

		for bone in [
			rig.leg(Side::Left).femur.name,
			rig.leg(Side::Left).shin.name,
			rig.leg(Side::Right).femur.name,
			rig.leg(Side::Right).shin.name,
			rig.spine().root.name,
		] {
			let pose = rig.pose().get(&bone).unwrap_or_else(|| panic!("missing pose for {bone}"));
			assert_eq!(pose.transform.translation, Vec3::ZERO, "unexpected translation on {bone}");
		}
	}
}
