use bevy::prelude::{Quat, Vec3};
use character_rigs::authoring::{ArmatureOffset, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;
use std::f32::consts::FRAC_PI_2;

use crate::animations::{Lay, LayOrientation};
use crate::{Animation, Effects};

impl Lay {
	pub fn sample_pose(&self, progress: f32) -> HumanoidPose {
		let mut pose = HumanoidPose::default();
		for side in [Side::Left, Side::Right] {
			pose.apply_leg(side, self.hip_flex, self.knee_flex);
			pose.leg_mut(side).hip_abduction = self.hip_spread * side.sign();
			self.apply_arm(&mut pose, side);
		}
		pose.apply_spine_pitch(self.spine_pitch(progress));
		if let Some(side) = self.down_side() {
			let roll = -side.sign() * 0.12;
			pose.spine.side_bend[1] += roll * 0.45;
			pose.spine.side_bend[2] += roll * 0.35;
			pose.spine.side_bend[3] += roll * 0.20;
		}
		let breath = Self::breath(progress) * self.breath_amp;
		pose.apply_neck_twisted(
			self.neck_turn * 0.55,
			0.0,
			self.neck_peak * 0.4 + 0.3 * breath,
			self.neck_turn * 0.45,
			0.0,
			self.neck_peak * 0.6 + 0.7 * breath,
		);
		pose
	}

	pub fn lie_rotation(&self) -> Quat {
		match self.orientation {
			LayOrientation::Back => Quat::from_rotation_x(-FRAC_PI_2),
			LayOrientation::Side(side) => {
				Quat::from_rotation_z(FRAC_PI_2 * side.sign()) * Quat::from_rotation_x(-FRAC_PI_2)
			}
		}
	}

	fn apply_arm(&self, pose: &mut HumanoidPose, side: Side) {
		let down = self.down_side() == Some(side);
		let hang = -side.sign() * if down { self.arm_hang * 0.35 } else { self.arm_hang };
		let elbow = if down { self.elbow_hang * 0.45 } else { self.elbow_hang };
		let inward = -side.sign() * if down { 0.06 } else { 0.16 };
		pose.apply_arm(side, 0.04, inward, 0.02, hang, -elbow);
	}
}

impl Animation<HumanoidV0Rig> for Lay {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		rig.write_pose(&self.sample_pose(progress));
	}

	fn effects_for(&self, _rig: &HumanoidV0Rig, progress: f32) -> Effects {
		if self.bones_only {
			return ArmatureOffset::IDENTITY;
		}
		ArmatureOffset(bevy::prelude::Transform {
			translation: Vec3::new(0.0, -self.visual_drop(progress), 0.0),
			rotation: self.lie_rotation(),
			scale: Vec3::ONE,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn back_lay_rotates_horizontal_and_keeps_a_resting_pose() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		let effects = Lay::back().apply(&mut rig, 0.0);
		let up = effects.0.rotation * Vec3::Y;
		if up.y.abs() >= 0.2 || up.z.abs() <= 0.8 {
			anyhow::bail!("supine body should lie along world Z, up={up:?}");
		}
		if rig.posed_angle("shin.L") <= 0.15 || rig.posed_angle("humerus.L") <= 0.15 {
			anyhow::bail!("lying pose should bend the knees and lower the arms");
		}
		Ok(())
	}

	#[test]
	fn side_orientation_places_the_selected_shoulder_down() -> anyhow::Result<()> {
		for side in [Side::Left, Side::Right] {
			let lay = Lay::side(side);
			let down_axis = match side {
				Side::Left => Vec3::NEG_X,
				Side::Right => Vec3::X,
			};
			let down = lay.lie_rotation() * down_axis;
			if down.y >= -0.8 || lay.down_side() != Some(side) {
				anyhow::bail!("selected shoulder should face ground for {side:?}: {down:?}");
			}
		}
		if Lay::back().down_side().is_some() {
			anyhow::bail!("back orientation has no down shoulder");
		}
		Ok(())
	}
}
