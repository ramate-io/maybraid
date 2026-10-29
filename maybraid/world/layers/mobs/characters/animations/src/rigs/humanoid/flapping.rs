use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::Flapping;
use crate::rigs::humanoid::wing::{apply_flight_body, apply_flight_wings};
use crate::Animation;

impl Animation<HumanoidV0Rig> for Flapping {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let mut pose = HumanoidPose::default();
		apply_flight_body(&mut pose);
		apply_flight_wings(&mut pose, self.flap_amount(progress));
		rig.write_pose(&pose);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::Vec3;
	use character_rigs::Side;

	fn forearm_tip(rig: &HumanoidV0Rig, side: Side) -> anyhow::Result<Vec3> {
		let name = match side {
			Side::Left => "forearm.L",
			Side::Right => "forearm.R",
		};
		let Some(id) = rig.binding.definition.id(name) else {
			anyhow::bail!("missing {name}");
		};
		let Some(affine) = rig.binding.definition.affine_in_character(&rig.pose, id) else {
			anyhow::bail!("could not compose {name}");
		};
		Ok(affine.transform_point3(rig.forearm_endpoints[side.index()]))
	}

	#[test]
	fn flapping_moves_shoulders() -> anyhow::Result<()> {
		let mut a = HumanoidV0Rig::imported();
		let mut b = HumanoidV0Rig::imported();
		Flapping::default().apply(&mut a, 0.1);
		Flapping::default().apply(&mut b, 0.1 + 0.5 / Flapping::default().speed);
		assert!(
			a.rotation("shoulder.L").angle_between(b.rotation("shoulder.L")) > 0.15,
			"half a cycle later the imported swing stroke has moved"
		);
		Ok(())
	}

	#[test]
	fn flapping_sweeps_t_pose_wings_in_z() {
		let flap = Flapping { speed: 1.0, range: 1.0 };
		let mut front = HumanoidV0Rig::for_clip_test();
		let mut back = HumanoidV0Rig::for_clip_test();
		flap.apply(&mut front, 0.25);
		flap.apply(&mut back, 0.75);
		let a = front.character_length("forearm.L");
		let b = back.character_length("forearm.L");
		assert!((a - b).length() > 0.3, "wing tip must move, {a:?} vs {b:?}");
		assert!(
			(a.z - b.z).abs() > (a.y - b.y).abs(),
			"previous stroke is parent Y (XZ), not a lift or length roll, {a:?} vs {b:?}"
		);
		let right_a = front.character_length("forearm.R");
		let right_b = back.character_length("forearm.R");
		assert!(
			(right_a.z - right_b.z).abs() > 0.2,
			"right wing must share the same stroke, {right_a:?} vs {right_b:?}"
		);
	}

	#[test]
	fn flapping_elbow_follows_both_signed_strokes_in_the_bind_pose() -> anyhow::Result<()> {
		let flap = Flapping { speed: 1.0, range: 1.0 };
		let mut up = HumanoidV0Rig::for_clip_test();
		let mut down = HumanoidV0Rig::for_clip_test();
		flap.apply(&mut up, 0.125);
		flap.apply(&mut down, 0.625);
		let left_up = forearm_tip(&up, Side::Left)?;
		let right_up = forearm_tip(&up, Side::Right)?;
		let left_down = forearm_tip(&down, Side::Left)?;
		let right_down = forearm_tip(&down, Side::Right)?;
		assert!(
			left_up.y > left_down.y,
			"upstroke elbow raises the bind-calibrated tip: {left_up:?} vs {left_down:?}"
		);
		assert!(
			right_up.y > right_down.y,
			"right elbow follows the signed stroke: {right_up:?} vs {right_down:?}"
		);
		assert!(
			(left_up.x + right_up.x).abs() < 1e-4,
			"upstroke mirrors x: {left_up:?} {right_up:?}"
		);
		assert!(
			(left_down.x + right_down.x).abs() < 1e-4,
			"downstroke mirrors x: {left_down:?} {right_down:?}"
		);
		Ok(())
	}
}
