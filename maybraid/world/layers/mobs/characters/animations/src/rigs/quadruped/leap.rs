use character_rigs::authoring::QuadrupedPose;
use character_rigs::rigs::quadruped_v0::QuadrupedV0Rig;
use character_rigs::Side;

use crate::animations::{
	smoothstep, QuadrupedLeap, AIR_END, LEAP_LAND_BLEND_FRACTION, TAKEOFF_END,
};
use crate::rigs::quadruped::apply::{apply_front_leg, apply_hind_leg, apply_neck, apply_spine};
use crate::{Animation, Progress};

const PAIR_STAGGER: f32 = 0.08;

#[derive(Clone, Copy)]
struct QuadLeapPose {
	hind_thigh: f32,
	front_thigh: f32,
	hind_shin: f32,
	front_shin: f32,
	spine: f32,
}

impl Animation<QuadrupedV0Rig> for QuadrupedLeap {
	fn apply_for(&self, rig: &mut QuadrupedV0Rig, progress: f32) {
		let mut pose = QuadrupedPose::default();
		sample_leap(self, progress, &mut pose);
		rig.write_pose(&pose);
	}
}

fn sample_leap(leap: &QuadrupedLeap, progress: f32, pose: &mut QuadrupedPose) {
	let t = Progress(progress).clamp();
	let authored = leap.pose_at(t);
	for side in [Side::Left, Side::Right] {
		let stagger = match side {
			Side::Left => 0.0,
			Side::Right => PAIR_STAGGER,
		};
		let delayed = Progress((t - stagger).max(0.0)).clamp();
		let side_pose = if (delayed - t).abs() < 1e-4 { authored } else { leap.pose_at(delayed) };
		apply_hind_leg(
			pose,
			side,
			side_pose.hind_thigh * 0.2,
			0.0,
			side_pose.hind_thigh,
			side_pose.hind_shin,
		);
		apply_front_leg(
			pose,
			side,
			side_pose.front_thigh * 0.2,
			0.0,
			side_pose.front_thigh,
			side_pose.front_shin,
		);
	}
	apply_spine(pose, authored.spine * 0.35, authored.spine);
	apply_neck(pose, -authored.spine * leap.neck_follow);
}

impl QuadrupedLeap {
	fn pose_at(&self, t: f32) -> QuadLeapPose {
		if t < TAKEOFF_END {
			self.takeoff(smoothstep(t / TAKEOFF_END))
		} else if t < AIR_END {
			self.air(smoothstep((t - TAKEOFF_END) / (AIR_END - TAKEOFF_END)))
		} else {
			let land_span = (1.0 - AIR_END).max(f32::EPSILON);
			let land_local = (t - AIR_END) / land_span;
			let land_u = smoothstep(land_local);
			let land_pose = self.land(land_u);
			let blend_end = LEAP_LAND_BLEND_FRACTION;
			if land_local < blend_end {
				let air_pose = self.air(1.0);
				let weight = smoothstep((land_local / blend_end).clamp(0.0, 1.0));
				blend_quad_leap_pose(air_pose, land_pose, weight)
			} else {
				land_pose
			}
		}
	}

	/// Hind push / front gather, then both extend off the ground.
	fn takeoff(&self, u: f32) -> QuadLeapPose {
		QuadLeapPose {
			hind_thigh: lerp(self.hind_push, self.hind_push * 0.25, u),
			front_thigh: lerp(-self.front_gather, -self.front_gather * 0.2, u),
			hind_shin: lerp(self.knee_extended, self.knee_extended * 0.4, u),
			front_shin: lerp(self.knee_air() * 0.7, self.knee_extended, u),
			spine: lerp(self.spine_gather * 0.4, self.spine_gather, u),
		}
	}

	/// Bound gather peaks mid-air; legs reach down toward the end.
	fn air(&self, u: f32) -> QuadLeapPose {
		let gather = (u * std::f32::consts::PI).sin();
		let tuck = lerp(0.15, self.air_tuck, gather);
		QuadLeapPose {
			hind_thigh: -tuck * 0.35,
			front_thigh: -tuck,
			hind_shin: lerp(self.knee_extended, self.knee_air(), gather),
			front_shin: lerp(self.knee_extended, self.knee_air(), gather),
			spine: lerp(self.spine_gather * 0.5, self.spine_gather, gather),
		}
	}

