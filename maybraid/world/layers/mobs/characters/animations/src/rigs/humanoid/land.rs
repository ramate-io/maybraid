use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::Land;
use crate::Animation;

impl Animation<HumanoidV0Rig> for Land {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let mut pose = HumanoidPose::default();
		pose.apply_leg(Side::Left, self.femur_swing(progress), self.shin_flex(progress));
		pose.apply_leg(Side::Right, self.femur_swing(progress), self.shin_flex(progress));
		pose.apply_root(self.root_swing(progress));

		for side in [Side::Left, Side::Right] {
			pose.apply_arm(
				side,
				0.0,
				self.shoulder_flex(side, progress),
				self.humerus_swing(side, progress),
				self.humerus_flex(side, progress),
				self.forearm_flex(progress),
			);
		}
		rig.write_pose(&pose);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::Animation;

	#[test]
	fn peak_compression_tucks_both_hands_from_the_bind_pose() -> anyhow::Result<()> {
		let land = Land::default();
		let peak = land.descent_duration() / land.cycle_duration();
		let rest = HumanoidV0Rig::for_clip_test();
		let mut rig = HumanoidV0Rig::for_clip_test();
		let effects = land.apply(&mut rig, peak);
		if !effects.is_identity() {
			return Err(anyhow::anyhow!("landing guard has no root offset"));
		}
		for side in ["L", "R"] {
			assert!(rig.posed_angle(&format!("humerus.{side}")) > 0.2);
			assert!(rig.posed_angle(&format!("forearm.{side}")) > 0.1);
			assert!(
				rig.rotation(&format!("humerus.{side}"))
					.dot(rest.rotation(&format!("humerus.{side}")))
					.abs() < 0.99,
				"the guard should move each humerus away from bind"
			);
		}
		Ok(())
	}
}
