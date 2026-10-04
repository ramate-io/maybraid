use bevy::prelude::Quat;
use character_rigs::authoring::ArmatureOffset;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::{FixedPosition, TuckedFlip};
use crate::{Animation, Effects};

impl Animation<HumanoidV0Rig> for TuckedFlip {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, _progress: f32) {
		let _ = self.tuck.apply_fixed(rig);
	}

	fn effects_for(&self, _rig: &HumanoidV0Rig, progress: f32) -> Effects {
		let pitch = self.pitch_radians(progress);
		if pitch.abs() > f32::EPSILON {
			ArmatureOffset::from_rotation(Quat::from_rotation_x(pitch))
		} else {
			ArmatureOffset::IDENTITY
		}
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;

	use super::*;
	use crate::animations::FlipDirection;

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	#[test]
	fn tucked_flip_returns_forward_pitch_effect() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let effects = TuckedFlip::default().apply(&mut rig, 0.25);
		assert!(effects.0.rotation.to_euler(bevy::prelude::EulerRot::XYZ).0 > 0.0);
		Ok(())
	}

	#[test]
	fn tucked_flip_applies_tuck_pose() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		TuckedFlip::default().apply(&mut rig, 0.5);

		let shin = tip(&rig, "shin.L");
		assert!(shin.z > 0.5, "knee flexes toward +Z, got {shin:?}");
		assert!(shin.y < 0.55, "bend passes one radian, got {shin:?}");
		Ok(())
	}

	#[test]
	fn backward_tucked_flip_pitches_negative() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let mut flip = TuckedFlip::default();
		flip.direction = FlipDirection::Backward;
		let effects = flip.apply(&mut rig, 0.25);
		assert!(effects.0.rotation.to_euler(bevy::prelude::EulerRot::XYZ).0 < 0.0);
		Ok(())
	}
}