	/// Front pair plants first, then the hind pair; shallow absorb.
	fn land(&self, u: f32) -> QuadLeapPose {
		let front_u = (u * 1.35).min(1.0);
		let hind_u = ((u - 0.25) / 0.75).clamp(0.0, 1.0);
		let front_absorb = (front_u * std::f32::consts::PI).sin();
		let hind_absorb = (hind_u * std::f32::consts::PI).sin();
		QuadLeapPose {
			hind_thigh: -self.air_tuck * 0.15 * (1.0 - hind_u)
				- self.land_compress * 0.2 * hind_absorb,
			front_thigh: -self.land_compress * 0.25 * front_absorb,
			hind_shin: self.land_compress * hind_absorb,
			front_shin: self.land_compress * front_absorb,
			spine: self.spine_gather * (1.0 - u) - self.spine_gather * 0.35 * front_absorb,
		}
	}
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
	a + (b - a) * t
}

fn blend_quad_leap_pose(from: QuadLeapPose, to: QuadLeapPose, weight: f32) -> QuadLeapPose {
	let w = weight.clamp(0.0, 1.0);
	QuadLeapPose {
		hind_thigh: lerp(from.hind_thigh, to.hind_thigh, w),
		front_thigh: lerp(from.front_thigh, to.front_thigh, w),
		hind_shin: lerp(from.hind_shin, to.hind_shin, w),
		front_shin: lerp(from.front_shin, to.front_shin, w),
		spine: lerp(from.spine, to.spine, w),
	}
}

#[cfg(test)]
fn quad_leap_pose_without_land_blend(leap: &QuadrupedLeap, t: f32) -> QuadLeapPose {
	if t < TAKEOFF_END {
		leap.takeoff(smoothstep(t / TAKEOFF_END))
	} else if t < AIR_END {
		leap.air(smoothstep((t - TAKEOFF_END) / (AIR_END - TAKEOFF_END)))
	} else {
		leap.land(smoothstep((t - AIR_END) / (1.0 - AIR_END).max(f32::EPSILON)))
	}
}

#[cfg(test)]
fn apply_leap_without_land_blend(leap: &QuadrupedLeap, rig: &mut QuadrupedV0Rig, progress: f32) {
	let mut pose = QuadrupedPose::default();
	let t = Progress(progress).clamp();
	let authored = quad_leap_pose_without_land_blend(leap, t);
	for side in [Side::Left, Side::Right] {
		let stagger = match side {
			Side::Left => 0.0,
			Side::Right => PAIR_STAGGER,
		};
		let delayed = Progress((t - stagger).max(0.0)).clamp();
		let side_pose = if (delayed - t).abs() < 1e-4 {
			authored
		} else {
			quad_leap_pose_without_land_blend(leap, delayed)
		};
		apply_hind_leg(
			&mut pose,
			side,
			side_pose.hind_thigh * 0.2,
			0.0,
			side_pose.hind_thigh,
			side_pose.hind_shin,
		);
		apply_front_leg(
			&mut pose,
			side,
			side_pose.front_thigh * 0.2,
			0.0,
			side_pose.front_thigh,
			side_pose.front_shin,
		);
	}
	apply_spine(&mut pose, authored.spine * 0.35, authored.spine);
	apply_neck(&mut pose, -authored.spine * leap.neck_follow);
	rig.write_pose(&pose);
}

#[cfg(test)]
mod tests {
	use bevy::prelude::*;

	use crate::animations::Leap;

	use super::*;

	fn apply_leap(rig: &mut QuadrupedV0Rig, progress: f32) -> crate::Effects {
		QuadrupedLeap::from_leap(&Leap::default()).apply(rig, progress)
	}

