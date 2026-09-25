use crozon_rigs::{humanoid::HumanoidRig, Side};

use crate::animations::Prone;
use crate::rigs::humanoid::apply::{
	apply_arm, apply_hip_fold, apply_leg, apply_neck_twisted, apply_spine_pitch,
};
use crate::{Animation, Effects};

impl<R: HumanoidRig> Animation<R> for Prone<R> {
	fn apply_for(&self, rig: &mut R, progress: f32) {
		let femur = self.femur_swing(progress);
		let shin = self.shin_flex(progress);
		apply_leg(rig, Side::Left, femur, shin);
		apply_leg(rig, Side::Right, femur, shin);
		apply_hip_fold(rig, Side::Left, femur * 0.35);
		apply_hip_fold(rig, Side::Right, femur * 0.35);
		let pitch = self.spine_pitch(progress);
		apply_spine_pitch(rig, pitch);
		let neck = self.neck_swing(progress);
		apply_neck_twisted(rig, 0.0, 0.0, neck, 0.0, 0.0, neck);
		let hold = self.arm_hold(progress);
		apply_arm(rig, Side::Left, 0.0, 0.0, 0.0, hold, hold);
		apply_arm(rig, Side::Right, 0.0, 0.0, 0.0, hold, hold);
	}

	fn effects_for(&self, _rig: &R, _progress: f32) -> Effects {
		Effects::default()
	}
}

#[cfg(test)]
mod tests {
	use crozon_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	use super::*;

	#[test]
	fn settled_prone_pitches_the_spine() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let prone = Prone::<HumanoidV0Rig>::default();
		let effects = prone.apply(&mut rig, 1.0);
		if effects.r#move.is_some() {
			return Err(anyhow::anyhow!("held prone must not Effects.move"));
		}
		let root = rig.pose().get(&rig.spine().root.name).ok_or_else(|| anyhow::anyhow!("root"))?;
		if root.twist.abs() < 0.3 {
			return Err(anyhow::anyhow!(
				"root should pitch toward horizontal, got twist {}",
				root.twist
			));
		}
		let lumbar = rig
			.pose()
			.get(&rig.spine().lumbar.name)
			.ok_or_else(|| anyhow::anyhow!("lumbar"))?;
		if lumbar.twist.abs() < 0.2 {
			return Err(anyhow::anyhow!(
				"lumbar should share the fold, got twist {}",
				lumbar.twist
			));
		}
		Ok(())
	}
}
