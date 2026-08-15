use bevy::prelude::{Quat, Transform};
use crozon_rigs::humanoid::HumanoidRig;

use crate::animations::{FixedPosition, TuckedFlip};
use crate::{Animation, Effects};

impl<R: HumanoidRig> Animation<R> for TuckedFlip<R> {
	fn apply_for(&self, rig: &mut R, _progress: f32) {
		let _ = self.tuck.apply_fixed(rig);
	}

	fn effects_for(&self, _rig: &R, progress: f32) -> Effects {
		let pitch = self.pitch_radians(progress);
		Effects {
			r#move: (pitch.abs() > f32::EPSILON)
				.then(|| Transform::from_rotation(Quat::from_rotation_x(pitch))),
		}
	}
}

#[cfg(test)]
mod tests {
	use crozon_rigs::{rigs::humanoid_v0::HumanoidV0Rig, Side};

	use super::*;
	use crate::animations::FlipDirection;

	#[test]
	fn tucked_flip_returns_forward_pitch_effect() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let effects = TuckedFlip::<HumanoidV0Rig>::default().apply(&mut rig, 0.25);
		let offset = effects.r#move.expect("rotation effect");
		assert!(offset.rotation.to_euler(bevy::prelude::EulerRot::XYZ).0 > 0.0);
		Ok(())
	}

	#[test]
	fn tucked_flip_applies_tuck_pose() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		TuckedFlip::<HumanoidV0Rig>::default().apply(&mut rig, 0.5);

		let shin = rig.pose().get(&rig.leg(Side::Left).shin.name).expect("shin");
		assert!(shin.flex > 1.0);
		Ok(())
	}

	#[test]
	fn backward_tucked_flip_pitches_negative() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let mut flip = TuckedFlip::<HumanoidV0Rig>::default();
		flip.direction = FlipDirection::Backward;
		let effects = flip.apply(&mut rig, 0.25);
		let offset = effects.r#move.expect("rotation effect");
		assert!(offset.rotation.to_euler(bevy::prelude::EulerRot::XYZ).0 < 0.0);
		Ok(())
	}
}
