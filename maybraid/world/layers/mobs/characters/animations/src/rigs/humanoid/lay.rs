use bevy::prelude::{Quat, Vec3};
use character_rigs::authoring::{ArmatureOffset, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;
use std::f32::consts::FRAC_PI_2;

use crate::animations::{Lay, LayOrientation};
use crate::{Animation, Effects};

impl Lay {
	/// Authored semantic pose at unwrapped `progress`. Rest is applied later by the rig.
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
		let breath = Lay::breath(progress) * self.breath_amp;
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

	/// Character-space rotation that puts the back or the chosen side on the ground.
	///
	/// Side-lying keeps the same head-to-toe axis as the back pose (`Rx(−90°)`),
	/// then rolls around it so the named shoulder goes down.
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
		let pose = self.sample_pose(progress);
		rig.write_pose(&pose);
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
	use crate::animations::{Idle, Prone, Transition};

	const FRAME_DT: f32 = 1.0 / 60.0;
	const SETTLE_SECS: f32 = 0.7;

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	fn visual_point(rig: &HumanoidV0Rig, effects: Effects, name: &str) -> Vec3 {
		effects.0.rotation * rig.character_point(name) + effects.0.translation
	}

	#[test]
	fn back_lie_puts_the_body_horizontal_in_z() -> anyhow::Result<()> {
		let mut posed = HumanoidV0Rig::for_clip_test();
		let effects = Lay::back().apply(&mut posed, 0.0);
		if effects.is_identity() {
			anyhow::bail!("lying down must use a visual armature offset");
		}
		let up = effects.0.rotation * Vec3::Y;
		if up.y.abs() >= 0.2 {
			anyhow::bail!("back lie should rotate character up off world Y, {up:?}");
		}
		if up.z.abs() <= 0.8 {
			anyhow::bail!("back lie should map character up along Z, {up:?}");
		}
		let head = visual_point(&posed, effects, "upper_neck");
		let hip = visual_point(&posed, effects, "root");
		let along = (head - hip).abs();
		if along.y >= along.z {
			anyhow::bail!("supine head/hip should run along Z, head {head:?} hip {hip:?}");
		}
		if posed.posed_angle("shin.L") <= 0.15 {
			anyhow::bail!("knees should keep a rest bend");
		}
		if posed.posed_angle("humerus.L") <= 0.15 {
			anyhow::bail!("arms should leave the T-pose");
		}
		Ok(())
	}

	#[test]
	fn side_lie_rolls_onto_the_chosen_shoulder() -> anyhow::Result<()> {
		let mut right = HumanoidV0Rig::for_clip_test();
		let mut left = HumanoidV0Rig::for_clip_test();
		let right_fx = Lay::side(Side::Right).apply(&mut right, 0.0);
		let left_fx = Lay::side(Side::Left).apply(&mut left, 0.0);
		let right_shoulder = right_fx.0.rotation * Vec3::X;
		let left_shoulder = left_fx.0.rotation * Vec3::NEG_X;
		if right_shoulder.y >= -0.8 {
			anyhow::bail!("right-side lie should put +X toward the ground, {right_shoulder:?}");
		}
		if left_shoulder.y >= -0.8 {
			anyhow::bail!("left-side lie should put −X toward the ground, {left_shoulder:?}");
		}
		let head = visual_point(&right, right_fx, "upper_neck");
		let hip = visual_point(&right, right_fx, "root");
		let along = (head - hip).abs();
		if along.y >= along.z {
			anyhow::bail!("side-lie head/hip should run along Z, head {head:?} hip {hip:?}");
		}
		if right.posed_angle("shin.L") <= Lay::back().knee_flex * 0.5 {
			anyhow::bail!("side-lie knees should tuck more than a supine rest");
		}
		Ok(())
	}

	#[test]
	fn lay_is_not_prone_spine_fold() -> anyhow::Result<()> {
		let mut lay = HumanoidV0Rig::for_clip_test();
		let mut prone = HumanoidV0Rig::for_clip_test();
		let lay_fx = Lay::back().apply(&mut lay, 0.0);
		let prone_fx = Prone::default().apply(&mut prone, 1.0);
		if !prone_fx.is_identity() {
			anyhow::bail!("prone stance keeps identity offset");
		}
		if lay_fx.is_identity() {
			anyhow::bail!("lay uses offset rotation instead of a prone spine");
		}
		let lay_root = tip(&lay, "root");
		let prone_root = tip(&prone, "root");
		if lay_root.z.abs() >= prone_root.z.abs() - 0.15 {
			anyhow::bail!(
				"lay spine should stay near rest vs prone pitch, {lay_root:?} vs {prone_root:?}"
			);
		}
		Ok(())
	}

	#[test]
	fn lay_drop_does_not_translate_bones() {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Lay::back().apply(&mut rig, 0.0);
		let femur = rig.binding.definition.id("femur.L").expect("femur");
		assert_eq!(
			rig.pose.get(femur).expect("pose").translation,
			rig.binding.effective_rest.get(femur).expect("rest").translation
		);
	}

	#[test]
	fn resampling_does_not_accumulate() -> anyhow::Result<()> {
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		let lay = Lay::back();
		lay.apply(&mut once, 0.35);
		lay.apply(&mut twice, 0.35);
		lay.apply(&mut twice, 0.35);
		for name in once.animation_bone_names() {
			let a = once.rotation(name);
			let b = twice.rotation(name);
			if a.dot(b).abs() <= 1.0 - 1e-5 {
				anyhow::bail!("re-sample must not drift on {name}");
			}
		}
		Ok(())
	}

	#[test]
	fn lie_down_from_idle_uses_from_visible_at_frame_steps() -> anyhow::Result<()> {
		let mut source = HumanoidV0Rig::for_clip_test();
		let from_offset = Idle::default().apply(&mut source, 0.0);
		let transition = Transition::from_visible(Lay::back(), source.pose.clone(), from_offset);
		let mut prev_up_y = 1.0;
		let mut t = 0.0;
		while t <= SETTLE_SECS + 1e-5 {
			let mut rig = HumanoidV0Rig::for_clip_test();
			let u = (t / SETTLE_SECS).clamp(0.0, 1.0);
			let effects = transition.apply(&mut rig, 0.0, u);
			let up_y = (effects.0.rotation * Vec3::Y).y.abs();
			if t > FRAME_DT && up_y > prev_up_y + 0.02 {
				anyhow::bail!("lie-down should keep rotating toward horizontal, t={t} {up_y}");
			}
			prev_up_y = up_y;
			t += FRAME_DT;
			if t >= SETTLE_SECS && up_y >= 0.2 {
				anyhow::bail!("settled back-lie should be horizontal, up_y={up_y}");
			}
		}
		Ok(())
	}

	#[test]
	fn get_up_from_visible_lay_unwinds_at_frame_steps() -> anyhow::Result<()> {
		let mut source = HumanoidV0Rig::for_clip_test();
		let from_offset = Lay::back().apply(&mut source, 0.0);
		let transition =
			Transition::from_visible(Idle::default(), source.pose.clone(), from_offset);
		let mut t = 0.0;
		let mut prev_up_y = (from_offset.0.rotation * Vec3::Y).y.abs();
		while t <= SETTLE_SECS + 1e-5 {
			let mut rig = HumanoidV0Rig::for_clip_test();
			let u = (t / SETTLE_SECS).clamp(0.0, 1.0);
			let effects = transition.apply(&mut rig, 0.0, u);
			let up_y = (effects.0.rotation * Vec3::Y).y.abs();
			if t > FRAME_DT && up_y + 0.02 < prev_up_y {
				anyhow::bail!("get-up should stand the armature, t={t} {up_y} < {prev_up_y}");
			}
			prev_up_y = up_y;
			t += FRAME_DT;
		}
		let mut stood = HumanoidV0Rig::for_clip_test();
		let effects = transition.apply(&mut stood, 0.0, 1.0);
		if (effects.0.rotation * Vec3::Y).y <= 0.8 {
			anyhow::bail!("get-up should restore character up, {:?}", effects.0.rotation);
		}
		Ok(())
	}

	#[test]
	fn breath_loop_is_small_and_non_accumulating_on_frame_steps() -> anyhow::Result<()> {
		let lay = Lay::back();
		let mut first = HumanoidV0Rig::for_clip_test();
		lay.apply(&mut first, 0.0);
		let mut min_z = tip(&first, "upper_back").z;
		let mut max_z = min_z;
		let mut t = FRAME_DT;
		while t <= 1.0 / crate::animations::lay::LAY_BREATH_FREQ + 1e-5 {
			let mut rig = HumanoidV0Rig::for_clip_test();
			lay.apply(&mut rig, t);
			let z = tip(&rig, "upper_back").z;
			min_z = min_z.min(z);
			max_z = max_z.max(z);
			t += FRAME_DT;
		}
		if max_z - min_z <= 0.002 {
			anyhow::bail!("breath should move the upper back, span {}", max_z - min_z);
		}
		if max_z - min_z >= 0.08 {
			anyhow::bail!("breath should stay a idle, span {}", max_z - min_z);
		}
		let mut later = HumanoidV0Rig::for_clip_test();
		lay.apply(&mut later, 0.0);
		lay.apply(&mut later, 0.0);
		if first.rotation("upper_back").dot(later.rotation("upper_back")).abs() <= 1.0 - 1e-5 {
			anyhow::bail!("re-sampling the same breath frame must not drift");
		}
		Ok(())
	}

	/// Pre-payload dual-field coefficients, reconstructed without `sample_pose`.
	///
	/// Locks back / side-left / side-right so folding `side` into
	/// `LayOrientation::Side(Side)` cannot change the authored look.
	fn expected_pose(lay: &Lay, progress: f32) -> HumanoidPose {
		let mut pose = HumanoidPose::default();
		for side in [Side::Left, Side::Right] {
			pose.apply_leg(side, lay.hip_flex, lay.knee_flex);
			pose.leg_mut(side).hip_abduction = lay.hip_spread * side.sign();
			let down = matches!(lay.orientation, LayOrientation::Side(down) if down == side);
			let hang = -side.sign() * if down { lay.arm_hang * 0.35 } else { lay.arm_hang };
			let elbow = if down { lay.elbow_hang * 0.45 } else { lay.elbow_hang };
			let inward = -side.sign() * if down { 0.06 } else { 0.16 };
			pose.apply_arm(side, 0.04, inward, 0.02, hang, -elbow);
		}
		pose.apply_spine_pitch(lay.spine_pitch(progress));
		if let LayOrientation::Side(side) = lay.orientation {
			let roll = -side.sign() * 0.12;
			pose.spine.side_bend[1] += roll * 0.45;
			pose.spine.side_bend[2] += roll * 0.35;
			pose.spine.side_bend[3] += roll * 0.20;
		}
		let breath = Lay::breath(progress) * lay.breath_amp;
		pose.apply_neck_twisted(
			lay.neck_turn * 0.55,
			0.0,
			lay.neck_peak * 0.4 + 0.3 * breath,
			lay.neck_turn * 0.45,
			0.0,
			lay.neck_peak * 0.6 + 0.7 * breath,
		);
		pose
	}

	fn expected_lie_rotation(lay: &Lay) -> Quat {
		match lay.orientation {
			LayOrientation::Back => Quat::from_rotation_x(-FRAC_PI_2),
			LayOrientation::Side(side) => {
				Quat::from_rotation_z(FRAC_PI_2 * side.sign()) * Quat::from_rotation_x(-FRAC_PI_2)
			}
		}
	}

	#[test]
	fn pose_equivalence_back_side_left_side_right() {
		let cases = [Lay::back(), Lay::side(Side::Left), Lay::side(Side::Right)];
		let progress = 0.25 / crate::animations::lay::LAY_BREATH_FREQ;
		for lay in cases {
			assert_eq!(lay.sample_pose(0.0), expected_pose(&lay, 0.0));
			assert_eq!(lay.sample_pose(progress), expected_pose(&lay, progress));
			assert_eq!(lay.lie_rotation(), expected_lie_rotation(&lay));
		}

		let back = Lay::back();
		let left = Lay::side(Side::Left);
		let right = Lay::side(Side::Right);
		let back_pose = back.sample_pose(0.0);
		let left_pose = left.sample_pose(0.0);
		let right_pose = right.sample_pose(0.0);

		assert_eq!(back.down_side(), None);
		assert_eq!(back_pose.spine.side_bend, [0.0; 4]);
		assert_eq!(back_pose.neck.lower.turn, 0.0);
		assert_eq!(back_pose.neck.upper.turn, 0.0);

		let left_roll = -Side::Left.sign() * 0.12;
		let right_roll = -Side::Right.sign() * 0.12;
		assert_eq!(left_pose.spine.side_bend[1], left_roll * 0.45);
		assert_eq!(left_pose.spine.side_bend[2], left_roll * 0.35);
		assert_eq!(left_pose.spine.side_bend[3], left_roll * 0.20);
		assert_eq!(right_pose.spine.side_bend[1], right_roll * 0.45);
		assert_eq!(right_pose.spine.side_bend[2], right_roll * 0.35);
		assert_eq!(right_pose.spine.side_bend[3], right_roll * 0.20);

		assert!(left.neck_turn < 0.0);
		assert!(right.neck_turn > 0.0);
		assert!(left_pose.neck.lower.turn < 0.0);
		assert!(right_pose.neck.lower.turn > 0.0);
		assert!((left_pose.neck.lower.turn + right_pose.neck.lower.turn).abs() < 1e-6);
		assert!((left_pose.neck.upper.turn + right_pose.neck.upper.turn).abs() < 1e-6);
		assert_eq!(left_pose.neck.lower.turn, left.neck_turn * 0.55);
		assert_eq!(right_pose.neck.lower.turn, right.neck_turn * 0.55);
		assert_eq!(left_pose.neck.upper.turn, left.neck_turn * 0.45);
		assert_eq!(right_pose.neck.upper.turn, right.neck_turn * 0.45);

		let right_shoulder = right.lie_rotation() * Vec3::X;
		let left_shoulder = left.lie_rotation() * Vec3::NEG_X;
		assert!(
			right_shoulder.y < -0.8,
			"right-side lie should put +X toward the ground, {right_shoulder:?}"
		);
		assert!(
			left_shoulder.y < -0.8,
			"left-side lie should put −X toward the ground, {left_shoulder:?}"
		);
		assert!((back.lie_rotation() * Vec3::Y).z.abs() > 0.8);

		let left_down = left_pose.arms[Side::Left.index()];
		let left_up = left_pose.arms[Side::Right.index()];
		assert_eq!(left_down.lateral_elevation, -Side::Left.sign() * left.arm_hang * 0.35);
		assert_eq!(left_down.elbow_flexion, -(left.elbow_hang * 0.45));
		assert_eq!(left_down.shoulder_lift, -Side::Left.sign() * 0.06);
		assert_eq!(left_up.lateral_elevation, -Side::Right.sign() * left.arm_hang);
		assert_eq!(left_up.elbow_flexion, -left.elbow_hang);
		assert_eq!(left_up.shoulder_lift, -Side::Right.sign() * 0.16);

		let right_down = right_pose.arms[Side::Right.index()];
		let right_up = right_pose.arms[Side::Left.index()];
		assert_eq!(right_down.lateral_elevation, -Side::Right.sign() * right.arm_hang * 0.35);
		assert_eq!(right_down.elbow_flexion, -(right.elbow_hang * 0.45));
		assert_eq!(right_down.shoulder_lift, -Side::Right.sign() * 0.06);
		assert_eq!(right_up.lateral_elevation, -Side::Left.sign() * right.arm_hang);
		assert_eq!(right_up.elbow_flexion, -right.elbow_hang);
		assert_eq!(right_up.shoulder_lift, -Side::Left.sign() * 0.16);
	}
}
