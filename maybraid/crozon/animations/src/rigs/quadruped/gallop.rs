use crozon_rigs::{quadruped::QuadrupedRig, Side};

use crate::animations::{Gallop, QuadrupedGallop};
use crate::rigs::quadruped::apply::{apply_front_leg, apply_hind_leg, apply_neck, apply_spine};
use crate::{Animation, Effects, Progress};

impl<R: QuadrupedRig> Animation<R> for Gallop {
	fn apply(&self, rig: &mut R, progress: f32) -> Effects {
		QuadrupedGallop::from_gallop(self).apply(rig, progress)
	}
}

impl<R: QuadrupedRig> Animation<R> for QuadrupedGallop<R> {
	fn apply(&self, rig: &mut R, progress: f32) -> Effects {
		let phase = Progress(progress).cycle();
		let gallop = self;

		// Transverse gallop footfall order: FL, HR, FR, HL.
		apply_front_leg_at_phase(rig, Side::Left, phase, 0.0, gallop);
		apply_hind_leg_at_phase(rig, Side::Right, phase, 0.25, gallop);
		apply_front_leg_at_phase(rig, Side::Right, phase, 0.5, gallop);
		apply_hind_leg_at_phase(rig, Side::Left, phase, 0.75, gallop);

		let spine_swing = thigh_swing(phase) * gallop.spine_swing;
		apply_spine(rig, spine_swing, -spine_swing * 0.5);
		apply_neck(rig, -spine_swing * gallop.neck_swing / gallop.spine_swing.max(1e-4));

		Effects::default()
	}
}

fn apply_front_leg_at_phase<R: QuadrupedRig>(
	rig: &mut R,
	side: Side,
	base_phase: f32,
	offset: f32,
	gallop: &QuadrupedGallop<R>,
) {
	let leg_phase = base_phase + offset;
	let swing = thigh_swing(leg_phase);
	let lift_sign = if side == Side::Left { -1.0 } else { 1.0 };

	apply_front_leg(
		rig,
		side,
		swing * gallop.shoulder_swing,
		shoulder_lift(swing, gallop.shoulder_lift) * lift_sign,
		swing * gallop.stride,
		knee_flex(leg_phase, gallop) - gallop.knee_extended,
	);
}

fn apply_hind_leg_at_phase<R: QuadrupedRig>(
	rig: &mut R,
	side: Side,
	base_phase: f32,
	offset: f32,
	gallop: &QuadrupedGallop<R>,
) {
	let leg_phase = base_phase + offset;
	let swing = thigh_swing(leg_phase);
	let lift_sign = if side == Side::Left { -1.0 } else { 1.0 };

	apply_hind_leg(
		rig,
		side,
		swing * gallop.hip_swing,
		hip_lift(swing, gallop.hip_lift) * lift_sign,
		swing * gallop.stride,
		knee_flex(leg_phase, gallop) - gallop.knee_extended,
	);
}

fn thigh_swing(phase: f32) -> f32 {
	let p = phase.fract();
	if p < 0.5 {
		4.0 * p - 1.0
	} else {
		3.0 - 4.0 * p
	}
}

fn shoulder_lift(leg_swing: f32, amplitude: f32) -> f32 {
	leg_swing * amplitude
}

fn hip_lift(leg_swing: f32, amplitude: f32) -> f32 {
	leg_swing * amplitude
}

fn knee_flex<Rig>(leg_phase: f32, gallop: &QuadrupedGallop<Rig>) -> f32 {
	let p = leg_phase.fract();
	let peak = if p < 0.5 { gallop.knee_extended } else { gallop.knee_contracted };
	let t = if p < 0.5 { p * 2.0 } else { (p - 0.5) * 2.0 };
	gallop.knee_neutral + (t * std::f32::consts::PI).sin() * (peak - gallop.knee_neutral)
}

#[cfg(test)]
mod tests {
	use crozon_rigs::{rigs::quadruped_v0::QuadrupedV0Rig, Side};

	use super::*;

	fn assert_pose_matches_at_phases(phases: &[f32]) {
		for &phase in phases {
			let mut from_gallop = QuadrupedV0Rig::imported();
			let mut from_quadruped = QuadrupedV0Rig::imported();
			Gallop::default().apply(&mut from_gallop, phase);
			QuadrupedGallop::<QuadrupedV0Rig>::default().apply(&mut from_quadruped, phase);

			for bone in from_gallop.animation_bones() {
				let Some(gallop_pose) = from_gallop.pose().get(&bone) else {
					continue;
				};
				let quadruped_pose = from_quadruped.pose().get(&bone).expect("quadruped pose");
				assert!(
					(gallop_pose.swing - quadruped_pose.swing).abs() < 1e-5,
					"swing mismatch on {bone} at {phase}"
				);
				assert!(
					(gallop_pose.flex - quadruped_pose.flex).abs() < 1e-5,
					"flex mismatch on {bone} at {phase}"
				);
			}
		}
	}

	#[test]
	fn gallop_delegates_to_quadruped_default() {
		assert_pose_matches_at_phases(&[0.0, 0.25, 0.5, 0.75]);
	}

	#[test]
	fn gallop_writes_swing_flex_for_left_front_thigh() {
		let mut rig = QuadrupedV0Rig::imported();
		QuadrupedGallop::<QuadrupedV0Rig>::default().apply(&mut rig, 0.0);

		let thigh = rig
			.pose()
			.get(&rig.front_leg(Side::Left).thigh.name)
			.expect("front thigh pose");
		assert!(thigh.swing.abs() > 0.0);
	}

	#[test]
	fn gallop_offsets_legs_across_stride() {
		let mut rig = QuadrupedV0Rig::imported();
		QuadrupedGallop::<QuadrupedV0Rig>::default().apply(&mut rig, 0.0);

		let front_left = rig
			.pose()
			.get(&rig.front_leg(Side::Left).thigh.name)
			.expect("front left thigh");
		let hind_right = rig
			.pose()
			.get(&rig.hind_leg(Side::Right).thigh.name)
			.expect("hind right thigh");
		assert_ne!(front_left.swing, hind_right.swing);
	}
}
