use character_rigs::authoring::QuadrupedPose;
use character_rigs::rigs::quadruped_v0::QuadrupedV0Rig;
use character_rigs::Side;

use crate::animations::{
	LeapPhase, LeapTimeline, LinearBlend, QuadrupedLeap, LEAP_LAND_BLEND_FRACTION,
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
		match LeapTimeline::sample(t) {
			(LeapPhase::Takeoff, u) => self.takeoff(u),
			(LeapPhase::Air, u) => self.air(u),
			(LeapPhase::Land, u) => {
				let land_span = (1.0 - crate::animations::AIR_END).max(f32::EPSILON);
				let land_local = (t - crate::animations::AIR_END) / land_span;
				let land = self.land(u);
				if land_local < LEAP_LAND_BLEND_FRACTION {
					let weight = crate::animations::smoothstep(
						(land_local / LEAP_LAND_BLEND_FRACTION.max(f32::EPSILON)).clamp(0.0, 1.0),
					);
					blend_quad_leap_pose(self.air(1.0), land, weight)
				} else {
					land
				}
			}
		}
	}

	/// Hind push / front gather, then both extend off the ground.
	fn takeoff(&self, u: f32) -> QuadLeapPose {
		QuadLeapPose {
			hind_thigh: LinearBlend::eval(self.hind_push, self.hind_push * 0.25, u),
			front_thigh: LinearBlend::eval(-self.front_gather, -self.front_gather * 0.2, u),
			hind_shin: LinearBlend::eval(self.knee_extended, self.knee_extended * 0.4, u),
			front_shin: LinearBlend::eval(self.knee_air() * 0.7, self.knee_extended, u),
			spine: LinearBlend::eval(self.spine_gather * 0.4, self.spine_gather, u),
		}
	}

	/// Bound gather peaks mid-air; legs reach down toward the end.
	fn air(&self, u: f32) -> QuadLeapPose {
		let gather = (u * std::f32::consts::PI).sin();
		let tuck = LinearBlend::eval(0.15, self.air_tuck, gather);
		QuadLeapPose {
			hind_thigh: -tuck * 0.35,
			front_thigh: -tuck,
			hind_shin: LinearBlend::eval(self.knee_extended, self.knee_air(), gather),
			front_shin: LinearBlend::eval(self.knee_extended, self.knee_air(), gather),
			spine: LinearBlend::eval(self.spine_gather * 0.5, self.spine_gather, gather),
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

fn blend_quad_leap_pose(from: QuadLeapPose, to: QuadLeapPose, weight: f32) -> QuadLeapPose {
	let t = weight.clamp(0.0, 1.0);
	let blend = |a: f32, b: f32| a + (b - a) * t;
	QuadLeapPose {
		hind_thigh: blend(from.hind_thigh, to.hind_thigh),
		front_thigh: blend(from.front_thigh, to.front_thigh),
		hind_shin: blend(from.hind_shin, to.hind_shin),
		front_shin: blend(from.front_shin, to.front_shin),
		spine: blend(from.spine, to.spine),
	}
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

	#[test]
	fn air_to_land_window_meets_both_composed_endpoints() -> anyhow::Result<()> {
		let leap = QuadrupedLeap::default();
		let land_span = 1.0 - crate::animations::AIR_END;
		let band_end = crate::animations::AIR_END + land_span * LEAP_LAND_BLEND_FRACTION;
		let at_land = leap.pose_at(crate::animations::AIR_END);
		let air_end = leap.air(1.0);
		assert!((at_land.hind_thigh - air_end.hind_thigh).abs() < 1e-6);
		assert!((at_land.front_shin - air_end.front_shin).abs() < 1e-6);
		let at_band_end = leap.pose_at(band_end);
		let land_end = leap.land(crate::animations::smoothstep(LEAP_LAND_BLEND_FRACTION));
		assert!((at_band_end.hind_thigh - land_end.hind_thigh).abs() < 1e-6);
		assert!((at_band_end.front_shin - land_end.front_shin).abs() < 1e-6);
		Ok(())
	}

	#[test]
	fn air_land_seam_is_fine_sample_continuous_without_root_motion() -> anyhow::Result<()> {
		let epsilon = 1e-4;
		let mut before = QuadrupedV0Rig::for_clip_test();
		let mut after = QuadrupedV0Rig::for_clip_test();
		QuadrupedLeap::default().apply(&mut before, crate::animations::AIR_END - epsilon);
		let effects =
			QuadrupedLeap::default().apply(&mut after, crate::animations::AIR_END + epsilon);
		assert!(effects.is_identity());
		for name in before.animation_bone_names() {
			assert!(
				before
					.rotation(name)
					.normalize()
					.angle_between(after.rotation(name).normalize())
					< 0.01,
				"air/land seam should stay continuous on {name}"
			);
		}
		Ok(())
	}
}
