use bevy::prelude::{Transform, Vec3};
use crozon_rigs::{humanoid::HumanoidRig, Side};

use crate::animations::Squat;
use crate::rigs::humanoid::apply::{apply_leg, apply_root};
use crate::{Animation, Effects};

impl<R: HumanoidRig> Animation<R> for Squat<R> {
	fn apply_for(&self, rig: &mut R, progress: f32) {
		let femur_swing = self.femur_swing(progress);
		let shin_flex = self.shin_flex(progress);

		apply_leg(rig, Side::Left, femur_swing, shin_flex);
		apply_leg(rig, Side::Right, femur_swing, shin_flex);
		apply_root(rig, self.root_swing(progress));
	}

	fn effects_for(&self, rig: &R, progress: f32) -> Effects {
		let drop = self.vertical_drop(progress, rig.segment_lengths());
		Effects {
			r#move: (drop > f32::EPSILON)
				.then(|| Transform::from_translation(Vec3::new(0.0, -drop, 0.0))),
		}
	}
}

#[cfg(test)]
mod tests {
	use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

	use bevy::prelude::{Transform, Vec3};
	use crozon_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	use super::*;

	#[test]
	fn stand_phase_keeps_pose_neutral() {
		let mut rig = HumanoidV0Rig::imported();
		let squat = Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0);
		let effects = squat.apply(&mut rig, 0.0);

		let femur = rig.pose().get(&rig.leg(Side::Left).femur.name).expect("left femur pose");
		let shin = rig.pose().get(&rig.leg(Side::Left).shin.name).expect("left shin pose");
		let root = rig.pose().get(&rig.spine().root.name).expect("root pose");
		assert_eq!(femur.swing, 0.0);
		assert_eq!(shin.flex, 0.0);
		assert_eq!(root.swing, 0.0);
		assert_eq!(femur.transform.translation, Vec3::ZERO);
		assert!(effects.r#move.is_none());
	}

	#[test]
	fn deepest_squat_matches_blender_reference_angles() {
		let mut rig = HumanoidV0Rig::imported();
		Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0).apply(&mut rig, 0.5);

		let femur = rig.pose().get(&rig.leg(Side::Left).femur.name).expect("left femur pose");
		let shin = rig.pose().get(&rig.leg(Side::Left).shin.name).expect("left shin pose");
		let root = rig.pose().get(&rig.spine().root.name).expect("root pose");

		let right_femur =
			rig.pose().get(&rig.leg(Side::Right).femur.name).expect("right femur pose");

		assert!((femur.swing + FRAC_PI_4).abs() < 1e-5);
		assert!((right_femur.swing + FRAC_PI_4).abs() < 1e-5);
		assert!((shin.flex - FRAC_PI_2).abs() < 1e-5);
		assert!((root.swing - 15.0_f32.to_radians()).abs() < 1e-5);
	}

	#[test]
	fn deepest_squat_returns_armature_drop_without_bone_translation() {
		let mut rig = HumanoidV0Rig::imported();
		rig.pose.insert(crozon_rigs::BonePose::new(
			rig.leg(Side::Left).femur.name.clone(),
			Transform::from_translation(Vec3::new(0.0, 0.25, 0.0)),
		));

		let squat = Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0);
		let effects = squat.apply(&mut rig, 0.5);

		let drop = squat.vertical_drop(0.5, rig.segment_lengths());
		assert!(drop > 0.0);
		assert_eq!(effects.r#move, Some(Transform::from_translation(Vec3::new(0.0, -drop, 0.0))));

		let femur = rig.pose().get(&rig.leg(Side::Left).femur.name).expect("left femur");
		assert_eq!(femur.transform.translation, Vec3::new(0.0, 0.25, 0.0));
	}
}
