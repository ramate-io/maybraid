use crozon_rigs::{humanoid::HumanoidRig, Side};

use crate::animations::{smoothstep, UprightLeap, AIR_END, TAKEOFF_END};
use crate::rigs::humanoid::apply::{apply_arm, apply_leg, apply_root};
use crate::{Animation, Effects, Progress};

#[derive(Clone, Copy)]
struct LeapPose {
	left_femur: f32,
	right_femur: f32,
	left_shin: f32,
	right_shin: f32,
	lean: f32,
	left_shoulder: f32,
	right_shoulder: f32,
	left_humerus: f32,
	right_humerus: f32,
	elbow: f32,
}

impl<R: HumanoidRig> Animation<R> for UprightLeap<R> {
	fn apply(&self, rig: &mut R, progress: f32) -> Effects {
		let pose = self.pose_at(Progress(progress).clamp());
		apply_leg(rig, Side::Left, pose.left_femur, pose.left_shin);
		apply_leg(rig, Side::Right, pose.right_femur, pose.right_shin);
		apply_root(rig, pose.lean);
		apply_arm(rig, Side::Left, pose.left_shoulder, 0.0, pose.left_humerus, 0.0, pose.elbow);
		apply_arm(rig, Side::Right, pose.right_shoulder, 0.0, pose.right_humerus, 0.0, pose.elbow);
		Effects::default()
	}
}

impl<Rig> UprightLeap<Rig> {
	fn pose_at(&self, t: f32) -> LeapPose {
		if t < TAKEOFF_END {
			self.takeoff(smoothstep(t / TAKEOFF_END))
		} else if t < AIR_END {
			self.air(smoothstep((t - TAKEOFF_END) / (AIR_END - TAKEOFF_END)))
		} else {
			self.land(smoothstep((t - AIR_END) / (1.0 - AIR_END).max(f32::EPSILON)))
		}
	}

	/// `u = 0` is a running split; `u = 1` is the push-off.
	fn takeoff(&self, u: f32) -> LeapPose {
		let lead_femur = lerp(-self.lead_stride, -self.lead_stride * 0.25, u);
		let trail_femur = lerp(self.trail_stride, self.trail_stride * 0.2, u);
		let lead_shin = lerp(self.takeoff_knee_lead, self.takeoff_knee_lead * 0.2, u);
		let trail_shin = lerp(self.takeoff_knee_trail, self.takeoff_knee_trail * 0.15, u);
		let lean = lerp(self.lean * 0.45, self.lean, u);
		let drive = lerp(self.arm_drive, self.arm_drive * 0.35, u);
		LeapPose {
			left_femur: trail_femur,
			right_femur: lead_femur,
			left_shin: trail_shin,
			right_shin: lead_shin,
			lean,
			left_shoulder: -drive * 0.25,
			right_shoulder: drive * 0.25,
			left_humerus: -drive,
			right_humerus: drive,
			elbow: self.elbow,
		}
	}

	/// `u = 0` leaves the ground; gather peaks near mid-air; `u = 1` reaches down.
	fn air(&self, u: f32) -> LeapPose {
		let gather = (u * std::f32::consts::PI).sin();
		let femur = lerp(-self.lead_stride * 0.25, -self.air_femur, gather);
		let shin = lerp(self.takeoff_knee_lead * 0.2, self.air_knee, gather);
		let reach = lerp(self.air_arm, self.air_arm * 0.4, u);
		LeapPose {
			left_femur: femur,
			right_femur: femur * 0.92,
			left_shin: shin,
			right_shin: shin * 0.9,
			lean: self.lean * (0.85 + 0.15 * gather),
			left_shoulder: -reach * 0.2,
			right_shoulder: reach * 0.15,
			left_humerus: -reach,
			right_humerus: -reach * 0.85,
			elbow: self.elbow * (0.85 + 0.2 * gather),
		}
	}

	/// `u = 0` is touchdown; absorb peaks mid-land; `u = 1` recovers to run-ready.
	fn land(&self, u: f32) -> LeapPose {
		let absorb = (u * std::f32::consts::PI).sin();
		let femur = -self.land_femur * absorb;
		let shin = self.land_knee * absorb;
		let lean = self.lean * (1.0 - u * 0.75);
		let arm = self.air_arm * 0.4 * (1.0 - u);
		LeapPose {
			left_femur: femur,
			right_femur: femur * 0.95,
			left_shin: shin,
			right_shin: shin * 0.95,
			lean,
			left_shoulder: -arm * 0.15,
			right_shoulder: arm * 0.1,
			left_humerus: -arm,
			right_humerus: -arm * 0.8,
			elbow: self.elbow * (1.0 - u * 0.25),
		}
	}
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
	a + (b - a) * t
}