	fn tip(rig: &QuadrupedV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	#[test]
	fn leap_delegates_to_quadruped_default() {
		for &phase in &[0.0, 0.1, 0.45, 0.85] {
			let mut from_leap = QuadrupedV0Rig::imported();
			let mut from_template = QuadrupedV0Rig::imported();
			apply_leap(&mut from_leap, phase);
			QuadrupedLeap::default().apply(&mut from_template, phase);
			for name in from_leap.animation_bone_names() {
				let leap_rot = from_leap.rotation(name);
				let template = from_template.rotation(name);
				assert!(
					leap_rot.dot(template).abs() > 1.0 - 1e-5,
					"rotation mismatch on {name} at {phase}"
				);
			}
		}
	}

	#[test]
	fn takeoff_hind_pushes_while_front_gathers() {
		let rest = QuadrupedV0Rig::imported();
		let mut rig = QuadrupedV0Rig::imported();
		apply_leap(&mut rig, 0.0);
		assert!(
			rig.rotation("posterior_thigh.L").dot(rest.rotation("posterior_thigh.L")).abs() < 0.95,
			"hind thigh should leave rest"
		);
		assert!(
			rig.rotation("anterior_shin.L").dot(rig.rotation("posterior_shin.L")).abs() < 0.999,
			"front and hind hinges should differ at takeoff"
		);
	}

	#[test]
	fn air_gathers_the_spine() {
		let mut rig = QuadrupedV0Rig::imported();
		apply_leap(&mut rig, 0.45);
		let lumbar = tip(&rig, "lumbar");
		assert!(lumbar.x < -0.04, "gather bends laterally, got {lumbar:?}");
		assert!(lumbar.z.abs() < 1e-3, "gather stays out of the sagittal plane, got {lumbar:?}");
	}

	#[test]
	fn land_front_compresses_before_hind() {
		let mut early = QuadrupedV0Rig::imported();
		apply_leap(&mut early, 0.78);
		let mut late = QuadrupedV0Rig::imported();
		apply_leap(&mut late, 0.92);
		assert!(
			early.rotation("anterior_shin.L").dot(early.rotation("posterior_shin.L")).abs() < 0.999,
			"front hinge leads at first contact"
		);
		assert!(
			late.rotation("posterior_shin.L").dot(early.rotation("posterior_shin.L")).abs() < 0.999,
			"hind hinge should change as the land continues"
		);
	}

	#[test]
	fn leap_has_no_root_motion() {
		let mut rig = QuadrupedV0Rig::imported();
		let effects = apply_leap(&mut rig, 0.45);
		assert!(effects.is_identity());
	}

	#[test]
	fn progress_clamps_past_one() {
		let mut a = QuadrupedV0Rig::imported();
		let mut b = QuadrupedV0Rig::imported();
		apply_leap(&mut a, 1.0);
		apply_leap(&mut b, 1.7);
		let left = a.rotation("posterior_thigh.L");
		let right = b.rotation("posterior_thigh.L");
		assert!(left.dot(right).abs() > 1.0 - 1e-5);
	}

	/// Matches [`player::body::JUMP_LAND_DURATION`]: land maps `AIR_END..1` over this many seconds.
	const JUMP_LAND_DURATION: f32 = 0.24;
	const LAND_PROGRESS_STEP_60FPS: f32 = (1.0 / 60.0) / JUMP_LAND_DURATION * (1.0 - AIR_END);
	const TOUCHDOWN_MAX_TIP_JUMP: f32 = 0.04;

	const FOOT_SHINS: [&str; 4] =
		["anterior_shin.L", "posterior_shin.L", "anterior_shin.R", "posterior_shin.R"];

	fn max_posed_tip_jump(before: &QuadrupedV0Rig, after: &QuadrupedV0Rig) -> (f32, &'static str) {
		let mut worst = 0.0f32;
		let mut worst_name = "";
		for name in before.animation_bone_names() {
			let delta = (before.posed_tip(name) - after.posed_tip(name)).length();
			if delta > worst {
				worst = delta;
				worst_name = name;
			}
		}
		(worst, worst_name)
	}

	fn pair_touchdown_progress(side: Side) -> f32 {
		match side {
			Side::Left => AIR_END,
			Side::Right => AIR_END + PAIR_STAGGER,
		}
	}

	fn assert_touchdown_tip_continuity(leap: &QuadrupedLeap, touchdown: f32, label: &str) {
		let eps = 1e-4f32;
		let mut before = QuadrupedV0Rig::for_clip_test();
		let mut after = QuadrupedV0Rig::for_clip_test();
		leap.apply(&mut before, touchdown - eps);
		leap.apply(&mut after, touchdown + eps);
		let (jump, bone) = max_posed_tip_jump(&before, &after);
		assert!(
			jump < TOUCHDOWN_MAX_TIP_JUMP,
			"{label} posed tip should stay continuous at touchdown (worst {bone} Δ={jump})"
		);
	}

	#[test]
	fn air_land_touchdown_max_posed_tip_jump_stays_bounded_at_left_pair() {
		let leap = QuadrupedLeap::from_leap(&Leap::default());
		assert_touchdown_tip_continuity(&leap, pair_touchdown_progress(Side::Left), "left pair");
	}

	#[test]
	fn air_land_touchdown_max_posed_tip_jump_stays_bounded_at_right_pair() {
		let leap = QuadrupedLeap::from_leap(&Leap::default());
		assert_touchdown_tip_continuity(&leap, pair_touchdown_progress(Side::Right), "right pair");
	}

	#[test]
	fn air_land_touchdown_pops_without_blend_band_at_left_pair() {
		let leap = QuadrupedLeap::from_leap(&Leap::default());
		let eps = 1e-4f32;
		let touchdown = pair_touchdown_progress(Side::Left);
		let mut before = QuadrupedV0Rig::for_clip_test();
		let mut after = QuadrupedV0Rig::for_clip_test();
		apply_leap_without_land_blend(&leap, &mut before, touchdown - eps);
		apply_leap_without_land_blend(&leap, &mut after, touchdown + eps);
		let (jump, bone) = max_posed_tip_jump(&before, &after);
		assert!(
			jump > TOUCHDOWN_MAX_TIP_JUMP,
			"pre-blend branch should pop at left touchdown (worst {bone} Δ={jump})"
		);
	}

	#[test]
	fn air_land_touchdown_pops_without_blend_band_at_right_pair() {
		let leap = QuadrupedLeap::from_leap(&Leap::default());
		let eps = 1e-4f32;
		let touchdown = pair_touchdown_progress(Side::Right);
		let mut before = QuadrupedV0Rig::for_clip_test();
		let mut after = QuadrupedV0Rig::for_clip_test();
		apply_leap_without_land_blend(&leap, &mut before, touchdown - eps);
		apply_leap_without_land_blend(&leap, &mut after, touchdown + eps);
		let (jump, bone) = max_posed_tip_jump(&before, &after);
		assert!(
			jump > TOUCHDOWN_MAX_TIP_JUMP,
			"pre-blend branch should pop at right touchdown (worst {bone} Δ={jump})"
		);
	}

	#[test]
	fn air_land_touchdown_frame_step_stays_bounded_at_left_pair() {
		let leap = QuadrupedLeap::from_leap(&Leap::default());
		let half = LAND_PROGRESS_STEP_60FPS * 0.5;
		let touchdown = pair_touchdown_progress(Side::Left);
		let mut before = QuadrupedV0Rig::for_clip_test();
		let mut after = QuadrupedV0Rig::for_clip_test();
		leap.apply(&mut before, touchdown - half);
		leap.apply(&mut after, touchdown + half);
		let (jump, bone) = max_posed_tip_jump(&before, &after);
		assert!(
			jump < TOUCHDOWN_MAX_TIP_JUMP * 2.5,
			"60 Hz land step across left touchdown should not spike (worst {bone} Δ={jump})"
		);
	}

	#[test]
	#[ignore = "manual posed-tip metrics for reviews"]
	fn touchdown_posed_tip_jump_metrics() {
		let leap = QuadrupedLeap::from_leap(&Leap::default());
		let eps = 1e-4f32;
		for (label, touchdown) in [
			("left_pair", pair_touchdown_progress(Side::Left)),
			("right_pair", pair_touchdown_progress(Side::Right)),
		] {
			let mut blended_before = QuadrupedV0Rig::for_clip_test();
			let mut blended_after = QuadrupedV0Rig::for_clip_test();
			leap.apply(&mut blended_before, touchdown - eps);
			leap.apply(&mut blended_after, touchdown + eps);
			let (blended_jump, blended_bone) = max_posed_tip_jump(&blended_before, &blended_after);

			let mut raw_before = QuadrupedV0Rig::for_clip_test();
			let mut raw_after = QuadrupedV0Rig::for_clip_test();
			apply_leap_without_land_blend(&leap, &mut raw_before, touchdown - eps);
			apply_leap_without_land_blend(&leap, &mut raw_after, touchdown + eps);
			let (raw_jump, raw_bone) = max_posed_tip_jump(&raw_before, &raw_after);

			eprintln!(
				"{label} touchdown={touchdown}: blended worst {blended_bone} Δ={blended_jump}, raw worst {raw_bone} Δ={raw_jump}"
			);
		}

		let global = AIR_END;
		for &progress in &[global - eps, global + eps, global + PAIR_STAGGER] {
			let mut rig = QuadrupedV0Rig::for_clip_test();
			leap.apply(&mut rig, progress);
			eprintln!("progress {progress}:");
			for name in FOOT_SHINS {
				eprintln!("  {name} tip {:?}", rig.posed_tip(name));
			}
			eprintln!("  neck tip {:?}", rig.posed_tip("neck"));
		}

		let mut before = QuadrupedV0Rig::for_clip_test();
		let mut after = QuadrupedV0Rig::for_clip_test();
		leap.apply(&mut before, AIR_END - eps);
		leap.apply(&mut after, AIR_END + eps);
		let (jump, bone) = max_posed_tip_jump(&before, &after);
		eprintln!("global AIR_END±eps max posed tip jump: {bone} Δ={jump}");
	}

	#[test]
	fn resampling_does_not_accumulate() {
		let leap = QuadrupedLeap::from_leap(&Leap::default());
		let mut once = QuadrupedV0Rig::for_clip_test();
		let mut twice = QuadrupedV0Rig::for_clip_test();
		leap.apply(&mut once, 0.55);
		leap.apply(&mut twice, 0.55);
		leap.apply(&mut twice, 0.55);
		for name in once.animation_bone_names() {
			let a = once.posed_tip(name);
			let b = twice.posed_tip(name);
			assert!((a - b).length() < 1e-5, "re-sample should not drift on {name}");
		}
	}

	#[test]
	fn land_blend_band_end_has_no_tip_velocity_spike() {
		let leap = QuadrupedLeap::from_leap(&Leap::default());
		let land_span = 1.0 - AIR_END;
		let band_end = AIR_END + land_span * LEAP_LAND_BLEND_FRACTION;
		let step = land_span * 0.01;
		let mut before = QuadrupedV0Rig::for_clip_test();
		let mut after = QuadrupedV0Rig::for_clip_test();
		leap.apply(&mut before, band_end - step);
		leap.apply(&mut after, band_end + step);
		let (jump, bone) = max_posed_tip_jump(&before, &after);
		assert!(
			jump < TOUCHDOWN_MAX_TIP_JUMP * 2.0,
			"blend band end should not spike posed tips (worst {bone} Δ={jump})"
		);
	}

	#[test]
	fn land_blend_band_interpolates_front_compress() {
		let leap = QuadrupedLeap::from_leap(&Leap::default());
		let land_span = 1.0 - AIR_END;
		let blend_mid = AIR_END + land_span * LEAP_LAND_BLEND_FRACTION * 0.5;
		let mut blended = QuadrupedV0Rig::for_clip_test();
		leap.apply(&mut blended, blend_mid);
		let mut air_end = QuadrupedV0Rig::for_clip_test();
		leap.apply(&mut air_end, AIR_END - 1e-4);
		let mut land_only = QuadrupedV0Rig::for_clip_test();
		apply_leap_without_land_blend(&leap, &mut land_only, blend_mid);
		let air_shin = air_end.posed_angle("anterior_shin.L");
		let blend_shin = blended.posed_angle("anterior_shin.L");
		let land_shin = land_only.posed_angle("anterior_shin.L");
		assert!(
			blend_shin < air_shin - 0.01 && blend_shin > land_shin - 0.02,
			"mid-band front compress between air end ({air_shin}) and land ({land_shin}), got {blend_shin}"
		);
	}

	#[test]
	fn land_blend_end_matches_pure_land_outside_band() {
		let leap = QuadrupedLeap::from_leap(&Leap::default());
		let outside = 1.0;
		let mut blended = QuadrupedV0Rig::for_clip_test();
		let mut pure = QuadrupedV0Rig::for_clip_test();
		leap.apply(&mut blended, outside);
		apply_leap_without_land_blend(&leap, &mut pure, outside);
		for name in blended.animation_bone_names() {
			let a = blended.rotation(name);
			let b = pure.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-4, "outside band matches pure land on {name}");
		}
	}
}
