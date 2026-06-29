use crozon_rigs::humanoid::HumanoidRig;

use crate::animations::{FixedPosition, FixedTuck};
use crate::rigs::humanoid::tuck::apply_tuck_profile;
use crate::{Animation, Effects};

impl<R: HumanoidRig> FixedPosition<R> for FixedTuck<R> {
	fn apply_fixed(&self, rig: &mut R) -> Effects {
		apply_tuck_profile(rig, &self.profile(), 1.0)
	}
}

impl<R: HumanoidRig> Animation<R> for FixedTuck<R> {
	fn apply(&self, rig: &mut R, _progress: f32) -> Effects {
		self.apply_fixed(rig)
	}
}

#[cfg(test)]
mod tests {
	use crozon_rigs::{humanoid::HumanoidRig, rigs::humanoid_v0::HumanoidV0Rig, Side};

	use super::*;
	use crate::animations::Tuck;

	#[test]
	fn fixed_tuck_ignores_progress() -> anyhow::Result<()> {
		let mut at_zero = HumanoidV0Rig::imported();
		let mut at_half = HumanoidV0Rig::imported();
		let fixed = FixedTuck::<HumanoidV0Rig>::default();

		fixed.apply(&mut at_zero, 0.0);
		fixed.apply(&mut at_half, 0.5);

		for bone in at_zero.animation_bones() {
			let Some(zero) = at_zero.pose().get(&bone) else {
				continue;
			};
			let half = at_half.pose().get(&bone).expect("matching bone");
			assert_eq!(zero.swing, half.swing, "swing drift on {bone}");
			assert_eq!(zero.flex, half.flex, "flex drift on {bone}");
			assert_eq!(zero.twist, half.twist, "twist drift on {bone}");
		}
		Ok(())
	}

	#[test]
	fn fixed_tuck_matches_full_tuck_sample() -> anyhow::Result<()> {
		let mut fixed = HumanoidV0Rig::imported();
		let mut ramped = HumanoidV0Rig::imported();
		FixedTuck::<HumanoidV0Rig>::default().apply_fixed(&mut fixed);
		Tuck::<HumanoidV0Rig>::default().apply(&mut ramped, 1.0);

		let fixed_shin = fixed.pose().get(&fixed.leg(Side::Left).shin.name).expect("shin");
		let ramped_shin = ramped.pose().get(&ramped.leg(Side::Left).shin.name).expect("shin");
		assert_eq!(fixed_shin.flex, ramped_shin.flex);
		Ok(())
	}
}
