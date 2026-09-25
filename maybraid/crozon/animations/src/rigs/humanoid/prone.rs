use crozon_rigs::{humanoid::HumanoidRig, Side};

use crate::animations::Prone;
use crate::rigs::humanoid::apply::{apply_arm, apply_leg, apply_neck, apply_root};
use crate::{Animation, Effects};

impl<R: HumanoidRig> Animation<R> for Prone<R> {
	fn apply_for(&self, rig: &mut R, progress: f32) {
		let femur = self.femur_swing(progress);
		let shin = self.shin_flex(progress);
		apply_leg(rig, Side::Left, femur, shin);
		apply_leg(rig, Side::Right, femur, shin);
		apply_root(rig, self.root_swing(progress));
		let neck = self.neck_swing(progress);
		apply_neck(rig, neck, 0.0, neck, 0.0);
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
		if root.swing.abs() < 0.5 {
			return Err(anyhow::anyhow!(
				"spine should pitch toward horizontal, got {}",
				root.swing
			));
		}
		Ok(())
	}
}
