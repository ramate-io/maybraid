//! Humanoid mapping for [`HeadNod`](crate::animations::HeadNod).

use crate::animations::HeadNod;
use crate::Animation;
use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

impl Animation<HumanoidV0Rig> for HeadNod {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let nod = self.nod_twist(progress);
		let spine = self.spine_follow(progress);
		if nod < 1e-5 && spine < 1e-5 {
			return;
		}

		let mut pose = HumanoidPose::default();
		pose.apply_spine_pitch(spine);
		pose.apply_neck_twisted(0.0, 0.0, nod * 0.35, 0.0, 0.0, nod * 0.65);

		rig.write_pose(&pose);
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	use crate::animations::HeadNod;
	use crate::Animation;

	const SAMPLE_PROGRESS: [f32; 7] = [0.0, 0.12, 0.24, 0.40, 0.54, 0.70, 1.0];
	const FIRST_DIP: f32 = 0.24;
	const SECOND_DIP: f32 = 0.54;
	const SHOULDER_MAX_TRAVEL: f32 = 0.02;

	fn bone_tail(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.character_point(name) + rig.character_length(name)
	}

	fn head_tip(rig: &HumanoidV0Rig) -> Vec3 {
		bone_tail(rig, "upper_neck")
	}

	fn chin_base(rig: &HumanoidV0Rig) -> Vec3 {
		rig.character_point("lower_neck")
	}

	fn head_pitch_deg(rig: &HumanoidV0Rig) -> f32 {
		let along = rig.character_length("upper_neck");
		if along.length_squared() < 1e-8 {
			return 0.0;
		}
		Vec3::Y.angle_between(along)
	}

	fn apply_nod(rig: &mut HumanoidV0Rig, progress: f32) {
		HeadNod.apply_for(rig, progress);
	}

	#[test]
	fn head_nod_posed_evidence_on_clip_test_rest() -> anyhow::Result<()> {
		let rest = HumanoidV0Rig::for_clip_test();
		let rest_tip = head_tip(&rest);
		let mut peak_one = HumanoidV0Rig::for_clip_test();
		apply_nod(&mut peak_one, FIRST_DIP);
		let dip_one = head_tip(&peak_one) - rest_tip;

		let mut peak_two = HumanoidV0Rig::for_clip_test();
		apply_nod(&mut peak_two, SECOND_DIP);
		let dip_two = head_tip(&peak_two) - rest_tip;

		for (label, delta) in [("first", dip_one), ("second", dip_two)] {
			assert!(delta.z > 0.03, "{label} dip moves tip forward (+Z), Δ={delta:?}");
			assert!(delta.y < -0.02, "{label} dip moves tip down (−Y), Δ={delta:?}");
			assert!(delta.x.abs() < 0.01, "{label} dip stays bilateral (x ≈ 0), Δ={delta:?}");
		}

		let pitch_one = head_pitch_deg(&peak_one).to_degrees();
		let pitch_two = head_pitch_deg(&peak_two).to_degrees();
		assert!(pitch_one > 15.0 && pitch_one < 30.0, "first dip head pitch {pitch_one:.1}°");
		assert!(pitch_two > 15.0 && pitch_two < 30.0, "second dip head pitch {pitch_two:.1}°");

		let mut valley = HumanoidV0Rig::for_clip_test();
		apply_nod(&mut valley, 0.40);
		let valley_tip = head_tip(&valley);
		assert!(
			(valley_tip - rest_tip).length() < 0.025,
			"valley returns near rest, tip {valley_tip:?} vs {rest_tip:?}"
		);
		assert!(head_pitch_deg(&valley).to_degrees() < 8.0, "valley pitch stays light");

		for progress in SAMPLE_PROGRESS {
			let mut posed = HumanoidV0Rig::for_clip_test();
			apply_nod(&mut posed, progress);

			for (name, rest_pt) in [
				("head_tip", rest_tip),
				("chin_base", chin_base(&rest)),
				("upper_back", rest.character_point("upper_back")),
				("shoulder.L", rest.character_point("shoulder.L")),
				("shoulder.R", rest.character_point("shoulder.R")),
			] {
				let pt = match name {
					"head_tip" => head_tip(&posed),
					"chin_base" => chin_base(&posed),
					other => posed.character_point(other),
				};
				assert!(pt.x.abs() < 0.02, "progress {progress}: {name} x ≈ 0, got {pt:?}");
				if name.starts_with("shoulder") {
					let travel = (pt - rest_pt).length();
					assert!(
						travel < SHOULDER_MAX_TRAVEL,
						"progress {progress}: {name} travel {travel:.4} m"
					);
				}
			}

			if progress == 0.0 || progress == 1.0 {
				assert!(
					(head_tip(&posed) - rest_tip).length() < 1e-3,
					"progress {progress} equals rest"
				);
			}
		}

		let rest_upper_back = rest.character_point("upper_back");
		let mut dipped = HumanoidV0Rig::for_clip_test();
		apply_nod(&mut dipped, FIRST_DIP);
		let spine_travel = (dipped.character_point("upper_back") - rest_upper_back).length();
		assert!(
			spine_travel < 0.04,
			"light spine follow only, upper_back moved {spine_travel:.4} m"
		);

		Ok(())
	}

