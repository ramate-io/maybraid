use crozon_rigs::{quadruped::QuadrupedRig, Side};

use crate::animations::{smoothstep, QuadrupedLeap, AIR_END, TAKEOFF_END};
use crate::rigs::quadruped::apply::{apply_front_leg, apply_hind_leg, apply_neck, apply_spine};
use crate::{Animation, Effects, Progress};

const PAIR_STAGGER: f32 = 0.08;

#[derive(Clone, Copy)]
struct QuadLeapPose {
	hind_thigh: f32,
	front_thigh: f32,
	hind_shin: f32,
	front_shin: f32,
	spine: f32,
}

impl<R: QuadrupedRig> Animation<R> for QuadrupedLeap<R> {
	fn apply(&self, rig: &mut R, progress: f32) -> Effects {
		let t = Progress(progress).clamp();
		let pose = self.pose_at(t);
		for side in [Side::Left, Side::Right] {
			let stagger = match side {
				Side::Left => 0.0,
				Side::Right => PAIR_STAGGER,
			};
			let delayed = Progress((t - stagger).max(0.0)).clamp();
			let side_pose = if (delayed - t).abs() < 1e-4 {
				pose
			} else {
				self.pose_at(delayed)
			};
			apply_hind_leg(
				rig,
				side,
				side_pose.hind_thigh * 0.2,
				0.0,
				side_pose.hind_thigh,
				side_pose.hind_shin,
			);
			apply_front_leg(
				rig,
				side,
				side_pose.front_thigh * 0.2,
				0.0,
				side_pose.front_thigh,
				side_pose.front_shin,
			);
		}
		apply_spine(rig, pose.spine * 0.35, pose.spine);
		apply_neck(rig, -pose.spine * self.neck_follow);
		Effects::default()
	}
}

impl<Rig> QuadrupedLeap<Rig> {
	fn pose_at(&self, t: f32) -> QuadLeapPose {
		if t < TAKEOFF_END {
			self.takeoff(smoothstep(t / TAKEOFF_END))
		} else if t < AIR_END {
			self.air(smoothstep((t - TAKEOFF_END) / (AIR_END - TAKEOFF_END)))
		} else {
			self.land(smoothstep((t - AIR_END) / (1.0 - AIR_END).max(f32::EPSILON)))
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
			hind_thigh: -self.air_tuck * 0.15 * (1.0 - hind_u) - self.land_compress * 0.2 * hind_absorb,
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

#[cfg(test)]
mod tests {
	use crozon_rigs::{rigs::quadruped_v0::QuadrupedV0Rig, Side};

	use crate::animations::Leap;

	use super::*;

	fn apply_leap(rig: &mut QuadrupedV0Rig, progress: f32) -> Effects {
		QuadrupedLeap::from_leap(&Leap::default()).apply(rig, progress)
	}

	fn hind_thigh(rig: &QuadrupedV0Rig, side: Side) -> f32 {
		rig.pose().get(&rig.hind_leg(side).thigh.name).expect("hind thigh").swing
	}

	fn front_thigh(rig: &QuadrupedV0Rig, side: Side) -> f32 {
		rig.pose().get(&rig.front_leg(side).thigh.name).expect("front thigh").swing
	}

	fn hind_shin(rig: &QuadrupedV0Rig, side: Side) -> f32 {
		rig.pose().get(&rig.hind_leg(side).shin.name).expect("hind shin").flex
	}

	fn front_shin(rig: &QuadrupedV0Rig, side: Side) -> f32 {
		rig.pose().get(&rig.front_leg(side).shin.name).expect("front shin").flex
	}

	fn lumbar(rig: &QuadrupedV0Rig) -> f32 {
		rig.pose().get(&rig.spine().lumbar.name).expect("lumbar").flex
	}

	#[test]
	fn leap_delegates_to_quadruped_default() {
		for &phase in &[0.0, 0.1, 0.45, 0.85] {
			let mut from_leap = QuadrupedV0Rig::imported();
			let mut from_template = QuadrupedV0Rig::imported();
			apply_leap(&mut from_leap, phase);
			QuadrupedLeap::<QuadrupedV0Rig>::default().apply(&mut from_template, phase);
			for bone in from_leap.animation_bones() {
				let Some(leap_pose) = from_leap.pose().get(&bone) else {
					continue;
				};
				let template = from_template.pose().get(&bone).expect("template pose");
				assert!(
					(leap_pose.swing - template.swing).abs() < 1e-5,
					"swing mismatch on {bone} at {phase}"
				);
				assert!(
					(leap_pose.flex - template.flex).abs() < 1e-5,
					"flex mismatch on {bone} at {phase}"
				);
			}
		}
	}

	#[test]
	fn takeoff_hind_pushes_while_front_gathers() {
		let mut rig = QuadrupedV0Rig::imported();
		apply_leap(&mut rig, 0.0);
		assert!(hind_thigh(&rig, Side::Left) > 0.3, "hind should push back");
		assert!(front_thigh(&rig, Side::Left) < -0.2, "front should gather");
		assert!(front_shin(&rig, Side::Left) > hind_shin(&rig, Side::Left));
	}

	#[test]
	fn air_gathers_the_spine() {
		let mut rig = QuadrupedV0Rig::imported();
		apply_leap(&mut rig, 0.45);
		assert!(lumbar(&rig) > 0.05);
	}

	#[test]
	fn land_front_compresses_before_hind() {
		let mut early = QuadrupedV0Rig::imported();
		apply_leap(&mut early, 0.78);
		let mut late = QuadrupedV0Rig::imported();
		apply_leap(&mut late, 0.92);
		assert!(front_shin(&early, Side::Left) > hind_shin(&early, Side::Left));
		assert!(hind_shin(&late, Side::Left) > hind_shin(&early, Side::Left));
	}

	#[test]
	fn leap_has_no_root_motion() {
		let mut rig = QuadrupedV0Rig::imported();
		let effects = apply_leap(&mut rig, 0.45);
		assert!(effects.r#move.is_none());
	}

	#[test]
	fn progress_clamps_past_one() {
		let mut a = QuadrupedV0Rig::imported();
		let mut b = QuadrupedV0Rig::imported();
		apply_leap(&mut a, 1.0);
		apply_leap(&mut b, 1.7);
		assert_eq!(hind_thigh(&a, Side::Left), hind_thigh(&b, Side::Left));
	}
}
