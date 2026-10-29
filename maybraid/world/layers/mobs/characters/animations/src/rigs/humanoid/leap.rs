use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::{
	smoothstep, LeapPhase, LeapTimeline, LinearBlend, Run, UprightLeap, AIR_END,
	LEAP_LAND_BLEND_FRACTION, LEAP_RUN_RECOVERY_BLEND_FRACTION, LEAP_TAKEOFF_BLEND_FRACTION,
	TAKEOFF_END,
};
use crate::rigs::mix::blend_clips;
use crate::{Animation, Progress};

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

#[derive(Clone, Copy)]
struct LeapLandFrame<'a> {
	leap: &'a UprightLeap,
	u: f32,
}

impl Animation<HumanoidV0Rig> for LeapLandFrame<'_> {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, _progress: f32) {
		write_leap_pose(rig, self.leap.land(self.u));
	}
}

fn write_leap_pose(rig: &mut HumanoidV0Rig, sample: LeapPose) {
	let mut pose = HumanoidPose::default();
	pose.apply_leg(Side::Left, sample.left_femur, sample.left_shin);
	pose.apply_leg(Side::Right, sample.right_femur, sample.right_shin);
	pose.apply_root(sample.lean);
	pose.apply_arm(Side::Left, sample.left_shoulder, 0.0, sample.left_humerus, 0.0, sample.elbow);
	pose.apply_arm(
		Side::Right,
		sample.right_shoulder,
		0.0,
		sample.right_humerus,
		0.0,
		sample.elbow,
	);
	rig.write_pose(&pose);
}

impl Animation<HumanoidV0Rig> for UprightLeap {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let t = Progress(progress).clamp();
		if t >= 1.0 {
			Run::default().apply_for(rig, 0.0);
			return;
		}
		if t >= AIR_END {
			let land_span = (1.0 - AIR_END).max(f32::EPSILON);
			let land_local = (t - AIR_END) / land_span;
			let recovery_start = 1.0 - LEAP_RUN_RECOVERY_BLEND_FRACTION;
			if land_local >= recovery_start {
				let weight = smoothstep(
					((land_local - recovery_start)
						/ LEAP_RUN_RECOVERY_BLEND_FRACTION.max(f32::EPSILON))
					.clamp(0.0, 1.0),
				);
				let (_, land_u) = LeapTimeline::sample(t);
				let land = LeapLandFrame { leap: self, u: land_u };
				blend_clips(rig, &land, land_u, &Run::default(), 0.0, weight);
				return;
			}
		}
		write_leap_pose(rig, self.pose_at(t));
	}
}

impl UprightLeap {
	fn pose_at(&self, t: f32) -> LeapPose {
		match LeapTimeline::sample(t) {
			(LeapPhase::Takeoff, u) => self.takeoff(u),
			(LeapPhase::Air, u) => {
				let air = self.air(u);
				let air_span = (AIR_END - TAKEOFF_END).max(f32::EPSILON);
				let air_local = (t - TAKEOFF_END) / air_span;
				if air_local < LEAP_TAKEOFF_BLEND_FRACTION {
					let weight = smoothstep(
						(air_local / LEAP_TAKEOFF_BLEND_FRACTION.max(f32::EPSILON)).clamp(0.0, 1.0),
					);
					blend_leap_pose(self.takeoff(1.0), air, weight)
				} else {
					air
				}
			}
			(LeapPhase::Land, u) => {
				let land_span = (1.0 - AIR_END).max(f32::EPSILON);
				let land_local = (t - AIR_END) / land_span;
				let land = self.land(u);
				if land_local < LEAP_LAND_BLEND_FRACTION {
					let weight = smoothstep(
						(land_local / LEAP_LAND_BLEND_FRACTION.max(f32::EPSILON)).clamp(0.0, 1.0),
					);
					blend_leap_pose(self.air(1.0), land, weight)
				} else {
					land
				}
			}
		}
	}