	#[test]
	fn head_nod_repeatability_and_resample_do_not_accumulate() -> anyhow::Result<()> {
		let nod = HeadNod;
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		nod.apply_for(&mut once, 0.38);
		nod.apply_for(&mut twice, 0.38);
		nod.apply_for(&mut twice, 0.38);
		for bone in once.animation_bone_names() {
			let a = once.rotation(bone);
			let b = twice.rotation(bone);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "re-sample drift on {bone}");
		}

		let mut a = HumanoidV0Rig::for_clip_test();
		let mut b = HumanoidV0Rig::for_clip_test();
		nod.apply_for(&mut a, FIRST_DIP);
		nod.apply_for(&mut b, FIRST_DIP);
		assert!((head_tip(&a) - head_tip(&b)).length() < 1e-4);
		Ok(())
	}

	#[test]
	fn head_nod_entry_and_exit_are_continuous_at_rest() -> anyhow::Result<()> {
		let nod = HeadNod;
		let mut entry = HumanoidV0Rig::for_clip_test();
		let mut exit = HumanoidV0Rig::for_clip_test();
		let rest_pose = entry.pose.clone();
		nod.apply_for(&mut entry, 0.0);
		nod.apply_for(&mut exit, 1.0);
		assert_eq!(entry.pose, rest_pose, "entry should not write bones");
		assert_eq!(exit.pose, rest_pose, "exit should not write bones");
		Ok(())
	}

	#[test]
	fn head_nod_unwritten_limbs_stay_at_rest_at_peak() -> anyhow::Result<()> {
		let nod = HeadNod;
		let mut posed = HumanoidV0Rig::for_clip_test();
		nod.apply_for(&mut posed, FIRST_DIP);

		for bone in ["humerus.L", "humerus.R", "forearm.L", "forearm.R"] {
			assert!(posed.posed_angle(bone) < 1e-4, "arm {bone} stays at rest");
		}
		for bone in ["femur.L", "femur.R", "shin.L", "shin.R"] {
			let angle = posed.posed_angle(bone);
			assert!(angle < 0.02, "leg {bone} stays at rest (angle={angle})");
		}

		Ok(())
	}

	#[test]
	fn head_nod_transition_from_visible_at_mid_dip_is_stable() -> anyhow::Result<()> {
		use crate::animations::{Idle, Transition};
		use character_rigs::authoring::ArmatureOffset;

		let mut dipped = HumanoidV0Rig::for_clip_test();
		HeadNod.apply_for(&mut dipped, FIRST_DIP);
		let from_pose = dipped.pose.clone();
		let dipped_tip = head_tip(&dipped);

		let transition = Transition::from_visible(HeadNod, from_pose, ArmatureOffset::IDENTITY);
		transition.apply(&mut dipped, SECOND_DIP, 0.5);

		assert!(
			(head_tip(&dipped) - dipped_tip).length() < 0.04,
			"mid-transition head tip should stay near the captured dip"
		);

		let mut from_idle = HumanoidV0Rig::for_clip_test();
		Idle::default().apply_for(&mut from_idle, 0.0);
		let idle_pose = from_idle.pose.clone();
		let rest_tip = head_tip(&from_idle);
		let into_dip = Transition::from_visible(HeadNod, idle_pose, ArmatureOffset::IDENTITY);
		into_dip.apply(&mut from_idle, FIRST_DIP, 0.5);

		let blended_tip = head_tip(&from_idle);
		let full_dip = {
			let mut full = HumanoidV0Rig::for_clip_test();
			HeadNod.apply_for(&mut full, FIRST_DIP);
			head_tip(&full)
		};
		let toward_dip = (blended_tip - rest_tip).z;
		let full_delta = (full_dip - rest_tip).z;
		assert!(
			toward_dip > full_delta * 0.25 && toward_dip < full_delta * 0.85,
			"50% blend lands between idle and first dip, z Δ {toward_dip:.4} vs full {full_delta:.4}"
		);
		Ok(())
	}
}
