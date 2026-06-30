use bevy::prelude::Vec3;
use crozon_rigs::{humanoid::HumanoidRig, RiggedAxis, Side};

use crate::animations::{Tuck, TuckProfile};
use crate::rigs::humanoid::apply::apply_leg;
use crate::{Animation, Effects};

/// Humerus tuck: swing/flex/twist on Y / X / Y (twist is long-axis spin for forearm aim).
fn humerus_tuck_axis(side: Side) -> RiggedAxis {
	match side {
		Side::Left => RiggedAxis { swing_axis: Vec3::Y, flex_axis: Vec3::X, twist_axis: Vec3::Y },
		Side::Right => {
			RiggedAxis { swing_axis: Vec3::Y, flex_axis: Vec3::NEG_X, twist_axis: Vec3::Y }
		}
	}
}

/// Apply tuck articulation scaled by `amount` in `[0.0, 1.0]`.
pub fn apply_tuck_profile<R: HumanoidRig>(
	rig: &mut R,
	profile: &TuckProfile,
	amount: f32,
) -> Effects {
	apply_leg(rig, Side::Left, profile.femur_swing(amount), profile.shin_flex(amount));
	apply_leg(rig, Side::Right, profile.femur_swing(amount), profile.shin_flex(amount));

	for side in [Side::Left, Side::Right] {
		let mut arm = rig.arm_pose(side);

		arm.shoulder =
			rig.articulate_on_rig(arm.shoulder, profile.shoulder_roll(side, amount), 0.0);
		arm.humerus = arm.humerus.articulate(
			humerus_tuck_axis(side),
			profile.humerus_swing(side, amount),
			profile.humerus_flex(side, amount),
			profile.humerus_twist(side, amount),
		);
		arm.forearm = rig.articulate_on_rig(arm.forearm, 0.0, profile.forearm_flex(amount));
		rig.pose_arm(arm);
	}

	Effects::default()
}

impl<R: HumanoidRig> Animation<R> for Tuck<R> {
	fn apply(&self, rig: &mut R, progress: f32) -> Effects {
		apply_tuck_profile(rig, &self.profile(), self.tuck_amount(progress))
	}
}

#[cfg(test)]
mod tests {
	use crozon_rigs::{rigs::humanoid_v0::HumanoidV0Rig, Side};

	use super::*;

	#[test]
	fn tuck_bends_knees_on_rig() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		Tuck::<HumanoidV0Rig>::default().apply(&mut rig, 0.5);

		let shin = rig.pose().get(&rig.leg(Side::Left).shin.name).expect("shin");
		assert!(shin.flex > 1.0);
		Ok(())
	}

	#[test]
	fn tuck_drives_humerus_swing_flex_and_twist() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let tuck = Tuck::<HumanoidV0Rig>::default();
		tuck.apply(&mut rig, 0.5);

		let profile = tuck.profile();
		let humerus = rig.pose().get(&rig.arm(Side::Left).humerus.name).expect("humerus");
		let forearm = rig.pose().get(&rig.arm(Side::Left).forearm.name).expect("forearm");
		assert!(humerus.swing.abs() > 0.05);
		assert!(humerus.flex.abs() > 0.05);
		assert!(humerus.twist.abs() > 0.05);
		assert!(humerus.swing.abs() <= profile.humerus_swing(Side::Left, 1.0).abs() + 1e-4);
		assert!(humerus.flex.abs() <= profile.humerus_flex(Side::Left, 1.0).abs() + 1e-4);
		assert!(humerus.twist.abs() <= profile.humerus_twist(Side::Left, 1.0).abs() + 1e-4);
		assert!(forearm.flex.abs() > 0.05);
		Ok(())
	}

	#[test]
	fn tuck_shoulders_roll_inward_symmetrically() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		Tuck::<HumanoidV0Rig>::default().apply(&mut rig, 1.0);

		let left = rig.pose().get(&rig.arm(Side::Left).shoulder.name).expect("left shoulder");
		let right = rig.pose().get(&rig.arm(Side::Right).shoulder.name).expect("right shoulder");
		assert!(left.swing.abs() > 0.1);
		assert!(right.swing.abs() > 0.1);
		assert!(left.swing.signum() != right.swing.signum());
		assert!(left.flex.abs() < 1e-4);
		assert!(right.flex.abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn tuck_humerus_twist_dominates_forearm_hinge() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		Tuck::<HumanoidV0Rig>::default().apply(&mut rig, 1.0);

		let humerus = rig.pose().get(&rig.arm(Side::Left).humerus.name).expect("humerus");
		let forearm = rig.pose().get(&rig.arm(Side::Left).forearm.name).expect("forearm");
		assert!(humerus.twist.abs() > forearm.flex.abs());
		Ok(())
	}
}