	/// `u = 0` is a running split; `u = 1` is the push-off.
	fn takeoff(&self, u: f32) -> LeapPose {
		let lead_femur = LinearBlend::eval(-self.lead_stride, -self.lead_stride * 0.25, u);
		let trail_femur = LinearBlend::eval(self.trail_stride, self.trail_stride * 0.2, u);
		let lead_shin = LinearBlend::eval(self.takeoff_knee_lead, self.takeoff_knee_lead * 0.2, u);
		let trail_shin =
			LinearBlend::eval(self.takeoff_knee_trail, self.takeoff_knee_trail * 0.15, u);
		let lean = LinearBlend::eval(self.lean * 0.45, self.lean, u);
		let drive = LinearBlend::eval(self.arm_drive, self.arm_drive * 0.35, u);
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
		let femur = LinearBlend::eval(-self.lead_stride * 0.25, -self.air_femur, gather);
		let shin = LinearBlend::eval(self.takeoff_knee_lead * 0.2, self.air_knee, gather);
		let reach = LinearBlend::eval(self.air_arm, self.air_arm * 0.4, u);
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

fn blend_leap_pose(from: LeapPose, to: LeapPose, weight: f32) -> LeapPose {
	let t = weight.clamp(0.0, 1.0);
	let blend = |a: f32, b: f32| a + (b - a) * t;
	LeapPose {
		left_femur: blend(from.left_femur, to.left_femur),
		right_femur: blend(from.right_femur, to.right_femur),
		left_shin: blend(from.left_shin, to.left_shin),
		right_shin: blend(from.right_shin, to.right_shin),
		lean: blend(from.lean, to.lean),
		left_shoulder: blend(from.left_shoulder, to.left_shoulder),
		right_shoulder: blend(from.right_shoulder, to.right_shoulder),
		left_humerus: blend(from.left_humerus, to.left_humerus),
		right_humerus: blend(from.right_humerus, to.right_humerus),
		elbow: blend(from.elbow, to.elbow),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::animations::{Leap, Squat, TwoFootedJump};

	fn femur_z(rig: &HumanoidV0Rig, side: Side) -> f32 {
		let name = match side {
			Side::Left => "femur.L",
			Side::Right => "femur.R",
		};
		rig.character_length(name).z
	}

	fn apply_leap(rig: &mut HumanoidV0Rig, progress: f32) -> crate::Effects {
		UprightLeap::from_leap(&Leap::default()).apply(rig, progress)
	}

	fn max_rotation_delta(a: &HumanoidV0Rig, b: &HumanoidV0Rig) -> f32 {
		a.animation_bone_names()
			.map(|name| a.rotation(name).normalize().angle_between(b.rotation(name).normalize()))
			.fold(0.0, f32::max)
	}

	#[test]
	fn leap_delegates_to_upright_default() {
		for &phase in &[0.0, 0.1, 0.45, 0.85] {
			let mut from_leap = HumanoidV0Rig::imported();
			let mut from_upright = HumanoidV0Rig::imported();
			apply_leap(&mut from_leap, phase);
			UprightLeap::default().apply(&mut from_upright, phase);
			for name in from_leap.animation_bone_names() {
				let leap_rot = from_leap.rotation(name);
				let upright_rot = from_upright.rotation(name);
				assert!(
					leap_rot.dot(upright_rot).abs() > 1.0 - 1e-5,
					"rotation mismatch on {name} at {phase}"
				);
			}
		}
	}

	#[test]
	fn takeoff_keeps_a_run_split() {
		let mut rig = HumanoidV0Rig::for_clip_test();
		apply_leap(&mut rig, 0.0);
		assert!(rig.posed_angle("femur.L") > 0.25, "trail femur leaves rest");
		assert!(rig.posed_angle("femur.R") > 0.25, "lead femur leaves rest");
		let left = rig.character_length("femur.L");
		let right = rig.character_length("femur.R");
		assert!(
			(left.z - right.z).abs() > 0.25,
			"takeoff keeps a run split, L={left:?} R={right:?}"
		);
	}

	#[test]
	fn takeoff_is_not_a_standing_squat() {
		let mut leap_rig = HumanoidV0Rig::for_clip_test();
		apply_leap(&mut leap_rig, 0.0);
		let mut squat_rig = HumanoidV0Rig::for_clip_test();
		Squat::for_loop(1.0, 1.0).apply(&mut squat_rig, 0.5);

		let leap_left = leap_rig.character_length("femur.L");
		let leap_right = leap_rig.character_length("femur.R");
		let squat_left = squat_rig.character_length("femur.L");
		assert!(
			(leap_left.z - leap_right.z).abs() > 0.15,
			"takeoff is a split, not a symmetric squat, L={leap_left:?} R={leap_right:?}"
		);
		assert!(
			(leap_left.z - squat_left.z).abs() > 0.15,
			"takeoff should not match a symmetric squat, leap={leap_left:?} squat={squat_left:?}"
		);
	}

	#[test]
	fn takeoff_differs_from_standing_jump_start() {
		let mut leap_rig = HumanoidV0Rig::imported();
		apply_leap(&mut leap_rig, 0.0);
		let mut jump_rig = HumanoidV0Rig::imported();
		TwoFootedJump::default().apply(&mut jump_rig, 0.0);
		assert!(leap_rig.rotation("femur.L").angle_between(jump_rig.rotation("femur.L")) > 0.25);
	}

	#[test]
	fn air_gathers_knees_more_than_takeoff() {
		let mut takeoff = HumanoidV0Rig::imported();
		apply_leap(&mut takeoff, 0.0);
		let mut air = HumanoidV0Rig::imported();
		apply_leap(&mut air, 0.45);
		assert!(air.posed_angle("shin.L") > takeoff.posed_angle("shin.L") + 0.15);
	}

	#[test]
	fn land_absorbs_then_recovers() {
		let mut mid = HumanoidV0Rig::imported();
		apply_leap(&mut mid, 0.86);
		let mut recovery = HumanoidV0Rig::imported();
		apply_leap(&mut recovery, 0.94);
		assert!(
			mid.posed_angle("femur.L") > recovery.posed_angle("femur.L") + 0.04,
			"landing flex eases before the authored run handoff"
		);
	}

	#[test]
	fn leap_has_no_root_motion() {
		let mut rig = HumanoidV0Rig::imported();
		let effects = apply_leap(&mut rig, 0.45);
		assert!(effects.is_identity());
	}

	#[test]
	fn progress_clamps_past_one() {
		let mut a = HumanoidV0Rig::imported();
		let mut b = HumanoidV0Rig::imported();
		apply_leap(&mut a, 1.0);
		apply_leap(&mut b, 1.7);
		assert!((femur_z(&a, Side::Left) - femur_z(&b, Side::Left)).abs() < 1e-5);
	}

	#[test]
	fn three_composed_windows_match_their_authored_endpoints() -> anyhow::Result<()> {
		let leap = UprightLeap::from_leap(&Leap::default());
		let air_span = AIR_END - TAKEOFF_END;
		let land_span = 1.0 - AIR_END;
		let endpoints = [
			(TAKEOFF_END, leap.takeoff(1.0)),
			(
				TAKEOFF_END + air_span * LEAP_TAKEOFF_BLEND_FRACTION,
				leap.air(smoothstep(LEAP_TAKEOFF_BLEND_FRACTION)),
			),
			(AIR_END, leap.air(1.0)),
			(
				AIR_END + land_span * LEAP_LAND_BLEND_FRACTION,
				leap.land(smoothstep(LEAP_LAND_BLEND_FRACTION)),
			),
			(
				AIR_END + land_span * (1.0 - LEAP_RUN_RECOVERY_BLEND_FRACTION),
				leap.land(smoothstep(1.0 - LEAP_RUN_RECOVERY_BLEND_FRACTION)),
			),
		];
		for (progress, expected) in endpoints {
			let mut actual = HumanoidV0Rig::for_clip_test();
			let mut reference = HumanoidV0Rig::for_clip_test();
			apply_leap(&mut actual, progress);
			write_leap_pose(&mut reference, expected);
			assert!(
				max_rotation_delta(&actual, &reference) < 1e-4,
				"window endpoint at {progress}"
			);
			assert_eq!(actual.scratch.depth, 0);
		}
		let mut end = HumanoidV0Rig::for_clip_test();
		let mut run = HumanoidV0Rig::for_clip_test();
		apply_leap(&mut end, 1.0);
		Run::default().apply(&mut run, 0.0);
		for name in end.animation_bone_names() {
			assert!(
				end.rotation(name).normalize().dot(run.rotation(name).normalize()).abs()
					> 1.0 - 1e-6,
				"final run handoff differs on {name}"
			);
		}
		Ok(())
	}

	#[test]
	fn composed_windows_stay_continuous_and_converge_at_60_and_240hz() -> anyhow::Result<()> {
		let mut max_60 = 0.0_f32;
		let mut previous = HumanoidV0Rig::for_clip_test();
		apply_leap(&mut previous, 0.0);
		for frame in 1..=60 {
			let mut current = HumanoidV0Rig::for_clip_test();
			let effects = apply_leap(&mut current, frame as f32 / 60.0);
			assert!(effects.is_identity());
			max_60 = max_60.max(max_rotation_delta(&current, &previous));
			previous = current;
		}
		let mut max_240 = 0.0_f32;
		let mut previous = HumanoidV0Rig::for_clip_test();
		apply_leap(&mut previous, 0.0);
		for frame in 1..=240 {
			let mut current = HumanoidV0Rig::for_clip_test();
			apply_leap(&mut current, frame as f32 / 240.0);
			max_240 = max_240.max(max_rotation_delta(&current, &previous));
			previous = current;
		}
		assert!(max_60.is_finite() && max_60 > 0.02);
		assert!(max_240 < max_60 * 0.35, "240 Hz {max_240} vs 60 Hz {max_60}");
		for progress in [TAKEOFF_END, AIR_END] {
			let mut before = HumanoidV0Rig::for_clip_test();
			let mut after = HumanoidV0Rig::for_clip_test();
			apply_leap(&mut before, progress - 1e-4);
			apply_leap(&mut after, progress + 1e-4);
			assert!(max_rotation_delta(&before, &after) < 0.01, "phase edge at {progress}");
		}
		let land_tail_start = AIR_END + (1.0 - AIR_END) * (1.0 - LEAP_RUN_RECOVERY_BLEND_FRACTION);
		let mut before = HumanoidV0Rig::for_clip_test();
		let mut after = HumanoidV0Rig::for_clip_test();
		apply_leap(&mut before, land_tail_start - 1e-4);
		apply_leap(&mut after, land_tail_start + 1e-4);
		assert!(max_rotation_delta(&before, &after) < 0.01, "run blend entry");
		let rest = HumanoidV0Rig::for_clip_test();
		let mut mid = HumanoidV0Rig::for_clip_test();
		apply_leap(&mut mid, 0.5);
		for name in ["lower_neck", "upper_neck"] {
			assert!(
				mid.rotation(name).normalize().dot(rest.rotation(name).normalize()).abs()
					> 1.0 - 1e-6
			);
		}
		Ok(())
	}
}
