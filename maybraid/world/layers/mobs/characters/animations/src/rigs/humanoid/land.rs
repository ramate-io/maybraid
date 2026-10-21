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
	use bevy::prelude::Vec3;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
	use character_rigs::Side;

	use super::*;
	use crate::animations::TwoFootedJump;
	use crate::Animation;

	const FOREARM_REACH: f32 = 0.48;
	const GUARD_FORWARD_MARGIN: f32 = 0.15;

	fn forearm_joint(rig: &HumanoidV0Rig, side: &str) -> Vec3 {
		rig.character_point(&format!("forearm.{side}"))
	}

	fn forearm_tip(rig: &HumanoidV0Rig, side: &str) -> Vec3 {
		let bone = format!("forearm.{side}");
		forearm_joint(rig, side) + rig.character_length(&bone) * FOREARM_REACH
	}

	fn upper_back_tail(rig: &HumanoidV0Rig) -> Vec3 {
		rig.character_point("upper_back") + rig.character_length("upper_back") * 0.15
	}

	fn neck_top(rig: &HumanoidV0Rig) -> Vec3 {
		rig.character_point("upper_neck") + rig.character_length("upper_neck") * 0.15
	}

	fn land_peak_progress(land: &Land) -> f32 {
		land.descent_duration() / land.cycle_duration()
	}

	fn assert_mirrored(left: Vec3, right: Vec3, label: &str) {
		assert!((left.x + right.x).abs() < 1e-3, "{label} x mirror: L={left:?} R={right:?}");
		assert!((left.y - right.y).abs() < 1e-3, "{label} y match: L={left:?} R={right:?}");
		assert!((left.z - right.z).abs() < 1e-3, "{label} z match: L={left:?} R={right:?}");
	}

	fn assert_hand_rest(rig: &HumanoidV0Rig, rest: &HumanoidV0Rig, side: &str) {
		let posed = forearm_tip(rig, side);
		let baseline = forearm_tip(rest, side);
		assert!(
			(posed - baseline).length() < 1e-3,
			"{side} hand should match T-pose rest, got {posed:?} vs {baseline:?}"
		);
	}

	#[test]
	fn land_arms_rest_at_touchdown_and_recovery() {
		let land = Land::default();
		let rest = HumanoidV0Rig::for_clip_test();
		for progress in [0.0, 1.0] {
			let mut rig = HumanoidV0Rig::for_clip_test();
			land.apply(&mut rig, progress);
			assert_hand_rest(&rig, &rest, "L");
			assert_hand_rest(&rig, &rest, "R");
			assert!(rig.posed_angle("humerus.L") < 0.02);
			assert!(rig.posed_angle("humerus.R") < 0.02);
		}
	}

	#[test]
	fn land_guard_pose_at_peak_compression() {
		let land = Land::default();
		let peak = land_peak_progress(&land);
		let mut rig = HumanoidV0Rig::for_clip_test();
		land.apply(&mut rig, peak);

		assert!(rig.posed_angle("shin.L") > 0.4, "legs compress at peak");

		let upper_back = upper_back_tail(&rig);
		let neck = neck_top(&rig);
		let tip_l = forearm_tip(&rig, "L");
		let tip_r = forearm_tip(&rig, "R");
		let elbow_l = forearm_joint(&rig, "L");
		let elbow_r = forearm_joint(&rig, "R");

		assert_mirrored(tip_l, tip_r, "hand tip");
		assert_mirrored(elbow_l, elbow_r, "elbow");

		assert!(
			tip_l.z > upper_back.z + GUARD_FORWARD_MARGIN,
			"hands in front of torso, tip z={} upper_back z={}",
			tip_l.z,
			upper_back.z
		);
		assert!(
			tip_l.y > upper_back.y - 0.65 && tip_l.y < neck.y + 0.05,
			"hands between chest and chin, tip y={} chest~{} chin~{}",
			tip_l.y,
			upper_back.y - 0.55,
			neck.y
		);
		assert!(tip_l.x > 0.0 && tip_r.x < 0.0, "hands stay on their sides");
		assert!(tip_l.x < 1.15 && tip_r.x > -1.15, "hands do not cross the midline");
	}

	#[test]
	fn land_resampling_does_not_accumulate() {
		let land = Land::default();
		let peak = land_peak_progress(&land);
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		land.apply(&mut once, peak);
		land.apply(&mut twice, peak);
		land.apply(&mut twice, peak);
		for name in once.animation_bone_names() {
			let a = once.rotation(name);
			let b = twice.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "re-sample must not drift on {name}");
		}
	}

	fn max_hand_tip_velocity<F>(mut sample: F, start: f32, end: f32, dt: f32) -> f32
	where
		F: FnMut(f32) -> Vec3,
	{
		let mut prev = sample(start);
		let mut max_vel = 0.0f32;
		let mut t = start + dt;
		while t <= end + dt * 0.5 {
			let tip = sample(t);
			max_vel = max_vel.max((tip - prev).length() / dt);
			prev = tip;
			t += dt;
		}
		max_vel
	}

	#[test]
	fn land_fall_blend_hand_velocity_stays_smooth() {
		use crate::animations::{smoothstep, Fall};
		use crate::rigs::mix::blend_clips;

		struct LandLegsOnly(Land);

		impl Animation<HumanoidV0Rig> for LandLegsOnly {
			fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
				let mut pose = HumanoidPose::default();
				pose.apply_leg(
					Side::Left,
					self.0.femur_swing(progress),
					self.0.shin_flex(progress),
				);
				pose.apply_leg(
					Side::Right,
					self.0.femur_swing(progress),
					self.0.shin_flex(progress),
				);
				pose.apply_root(self.0.root_swing(progress));
				rig.write_pose(&pose);
			}
		}

		let jump = TwoFootedJump::default();
		let land = jump.landing_squat(HumanoidV0Rig::for_clip_test().segment_lengths);
		let lengths = HumanoidV0Rig::for_clip_test().segment_lengths;
		let timings = jump.timings(lengths);
		let window = timings.land_pose_blend_duration();
		let start = timings.air_end() - window * 0.25;
		let end = timings.air_end() + window;
		let dt = 1.0 / 60.0;

		let max_after = max_hand_tip_velocity(
			|t| {
				let mut rig = HumanoidV0Rig::for_clip_test();
				jump.apply(&mut rig, t);
				forearm_tip(&rig, "L")
			},
			start,
			end,
			dt,
		);

		let max_before = max_hand_tip_velocity(
			|t| {
				let mut rig = HumanoidV0Rig::for_clip_test();
				let local = t - timings.air_end();
				let land_duration = timings.land_duration().max(f32::EPSILON);
				let land_progress = local.max(0.0) / land_duration;
				let transition_progress = (local / window).clamp(0.0, 1.0);
				if local >= 0.0 && transition_progress < 1.0 {
					blend_clips(
						&mut rig,
						&Fall::default(),
						1.0,
						&LandLegsOnly(land.clone()),
						land_progress,
						smoothstep(transition_progress),
					);
				} else if local >= 0.0 {
					LandLegsOnly(land.clone()).apply_for(&mut rig, land_progress);
				} else {
					Fall::default().apply_for(&mut rig, 1.0);
				}
				forearm_tip(&rig, "L")
			},
			start,
			end,
			dt,
		);

		assert!(
			max_after < max_before,
			"guard tuck should ease the fall→land hand snap at 60 Hz ({max_after} vs legs-only {max_before})"
		);

		let mut min_tip_z = f32::INFINITY;
		let mut t = start;
		while t <= end {
			let mut rig = HumanoidV0Rig::for_clip_test();
			jump.apply(&mut rig, t);
			min_tip_z = min_tip_z.min(forearm_tip(&rig, "L").z);
			t += dt;
		}
		assert!(
			min_tip_z > -0.05,
			"hands should not swing behind the torso during fall→land blend, min tip z={min_tip_z}"
		);
	}
}