#[cfg(test)]
mod tests {
	use crozon_rigs::{rigs::humanoid_v0::HumanoidV0Rig, Side};

	use super::*;
	use crate::animations::{Leap, Squat, TwoFootedJump};

	fn femur_swing(rig: &HumanoidV0Rig, side: Side) -> f32 {
		rig.pose().get(&rig.leg(side).femur.name).expect("femur").swing
	}

	fn shin_flex(rig: &HumanoidV0Rig, side: Side) -> f32 {
		rig.pose().get(&rig.leg(side).shin.name).expect("shin").flex
	}

	fn apply_leap(rig: &mut HumanoidV0Rig, progress: f32) -> Effects {
		UprightLeap::from_leap(&Leap::default()).apply(rig, progress)
	}

	#[test]
	fn leap_delegates_to_upright_default() {
		for &phase in &[0.0, 0.1, 0.45, 0.85] {
			let mut from_leap = HumanoidV0Rig::imported();
			let mut from_upright = HumanoidV0Rig::imported();
			apply_leap(&mut from_leap, phase);
			UprightLeap::<HumanoidV0Rig>::default().apply(&mut from_upright, phase);
			for bone in from_leap.animation_bones() {
				let Some(leap_pose) = from_leap.pose().get(&bone) else {
					continue;
				};
				let upright = from_upright.pose().get(&bone).expect("upright pose");
				assert_eq!(leap_pose.swing, upright.swing, "swing mismatch on {bone} at {phase}");
				assert_eq!(leap_pose.flex, upright.flex, "flex mismatch on {bone} at {phase}");
			}
		}
	}

	#[test]
	fn takeoff_keeps_a_run_split() {
		let mut rig = HumanoidV0Rig::imported();
		apply_leap(&mut rig, 0.0);
		let left = femur_swing(&rig, Side::Left);
		let right = femur_swing(&rig, Side::Right);
		assert!(left > 0.3, "trail femur should be back, swing={left}");
		assert!(right < -0.3, "lead femur should be forward, swing={right}");
	}

	#[test]
	fn takeoff_is_not_a_standing_squat() {
		let mut leap_rig = HumanoidV0Rig::imported();
		apply_leap(&mut leap_rig, 0.0);
		let leap_left = femur_swing(&leap_rig, Side::Left);
		let leap_right = femur_swing(&leap_rig, Side::Right);

		let mut squat_rig = HumanoidV0Rig::imported();
		Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0).apply(&mut squat_rig, 0.5);
		let squat = femur_swing(&squat_rig, Side::Left);

		assert_ne!(leap_left.signum(), leap_right.signum());
		assert!(
			(leap_left - squat).abs() > 0.2 || (leap_right - squat).abs() > 0.2,
			"takeoff should not match a symmetric squat"
		);
	}

	#[test]
	fn takeoff_differs_from_standing_jump_start() {
		let mut leap_rig = HumanoidV0Rig::imported();
		apply_leap(&mut leap_rig, 0.0);
		let mut jump_rig = HumanoidV0Rig::imported();
		TwoFootedJump::<HumanoidV0Rig>::default().apply(&mut jump_rig, 0.0);
		assert!(
			(femur_swing(&leap_rig, Side::Left) - femur_swing(&jump_rig, Side::Left)).abs() > 0.3
		);
	}

	#[test]
	fn air_gathers_knees_more_than_takeoff() {
		let mut takeoff = HumanoidV0Rig::imported();
		apply_leap(&mut takeoff, 0.0);
		let mut air = HumanoidV0Rig::imported();
		apply_leap(&mut air, 0.45);
		assert!(shin_flex(&air, Side::Left) > shin_flex(&takeoff, Side::Left) + 0.2);
	}

	#[test]
	fn land_absorbs_then_recovers() {
		let mut mid = HumanoidV0Rig::imported();
		apply_leap(&mut mid, 0.86);
		let mut end = HumanoidV0Rig::imported();
		apply_leap(&mut end, 1.0);
		assert!(femur_swing(&mid, Side::Left).abs() > femur_swing(&end, Side::Left).abs() + 0.05);
	}

	#[test]
	fn leap_has_no_root_motion() {
		let mut rig = HumanoidV0Rig::imported();
		let effects = apply_leap(&mut rig, 0.45);
		assert!(effects.r#move.is_none());
	}

	#[test]
	fn progress_clamps_past_one() {
		let mut a = HumanoidV0Rig::imported();
		let mut b = HumanoidV0Rig::imported();
		apply_leap(&mut a, 1.0);
		apply_leap(&mut b, 1.7);
		assert_eq!(femur_swing(&a, Side::Left), femur_swing(&b, Side::Left));
	}
}
